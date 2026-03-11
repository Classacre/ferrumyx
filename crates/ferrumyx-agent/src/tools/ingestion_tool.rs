use async_trait::async_trait;
use ironclaw::context::JobContext;
use ironclaw::tools::{Tool, ToolError, ToolOutput};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::{Instant, timeout};

use ferrumyx_db::Database;
use ferrumyx_ingestion::embedding::{
    EmbeddingBackend as IngestionEmbeddingBackend,
    EmbeddingConfig as IngestionEmbeddingConfig,
};
use ferrumyx_ingestion::pipeline::{run_ingestion, IngestionJob, IngestionProgress, IngestionSourceSpec};
use ferrumyx_ingestion::repository::IngestionRepository;

/// Tool to run the Ferrumyx end-to-end knowledge ingestion pipeline natively.
pub struct IngestionTool {
    db: Arc<Database>,
}

impl IngestionTool {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[derive(Debug, Clone)]
struct IngestionRuntimeDefaults {
    max_results: usize,
    idle_timeout_secs: u64,
    max_runtime_secs: u64,
    pubmed_api_key: Option<String>,
    semantic_scholar_api_key: Option<String>,
    unpaywall_email: Option<String>,
    enable_embeddings: bool,
    embedding_cfg: Option<IngestionEmbeddingConfig>,
}

impl Default for IngestionRuntimeDefaults {
    fn default() -> Self {
        Self {
            max_results: 50,
            idle_timeout_secs: 600,
            max_runtime_secs: 14_400,
            pubmed_api_key: std::env::var("FERRUMYX_PUBMED_API_KEY")
                .ok()
                .filter(|v| !v.trim().is_empty()),
            semantic_scholar_api_key: std::env::var("FERRUMYX_SEMANTIC_SCHOLAR_API_KEY")
                .ok()
                .or_else(|| std::env::var("SEMANTIC_SCHOLAR_API_KEY").ok())
                .filter(|v| !v.trim().is_empty()),
            unpaywall_email: std::env::var("FERRUMYX_UNPAYWALL_EMAIL")
                .ok()
                .filter(|v| !v.trim().is_empty()),
            enable_embeddings: std::env::var("FERRUMYX_INGESTION_ENABLE_EMBEDDINGS")
                .ok()
                .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true")),
            embedding_cfg: None,
        }
    }
}

fn config_path() -> PathBuf {
    std::env::var("FERRUMYX_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("ferrumyx.toml"))
}

fn toml_u64(root: &toml::Value, path: &[&str], default: u64) -> u64 {
    let mut cur = root;
    for p in path {
        match cur.get(*p) {
            Some(next) => cur = next,
            None => return default,
        }
    }
    cur.as_integer()
        .and_then(|v| if v >= 0 { Some(v as u64) } else { None })
        .unwrap_or(default)
}

fn toml_bool(root: &toml::Value, path: &[&str], default: bool) -> bool {
    let mut cur = root;
    for p in path {
        match cur.get(*p) {
            Some(next) => cur = next,
            None => return default,
        }
    }
    cur.as_bool().unwrap_or(default)
}

fn toml_string(root: &toml::Value, path: &[&str]) -> Option<String> {
    let mut cur = root;
    for p in path {
        cur = cur.get(*p)?;
    }
    cur.as_str()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn first_nonempty_toml_string(root: &toml::Value, paths: &[&[&str]]) -> Option<String> {
    for path in paths {
        if let Some(v) = toml_string(root, path) {
            return Some(v);
        }
    }
    None
}

fn load_runtime_defaults() -> IngestionRuntimeDefaults {
    let mut defaults = IngestionRuntimeDefaults::default();
    let path = config_path();
    let Ok(content) = fs::read_to_string(path) else {
        return defaults;
    };
    let Ok(root) = toml::from_str::<toml::Value>(&content) else {
        return defaults;
    };

    defaults.max_results = toml_u64(&root, &["ingestion", "default_max_results"], defaults.max_results as u64)
        .clamp(1, 5000) as usize;
    defaults.idle_timeout_secs =
        toml_u64(&root, &["ingestion", "watchdog", "idle_timeout_secs"], defaults.idle_timeout_secs)
            .clamp(60, 3600);
    defaults.max_runtime_secs =
        toml_u64(&root, &["ingestion", "watchdog", "max_runtime_secs"], defaults.max_runtime_secs)
            .clamp(600, 86_400);
    if defaults.pubmed_api_key.is_none() {
        defaults.pubmed_api_key = first_nonempty_toml_string(
            &root,
            &[
                &["ingestion", "pubmed", "api_key"],
                &["ingestion", "pubmed", "api_key_secret"],
            ],
        );
    }
    if defaults.semantic_scholar_api_key.is_none() {
        defaults.semantic_scholar_api_key = first_nonempty_toml_string(
            &root,
            &[
                &["ingestion", "semanticscholar", "api_key"],
                &["ingestion", "semanticscholar", "api_key_secret"],
            ],
        );
    }
    if defaults.unpaywall_email.is_none() {
        defaults.unpaywall_email = first_nonempty_toml_string(
            &root,
            &[&["ingestion", "unpaywall", "email"]],
        );
    }
    defaults.enable_embeddings = toml_bool(
        &root,
        &["ingestion", "enable_embeddings"],
        defaults.enable_embeddings,
    );

    if defaults.enable_embeddings {
        let backend = toml_string(&root, &["embedding", "backend"])
            .unwrap_or_else(|| "rust_native".to_string())
            .to_lowercase();
        let model = toml_string(&root, &["embedding", "embedding_model"])
            .unwrap_or_else(|| "microsoft/BiomedNLP-BiomedBERT-base-uncased-abstract-fulltext".to_string());
        let base_url = toml_string(&root, &["embedding", "base_url"]);
        let batch_size = toml_u64(&root, &["embedding", "batch_size"], 32)
            .clamp(1, 256) as usize;
        let dim = toml_u64(
            &root,
            &["embedding", "embedding_dim"],
            if backend == "rust_native" || backend == "biomedbert" { 768 } else { 1536 },
        )
        .clamp(64, 8192) as usize;

        let mapped_backend = match backend.as_str() {
            "openai" => IngestionEmbeddingBackend::OpenAi,
            "gemini" => IngestionEmbeddingBackend::Gemini,
            "openai_compatible" => IngestionEmbeddingBackend::OpenAiCompatible,
            "ollama" => IngestionEmbeddingBackend::Ollama,
            "biomedbert" => IngestionEmbeddingBackend::BiomedBert,
            _ => IngestionEmbeddingBackend::RustNative,
        };

        let api_key = toml_string(&root, &["embedding", "api_key"]).or_else(|| match mapped_backend {
            IngestionEmbeddingBackend::OpenAi => {
                std::env::var("FERRUMYX_OPENAI_API_KEY")
                    .ok()
                    .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            }
            IngestionEmbeddingBackend::Gemini => {
                std::env::var("FERRUMYX_GEMINI_API_KEY")
                    .ok()
                    .or_else(|| std::env::var("GEMINI_API_KEY").ok())
            }
            IngestionEmbeddingBackend::OpenAiCompatible => {
                std::env::var("FERRUMYX_COMPAT_API_KEY")
                    .ok()
                    .or_else(|| std::env::var("LLM_API_KEY").ok())
            }
            _ => None,
        }).filter(|s| !s.trim().is_empty());

        defaults.embedding_cfg = Some(IngestionEmbeddingConfig {
            backend: mapped_backend,
            api_key,
            model,
            dim,
            batch_size,
            base_url,
        });
    }

    defaults
}

fn require_str<'a>(params: &'a serde_json::Value, name: &str) -> Result<&'a str, ToolError> {
    params
        .get(name)
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::InvalidParameters(format!("missing '{}' parameter", name)))
}

