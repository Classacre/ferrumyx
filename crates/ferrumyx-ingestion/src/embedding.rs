//! Embedding client — calls the configured embedding backend to produce
//! vectors for paper chunks, then writes them to the chunk's embedding field.
//!
//! Supports multiple backends:
//!   - OpenAI         (text-embedding-3-small / text-embedding-3-large)
//!   - Gemini         (text-embedding-004)
//!   - OpenAI-compat  (any /v1/embeddings endpoint — Groq, Together, etc.)
//!   - BiomedBERT     (local Docker service on :8002) [deprecated - use RustNative]
//!   - Ollama         (nomic-embed-text or any ollama embedding model)
//!   - RustNative     (pure Rust Candle BiomedBERT - no Python!) [recommended]
//!
//! The embed pipeline step runs after bulk_insert_chunks and writes back
//! to the chunk's embedding field in LanceDB.

use anyhow::{Context, Result};
use ferrumyx_common::sandbox::SandboxClient as Client;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

#[cfg(feature = "rust-embed")]
use ferrumyx_embed::{EmbeddingConfig as RustEmbedConfig};

use crate::repository::IngestionRepository;

// ── Backend config ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmbeddingConfig {
    pub backend:         EmbeddingBackend,
    pub api_key:         Option<String>,
    pub model:           String,
    pub dim:             usize,
    pub batch_size:      usize,
    pub base_url:        Option<String>,  // for compat/ollama/biomedbert
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EmbeddingBackend {
    OpenAi,
    Gemini,
    OpenAiCompatible,
    BiomedBert,      // Docker Python service (deprecated)
    Ollama,
    RustNative,      // Pure Rust Candle BiomedBERT (recommended)
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            backend:    EmbeddingBackend::RustNative,  // Default to pure Rust
            api_key:    None,
            // Microsoft BiomedBERT trained on PubMed abstracts + full-text articles
            // Better for biomedical literature than general PubMedBERT
            model:      "microsoft/BiomedNLP-BiomedBERT-base-uncased-abstract-fulltext".to_string(),
            dim:        768,
            batch_size: 32,
            base_url:   None,
        }
    }
}

// ── Embedding client ──────────────────────────────────────────────────────────

pub struct EmbeddingClient {
    cfg:    EmbeddingConfig,
    client: Client,
}

impl EmbeddingClient {
    pub fn new(cfg: EmbeddingConfig) -> Self {
        Self { cfg, client: Client::new().unwrap() }
    }

