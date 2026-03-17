use async_trait::async_trait;
use ironclaw::context::JobContext;
use ironclaw::tools::{Tool, ToolError, ToolOutput};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

use super::runtime_profile::RuntimeProfile;
use ferrumyx_common::query::QueryRequest;
use ferrumyx_db::Database;
use ferrumyx_ingestion::embedding::{
    EmbeddingBackend as IngestionEmbeddingBackend, EmbeddingConfig as IngestionEmbeddingConfig,
};
use ferrumyx_ingestion::pipeline::{run_ingestion, IngestionJob, IngestionSourceSpec};
use ferrumyx_ingestion::repository::IngestionRepository;
use ferrumyx_ranker::{ProviderRefreshRequest, TargetQueryEngine};

/// Tool to run a bounded autonomous loop over ingestion -> scoring -> ranking.
pub struct AutonomousCycleTool {
    db: Arc<Database>,
}

impl AutonomousCycleTool {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminationReason {
    MaxCycles,
    Plateau,
}

impl TerminationReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::MaxCycles => "max_cycles_reached",
            Self::Plateau => "dynamic_plateau",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NoveltyPressureMode {
    Off,
    Auto,
    Aggressive,
}

impl NoveltyPressureMode {
    fn from_str(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "off" | "disabled" | "none" => Self::Off,
            "aggressive" | "high" => Self::Aggressive,
            _ => Self::Auto,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Auto => "auto",
            Self::Aggressive => "aggressive",
        }
    }
}

