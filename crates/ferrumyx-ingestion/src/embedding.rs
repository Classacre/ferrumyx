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
use reqwest::Client;
use std::collections::HashSet;
use std::process::Command;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

// Internal embedder module
use crate::embed::config::EmbeddingSpeedMode;
use crate::embed::EmbeddingConfig as RustEmbedConfig;

use crate::repository::IngestionRepository;

// ── Backend config ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmbeddingConfig {
    pub backend: EmbeddingBackend,
    pub api_key: Option<String>,
    pub model: String,
    pub dim: usize,
    pub batch_size: usize,
    pub base_url: Option<String>, // for compat/ollama/biomedbert
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EmbeddingBackend {
    OpenAi,
    Gemini,
    OpenAiCompatible,
    BiomedBert, // Docker Python service (deprecated)
    Ollama,
    FastEmbed, // ONNX Runtime fast path (recommended for CPU throughput)
    RustNative, // Pure Rust Candle BiomedBERT (recommended)
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            backend: EmbeddingBackend::RustNative, // Default to pure Rust
            api_key: None,
            // Microsoft BiomedBERT trained on PubMed abstracts + full-text articles
            // Better for biomedical literature than general PubMedBERT
            model: "microsoft/BiomedNLP-BiomedBERT-base-uncased-abstract-fulltext".to_string(),
            dim: 768,
            batch_size: 32,
            base_url: None,
        }
    }
}

// ── Embedding client ──────────────────────────────────────────────────────────

pub struct EmbeddingClient {
    cfg: EmbeddingConfig,
    client: Client,
}

impl EmbeddingClient {
    pub fn new(cfg: EmbeddingConfig) -> Self {
        Self {
            cfg,
            client: Client::new(),
        }
    }

