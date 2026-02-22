//! Embedding client — calls the configured embedding backend to produce
//! vectors for paper chunks, then writes them to paper_chunks.embedding.
//!
//! Supports the same backends as ferrumyx-llm:
//!   - OpenAI         (text-embedding-3-small / text-embedding-3-large)
//!   - Gemini         (text-embedding-004)
//!   - OpenAI-compat  (any /v1/embeddings endpoint — Groq, Together, etc.)
//!   - BiomedBERT     (local Docker service on :8002)
//!   - Ollama         (nomic-embed-text or any ollama embedding model)
//!
//! The embed pipeline step runs after bulk_insert_chunks and writes back
//! to paper_chunks.embedding (pgvector float4[]).
//!
//! IronClaw alignment: IronClaw also calls the LLM provider's embed endpoint,
//! stores vectors in pgvector, and uses them for hybrid FTS+vector search
//! via RRF. We follow the same pattern (see ironclaw/src/workspace/search.rs).

use anyhow::{Context, Result};
use reqwest::Client;
use sqlx::PgPool;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

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
    BiomedBert,
    Ollama,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            backend:    EmbeddingBackend::OpenAi,
            api_key:    None,
            model:      "text-embedding-3-small".to_string(),
            dim:        1536,
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
        Self { cfg, client: Client::new() }
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
            .post("https://api.openai.com/v1/embeddings")
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
            .post(&url)
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
        let mut req = self.client.post(&url).json(&body);
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
            .post(&url)
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
            let resp: serde_json::Value = self.client.post(&url).json(&body).send().await?
                .json().await?;
            let vec: Vec<f32> = resp["embedding"].as_array().unwrap_or(&vec![])
                .iter().map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect();
            out.push(vec);
        }
        Ok(out)
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
/// to paper_chunks.embedding via pgvector.
///
/// Called as the final step in run_ingestion after bulk_insert_chunks.
/// Mirrors IronClaw's pattern: embed → write to workspace_chunks.embedding.
#[instrument(skip(client, pool))]
pub async fn embed_pending_chunks(
    client: &EmbeddingClient,
    pool:   &PgPool,
    paper_id: Uuid,
) -> Result<usize> {
    // Fetch chunks without embeddings for this paper
    let chunks: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, content FROM paper_chunks
         WHERE paper_id = $1 AND embedding IS NULL
         ORDER BY chunk_index"
    )
    .bind(paper_id)
    .fetch_all(pool)
    .await
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

        // L2-normalise and write
        let mut tx = pool.begin().await?;
        for (chunk_id, vec) in batch_ids.iter().zip(vecs.iter()) {
            let norm = l2_norm(vec);
            let normalised: Vec<f32> = vec.iter().map(|x| x / norm).collect();

            // pgvector: insert as float4[] cast — sqlx PgVector wrapper not needed
            // because we write raw via a CAST
            sqlx::query(
                "UPDATE paper_chunks
                 SET embedding = $1::vector
                 WHERE id = $2"
            )
            .bind(pgvector::Vector::from(normalised))
            .bind(chunk_id)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("embedding write failed for chunk {chunk_id}"))?;

            total_embedded += 1;
        }
        tx.commit().await?;
        debug!("Committed {} embeddings", batch_ids.len());
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

/// Hybrid search over paper_chunks: FTS + pgvector cosine → RRF fusion.
pub async fn hybrid_search(
    pool:         &PgPool,
    query_text:   &str,
    query_vec:    Option<Vec<f32>>,
    cfg:          &HybridSearchConfig,
) -> Result<Vec<SearchResult>> {
    let mut fts_rows:    Vec<(Uuid, Uuid, String, i64)> = vec![];
    let mut vector_rows: Vec<(Uuid, Uuid, String, i64)> = vec![];

    // 1. Full-text search via PostgreSQL tsvector
    if cfg.use_fts {
        fts_rows = sqlx::query_as(
            "SELECT pc.id, pc.paper_id, pc.content,
                    ROW_NUMBER() OVER (ORDER BY ts_rank_cd(to_tsvector('english', pc.content),
                                       plainto_tsquery('english', $1)) DESC) AS rank
             FROM paper_chunks pc
             WHERE to_tsvector('english', pc.content) @@ plainto_tsquery('english', $1)
             ORDER BY rank
             LIMIT $2"
        )
        .bind(query_text)
        .bind(cfg.pre_fusion_limit as i64)
        .fetch_all(pool)
        .await
        .context("FTS query failed")?;
    }

    // 2. Vector search via pgvector cosine distance
    if cfg.use_vector {
        if let Some(ref qv) = query_vec {
            let norm = l2_norm(qv);
            let normalised: Vec<f32> = qv.iter().map(|x| x / norm).collect();
            vector_rows = sqlx::query_as(
                "SELECT pc.id, pc.paper_id, pc.content,
                        ROW_NUMBER() OVER (ORDER BY pc.embedding <=> $1::vector ASC) AS rank
                 FROM paper_chunks pc
                 WHERE pc.embedding IS NOT NULL
                 ORDER BY pc.embedding <=> $1::vector
                 LIMIT $2"
            )
            .bind(pgvector::Vector::from(normalised))
            .bind(cfg.pre_fusion_limit as i64)
            .fetch_all(pool)
            .await
            .context("Vector search query failed")?;
        }
    }

    // 3. RRF fusion (same algorithm as IronClaw's reciprocal_rank_fusion)
    let k = cfg.rrf_k as f32;
    let mut scores: std::collections::HashMap<Uuid, (Uuid, String, f32, Option<u32>, Option<u32>)>
        = std::collections::HashMap::new();

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
    fn test_search_result_hybrid_flag() {
        let r = SearchResult {
            chunk_id: Uuid::new_v4(), paper_id: Uuid::new_v4(),
            content: "test".to_string(), score: 0.8,
            fts_rank: Some(1), vector_rank: Some(2),
        };
        assert!(r.is_hybrid());

        let r2 = SearchResult { fts_rank: None, vector_rank: Some(1), ..r.clone() };
        assert!(!r2.is_hybrid());
    }

    #[test]
    fn test_embedding_config_default() {
        let cfg = EmbeddingConfig::default();
        assert_eq!(cfg.backend, EmbeddingBackend::OpenAi);
        assert_eq!(cfg.dim, 1536);
        assert_eq!(cfg.batch_size, 32);
    }
}