fn config_path() -> PathBuf {
    std::env::var("FERRUMYX_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("ferrumyx.toml"))
}

fn resolve_pubmed_api_key() -> Option<String> {
    if let Ok(v) = std::env::var("FERRUMYX_PUBMED_API_KEY") {
        let t = v.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    let content = fs::read_to_string(config_path()).ok()?;
    let root = toml::from_str::<toml::Value>(&content).ok()?;
    root.get("ingestion")
        .and_then(|v| v.get("pubmed"))
        .and_then(|v| v.get("api_key").or_else(|| v.get("api_key_secret")))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

fn resolve_semantic_scholar_api_key() -> Option<String> {
    if let Ok(v) = std::env::var("FERRUMYX_SEMANTIC_SCHOLAR_API_KEY") {
        let t = v.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    if let Ok(v) = std::env::var("SEMANTIC_SCHOLAR_API_KEY") {
        let t = v.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    let content = fs::read_to_string(config_path()).ok()?;
    let root = toml::from_str::<toml::Value>(&content).ok()?;
    root.get("ingestion")
        .and_then(|v| v.get("semanticscholar"))
        .and_then(|v| v.get("api_key").or_else(|| v.get("api_key_secret")))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

fn resolve_unpaywall_email() -> Option<String> {
    if let Ok(v) = std::env::var("FERRUMYX_UNPAYWALL_EMAIL") {
        let t = v.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    let content = fs::read_to_string(config_path()).ok()?;
    let root = toml::from_str::<toml::Value>(&content).ok()?;
    root.get("ingestion")
        .and_then(|v| v.get("unpaywall"))
        .and_then(|v| v.get("email"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
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

fn resolve_default_embedding_cfg() -> Option<IngestionEmbeddingConfig> {
    let path = config_path();
    let content = fs::read_to_string(path).ok()?;
    let root = toml::from_str::<toml::Value>(&content).ok()?;
    let enabled = toml_bool(
        &root,
        &["ingestion", "enable_embeddings"],
        std::env::var("FERRUMYX_INGESTION_ENABLE_EMBEDDINGS")
            .ok()
            .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true")),
    );
    if !enabled {
        return None;
    }

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
            IngestionEmbeddingBackend::OpenAiCompatible => std::env::var("FERRUMYX_COMPAT_API_KEY")
                .ok()
                .or_else(|| std::env::var("LLM_API_KEY").ok()),
            _ => None,
        })
        .filter(|v| !v.trim().is_empty());

    Some(IngestionEmbeddingConfig {
        backend: mapped_backend,
        api_key,
        model,
        dim,
        batch_size,
        base_url,
    })
}

fn build_source_list(
    profile_name: &str,
    include_semantic: bool,
    include_crossref: bool,
) -> Vec<IngestionSourceSpec> {
    if profile_name == "full" {
        let mut sources = vec![
            IngestionSourceSpec::PubMed,
            IngestionSourceSpec::EuropePmc,
            IngestionSourceSpec::Arxiv,
            IngestionSourceSpec::BioRxiv,
            IngestionSourceSpec::MedRxiv,
            IngestionSourceSpec::ClinicalTrials,
        ];
        if include_semantic {
            sources.push(IngestionSourceSpec::SemanticScholar);
        }
        if include_crossref {
            sources.push(IngestionSourceSpec::CrossRef);
        }
        sources
    } else {
        let mut sources = vec![IngestionSourceSpec::PubMed, IngestionSourceSpec::EuropePmc];
        if include_semantic {
            sources.push(IngestionSourceSpec::SemanticScholar);
        }
        if include_crossref {
            sources.push(IngestionSourceSpec::CrossRef);
        }
        sources
    }
}

#[async_trait]
impl Tool for AutonomousCycleTool {
    fn name(&self) -> &str {
        "run_autonomous_cycle"
    }

    fn description(&self) -> &str {
        "Runs iterative autonomous discovery cycles and stops when ranking score gain plateaus."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "gene": { "type": "string", "description": "Gene symbol (for example KRAS)" },
                "cancer_type": { "type": "string", "description": "Cancer type text (for example pancreatic cancer)" },
                "query_text": { "type": "string", "description": "Research question used for ranking output" },
                "cancer_code": { "type": "string", "description": "OncoTree-like cancer code (for example PAAD)" },
                "mutation": { "type": "string", "description": "Optional mutation (for example G12D)" },
                "max_results": { "type": "integer", "description": "Per-cycle ingestion paper cap (default: 10)" },
                "source_profile": {
                    "type": "string",
                    "description": "Ingestion source profile: auto|fast|full (default: auto)"
                },
                "max_cycles": { "type": "integer", "description": "Runtime safety cap on autonomous loops (default: 8, max: 20)" },
                "improvement_threshold": {
                    "type": "number",
                    "description": "Optional absolute top-score gain floor override (otherwise dynamic trend-based stop is used)"
                },
                "adaptive_broadening": {
                    "type": "boolean",
                    "description": "Automatically broaden search when low-yield cycles are detected (default: true)"
                },
                "novelty_pressure_mode": {
                    "type": "string",
                    "description": "Adapts retrieval when DB duplicate pressure is high: off|auto|aggressive (default: auto)"
                },
                "cycle_timeout_secs": {
                    "type": "integer",
                    "description": "Per-cycle watchdog timeout in seconds across ingestion/scoring/ranking (default: 1800)"
                }
            },
            "required": ["gene", "cancer_type"]
        })
    }

    fn execution_timeout(&self) -> Duration {
        // Autonomous loop may run ingestion + scoring + provider refresh for multiple cycles.
        Duration::from_secs(4 * 60 * 60)
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let gene = require_str(&params, "gene")?.to_string();
        let cancer_type = require_str(&params, "cancer_type")?.to_string();
        let query_text = params
            .get("query_text")
            .and_then(|v| v.as_str())
            .unwrap_or("Prioritize actionable cancer targets using current evidence.")
            .to_string();
        let cancer_code = params
            .get("cancer_code")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let mutation = params
            .get("mutation")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let max_results = params
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(10)
            .clamp(1, 200);
        let source_profile_in = params
            .get("source_profile")
            .and_then(|v| v.as_str())
            .unwrap_or("auto")
            .to_lowercase();
        let max_cycles = params
            .get("max_cycles")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(8)
            .clamp(1, 20);
        let user_improvement_floor = params
            .get("improvement_threshold")
            .and_then(|v| v.as_f64())
            .map(|v| v.clamp(0.0, 0.5));
        let adaptive_broadening = params
            .get("adaptive_broadening")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let novelty_pressure_mode = NoveltyPressureMode::from_str(
            params
                .get("novelty_pressure_mode")
                .and_then(|v| v.as_str())
                .unwrap_or("auto"),
        );
        let cycle_timeout = Duration::from_secs(
            params
                .get("cycle_timeout_secs")
                .and_then(|v| v.as_u64())
                .unwrap_or(1_800)
                .clamp(120, 7_200),
        );

        let started = std::time::Instant::now();
        let repo = Arc::new(IngestionRepository::new(self.db.clone()));
        let ranker = TargetQueryEngine::new(self.db.clone());
        let pubmed_api_key = resolve_pubmed_api_key();
        let semantic_scholar_api_key = resolve_semantic_scholar_api_key();
        let unpaywall_email = resolve_unpaywall_email();
        let embedding_cfg = resolve_default_embedding_cfg();
        let profile = RuntimeProfile::detect_and_prepare();
        let source_profile = if source_profile_in == "auto" {
            if profile.use_full_text_default() {
                "full".to_string()
            } else {
                "fast".to_string()
            }
        } else {
            source_profile_in
        };
        let tuned_max_results = profile.tuned_max_results(max_results);
        let has_semantic_key = semantic_scholar_api_key
            .as_ref()
            .is_some_and(|k| !k.trim().is_empty());
        let mut embedding_cfg = embedding_cfg;
        if let Some(cfg) = embedding_cfg.as_mut() {
            cfg.batch_size = profile.tuned_embedding_batch_size(cfg.batch_size);
        }

        let mut cycles = Vec::new();
        let mut previous_top_score = 0.0_f64;
        let mut total_papers_inserted = 0usize;
        let mut total_chunks_inserted = 0usize;
        let mut total_typed_relations = 0usize;
        let mut stagnant_cycles = 0usize;
        let mut dynamic_patience = 2usize;
        let mut low_yield_streak = 0usize;
        let mut adaptive_max_results = tuned_max_results;
        let mut adaptive_mutation = mutation.clone();
        let mut active_source_profile = source_profile.clone();
        let mut adaptive_include_crossref = false;
        let mut adaptive_source_cache_enabled = true;
        let mut adaptive_source_cache_ttl_secs = Some(30 * 60);
        let mut novelty_pressure_activations = 0usize;
        let mut max_score_gain = 0.0_f64;
        let mut max_evidence_gain = 0.0_f64;
        let mut max_novelty_ratio = 0.0_f64;
        let mut termination_reason = TerminationReason::MaxCycles;

        for cycle in 1..=max_cycles {
            let cycle_sources = build_source_list(
                &active_source_profile,
                has_semantic_key,
                adaptive_include_crossref,
            );
            let cycle_max_results = adaptive_max_results;
            let cycle_mutation = adaptive_mutation.clone();
            let cycle_source_cache_enabled = adaptive_source_cache_enabled;
            let cycle_source_cache_ttl_secs = adaptive_source_cache_ttl_secs;
            let ingest = timeout(
                cycle_timeout,
                run_ingestion(
                    IngestionJob {
                        gene: gene.clone(),
                        mutation: cycle_mutation.clone(),
                        cancer_type: cancer_type.clone(),
                        max_results: cycle_max_results,
                        sources: cycle_sources,
                        pubmed_api_key: pubmed_api_key.clone(),
                        semantic_scholar_api_key: semantic_scholar_api_key.clone(),
                        unpaywall_email: unpaywall_email.clone(),
                        embedding_cfg: embedding_cfg.clone(),
                        enable_scihub_fallback: false,
                        full_text_enabled: active_source_profile == "full",
                        source_timeout_secs: Some(profile.source_timeout_secs()),
                        full_text_step_timeout_secs: Some(15),
                        full_text_prefetch_workers: None,
                        source_cache_enabled: cycle_source_cache_enabled,
                        source_cache_ttl_secs: cycle_source_cache_ttl_secs,
                    },
                    repo.clone(),
                    None,
                ),
            )
            .await
            .map_err(|_| {
                ToolError::ExecutionFailed(format!(
                    "cycle {cycle} ingestion exceeded watchdog timeout ({}s)",
                    cycle_timeout.as_secs()
                ))
            })?;

            // Recompute across discovered genes so autonomous runs can surface
            // alternatives beyond the initial seed gene.
            let recomputed = timeout(
                cycle_timeout,
                ferrumyx_kg::compute_target_scores(self.db.clone()),
            )
            .await
            .map_err(|_| {
                ToolError::ExecutionFailed(format!(
                    "cycle {cycle} scoring exceeded watchdog timeout ({}s)",
                    cycle_timeout.as_secs()
                ))
            })?
            .map_err(|e| {
                ToolError::ExecutionFailed(format!("cycle {cycle} scoring failed: {e}"))
            })?;

            let refresh = timeout(
                cycle_timeout,
                ranker.refresh_provider_signals(ProviderRefreshRequest {
                    genes: vec![gene.clone()],
                    cancer_code: cancer_code.clone(),
                    max_genes: 8,
                    batch_size: 4,
                    retries: 1,
                    offline_strict: false,
                }),
            )
            .await
            .map_err(|_| {
                ToolError::ExecutionFailed(format!(
                    "cycle {cycle} provider refresh exceeded watchdog timeout ({}s)",
                    cycle_timeout.as_secs()
                ))
            })?
            .map_err(|e| {
                ToolError::ExecutionFailed(format!("cycle {cycle} provider refresh failed: {e}"))
            })?;

            let query = QueryRequest {
                query_text: query_text.clone(),
                cancer_code: cancer_code.clone(),
                gene_symbol: None,
                mutation: mutation.clone(),
                max_results: 10,
            };
            let ranked = timeout(cycle_timeout, ranker.execute_query(query))
                .await
                .map_err(|_| {
                    ToolError::ExecutionFailed(format!(
                        "cycle {cycle} ranking exceeded watchdog timeout ({}s)",
                        cycle_timeout.as_secs()
                    ))
                })?
                .map_err(|e| {
                    ToolError::ExecutionFailed(format!("cycle {cycle} ranking failed: {e}"))
                })?;
            let top_score = ranked.first().map(|r| r.composite_score).unwrap_or(0.0);
            let improvement = top_score - previous_top_score;
            let score_gain = improvement.max(0.0);
            let typed_relations_this_cycle = ingest.perf_telemetry.typed_relation_fact_count;
            let novelty_ratio = if ingest.papers_found > 0 {
                ingest.papers_inserted as f64 / ingest.papers_found as f64
            } else {
                0.0
            };
            let duplicate_pressure = if ingest.papers_found > 0 {
                ingest.papers_duplicate as f64 / ingest.papers_found as f64
            } else {
                0.0
            };
            let evidence_gain = ingest.papers_inserted as f64
                + (ingest.chunks_inserted as f64 * 0.20)
                + (typed_relations_this_cycle as f64 * 0.05);
            total_papers_inserted += ingest.papers_inserted;
            total_chunks_inserted += ingest.chunks_inserted;
            total_typed_relations += typed_relations_this_cycle;

            if score_gain > max_score_gain {
                max_score_gain = score_gain;
            }
            if evidence_gain > max_evidence_gain {
                max_evidence_gain = evidence_gain;
            }
            if novelty_ratio > max_novelty_ratio {
                max_novelty_ratio = novelty_ratio;
            }

            dynamic_patience = if max_novelty_ratio > 0.30 || max_evidence_gain > 8.0 {
                3
            } else {
                2
            };
            let score_floor = user_improvement_floor.unwrap_or_else(|| {
                if max_score_gain > 0.0 {
                    max_score_gain * 0.15
                } else {
                    0.0
                }
            });
            let evidence_floor = if max_evidence_gain > 0.0 {
                max_evidence_gain * 0.20
            } else {
                0.0
            };
            let novelty_floor = if max_novelty_ratio > 0.0 {
                max_novelty_ratio * 0.20
            } else {
                0.0
            };
            let stagnating_cycle = cycle > 1
                && score_gain <= score_floor
                && evidence_gain <= evidence_floor
                && novelty_ratio <= novelty_floor;
            if cycle > 1 {
                if stagnating_cycle {
                    stagnant_cycles += 1;
                } else {
                    stagnant_cycles = 0;
                }
            }
            let cross_source_dedup_dropped =
                ingest.papers_found_raw.saturating_sub(ingest.papers_found);
            let novelty_pressure_score =
                (duplicate_pressure * 1.25 - novelty_ratio).clamp(0.0, 1.0);
            let novelty_pressure_triggered = match novelty_pressure_mode {
                NoveltyPressureMode::Off => false,
                NoveltyPressureMode::Auto => {
                    duplicate_pressure >= 0.65
                        && novelty_ratio <= 0.25
                        && novelty_pressure_score >= 0.30
                }
                NoveltyPressureMode::Aggressive => {
                    duplicate_pressure >= 0.50 && novelty_pressure_score >= 0.20
                }
            };
            if novelty_pressure_triggered {
                novelty_pressure_activations += 1;
            }

            cycles.push(json!({
                "cycle": cycle,
                "search_scope": {
                    "source_profile": active_source_profile,
                    "mutation": cycle_mutation,
                    "max_results": cycle_max_results,
                    "source_cache_enabled": cycle_source_cache_enabled,
                    "source_cache_ttl_secs": cycle_source_cache_ttl_secs
                },
                "ingestion": {
                    "papers_found_raw": ingest.papers_found_raw,
                    "papers_found": ingest.papers_found,
                    "cross_source_dedup_dropped": cross_source_dedup_dropped,
                    "papers_inserted": ingest.papers_inserted,
                    "papers_duplicate": ingest.papers_duplicate,
                    "chunks_inserted": ingest.chunks_inserted,
                    "duration_ms": ingest.duration_ms,
                    "typed_relation_facts": typed_relations_this_cycle,
                    "unique_predicate_count": ingest.perf_telemetry.unique_predicate_count
                },
                "scoring": {
                    "target_scores_upserted": recomputed
                },
                "provider_refresh": refresh,
                "ranking": {
                    "top_score": top_score,
                    "top_gene": ranked.first().map(|r| r.gene_symbol.clone()),
                    "result_count": ranked.len()
                },
                "improvement": improvement,
                "dynamic_plateau": {
                    "score_gain": score_gain,
                    "score_gain_floor": score_floor,
                    "evidence_gain": evidence_gain,
                    "evidence_gain_floor": evidence_floor,
                    "novelty_ratio": novelty_ratio,
                    "novelty_floor": novelty_floor,
                    "stagnating_cycle": stagnating_cycle,
                    "stagnant_cycles": stagnant_cycles,
                    "patience": dynamic_patience
                },
                "novelty_pressure": {
                    "mode": novelty_pressure_mode.as_str(),
                    "duplicate_pressure": duplicate_pressure,
                    "novelty_ratio": novelty_ratio,
                    "pressure_score": novelty_pressure_score,
                    "triggered": novelty_pressure_triggered
                },
                "stagnant_cycles": stagnant_cycles,
                "evidence_totals": {
                    "papers_inserted": total_papers_inserted,
                    "chunks_inserted": total_chunks_inserted,
                    "typed_relations": total_typed_relations
                }
            }));

            let low_yield = ingest.papers_inserted <= 1 && ingest.chunks_inserted <= 4;
            if low_yield {
                low_yield_streak += 1;
            } else {
                low_yield_streak = 0;
            }

            if adaptive_broadening {
                if low_yield {
                    adaptive_max_results = ((adaptive_max_results as f64) * 1.5).ceil() as usize;
                    adaptive_max_results = adaptive_max_results.clamp(1, 400);
                    if adaptive_mutation.is_some() && cycle >= 2 {
                        adaptive_mutation = None;
                    }
                }
                if low_yield_streak >= 2 && active_source_profile == "fast" {
                    active_source_profile = "full".to_string();
                }
            }
            if novelty_pressure_triggered {
                adaptive_max_results = match novelty_pressure_mode {
                    NoveltyPressureMode::Aggressive => {
                        ((adaptive_max_results as f64) * 2.2).ceil() as usize
                    }
                    NoveltyPressureMode::Auto => {
                        ((adaptive_max_results as f64) * 1.8).ceil() as usize
                    }
                    NoveltyPressureMode::Off => adaptive_max_results,
                }
                .clamp(1, 700);
                adaptive_include_crossref = true;
                adaptive_source_cache_enabled = false;
                adaptive_source_cache_ttl_secs = Some(5 * 60);
                if adaptive_mutation.is_some() {
                    adaptive_mutation = None;
                }
                if active_source_profile == "fast" {
                    active_source_profile = "full".to_string();
                }
            } else if adaptive_source_cache_enabled {
                adaptive_source_cache_ttl_secs = Some(30 * 60);
            } else {
                // Restore cache after one forced-uncached cycle.
                adaptive_source_cache_enabled = true;
                adaptive_source_cache_ttl_secs = Some(30 * 60);
            }

            if cycle > 1 && stagnant_cycles >= dynamic_patience {
                termination_reason = TerminationReason::Plateau;
                break;
            }
            previous_top_score = top_score;
        }

        Ok(ToolOutput::success(
            json!({
                "status": "ok",
                "gene": gene,
                "cancer_type": cancer_type,
                "cycles": cycles,
                "termination_reason": termination_reason.as_str(),
                "evidence_summary": {
                    "papers_inserted_total": total_papers_inserted,
                    "chunks_inserted_total": total_chunks_inserted,
                    "typed_relations_total": total_typed_relations,
                    "peak_score_gain": max_score_gain,
                    "peak_evidence_gain": max_evidence_gain,
                    "peak_novelty_ratio": max_novelty_ratio,
                    "final_dynamic_patience": dynamic_patience
                },
                "adaptive_strategy": {
                    "enabled": adaptive_broadening,
                    "source_profile_initial": source_profile,
                    "source_profile_final": active_source_profile,
                    "user_improvement_floor": user_improvement_floor,
                    "novelty_pressure_mode": novelty_pressure_mode.as_str(),
                    "novelty_pressure_activations": novelty_pressure_activations,
                    "crossref_enabled_final": adaptive_include_crossref
                },
                "runtime_profile": {
                    "ram_gb": profile.ram_gb,
                    "logical_cpus": profile.logical_cpus,
                    "has_nvidia_gpu": profile.has_nvidia_gpu,
                    "has_cuda_toolkit": profile.has_cuda_toolkit,
                    "cuda_install_attempted": profile.cuda_install_attempted,
                    "source_profile": active_source_profile,
                    "max_results_tuned": tuned_max_results,
                    "source_timeout_secs": profile.source_timeout_secs()
                }
            }),
            started.elapsed(),
        ))
    }
}

fn require_str<'a>(params: &'a serde_json::Value, name: &str) -> Result<&'a str, ToolError> {
    params
        .get(name)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            ToolError::InvalidParameters(format!("missing required string parameter: {name}"))
        })
}