    /// Embed a batch of texts; returns `(N, dim)` f32 vectors.
    #[instrument(skip(self, texts), fields(n = texts.len(), backend = ?self.cfg.backend))]
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        match self.cfg.backend {
            EmbeddingBackend::OpenAi => self.embed_openai(texts).await,
            EmbeddingBackend::Gemini => self.embed_gemini(texts).await,
            EmbeddingBackend::OpenAiCompatible => self.embed_compat(texts).await,
            EmbeddingBackend::BiomedBert => self.embed_biomedbert(texts).await,
            EmbeddingBackend::Ollama => self.embed_ollama(texts).await,
            EmbeddingBackend::FastEmbed => self.embed_fastembed(texts).await,
            EmbeddingBackend::RustNative => self.embed_rust_native(texts).await,
        }
    }

    // ── OpenAI ─────────────────────────────────────────────────────────────

    async fn embed_openai(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let key = self.cfg.api_key.as_deref().unwrap_or("");
        let body = serde_json::json!({
            "model": &self.cfg.model,
            "input": texts,
        });
        let resp: serde_json::Value = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .bearer_auth(key)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;
        parse_openai_embeddings(&resp)
    }

    // ── Gemini ─────────────────────────────────────────────────────────────

    async fn embed_gemini(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let key = self.cfg.api_key.as_deref().unwrap_or("");
        let requests: Vec<serde_json::Value> = texts
            .iter()
            .map(|t| {
                serde_json::json!({
                    "model": format!("models/{}", self.cfg.model),
                    "content": { "parts": [{"text": t}] }
                })
            })
            .collect();
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:batchEmbedContents?key={}",
            self.cfg.model, key
        );
        let resp: serde_json::Value = self
            .client
            .post(&url)
            .json(&serde_json::json!({"requests": requests}))
            .send()
            .await?
            .json()
            .await?;
        Ok(resp["embeddings"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|e| {
                e["values"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                    .collect()
            })
            .collect())
    }

    // ── OpenAI-compatible ──────────────────────────────────────────────────

    async fn embed_compat(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let base = self
            .cfg
            .base_url
            .as_deref()
            .unwrap_or("http://localhost:11434")
            .trim_end_matches('/');
        let url = format!("{}/v1/embeddings", base);
        let body = serde_json::json!({
            "model": &self.cfg.model,
            "input": texts,
        });
        let mut req = self.client.request(reqwest::Method::POST, &url).json(&body);
        if let Some(ref k) = self.cfg.api_key {
            req = req.bearer_auth(k);
        }
        let resp: serde_json::Value = req.send().await?.json().await?;
        parse_openai_embeddings(&resp)
    }

    // ── BiomedBERT Docker service ──────────────────────────────────────────

    async fn embed_biomedbert(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let base = self
            .cfg
            .base_url
            .as_deref()
            .unwrap_or("http://localhost:8002");
        let url = format!("{}/embed", base);
        let body = serde_json::json!({
            "texts":     texts,
            "normalize": true,
        });
        let resp: serde_json::Value = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .with_context(|| {
                format!(
                    "BiomedBERT service unreachable at {url}. \
                 Start it with: docker compose --profile embed up -d"
                )
            })?
            .json()
            .await?;
        Ok(resp["embeddings"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|row| {
                row.as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                    .collect()
            })
            .collect())
    }

    // ── Ollama ─────────────────────────────────────────────────────────────

    async fn embed_ollama(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let base = self
            .cfg
            .base_url
            .as_deref()
            .unwrap_or("http://localhost:11434");
        let mut out = Vec::with_capacity(texts.len());
        for text in texts {
            let url = format!("{}/api/embeddings", base);
            let body = serde_json::json!({"model": &self.cfg.model, "prompt": text});
            let resp: serde_json::Value = self
                .client
                .request(reqwest::Method::POST, &url)
                .json(&body)
                .send()
                .await?
                .json()
                .await?;
            let vec: Vec<f32> = resp["embedding"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect();
            out.push(vec);
        }
        Ok(out)
    }

    // ── FastEmbed (ONNX Runtime) ───────────────────────────────────────────
    // Throughput-oriented local embedding path with broad 768-d model support.

    async fn embed_fastembed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        #[cfg(not(feature = "fastembed_backend"))]
        {
            let _ = texts;
            anyhow::bail!(
                "FastEmbed backend requested but ferrumyx-ingestion was built without `fastembed_backend` feature"
            );
        }
        #[cfg(feature = "fastembed_backend")]
        {
        use fastembed::{TextEmbedding, TextInitOptions};
        use std::sync::{Arc, Mutex};

        static FAST_MODEL: std::sync::OnceLock<Arc<Mutex<TextEmbedding>>> = std::sync::OnceLock::new();
        static FAST_MODEL_INIT_LOCK: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();

        let model_name = map_fastembed_model(&self.cfg.model);
        let max_length = resolve_embed_speed_mode().max_length();
        let cache_dir = resolve_embed_cache_dir()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("data").join("cache").join("hf-hub"));

        if FAST_MODEL.get().is_none() {
            let init_lock = FAST_MODEL_INIT_LOCK.get_or_init(|| tokio::sync::Mutex::new(()));
            let _guard = init_lock.lock().await;
            if FAST_MODEL.get().is_none() {
                let init = tokio::task::spawn_blocking(move || {
                    let options = TextInitOptions::new(model_name)
                        .with_cache_dir(cache_dir)
                        .with_show_download_progress(false)
                        .with_max_length(max_length);
                    TextEmbedding::try_new(options)
                })
                .await
                .map_err(|e| anyhow::anyhow!("FastEmbed init join failed: {e}"))?
                .map_err(|e| anyhow::anyhow!("FastEmbed init failed: {e}"))?;
                let _ = FAST_MODEL.set(Arc::new(Mutex::new(init)));
            }
        }

        let model = FAST_MODEL
            .get()
            .ok_or_else(|| anyhow::anyhow!("FastEmbed unavailable after initialization"))?
            .clone();
        let docs = texts.to_vec();
        let batch_size = self.cfg.batch_size;
        tokio::task::spawn_blocking(move || {
            let mut guard = model
                .lock()
                .map_err(|_| anyhow::anyhow!("FastEmbed mutex poisoned"))?;
            guard
                .embed(docs, Some(batch_size))
                .map_err(|e| anyhow::anyhow!("FastEmbed inference failed: {e}"))
        })
        .await
        .map_err(|e| anyhow::anyhow!("FastEmbed inference join failed: {e}"))?
        }
    }

    // ── Rust Native (Candle BiomedBERT) ─────────────────────────────────────
    // Pure Rust inference - no Python, no Docker service needed!

    async fn embed_rust_native(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        use std::sync::Arc;
        use tokio::sync::Mutex;

        // Reused embedder instance across calls.
        static EMBEDDER: std::sync::OnceLock<Arc<Mutex<crate::embed::BiomedBertEmbedder>>> =
            std::sync::OnceLock::new();
        // Serialize first-time initialization to avoid concurrent partial model
        // downloads causing corrupted cache files.
        static EMBEDDER_INIT_LOCK: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();

        let use_gpu = resolve_embed_use_gpu();
        let speed_mode = resolve_embed_speed_mode();
        let config = RustEmbedConfig {
            model_id: self.cfg.model.clone(),
            batch_size: self.cfg.batch_size,
            max_length: speed_mode.max_length(),
            // Keep normalisation centralised in the pipeline step so every
            // backend (remote and local) follows one consistent write path.
            normalize: false,
            pooling: crate::embed::PoolingStrategy::Mean,
            use_gpu,
            cache_size: 1000,
            cache_dir: resolve_embed_cache_dir(),
        };

        if EMBEDDER.get().is_none() {
            let init_lock = EMBEDDER_INIT_LOCK.get_or_init(|| Mutex::new(()));
            let _guard = init_lock.lock().await;

            if EMBEDDER.get().is_none() {
                let embedder = match crate::embed::BiomedBertEmbedder::new(config.clone()).await {
                    Ok(embedder) => embedder,
                    Err(primary_err) => {
                        let primary_msg = primary_err.to_string();
                        let looks_like_legacy_pth_issue = primary_msg
                            .to_ascii_lowercase()
                            .contains("invalid zip archive");
                        if !looks_like_legacy_pth_issue {
                            return Err(anyhow::anyhow!(
                                "Failed to initialize Rust embedder: {primary_err}"
                            ));
                        }

                        let fallback_model = std::env::var("FERRUMYX_EMBED_FALLBACK_MODEL")
                            .ok()
                            .map(|v| v.trim().to_string())
                            .filter(|v| !v.is_empty())
                            .unwrap_or_else(|| "intfloat/e5-base-v2".to_string());

                        if fallback_model == config.model_id {
                            return Err(anyhow::anyhow!(
                                "Failed to initialize Rust embedder: {primary_err}"
                            ));
                        }

                        warn!(
                            primary_model = %config.model_id,
                            fallback_model = %fallback_model,
                            error = %primary_msg,
                            "Rust embedder init failed on primary model; retrying with fallback safetensors model"
                        );

                        let mut fallback_cfg = config.clone();
                        fallback_cfg.model_id = fallback_model;
                        crate::embed::BiomedBertEmbedder::new(fallback_cfg).await.map_err(
                            |fallback_err| {
                                anyhow::anyhow!(
                                    "Failed to initialize Rust embedder: primary='{primary_msg}', fallback='{fallback_err}'"
                                )
                            },
                        )?
                    }
                };
                let _ = EMBEDDER.set(Arc::new(Mutex::new(embedder)));
            }
        }

        let embedder = EMBEDDER
            .get()
            .ok_or_else(|| anyhow::anyhow!("Rust embedder unavailable after initialization"))?;

        let embedder_guard = embedder.lock().await;
        embedder_guard
            .embed(texts)
            .await
            .map_err(|e| anyhow::anyhow!("Rust embedding failed: {}", e))
    }
}