    /// Embed a batch of texts; returns `(N, dim)` f32 vectors.
    #[instrument(skip(self, texts), fields(n = texts.len(), backend = ?self.cfg.backend))]
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() { return Ok(vec![]); }
        match self.cfg.backend {
            EmbeddingBackend::OpenAi           => self.embed_openai(texts).await,
            EmbeddingBackend::Gemini           => self.embed_gemini(texts).await,
            EmbeddingBackend::OpenAiCompatible => self.embed_compat(texts).await,
            EmbeddingBackend::BiomedBert       => self.embed_biomedbert(texts).await,
            EmbeddingBackend::Ollama           => self.embed_ollama(texts).await,
            EmbeddingBackend::RustNative       => self.embed_rust_native(texts).await,
        }
    }

    // ── OpenAI ─────────────────────────────────────────────────────────────

    async fn embed_openai(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let key = self.cfg.api_key.as_deref().unwrap_or("");
        let body = serde_json::json!({
            "model": &self.cfg.model,
            "input": texts,
        });
        let resp: serde_json::Value = self.client
            .post("https://api.openai.com/v1/embeddings")?
            .bearer_auth(key)
            .json(&body)
            .send().await?
            .json().await?;
        parse_openai_embeddings(&resp)
    }

    // ── Gemini ─────────────────────────────────────────────────────────────

    async fn embed_gemini(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let key = self.cfg.api_key.as_deref().unwrap_or("");
        let requests: Vec<serde_json::Value> = texts.iter().map(|t| serde_json::json!({
            "model": format!("models/{}", self.cfg.model),
            "content": { "parts": [{"text": t}] }
        })).collect();
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:batchEmbedContents?key={}",
            self.cfg.model, key
        );
        let resp: serde_json::Value = self.client
            .post(&url)?
            .json(&serde_json::json!({"requests": requests}))
            .send().await?
            .json().await?;
        Ok(resp["embeddings"].as_array().unwrap_or(&vec![])
            .iter()
            .map(|e| e["values"].as_array().unwrap_or(&vec![])
                .iter().map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect())
            .collect())
    }

    // ── OpenAI-compatible ──────────────────────────────────────────────────

    async fn embed_compat(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let base = self.cfg.base_url.as_deref()
            .unwrap_or("http://localhost:11434").trim_end_matches('/');
        let url = format!("{}/v1/embeddings", base);
        let body = serde_json::json!({
            "model": &self.cfg.model,
            "input": texts,
        });
        let mut req = self.client.request(reqwest::Method::POST, &url)?.json(&body);
        if let Some(ref k) = self.cfg.api_key {
            req = req.bearer_auth(k);
        }
        let resp: serde_json::Value = req.send().await?.json().await?;
        parse_openai_embeddings(&resp)
    }

    // ── BiomedBERT Docker service ──────────────────────────────────────────

    async fn embed_biomedbert(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let base = self.cfg.base_url.as_deref().unwrap_or("http://localhost:8002");
        let url = format!("{}/embed", base);
        let body = serde_json::json!({
            "texts":     texts,
            "normalize": true,
        });
        let resp: serde_json::Value = self.client
            .post(&url)?
            .json(&body)
            .send().await
            .with_context(|| format!(
                "BiomedBERT service unreachable at {url}. \
                 Start it with: docker compose --profile embed up -d"
            ))?
            .json().await?;
        Ok(resp["embeddings"].as_array().unwrap_or(&vec![])
            .iter()
            .map(|row| row.as_array().unwrap_or(&vec![])
                .iter().map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect())
            .collect())
    }

    // ── Ollama ─────────────────────────────────────────────────────────────

    async fn embed_ollama(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let base = self.cfg.base_url.as_deref().unwrap_or("http://localhost:11434");
        let mut out = Vec::with_capacity(texts.len());
        for text in texts {
            let url = format!("{}/api/embeddings", base);
            let body = serde_json::json!({"model": &self.cfg.model, "prompt": text});
            let resp: serde_json::Value = self.client.request(reqwest::Method::POST, &url)?.json(&body).send().await?
                .json().await?;
            let vec: Vec<f32> = resp["embedding"].as_array().unwrap_or(&vec![])
                .iter().map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect();
            out.push(vec);
        }
        Ok(out)
    }

    // ── Rust Native (Candle BiomedBERT) ─────────────────────────────────────
    // Pure Rust inference - no Python, no Docker service needed!

    #[cfg(feature = "rust-embed")]
    async fn embed_rust_native(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        use std::sync::Arc;
        use tokio::sync::Mutex;
        use ferrumyx_embed::{BiomedBertEmbedder, EmbeddingConfig as RustEmbedConfig};
        
        // Get or initialize the embedder (lazy static for reuse)
        static EMBEDDER: std::sync::OnceLock<Arc<Mutex<BiomedBertEmbedder>>> = std::sync::OnceLock::new();
        
        let embedder = EMBEDDER.get_or_init(|| {
            // Create config from our embedding config
            let config = RustEmbedConfig {
                model_id: self.cfg.model.clone(),
                batch_size: self.cfg.batch_size,
                max_length: 512,
                normalize: true,
                pooling: ferrumyx_embed::PoolingStrategy::Mean,
                use_gpu: false, // CPU for now, can be configured
                cache_size: 1000,
                cache_dir: None,
            };
            
            // We need to block on initialization since OnceLock is sync
            let embedder = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    BiomedBertEmbedder::new(config).await
                        .expect("Failed to initialize Rust embedder")
                })
            });
            
            Arc::new(Mutex::new(embedder))
        });
        
        let embedder_guard = embedder.lock().await;
        embedder_guard.embed(texts).await
            .map_err(|e| anyhow::anyhow!("Rust embedding failed: {}", e))
    }

    #[cfg(not(feature = "rust-embed"))]
    async fn embed_rust_native(&self, _texts: &[String]) -> Result<Vec<Vec<f32>>> {
        Err(anyhow::anyhow!(
            "RustNative backend requires 'rust-embed' feature. \
             Enable it in Cargo.toml or use another backend."
        ))
    }
}

