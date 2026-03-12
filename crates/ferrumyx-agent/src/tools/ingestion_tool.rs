use async_trait::async_trait;
use ironclaw::context::JobContext;
use ironclaw::tools::{Tool, ToolError, ToolOutput};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::{timeout, Instant};

use super::runtime_profile::RuntimeProfile;
use ferrumyx_db::Database;
use ferrumyx_ingestion::embedding::{
    EmbeddingBackend as IngestionEmbeddingBackend, EmbeddingConfig as IngestionEmbeddingConfig,
};
use ferrumyx_ingestion::pipeline::{
    run_ingestion, IngestionJob, IngestionProgress, IngestionSourceSpec,
};
use ferrumyx_ingestion::repository::IngestionRepository;
use ferrumyx_ranker::{ProviderRefreshRequest, TargetQueryEngine};

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
    source_timeout_secs: Option<u64>,
    full_text_step_timeout_secs: Option<u64>,
    full_text_total_timeout_secs: Option<u64>,
    full_text_prefetch_workers: Option<usize>,
    paper_process_workers: Option<usize>,
    perf_mode: String,
    source_cache_enabled: bool,
    source_cache_ttl_secs: u64,
    entity_batch_size: usize,
    fact_batch_size: usize,
    strict_fuzzy_dedup: bool,
    source_max_inflight: usize,
    source_retries: usize,
    pdf_host_concurrency: usize,
    pdf_parse_cache_enabled: bool,
    full_text_negative_cache_enabled: bool,
    full_text_negative_cache_ttl_secs: u64,
    chunk_fingerprint_cache_enabled: bool,
    chunk_fingerprint_cache_ttl_secs: u64,
    heavy_lane_async_enabled: bool,
    min_ner_chars: usize,
    max_relation_genes_per_chunk: usize,
    async_post_ingest_scoring: bool,
    source_profile: String,
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
            source_timeout_secs: std::env::var("FERRUMYX_INGESTION_SOURCE_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .map(|v| v.clamp(5, 300)),
            full_text_step_timeout_secs: std::env::var(
                "FERRUMYX_INGESTION_FULLTEXT_STEP_TIMEOUT_SECS",
            )
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .map(|v| v.clamp(5, 120))
            .or(Some(15)),
            full_text_total_timeout_secs: std::env::var(
                "FERRUMYX_INGESTION_FULLTEXT_TOTAL_TIMEOUT_SECS",
            )
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .map(|v| v.clamp(8, 180))
            .or(Some(28)),
            full_text_prefetch_workers: std::env::var(
                "FERRUMYX_INGESTION_FULLTEXT_PREFETCH_WORKERS",
            )
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .map(|v| v.clamp(1, 32)),
            paper_process_workers: std::env::var("FERRUMYX_PAPER_PROCESS_WORKERS")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .map(|v| v.clamp(1, 16)),
            perf_mode: std::env::var("FERRUMYX_INGESTION_PERF_MODE")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "auto".to_string())
                .to_lowercase(),
            source_cache_enabled: std::env::var("FERRUMYX_INGESTION_SOURCE_CACHE_ENABLED")
                .ok()
                .map_or(true, |v| v == "1" || v.eq_ignore_ascii_case("true")),
            source_cache_ttl_secs: std::env::var("FERRUMYX_INGESTION_SOURCE_CACHE_TTL_SECS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(1800)
                .clamp(60, 86_400),
            entity_batch_size: std::env::var("FERRUMYX_INGESTION_ENTITY_BATCH_SIZE")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(256)
                .clamp(16, 2048),
            fact_batch_size: std::env::var("FERRUMYX_INGESTION_FACT_BATCH_SIZE")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(512)
                .clamp(16, 4096),
            strict_fuzzy_dedup: std::env::var("FERRUMYX_STRICT_FUZZY_DEDUP")
                .ok()
                .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true")),
            source_max_inflight: std::env::var("FERRUMYX_INGESTION_SOURCE_MAX_INFLIGHT")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(4)
                .clamp(1, 16),
            source_retries: std::env::var("FERRUMYX_INGESTION_SOURCE_RETRIES")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(2)
                .clamp(0, 5),
            pdf_host_concurrency: std::env::var("FERRUMYX_PDF_HOST_CONCURRENCY")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(4)
                .clamp(1, 16),
            pdf_parse_cache_enabled: std::env::var("FERRUMYX_PDF_PARSE_CACHE_ENABLED")
                .ok()
                .is_none_or(|v| v == "1" || v.eq_ignore_ascii_case("true")),
            full_text_negative_cache_enabled: std::env::var(
                "FERRUMYX_FULLTEXT_NEGATIVE_CACHE_ENABLED",
            )
            .ok()
            .is_none_or(|v| v == "1" || v.eq_ignore_ascii_case("true")),
            full_text_negative_cache_ttl_secs: std::env::var(
                "FERRUMYX_FULLTEXT_NEGATIVE_CACHE_TTL_SECS",
            )
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(6 * 60 * 60)
            .clamp(60, 604_800),
            chunk_fingerprint_cache_enabled: std::env::var(
                "FERRUMYX_CHUNK_FINGERPRINT_CACHE_ENABLED",
            )
            .ok()
            .is_none_or(|v| v == "1" || v.eq_ignore_ascii_case("true")),
            chunk_fingerprint_cache_ttl_secs: std::env::var(
                "FERRUMYX_CHUNK_FINGERPRINT_CACHE_TTL_SECS",
            )
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(172_800)
            .clamp(300, 1_209_600),
            heavy_lane_async_enabled: std::env::var("FERRUMYX_INGESTION_HEAVY_LANE_ASYNC")
                .ok()
                .is_none_or(|v| v == "1" || v.eq_ignore_ascii_case("true")),
            min_ner_chars: std::env::var("FERRUMYX_INGESTION_MIN_NER_CHARS")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(500)
                .clamp(120, 5000),
            max_relation_genes_per_chunk: std::env::var(
                "FERRUMYX_INGESTION_MAX_RELATION_GENES_PER_CHUNK",
            )
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(4)
            .clamp(1, 16),
            async_post_ingest_scoring: std::env::var("FERRUMYX_INGESTION_ASYNC_POST_SCORE")
                .ok()
                .is_none_or(|v| v == "1" || v.eq_ignore_ascii_case("true")),
            source_profile: std::env::var("FERRUMYX_INGESTION_SOURCE_PROFILE")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "fast".to_string())
                .to_lowercase(),
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

    defaults.max_results = toml_u64(
        &root,
        &["ingestion", "default_max_results"],
        defaults.max_results as u64,
    )
    .clamp(1, 5000) as usize;
    defaults.idle_timeout_secs = toml_u64(
        &root,
        &["ingestion", "watchdog", "idle_timeout_secs"],
        defaults.idle_timeout_secs,
    )
    .clamp(60, 3600);
    defaults.max_runtime_secs = toml_u64(
        &root,
        &["ingestion", "watchdog", "max_runtime_secs"],
        defaults.max_runtime_secs,
    )
    .clamp(600, 86_400);
    defaults.source_timeout_secs = Some(toml_u64(
        &root,
        &["ingestion", "performance", "source_timeout_secs"],
        0,
    ))
    .filter(|v| *v > 0)
    .map(|v| v.clamp(5, 300));
    defaults.full_text_step_timeout_secs = Some(
        toml_u64(
            &root,
            &["ingestion", "performance", "full_text_step_timeout_secs"],
            15,
        )
        .clamp(5, 120),
    );
    defaults.full_text_total_timeout_secs = Some(
        toml_u64(
            &root,
            &["ingestion", "performance", "full_text_total_timeout_secs"],
            28,
        )
        .clamp(8, 180),
    );
    defaults.full_text_prefetch_workers = Some(
        toml_u64(
            &root,
            &["ingestion", "performance", "full_text_prefetch_workers"],
            0,
        )
        .clamp(0, 32) as usize,
    )
    .filter(|v| *v > 0);
    defaults.paper_process_workers = Some(
        toml_u64(
            &root,
            &["ingestion", "performance", "paper_process_workers"],
            0,
        )
        .clamp(0, 16) as usize,
    )
    .filter(|v| *v > 0);
    defaults.perf_mode = toml_string(&root, &["ingestion", "performance", "perf_mode"])
        .unwrap_or_else(|| defaults.perf_mode.clone())
        .to_lowercase();
    defaults.source_cache_enabled = toml_bool(
        &root,
        &["ingestion", "performance", "source_cache_enabled"],
        defaults.source_cache_enabled,
    );
    defaults.source_cache_ttl_secs = toml_u64(
        &root,
        &["ingestion", "performance", "source_cache_ttl_secs"],
        defaults.source_cache_ttl_secs,
    )
    .clamp(60, 86_400);
    defaults.entity_batch_size = toml_u64(
        &root,
        &["ingestion", "performance", "entity_batch_size"],
        defaults.entity_batch_size as u64,
    )
    .clamp(16, 2048) as usize;
    defaults.fact_batch_size = toml_u64(
        &root,
        &["ingestion", "performance", "fact_batch_size"],
        defaults.fact_batch_size as u64,
    )
    .clamp(16, 4096) as usize;
    defaults.strict_fuzzy_dedup = toml_bool(
        &root,
        &["ingestion", "performance", "strict_fuzzy_dedup"],
        false,
    );
    defaults.source_max_inflight = toml_u64(
        &root,
        &["ingestion", "performance", "source_max_inflight"],
        defaults.source_max_inflight as u64,
    )
    .clamp(1, 16) as usize;
    defaults.source_retries = toml_u64(
        &root,
        &["ingestion", "performance", "source_retries"],
        defaults.source_retries as u64,
    )
    .clamp(0, 5) as usize;
    defaults.pdf_host_concurrency = toml_u64(
        &root,
        &["ingestion", "performance", "pdf_host_concurrency"],
        defaults.pdf_host_concurrency as u64,
    )
    .clamp(1, 16) as usize;
    defaults.pdf_parse_cache_enabled = toml_bool(
        &root,
        &["ingestion", "performance", "pdf_parse_cache_enabled"],
        defaults.pdf_parse_cache_enabled,
    );
    defaults.full_text_negative_cache_enabled = toml_bool(
        &root,
        &[
            "ingestion",
            "performance",
            "full_text_negative_cache_enabled",
        ],
        defaults.full_text_negative_cache_enabled,
    );
    defaults.full_text_negative_cache_ttl_secs = toml_u64(
        &root,
        &[
            "ingestion",
            "performance",
            "full_text_negative_cache_ttl_secs",
        ],
        defaults.full_text_negative_cache_ttl_secs,
    )
    .clamp(60, 604_800);
    defaults.chunk_fingerprint_cache_enabled = toml_bool(
        &root,
        &[
            "ingestion",
            "performance",
            "chunk_fingerprint_cache_enabled",
        ],
        defaults.chunk_fingerprint_cache_enabled,
    );
    defaults.chunk_fingerprint_cache_ttl_secs = toml_u64(
        &root,
        &[
            "ingestion",
            "performance",
            "chunk_fingerprint_cache_ttl_secs",
        ],
        defaults.chunk_fingerprint_cache_ttl_secs,
    )
    .clamp(300, 1_209_600);
    defaults.heavy_lane_async_enabled = toml_bool(
        &root,
        &["ingestion", "performance", "heavy_lane_async_enabled"],
        defaults.heavy_lane_async_enabled,
    );
    defaults.min_ner_chars = toml_u64(
        &root,
        &["ingestion", "performance", "min_ner_chars"],
        defaults.min_ner_chars as u64,
    )
    .clamp(120, 5000) as usize;
    defaults.max_relation_genes_per_chunk = toml_u64(
        &root,
        &["ingestion", "performance", "max_relation_genes_per_chunk"],
        defaults.max_relation_genes_per_chunk as u64,
    )
    .clamp(1, 16) as usize;
    defaults.async_post_ingest_scoring = toml_bool(
        &root,
        &["ingestion", "performance", "async_post_ingest_scoring"],
        defaults.async_post_ingest_scoring,
    );
    defaults.source_profile = toml_string(&root, &["ingestion", "performance", "source_profile"])
        .unwrap_or_else(|| "fast".to_string())
        .to_lowercase();
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
        defaults.unpaywall_email =
            first_nonempty_toml_string(&root, &[&["ingestion", "unpaywall", "email"]]);
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
        let model = toml_string(&root, &["embedding", "embedding_model"]).unwrap_or_else(|| {
            "microsoft/BiomedNLP-BiomedBERT-base-uncased-abstract-fulltext".to_string()
        });
        let base_url = toml_string(&root, &["embedding", "base_url"]);
        let batch_size = toml_u64(&root, &["embedding", "batch_size"], 32).clamp(1, 256) as usize;
        let dim = toml_u64(
            &root,
            &["embedding", "embedding_dim"],
            if backend == "rust_native" || backend == "biomedbert" {
                768
            } else {
                1536
            },
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

        let api_key = toml_string(&root, &["embedding", "api_key"])
            .or_else(|| match mapped_backend {
                IngestionEmbeddingBackend::OpenAi => std::env::var("FERRUMYX_OPENAI_API_KEY")
                    .ok()
                    .or_else(|| std::env::var("OPENAI_API_KEY").ok()),
                IngestionEmbeddingBackend::Gemini => std::env::var("FERRUMYX_GEMINI_API_KEY")
                    .ok()
                    .or_else(|| std::env::var("GEMINI_API_KEY").ok()),
                IngestionEmbeddingBackend::OpenAiCompatible => {
                    std::env::var("FERRUMYX_COMPAT_API_KEY")
                        .ok()
                        .or_else(|| std::env::var("LLM_API_KEY").ok())
                }
                _ => None,
            })
            .filter(|s| !s.trim().is_empty());

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

fn build_source_list(profile: &str, include_semantic: bool) -> Vec<IngestionSourceSpec> {
    let mut sources = if profile == "full" {
        vec![
            IngestionSourceSpec::PubMed,
            IngestionSourceSpec::EuropePmc,
            IngestionSourceSpec::Arxiv,
            IngestionSourceSpec::BioRxiv,
            IngestionSourceSpec::MedRxiv,
            IngestionSourceSpec::ClinicalTrials,
        ]
    } else {
        vec![IngestionSourceSpec::PubMed, IngestionSourceSpec::EuropePmc]
    };
    if include_semantic {
        sources.push(IngestionSourceSpec::SemanticScholar);
    }
    sources
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
        let profile = RuntimeProfile::detect_and_prepare();
        std::env::set_var(
            "FERRUMYX_STRICT_FUZZY_DEDUP",
            if defaults.strict_fuzzy_dedup {
                "1"
            } else {
                "0"
            },
        );
        std::env::set_var(
            "FERRUMYX_INGESTION_SOURCE_CACHE_ENABLED",
            if defaults.source_cache_enabled {
                "1"
            } else {
                "0"
            },
        );
        std::env::set_var(
            "FERRUMYX_INGESTION_SOURCE_CACHE_TTL_SECS",
            defaults.source_cache_ttl_secs.to_string(),
        );
        std::env::set_var(
            "FERRUMYX_INGESTION_ENTITY_BATCH_SIZE",
            defaults.entity_batch_size.to_string(),
        );
        std::env::set_var(
            "FERRUMYX_INGESTION_FACT_BATCH_SIZE",
            defaults.fact_batch_size.to_string(),
        );
        std::env::set_var(
            "FERRUMYX_INGESTION_SOURCE_MAX_INFLIGHT",
            defaults.source_max_inflight.to_string(),
        );
        std::env::set_var(
            "FERRUMYX_INGESTION_SOURCE_RETRIES",
            defaults.source_retries.to_string(),
        );
        std::env::set_var(
            "FERRUMYX_PDF_HOST_CONCURRENCY",
            defaults.pdf_host_concurrency.to_string(),
        );
        std::env::set_var(
            "FERRUMYX_PDF_PARSE_CACHE_ENABLED",
            if defaults.pdf_parse_cache_enabled {
                "1"
            } else {
                "0"
            },
        );
        std::env::set_var(
            "FERRUMYX_FULLTEXT_NEGATIVE_CACHE_ENABLED",
            if defaults.full_text_negative_cache_enabled {
                "1"
            } else {
                "0"
            },
        );
        std::env::set_var(
            "FERRUMYX_FULLTEXT_NEGATIVE_CACHE_TTL_SECS",
            defaults.full_text_negative_cache_ttl_secs.to_string(),
        );
        std::env::set_var(
            "FERRUMYX_CHUNK_FINGERPRINT_CACHE_ENABLED",
            if defaults.chunk_fingerprint_cache_enabled {
                "1"
            } else {
                "0"
            },
        );
        std::env::set_var(
            "FERRUMYX_CHUNK_FINGERPRINT_CACHE_TTL_SECS",
            defaults.chunk_fingerprint_cache_ttl_secs.to_string(),
        );
        std::env::set_var(
            "FERRUMYX_INGESTION_HEAVY_LANE_ASYNC",
            if defaults.heavy_lane_async_enabled {
                "1"
            } else {
                "0"
            },
        );
        std::env::set_var(
            "FERRUMYX_INGESTION_MIN_NER_CHARS",
            defaults.min_ner_chars.to_string(),
        );
        if let Some(v) = defaults.full_text_total_timeout_secs {
            std::env::set_var(
                "FERRUMYX_INGESTION_FULLTEXT_TOTAL_TIMEOUT_SECS",
                v.to_string(),
            );
        }
        std::env::set_var(
            "FERRUMYX_INGESTION_MAX_RELATION_GENES_PER_CHUNK",
            defaults.max_relation_genes_per_chunk.to_string(),
        );
        std::env::set_var(
            "FERRUMYX_INGESTION_ASYNC_POST_SCORE",
            if defaults.async_post_ingest_scoring {
                "1"
            } else {
                "0"
            },
        );

        let gene = require_str(&params, "gene")?.to_string();
        let gene_for_refresh = gene.clone();
        let cancer_type = require_str(&params, "cancer_type")?.to_string();

        let mutation = params
            .get("mutation")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let requested_max_results = params
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(defaults.max_results)
            .clamp(1, 5000);
        let perf_mode = match defaults.perf_mode.as_str() {
            "throughput" => "throughput",
            "balanced" => "balanced",
            "safe" => "safe",
            _ => "auto",
        };
        let max_results = {
            let base = profile.tuned_max_results(requested_max_results);
            match perf_mode {
                "throughput" => (base.saturating_mul(2)).clamp(1, 5000),
                "safe" => (base / 2).max(1),
                _ => base,
            }
        };
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

        let has_semantic_key = defaults
            .semantic_scholar_api_key
            .as_ref()
            .is_some_and(|k| !k.trim().is_empty());
        let mut embedding_cfg = defaults.embedding_cfg.clone();
        if let Some(cfg) = embedding_cfg.as_mut() {
            cfg.batch_size = profile.tuned_embedding_batch_size(cfg.batch_size);
        }

        let source_timeout_secs = match perf_mode {
            "throughput" => defaults
                .source_timeout_secs
                .or(Some(profile.source_timeout_secs().saturating_sub(4).max(8))),
            "safe" => defaults
                .source_timeout_secs
                .or(Some(profile.source_timeout_secs().saturating_add(8))),
            _ => defaults
                .source_timeout_secs
                .or(Some(profile.source_timeout_secs())),
        };
        let full_text_prefetch_workers = defaults.full_text_prefetch_workers.or_else(|| {
            let cpus = profile.logical_cpus.max(1);
            let v = match perf_mode {
                "throughput" => (cpus / 2).clamp(2, 12),
                "safe" => (cpus / 4).clamp(1, 4),
                _ => (cpus / 3).clamp(2, 8),
            };
            Some(v)
        });
        let paper_process_workers = defaults.paper_process_workers.or_else(|| {
            let cpus = profile.logical_cpus.max(1);
            let v = match perf_mode {
                "throughput" => (cpus / 2).clamp(2, 16),
                "safe" => (cpus / 4).clamp(1, 6),
                _ => (cpus / 3).clamp(2, 10),
            };
            Some(v)
        });
        if let Some(ppw) = paper_process_workers {
            std::env::set_var("FERRUMYX_PAPER_PROCESS_WORKERS", ppw.to_string());
        }

        let job = IngestionJob {
            gene,
            mutation,
            cancer_type,
            max_results,
            sources: build_source_list(&defaults.source_profile, has_semantic_key),
            pubmed_api_key: defaults.pubmed_api_key,
            semantic_scholar_api_key: defaults.semantic_scholar_api_key,
            unpaywall_email: defaults.unpaywall_email,
            embedding_cfg,
            enable_scihub_fallback: false,
            full_text_enabled: profile.use_full_text_default(),
            source_timeout_secs,
            full_text_step_timeout_secs: defaults.full_text_step_timeout_secs,
            full_text_prefetch_workers,
            source_cache_enabled: defaults.source_cache_enabled,
            source_cache_ttl_secs: Some(defaults.source_cache_ttl_secs),
        };

        let repo = Arc::new(IngestionRepository::new(self.db.clone()));
        let (progress_tx, mut progress_rx) = broadcast::channel::<IngestionProgress>(512);
        let ingest_repo = repo.clone();
        let ingest_task =
            tokio::spawn(async move { run_ingestion(job, ingest_repo, Some(progress_tx)).await });

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
        let mut recomputed = 0u32;
        let mut provider_refreshed_genes = 0usize;
        let mut provider_errors = 0u64;
        let scoring_mode = if defaults.async_post_ingest_scoring {
            let db_for_task = self.db.clone();
            tokio::spawn(async move {
                let recompute = ferrumyx_kg::compute_target_scores(db_for_task.clone())
                    .await
                    .unwrap_or(0);
                let provider_refresh = TargetQueryEngine::new(db_for_task.clone())
                    .refresh_provider_signals(ProviderRefreshRequest {
                        genes: vec![gene_for_refresh],
                        cancer_code: None,
                        max_genes: 8,
                        batch_size: 4,
                        retries: 1,
                    })
                    .await
                    .ok();
                let refreshed = provider_refresh
                    .as_ref()
                    .map(|r| r.genes_processed)
                    .unwrap_or(0);
                let errors = provider_refresh
                    .as_ref()
                    .map(|r| {
                        (r.gtex_failed + r.tcga_failed + r.chembl_failed + r.reactome_failed) as u64
                    })
                    .unwrap_or(0);
                tracing::info!(
                    recomputed = recompute,
                    provider_refreshed_genes = refreshed,
                    provider_errors = errors,
                    "Async post-ingestion scoring/refresh completed"
                );
            });
            "async_queued"
        } else {
            recomputed = ferrumyx_kg::compute_target_scores(self.db.clone())
                .await
                .unwrap_or(0);
            let provider_refresh = TargetQueryEngine::new(self.db.clone())
                .refresh_provider_signals(ProviderRefreshRequest {
                    genes: vec![gene_for_refresh],
                    cancer_code: None,
                    max_genes: 8,
                    batch_size: 4,
                    retries: 1,
                })
                .await
                .ok();
            provider_refreshed_genes = provider_refresh
                .as_ref()
                .map(|r| r.genes_processed)
                .unwrap_or(0);
            provider_errors = provider_refresh
                .as_ref()
                .map(|r| {
                    (r.gtex_failed + r.tcga_failed + r.chembl_failed + r.reactome_failed) as u64
                })
                .unwrap_or(0);
            "sync"
        };

        let output_text = format!(
            "Ingestion completed in {}ms. Source fetch returned {} papers, {} unique after cross-source dedupe. Inserted {} new papers and {} knowledge chunks into LanceDB. Skipped {} existing duplicates. Recomputed {} target scores. Provider refresh processed {} genes (errors={}). Post-ingestion scoring mode={}. Watchdog policy: idle={}s, max_runtime={}s. Runtime profile: ram={:.1}GB, cpu_logical={}, nvidia_gpu={}, cuda_toolkit={}, cuda_install_attempted={}, perf_mode={}, tuned_max_results={}, full_text_enabled={}, source_timeout_secs={}, prefetch_workers={:?}, paper_workers={:?}, source_cache_enabled={}, source_cache_ttl_secs={}, entity_batch_size={}, fact_batch_size={}. Source telemetry: {}",
            result.duration_ms,
            result.papers_found_raw,
            result.papers_found,
            result.papers_inserted,
            result.chunks_inserted,
            result.papers_duplicate,
            recomputed,
            provider_refreshed_genes,
            provider_errors,
            scoring_mode,
            idle_timeout.as_secs(),
            max_runtime.as_secs(),
            profile.ram_gb,
            profile.logical_cpus,
            profile.has_nvidia_gpu,
            profile.has_cuda_toolkit,
            profile.cuda_install_attempted,
            perf_mode,
            max_results,
            profile.use_full_text_default(),
            source_timeout_secs.unwrap_or_else(|| profile.source_timeout_secs()),
            full_text_prefetch_workers,
            paper_process_workers,
            defaults.source_cache_enabled,
            defaults.source_cache_ttl_secs,
            defaults.entity_batch_size,
            defaults.fact_batch_size,
            result
                .source_telemetry
                .iter()
                .map(|s| format!("{}:fetched={},err={}", s.source, s.fetched, s.error.clone().unwrap_or_else(|| "none".to_string())))
                .collect::<Vec<_>>()
                .join("; "),
        );

        Ok(ToolOutput::text(
            output_text,
            Duration::from_millis(result.duration_ms),
        ))
    }
}