fn resolve_embed_use_gpu() -> bool {
    static RESOLVED_USE_GPU: OnceLock<bool> = OnceLock::new();

    *RESOLVED_USE_GPU.get_or_init(|| match std::env::var("FERRUMYX_EMBED_USE_GPU") {
        Ok(v) => {
            let t = v.trim().to_ascii_lowercase();
            if t == "1" || t == "true" || t == "yes" {
                true
            } else if t == "0" || t == "false" || t == "no" {
                false
            } else {
                warn!(
                    env_var = "FERRUMYX_EMBED_USE_GPU",
                    value = %v,
                    "Invalid GPU override; falling back to automatic detection"
                );
                resolve_embed_use_gpu_uncached()
            }
        }
        Err(_) => resolve_embed_use_gpu_uncached(),
    })
}

fn resolve_embed_use_gpu_uncached() -> bool {
    let has_nvidia = detect_nvidia_gpu();
    if has_nvidia && !command_success("nvcc", &["--version"]) {
        let _ = try_install_cuda_toolkit_once();
    }
    has_nvidia || command_success("nvcc", &["--version"])
}

fn resolve_embed_speed_mode() -> EmbeddingSpeedMode {
    static RESOLVED_SPEED_MODE: OnceLock<EmbeddingSpeedMode> = OnceLock::new();

    *RESOLVED_SPEED_MODE.get_or_init(|| match std::env::var(EmbeddingSpeedMode::ENV_VAR) {
        Ok(raw) if !raw.trim().is_empty() => match EmbeddingSpeedMode::parse(&raw) {
            Some(mode) => mode,
            None => {
                warn!(
                    env_var = EmbeddingSpeedMode::ENV_VAR,
                    value = %raw,
                    "Invalid embedding speed mode override; falling back to automatic selection"
                );
                resolve_embed_speed_mode_auto()
            }
        },
        _ => resolve_embed_speed_mode_auto(),
    })
}