#[async_trait]
impl Tool for IngestionTool {
    fn name(&self) -> &str {
        "ingest_literature"
    }

    fn description(&self) -> &str {
        "Ingests scientific literature for a given gene, mutation, and cancer type. Extracts text chunks and builds the knowledge graph."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "gene": {
                    "type": "string",
                    "description": "The gene symbol to search for (e.g., KRAS)"
                },
                "cancer_type": {
                    "type": "string",
                    "description": "The type of cancer (e.g., pancreatic cancer)"
                },
                "mutation": {
                    "type": "string",
                    "description": "Optional specific mutation (e.g., G12D)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of papers to fetch (default: 50)"
                },
                "idle_timeout_secs": {
                    "type": "integer",
                    "description": "Abort only when no ingestion progress heartbeat is received for this many seconds (default: 600)"
                },
                "max_runtime_secs": {
                    "type": "integer",
                    "description": "Soft safety cap for total ingestion runtime in seconds (default: 14400)"
                }
            },
            "required": ["gene", "cancer_type"]
        })
    }

    fn execution_timeout(&self) -> Duration {
        // Keep framework-level timeout very high; activity-based watchdog below
        // performs stall detection and controlled termination.
        Duration::from_secs(6 * 60 * 60)
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let defaults = load_runtime_defaults();

        let gene = require_str(&params, "gene")?.to_string();
        let cancer_type = require_str(&params, "cancer_type")?.to_string();

        let mutation = params
            .get("mutation")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let max_results = params
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(defaults.max_results)
            .clamp(1, 5000);
        let idle_timeout = Duration::from_secs(
            params
                .get("idle_timeout_secs")
                .and_then(|v| v.as_u64())
                .unwrap_or(defaults.idle_timeout_secs)
                .clamp(60, 3600),
        );
        let max_runtime = Duration::from_secs(
            params
                .get("max_runtime_secs")
                .and_then(|v| v.as_u64())
                .unwrap_or(defaults.max_runtime_secs)
                .clamp(600, 86_400),
        );

        let job = IngestionJob {
            gene,
            mutation,
            cancer_type,
            max_results,
            sources: vec![
                IngestionSourceSpec::PubMed,
                IngestionSourceSpec::EuropePmc,
                IngestionSourceSpec::SemanticScholar,
                IngestionSourceSpec::Arxiv,
                IngestionSourceSpec::BioRxiv,
                IngestionSourceSpec::MedRxiv,
                IngestionSourceSpec::ClinicalTrials,
            ],
            pubmed_api_key: defaults.pubmed_api_key,
            semantic_scholar_api_key: defaults.semantic_scholar_api_key,
            unpaywall_email: defaults.unpaywall_email,
            embedding_cfg: defaults.embedding_cfg.clone(),
            enable_scihub_fallback: false,
        };

        let repo = Arc::new(IngestionRepository::new(self.db.clone()));
        let (progress_tx, mut progress_rx) = broadcast::channel::<IngestionProgress>(512);
        let ingest_repo = repo.clone();
        let ingest_task = tokio::spawn(async move {
            run_ingestion(job, ingest_repo, Some(progress_tx)).await
        });

        let started_at = Instant::now();
        let hard_deadline = started_at + max_runtime;
        let mut last_progress_at = started_at;
        let mut last_stage = "starting".to_string();
        let mut last_message = "bootstrapping ingestion".to_string();
        let mut last_papers_found = 0usize;
        let mut last_papers_inserted = 0usize;
        let mut last_chunks_inserted = 0usize;

        let result = loop {
            if ingest_task.is_finished() {
                break match ingest_task.await {
                    Ok(res) => res,
                    Err(e) => {
                        if e.is_cancelled() {
                            return Err(ToolError::ExecutionFailed(
                                "ingestion task cancelled".to_string(),
                            ));
                        }
                        return Err(ToolError::ExecutionFailed(format!(
                            "ingestion task join failed: {e}"
                        )));
                    }
                };
            }

            let now = Instant::now();
            if now >= hard_deadline {
                ingest_task.abort();
                return Err(ToolError::Timeout(max_runtime));
            }

            let remaining = hard_deadline.saturating_duration_since(now);
            let wait_for = idle_timeout.min(remaining);
            match timeout(wait_for, progress_rx.recv()).await {
                Ok(Ok(progress)) => {
                    last_progress_at = Instant::now();
                    last_stage = progress.stage;
                    last_message = progress.message;
                    last_papers_found = progress.papers_found;
                    last_papers_inserted = progress.papers_inserted;
                    last_chunks_inserted = progress.chunks_inserted;
                }
                Ok(Err(broadcast::error::RecvError::Lagged(_))) => {
                    // Stream backpressure is acceptable; newer heartbeats follow.
                    continue;
                }
                Ok(Err(broadcast::error::RecvError::Closed)) => {
                    // Producer ended; loop will pick up task completion shortly.
                    continue;
                }
                Err(_) => {
                    let idle_for = Instant::now().saturating_duration_since(last_progress_at);
                    if idle_for >= idle_timeout {
                        ingest_task.abort();
                        return Err(ToolError::ExecutionFailed(format!(
                            "ingestion stalled (no progress heartbeat for {}s). Last stage='{}', message='{}', found={}, inserted={}, chunks={}",
                            idle_for.as_secs(),
                            last_stage,
                            last_message,
                            last_papers_found,
                            last_papers_inserted,
                            last_chunks_inserted
                        )));
                    }
                }
            }
        };
        let recomputed = ferrumyx_kg::compute_target_scores(self.db.clone())
            .await
            .unwrap_or(0);

        let output_text = format!(
            "Ingestion completed in {}ms. Found {} papers across sources. Inserted {} new papers and {} knowledge chunks into LanceDB. Skipped {} duplicates. Recomputed {} target scores. Watchdog policy: idle={}s, max_runtime={}s.",
            result.duration_ms,
            result.papers_found,
            result.papers_inserted,
            result.chunks_inserted,
            result.papers_duplicate,
            recomputed,
            idle_timeout.as_secs(),
            max_runtime.as_secs(),
        );

        Ok(ToolOutput::text(
            output_text,
            Duration::from_millis(result.duration_ms),
        ))
    }
}