fn parse_openai_embeddings(resp: &serde_json::Value) -> Result<Vec<Vec<f32>>> {
    Ok(resp["data"].as_array().unwrap_or(&vec![])
        .iter()
        .map(|item| item["embedding"].as_array().unwrap_or(&vec![])
            .iter().map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect())
        .collect())
}

// ── Pipeline step: embed pending chunks ───────────────────────────────────────

/// Embed all paper_chunks that have no embedding yet, writing vectors back
/// to the chunk's embedding field in LanceDB.
///
/// Called as the final step in run_ingestion after bulk_insert_chunks.
#[instrument(skip(client, repo))]
pub async fn embed_pending_chunks(
    client: &EmbeddingClient,
    repo:   &IngestionRepository,
    paper_id: Uuid,
) -> Result<usize> {
    // Fetch chunks without embeddings for this paper
    let chunks = repo.find_chunks_without_embeddings(paper_id).await
        .context("fetch pending chunks failed")?;

    if chunks.is_empty() {
        debug!(paper_id = %paper_id, "No pending chunks to embed");
        return Ok(0);
    }

    info!(paper_id = %paper_id, n = chunks.len(), "Embedding chunks");

    let ids:   Vec<Uuid>   = chunks.iter().map(|(id, _)| *id).collect();
    let texts: Vec<String> = chunks.iter().map(|(_, t)| t.clone()).collect();

    let mut total_embedded = 0usize;

    // Process in batches
    for (batch_ids, batch_texts) in ids.chunks(client.cfg.batch_size)
        .zip(texts.chunks(client.cfg.batch_size))
    {
        let vecs = match client.embed_batch(batch_texts).await {
            Ok(v) => v,
            Err(e) => {
                warn!("Embedding batch failed: {e} — skipping batch");
                continue;
            }
        };

        // L2-normalize all vectors
        let normalized_vecs: Vec<Vec<f32>> = vecs.iter()
            .map(|vec| {
                let norm = l2_norm(vec);
                vec.iter().map(|x| x / norm).collect()
            })
            .collect();

        // Update each chunk with its embedding
        for (chunk_id, embedding) in batch_ids.iter().zip(normalized_vecs.iter()) {
            if let Err(e) = repo.update_chunk_embedding(*chunk_id, embedding.clone()).await {
                warn!("Failed to update embedding for chunk {}: {}", chunk_id, e);
                continue;
            }
            total_embedded += 1;
        }
        
        debug!("Updated {} embeddings", batch_ids.len());
    }

    info!(paper_id = %paper_id, total_embedded, "Embedding step complete");
    Ok(total_embedded)
}

fn l2_norm(v: &[f32]) -> f32 {
    let s: f32 = v.iter().map(|x| x * x).sum();
    s.sqrt().max(1e-10)
}

// ── Hybrid search (FTS + vector RRF) — IronClaw-aligned ──────────────────────
//
// IronClaw pattern (ironclaw/src/workspace/search.rs):
//   1. FTS:    SELECT ... ts_rank_cd(...) ORDER BY rank LIMIT N
//   2. Vector: SELECT ... 1 - (embedding <=> query_vec) ORDER BY dist LIMIT N
//   3. RRF:    score(d) = Σ 1/(k + rank(d)) across methods
//   4. Normalise to [0,1], sort descending, truncate to limit
//
// Ferrumyx adds a cancer_type / gene filter on top.

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub chunk_id:    Uuid,
    pub paper_id:    Uuid,
    pub content:     String,
    pub score:       f32,
    pub fts_rank:    Option<u32>,
    pub vector_rank: Option<u32>,
}

impl SearchResult {
    pub fn is_hybrid(&self) -> bool {
        self.fts_rank.is_some() && self.vector_rank.is_some()
    }
}

pub struct HybridSearchConfig {
    pub limit:            usize,
    pub rrf_k:            u32,
    pub pre_fusion_limit: usize,
    pub use_fts:          bool,
    pub use_vector:       bool,
}

impl Default for HybridSearchConfig {
    fn default() -> Self {
        Self {
            limit:            20,
            rrf_k:            60,
            pre_fusion_limit: 100,
            use_fts:          true,
            use_vector:       true,
        }
    }
}