fn resolve_embed_speed_mode_auto() -> EmbeddingSpeedMode {
    if resolve_embed_use_gpu() {
        EmbeddingSpeedMode::Quality
    } else {
        EmbeddingSpeedMode::Fast
    }
}

fn resolve_embed_cache_dir() -> Option<String> {
    if let Ok(raw) = std::env::var("FERRUMYX_EMBED_CACHE_DIR") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    // Keep a stable on-disk model cache in-repo by default so Candle/HF model
    // downloads are reused across runs.
    let fallback = PathBuf::from("data").join("cache").join("hf-hub");
    Some(fallback.to_string_lossy().to_string())
}

#[cfg(feature = "fastembed_backend")]
fn map_fastembed_model(raw: &str) -> fastembed::EmbeddingModel {
    use fastembed::EmbeddingModel as Model;
    let key = raw.trim().to_ascii_lowercase();
    match key.as_str() {
        "bgebaseenv15" | "bge_base_en_v1_5" | "baai/bge-base-en-v1.5" => Model::BGEBaseENV15,
        "bgebaseenv15q" | "bge_base_en_v1_5_q" | "qdrant/bge-base-en-v1.5-onnx-q" => {
            Model::BGEBaseENV15Q
        }
        "allmpnetbasev2" | "all-mpnet-base-v2" | "sentence-transformers/all-mpnet-base-v2" => {
            Model::AllMpnetBaseV2
        }
        "multilinguale5base" | "multilingual-e5-base" | "intfloat/multilingual-e5-base" => {
            Model::MultilingualE5Base
        }
        "jinaembeddingsv2baseen" | "jinaai/jina-embeddings-v2-base-en" => {
            Model::JinaEmbeddingsV2BaseEN
        }
        "nomicembedtextv15" | "nomic-ai/nomic-embed-text-v1.5" => Model::NomicEmbedTextV15,
        "nomicembedtextv15q" => Model::NomicEmbedTextV15Q,
        "snowflakearcticembedm" | "snowflake/snowflake-arctic-embed-m" => {
            Model::SnowflakeArcticEmbedM
        }
        "snowflakearcticembedmq" => Model::SnowflakeArcticEmbedMQ,
        // Throughput-first default while preserving 768-d schema compatibility.
        _ => Model::BGEBaseENV15Q,
    }
}

pub fn fastembed_enabled() -> bool {
    cfg!(feature = "fastembed_backend")
}

fn command_success(bin: &str, args: &[&str]) -> bool {
    Command::new(bin)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn command_output_contains(bin: &str, args: &[&str], needle: &str) -> bool {
    Command::new(bin)
        .args(args)
        .output()
        .ok()
        .and_then(|o| {
            if !o.status.success() {
                return None;
            }
            Some(String::from_utf8_lossy(&o.stdout).to_string())
        })
        .is_some_and(|s| {
            s.to_ascii_lowercase()
                .contains(&needle.to_ascii_lowercase())
        })
}

fn detect_nvidia_gpu() -> bool {
    if command_success("nvidia-smi", &["--query-gpu=name", "--format=csv,noheader"]) {
        return true;
    }
    #[cfg(target_os = "windows")]
    {
        if command_output_contains(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                "(Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name) -join \"`n\"",
            ],
            "nvidia",
        ) {
            return true;
        }
        if command_output_contains(
            "wmic",
            &["path", "win32_VideoController", "get", "name"],
            "nvidia",
        ) {
            return true;
        }
    }
    false
}

fn try_install_cuda_toolkit_once() -> bool {
    static ATTEMPTED: OnceLock<bool> = OnceLock::new();
    *ATTEMPTED.get_or_init(try_install_cuda_toolkit)
}

fn try_install_cuda_toolkit() -> bool {
    #[cfg(not(target_os = "windows"))]
    {
        return false;
    }
    #[cfg(target_os = "windows")]
    {
        if !command_success("winget", &["--version"]) {
            return false;
        }
        let candidate_ids = ["Nvidia.CUDA", "NVIDIA.CUDA"];
        for id in candidate_ids {
            let ok = Command::new("winget")
                .args([
                    "install",
                    "-e",
                    "--id",
                    id,
                    "--silent",
                    "--accept-source-agreements",
                    "--accept-package-agreements",
                ])
                .spawn()
                .and_then(|mut child| {
                    let start = std::time::Instant::now();
                    loop {
                        if let Some(status) = child.try_wait()? {
                            return Ok(status.success());
                        }
                        if start.elapsed() > Duration::from_secs(180) {
                            let _ = child.kill();
                            return Ok(false);
                        }
                        std::thread::sleep(Duration::from_millis(500));
                    }
                })
                .unwrap_or(false);
            if ok && command_success("nvcc", &["--version"]) {
                return true;
            }
        }
        false
    }
}