/// Hybrid search over paper_chunks: FTS + vector → RRF fusion.
/// Uses LanceDB for vector search and full-text search.
pub async fn hybrid_search(
    repo:         &IngestionRepository,
    query_text:   &str,
    query_vec:    Option<Vec<f32>>,
    cfg:          &HybridSearchConfig,
) -> Result<Vec<SearchResult>> {
    use ferrumyx_db::chunks::ChunkRepository;
    use std::collections::HashMap;
    
    let mut fts_rows:    Vec<(Uuid, Uuid, String, i64)> = vec![];
    let mut vector_rows: Vec<(Uuid, Uuid, String, i64)> = vec![];

    // 1. Full-text search via LanceDB
    // Note: LanceDB doesn't have built-in FTS, so we do a simple content search
    // For production, consider integrating with a dedicated FTS engine like Tantivy
    if cfg.use_fts {
        let chunk_repo = ChunkRepository::new(repo.db());
        // Simple contains search - in production, use Tantivy for proper FTS
        let all_chunks = chunk_repo.list(0, cfg.pre_fusion_limit).await
            .context("FTS query failed")?;
        
        // Filter chunks that contain the query text (case-insensitive)
        let query_lower = query_text.to_lowercase();
        let mut matches: Vec<_> = all_chunks
            .into_iter()
            .filter(|c| c.content.to_lowercase().contains(&query_lower))
            .enumerate()
            .map(|(i, c)| (c.id, c.paper_id, c.content, i as i64 + 1))
            .take(cfg.pre_fusion_limit)
            .collect();
        
        fts_rows.append(&mut matches);
    }

    // 2. Vector search via LanceDB
    if cfg.use_vector {
        if let Some(ref qv) = query_vec {
            let norm = l2_norm(qv);
            let normalised: Vec<f32> = qv.iter().map(|x| x / norm).collect();
            
            let chunk_repo = ChunkRepository::new(repo.db());
            let similar_chunks = chunk_repo.search_similar(&normalised, cfg.pre_fusion_limit).await
                .context("Vector search query failed")?;
            
            vector_rows = similar_chunks
                .into_iter()
                .enumerate()
                .map(|(i, c)| (c.id, c.paper_id, c.content, i as i64 + 1))
                .collect();
        }
    }

    // 3. RRF fusion (same algorithm as IronClaw's reciprocal_rank_fusion)
    let k = cfg.rrf_k as f32;
    let mut scores: HashMap<Uuid, (Uuid, String, f32, Option<u32>, Option<u32>)>
        = HashMap::new();

    for (chunk_id, paper_id, content, rank) in &fts_rows {
        let rrf = 1.0 / (k + *rank as f32);
        let entry = scores.entry(*chunk_id).or_insert((*paper_id, content.clone(), 0.0, None, None));
        entry.2 += rrf;
        entry.3 = Some(*rank as u32);
    }
    for (chunk_id, paper_id, content, rank) in &vector_rows {
        let rrf = 1.0 / (k + *rank as f32);
        let entry = scores.entry(*chunk_id).or_insert((*paper_id, content.clone(), 0.0, None, None));
        entry.2 += rrf;
        entry.4 = Some(*rank as u32);
    }

    // 4. Normalise → sort → truncate
    let max_score = scores.values().map(|v| v.2).fold(0.0f32, f32::max);
    let mut results: Vec<SearchResult> = scores.into_iter().map(|(chunk_id, (paper_id, content, score, fts_rank, vector_rank))| {
        SearchResult {
            chunk_id, paper_id, content,
            score: if max_score > 0.0 { score / max_score } else { 0.0 },
            fts_rank, vector_rank,
        }
    }).collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(cfg.limit);

    debug!(
        query = query_text,
        fts_hits = fts_rows.len(),
        vector_hits = vector_rows.len(),
        fused = results.len(),
        "Hybrid search complete"
    );

    Ok(results)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_l2_norm_unit_vector() {
        let v = vec![3.0f32, 4.0f32];
        assert!((l2_norm(&v) - 5.0).abs() < 1e-5);
    }

    #[test]
    fn test_l2_norm_zero_is_safe() {
        let v = vec![0.0f32, 0.0f32];
        assert!(l2_norm(&v) > 0.0);  // returns 1e-10, not 0
    }

    #[test]
    fn test_default_config_uses_rust_native() {
        let cfg = EmbeddingConfig::default();
        assert_eq!(cfg.backend, EmbeddingBackend::RustNative);
        assert_eq!(cfg.dim, 768);
    }
}