fn parse_openai_embeddings(resp: &serde_json::Value) -> Result<Vec<Vec<f32>>> {
    Ok(resp["data"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|item| {
            item["embedding"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect()
        })
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
    repo: &IngestionRepository,
    paper_id: Uuid,
) -> Result<usize> {
    let mut pending_chunks = repo
        .find_chunks_without_embeddings(paper_id)
        .await
        .context("fetch pending chunks failed")?;

    if let Some(cap) = resolve_throughput_embedding_chunk_cap() {
        if pending_chunks.len() > cap {
            info!(
                paper_id = %paper_id,
                pending = pending_chunks.len(),
                cap,
                "Embedding workload capped for throughput mode"
            );
            pending_chunks.truncate(cap);
        }
    }

    if pending_chunks.is_empty() {
        debug!(paper_id = %paper_id, "No pending chunks to embed");
        return Ok(0);
    }

    info!(paper_id = %paper_id, n = pending_chunks.len(), "Embedding chunks");
    embed_chunk_updates(client, repo, pending_chunks).await
}

/// Embed pending chunks across multiple papers in one global batch stream.
/// This improves throughput versus paper-by-paper embedding when each paper
/// only contributes a small number of chunks.
#[instrument(skip(client, repo, paper_ids), fields(n_papers = paper_ids.len()))]
pub async fn embed_pending_chunks_for_papers(
    client: &EmbeddingClient,
    repo: &IngestionRepository,
    paper_ids: &[Uuid],
) -> Result<usize> {
    if paper_ids.is_empty() {
        return Ok(0);
    }

    let mut seen = HashSet::with_capacity(paper_ids.len());
    let cap = resolve_throughput_embedding_chunk_cap();
    let mut pending_chunks: Vec<(Uuid, String)> = Vec::new();

    for paper_id in paper_ids {
        if !seen.insert(*paper_id) {
            continue;
        }
        let mut chunks = repo
            .find_chunks_without_embeddings(*paper_id)
            .await
            .with_context(|| format!("fetch pending chunks failed for paper {paper_id}"))?;
        if let Some(per_paper_cap) = cap {
            if chunks.len() > per_paper_cap {
                info!(
                    paper_id = %paper_id,
                    pending = chunks.len(),
                    cap = per_paper_cap,
                    "Embedding workload capped for throughput mode"
                );
                chunks.truncate(per_paper_cap);
            }
        }
        pending_chunks.extend(chunks);
    }

    if pending_chunks.is_empty() {
        debug!("No pending chunks across requested papers");
        return Ok(0);
    }

    info!(
        n_papers = seen.len(),
        n_chunks = pending_chunks.len(),
        batch_size = client.cfg.batch_size,
        "Embedding chunks across papers"
    );
    embed_chunk_updates(client, repo, pending_chunks).await
}

async fn embed_chunk_updates(
    client: &EmbeddingClient,
    repo: &IngestionRepository,
    pending_chunks: Vec<(Uuid, String)>,
) -> Result<usize> {
    let mut total_embedded = 0usize;

    for batch in pending_chunks.chunks(client.cfg.batch_size) {
        let batch_ids: Vec<Uuid> = batch.iter().map(|(id, _)| *id).collect();
        let batch_texts: Vec<String> = batch.iter().map(|(_, t)| t.clone()).collect();

        let vecs = match client.embed_batch(&batch_texts).await {
            Ok(v) => v,
            Err(e) => {
                warn!("Embedding batch failed: {e} — skipping batch");
                continue;
            }
        };

        if vecs.len() != batch_ids.len() {
            warn!(
                expected = batch_ids.len(),
                received = vecs.len(),
                "Embedding batch size mismatch; truncating to available vectors"
            );
        }

        let updates: Vec<(Uuid, Vec<f32>)> = batch_ids
            .iter()
            .zip(vecs.into_iter())
            .map(|(chunk_id, embedding)| {
                let norm = l2_norm(&embedding);
                let normalized = embedding.into_iter().map(|x| x / norm).collect();
                (*chunk_id, normalized)
            })
            .collect();

        match repo.bulk_update_embeddings(&updates).await {
            Ok(n) => {
                total_embedded += n;
                debug!("Updated {} embeddings", n);
            }
            Err(e) => warn!("Bulk embedding update failed: {e}"),
        };
    }

    info!(total_embedded, "Embedding step complete");
    Ok(total_embedded)
}

fn resolve_throughput_embedding_chunk_cap() -> Option<usize> {
    std::env::var("FERRUMYX_EMBED_THROUGHPUT_MAX_CHUNKS_PER_PAPER")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .map(|v| v.clamp(1, 4_096))
}

fn l2_norm(v: &[f32]) -> f32 {
    let s: f32 = v.iter().map(|x| x * x).sum();
    s.sqrt().max(1e-10)
}

// ── Hybrid search (FTS + vector RRF) — Ferrumyx Runtime Core-aligned ──────────────────────

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub chunk_id: Uuid,
    pub paper_id: Uuid,
    pub content: String,
    pub score: f32,
    pub fts_rank: Option<u32>,
    pub vector_rank: Option<u32>,
}

impl SearchResult {
    pub fn is_hybrid(&self) -> bool {
        self.fts_rank.is_some() && self.vector_rank.is_some()
    }
}

pub struct HybridSearchConfig {
    pub limit: usize,
    pub rrf_k: u32,
    pub pre_fusion_limit: usize,
    pub use_fts: bool,
    pub use_vector: bool,
}

impl Default for HybridSearchConfig {
    fn default() -> Self {
        Self {
            limit: 20,
            rrf_k: 60,
            pre_fusion_limit: 100,
            use_fts: true,
            use_vector: true,
        }
    }
}

/// Hybrid search over paper_chunks: FTS + vector → RRF fusion.
pub async fn hybrid_search(
    repo: &IngestionRepository,
    query_text: &str,
    query_vec: Option<Vec<f32>>,
    cfg: &HybridSearchConfig,
) -> Result<Vec<SearchResult>> {
    use ferrumyx_db::chunks::ChunkRepository;
    use std::collections::HashMap;

    let mut fts_rows: Vec<(Uuid, Uuid, String, i64)> = vec![];
    let mut vector_rows: Vec<(Uuid, Uuid, String, i64)> = vec![];

    // 1. Full-text search via LanceDB
    if cfg.use_fts {
        let chunk_repo = ChunkRepository::new(repo.db());
        let all_chunks = chunk_repo
            .list(0, cfg.pre_fusion_limit)
            .await
            .context("FTS query failed")?;

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
            let similar_chunks = chunk_repo
                .search_similar(&normalised, cfg.pre_fusion_limit)
                .await
                .context("Vector search query failed")?;

            vector_rows = similar_chunks
                .into_iter()
                .enumerate()
                .map(|(i, c)| (c.id, c.paper_id, c.content, i as i64 + 1))
                .collect();
        }
    }

    // 3. RRF fusion
    let k = cfg.rrf_k as f32;
    let mut scores: HashMap<Uuid, (Uuid, String, f32, Option<u32>, Option<u32>)> = HashMap::new();

    for (chunk_id, paper_id, content, rank) in &fts_rows {
        let rrf = 1.0 / (k + *rank as f32);
        let entry =
            scores
                .entry(*chunk_id)
                .or_insert((*paper_id, content.clone(), 0.0, None, None));
        entry.2 += rrf;
        entry.3 = Some(*rank as u32);
    }
    for (chunk_id, paper_id, content, rank) in &vector_rows {
        let rrf = 1.0 / (k + *rank as f32);
        let entry =
            scores
                .entry(*chunk_id)
                .or_insert((*paper_id, content.clone(), 0.0, None, None));
        entry.2 += rrf;
        entry.4 = Some(*rank as u32);
    }

    // 4. Normalise → sort → truncate
    let max_score = scores.values().map(|v| v.2).fold(0.0f32, f32::max);
    let mut results: Vec<SearchResult> = scores
        .into_iter()
        .map(
            |(chunk_id, (paper_id, content, score, fts_rank, vector_rank))| SearchResult {
                chunk_id,
                paper_id,
                content,
                score: if max_score > 0.0 {
                    score / max_score
                } else {
                    0.0
                },
                fts_rank,
                vector_rank,
            },
        )
        .collect();

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(cfg.limit);

    Ok(results)
}

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
        assert!(l2_norm(&v) > 0.0);
    }

    #[test]
    fn test_default_config_uses_rust_native() {
        let cfg = EmbeddingConfig::default();
        assert_eq!(cfg.backend, EmbeddingBackend::RustNative);
    }
}

