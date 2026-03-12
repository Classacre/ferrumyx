//! End-to-end ingestion pipeline.
//!
//! Orchestrates the full flow for a single ingestion job:
//!   1. Build query from gene/mutation/cancer params
//!   2. Search each enabled source (PubMed, Europe PMC, …)
//!   3. Deduplicate by DOI/PMID/SimHash
//!   4. Upsert papers to LanceDB
//!   5. Fetch full-text PDFs where available (open access)
//!   6. Parse PDFs with Ferrules (Rust-native) for section-aware extraction
//!   7. Chunk documents (abstract + full-text sections)
//!   8. Bulk insert chunks
//!   9. Embed chunks if configured
//!   10. Emit progress events via broadcast channel
//!
//! The pipeline is designed to be called from both the IronClaw tool
//! (`ferrumyx-agent/src/tools/ingestion_tool.rs`) and the web API.

use serde::{Deserialize, Serialize};
use std::collections::{hash_map::DefaultHasher, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use tempfile::NamedTempFile;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::OnceCell;
use tokio::sync::Semaphore;
use tokio::time::sleep;
use tokio::time::timeout;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use crate::chunker::{chunk_document, ChunkerConfig, DocumentSection};
use crate::embedding::{embed_pending_chunks, EmbeddingClient, EmbeddingConfig};
use crate::models::SectionType;
use crate::pdf_parser::parse_pdf_sections;
use crate::repository::IngestionRepository;
use crate::sources::arxiv::ArxivClient;
use crate::sources::biorxiv::BioRxivClient;
use crate::sources::clinicaltrials::ClinicalTrialsClient;
use crate::sources::crossref::CrossRefClient;
use crate::sources::europepmc::EuropePmcClient;
use crate::sources::pubmed::PubMedClient;
use crate::sources::semanticscholar::SemanticScholarClient;
use crate::sources::unpaywall::UnpaywallClient;
use crate::sources::LiteratureSource;
use ferrumyx_db::entities::EntityRepository;
use ferrumyx_db::schema::{Entity as DbEntity, EntityType as DbEntityType, KgFact};
use ferrumyx_kg::extraction::build_facts;
use ferrumyx_kg::ner::{EntityType as NerEntityType, TrieNer};
use sha2::{Digest, Sha256};

static SHARED_NER: OnceCell<Arc<TrieNer>> = OnceCell::const_new();
static PDF_HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static PDF_HOST_LIMITS: OnceLock<std::sync::Mutex<HashMap<String, Arc<Semaphore>>>> =
    OnceLock::new();

#[derive(Debug, Serialize, Deserialize)]
struct ParsedPdfCacheEntry {
    parse_ok: bool,
    sections: Vec<DocumentSection>,
}

async fn get_or_init_ner() -> anyhow::Result<Arc<TrieNer>> {
    SHARED_NER
        .get_or_try_init(|| async {
            info!("Initializing shared NER cache (HGNC + OncoTree)...");
            let ner = tokio::task::spawn_blocking(TrieNer::with_complete_databases)
                .await
                .map_err(|e| anyhow::anyhow!("NER init join error: {e}"))??;
            Ok::<Arc<TrieNer>, anyhow::Error>(Arc::new(ner))
        })
        .await
        .cloned()
}

#[derive(Debug, Default)]
struct PaperProcessingResult {
    chunks_inserted: usize,
    chunks_embedded: usize,
    quality_gate_skipped: bool,
    errors: Vec<String>,
}

// ── Job config ────────────────────────────────────────────────────────────────

/// Parameters for a single ingestion run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionJob {
    pub gene: String,
    pub mutation: Option<String>,
    pub cancer_type: String,
    pub max_results: usize,
    pub sources: Vec<IngestionSourceSpec>,
    /// Optional NCBI API key for higher rate limits.
    pub pubmed_api_key: Option<String>,
    /// Optional Semantic Scholar API key for higher throughput/quotas.
    pub semantic_scholar_api_key: Option<String>,
    /// Optional Unpaywall contact email used for DOI OA resolution.
    pub unpaywall_email: Option<String>,
    /// If provided, chunks are embedded immediately after insert.
    /// If None, a separate embed pass is needed (e.g. scheduled background job).
    pub embedding_cfg: Option<EmbeddingConfig>,
    /// Whether to attempt downloading paywalled PDFs via Sci-Hub.
    /// WARNING: Use at your own risk. Disabled by default.
    pub enable_scihub_fallback: bool,
    /// Enable full-text retrieval/parsing (PDF/XML tiers). Disable for fast abstract-first cycles.
    pub full_text_enabled: bool,
    /// Per-source network timeout (seconds) for search calls.
    /// Prevents a single upstream API stall from blocking the full run.
    pub source_timeout_secs: Option<u64>,
    /// Timeout budget per full-text strategy step (seconds).
    pub full_text_step_timeout_secs: Option<u64>,
    /// Bounded concurrency for full-text prefetch workers.
    /// If None, runtime auto-tuning decides.
    pub full_text_prefetch_workers: Option<usize>,
    /// Enable persistent per-source search cache.
    pub source_cache_enabled: bool,
    /// TTL for source search cache entries.
    pub source_cache_ttl_secs: Option<u64>,
}

/// Which literature sources to search.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum IngestionSourceSpec {
    PubMed,
    EuropePmc,
    BioRxiv,
    MedRxiv,
    Arxiv,
    ClinicalTrials,
    CrossRef,
    SemanticScholar,
}

impl Default for IngestionJob {
    fn default() -> Self {
        Self {
            gene: "KRAS".to_string(),
            mutation: Some("G12D".to_string()),
            cancer_type: "pancreatic cancer".to_string(),
            max_results: 100,
            sources: vec![
                IngestionSourceSpec::PubMed,
                IngestionSourceSpec::EuropePmc,
                IngestionSourceSpec::BioRxiv,
                IngestionSourceSpec::MedRxiv,
                IngestionSourceSpec::Arxiv,
                IngestionSourceSpec::ClinicalTrials,
                IngestionSourceSpec::CrossRef,
            ],
            pubmed_api_key: None,
            semantic_scholar_api_key: None,
            unpaywall_email: None,
            embedding_cfg: None,
            enable_scihub_fallback: false,
            full_text_enabled: true,
            source_timeout_secs: Some(45),
            full_text_step_timeout_secs: Some(15),
            full_text_prefetch_workers: None,
            source_cache_enabled: true,
            source_cache_ttl_secs: Some(30 * 60),
        }
    }
}

// ── Progress events ───────────────────────────────────────────────────────────

/// Progress event emitted during a pipeline run (cloneable for broadcast).
#[derive(Debug, Clone, Serialize)]
pub struct IngestionProgress {
    pub job_id: Uuid,
    pub stage: String,
    pub message: String,
    pub papers_found: usize,
    pub papers_inserted: usize,
    pub chunks_inserted: usize,
    pub error: Option<String>,
}

impl IngestionProgress {
    fn new(job_id: Uuid, stage: &str, message: &str) -> Self {
        Self {
            job_id,
            stage: stage.to_string(),
            message: message.to_string(),
            papers_found: 0,
            papers_inserted: 0,
            chunks_inserted: 0,
            error: None,
        }
    }
}

// ── Result summary ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct IngestionSourceTelemetry {
    pub source: String,
    pub fetched: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IngestionResult {
    pub job_id: Uuid,
    pub query: String,
    pub papers_found_raw: usize,
    pub papers_found: usize,
    pub papers_inserted: usize,
    pub papers_duplicate: usize,
    pub chunks_inserted: usize,
    pub chunks_embedded: usize,
    pub source_telemetry: Vec<IngestionSourceTelemetry>,
    pub perf_telemetry: IngestionPerfTelemetry,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct IngestionPerfTelemetry {
    pub search_ms: u64,
    pub dedup_ms: u64,
    pub upsert_ms: u64,
    pub prefetch_ms: u64,
    pub process_ms: u64,
    pub pdf_cache_hits: usize,
    pub pdf_cache_misses: usize,
    pub quality_gate_skips: usize,
}

// ── Pipeline orchestrator ─────────────────────────────────────────────────────

/// Runs the end-to-end ingestion pipeline for one job.
///
/// Progress events are sent via `progress_tx` if provided.
/// The pipeline is non-destructive: on errors it logs and continues.
#[instrument(skip(repo, progress_tx))]
pub async fn run_ingestion(
    job: IngestionJob,
    repo: Arc<IngestionRepository>,
    progress_tx: Option<broadcast::Sender<IngestionProgress>>,
) -> IngestionResult {
    let job_id = Uuid::new_v4();
    let t0 = std::time::Instant::now();

    // Build search query
    let query = build_query(&job);
    info!(job_id = %job_id, query = %query, "Starting ingestion pipeline");

    let emit = |stage: &str, msg: &str, mut prog: IngestionProgress| {
        prog.stage = stage.to_string();
        prog.message = msg.to_string();
        if let Some(ref tx) = progress_tx {
            let _ = tx.send(prog);
        }
    };

    let mut base_progress = IngestionProgress::new(job_id, "init", "");
    emit(
        "init",
        "Ingestion pipeline initialized",
        base_progress.clone(),
    );

    // Build embedding client once if configured
    let embed_client = job.embedding_cfg.as_ref().map(|cfg| {
        info!("Embedding enabled: {:?} / {}", cfg.backend, cfg.model);
        Arc::new(EmbeddingClient::new(cfg.clone()))
    });

    // Initialize (or reuse) NER once per process to avoid repeated HGNC/OncoTree downloads.
    emit(
        "init",
        "Loading biomedical NER databases",
        base_progress.clone(),
    );
    let ner = match get_or_init_ner().await {
        Ok(ner) => ner,
        Err(e) => {
            let msg = format!("Failed to initialize NER with complete databases: {e}. Ingestion aborted to ensure quality.");
            warn!("{}", &msg);
            return IngestionResult {
                job_id,
                query: query.clone(),
                papers_found: 0,
                papers_found_raw: 0,
                papers_inserted: 0,
                papers_duplicate: 0,
                chunks_inserted: 0,
                chunks_embedded: 0,
                source_telemetry: Vec::new(),
                perf_telemetry: IngestionPerfTelemetry::default(),
                errors: vec![msg],
                duration_ms: (std::time::Instant::now() - t0).as_millis() as u64,
            };
        }
    };
    info!(
        "NER initialized: {} patterns loaded",
        ner.stats().total_patterns
    );
    base_progress.message = "NER initialized".to_string();
    emit(
        "init",
        &format!("NER initialized: {} patterns", ner.stats().total_patterns),
        base_progress.clone(),
    );

    let mut result = IngestionResult {
        job_id,
        query: query.clone(),
        papers_found_raw: 0,
        papers_found: 0,
        papers_inserted: 0,
        papers_duplicate: 0,
        chunks_inserted: 0,
        chunks_embedded: 0,
        source_telemetry: Vec::new(),
        perf_telemetry: IngestionPerfTelemetry::default(),
        errors: Vec::new(),
        duration_ms: 0,
    };

    let prog_base = IngestionProgress::new(job_id, "search", "");
    emit(
        "search",
        &format!("Searching with query: {query}"),
        prog_base.clone(),
    );

    // ── 1. Collect papers from all enabled sources ────────────────────────────
    let t_search = std::time::Instant::now();
    let mut all_papers = Vec::new();
    let source_count = job.sources.len().max(1);
    let per_source_max_results = ((job.max_results as f64 / source_count as f64) * 1.35)
        .ceil()
        .max(5.0) as usize;
    let source_inflight_limit = resolve_source_max_inflight();
    let source_semaphore = Arc::new(Semaphore::new(source_inflight_limit));

    let mut source_tasks = tokio::task::JoinSet::new();
    for source in job.sources.clone() {
        let source_query = build_query_for_source(&job, &source);
        let max_results = per_source_max_results;
        let pubmed_api_key = job.pubmed_api_key.clone();
        let semantic_scholar_api_key = job.semantic_scholar_api_key.clone();
        let source_timeout =
            std::time::Duration::from_secs(job.source_timeout_secs.unwrap_or(45).clamp(5, 300));
        let source_cache_enabled = job.source_cache_enabled;
        let source_cache_ttl_secs = job.source_cache_ttl_secs;
        let source_semaphore = source_semaphore.clone();
        source_tasks.spawn(async move {
            let _permit = source_semaphore
                .acquire_owned()
                .await
                .expect("source semaphore closed");
            let source_result = timeout(source_timeout, async {
                search_source_with_cache(
                    source.clone(),
                    &source_query,
                    max_results,
                    pubmed_api_key,
                    semantic_scholar_api_key,
                    source_cache_enabled,
                    source_cache_ttl_secs,
                )
                .await
            })
            .await
            .unwrap_or_else(|_| {
                Err(anyhow::anyhow!(
                    "source request exceeded timeout ({}s)",
                    source_timeout.as_secs()
                ))
            });
            (source, source_result)
        });
    }

    while let Some(joined) = source_tasks.join_next().await {
        match joined {
            Ok((source, Ok(papers))) => {
                info!(source = ?source, n = papers.len(), "Papers retrieved");
                result.source_telemetry.push(IngestionSourceTelemetry {
                    source: format!("{:?}", source),
                    fetched: papers.len(),
                    error: None,
                });
                all_papers.extend(papers);
            }
            Ok((source, Err(e))) => {
                let msg = format!("Source {:?} error: {e}", source);
                warn!("{}", &msg);
                result.source_telemetry.push(IngestionSourceTelemetry {
                    source: format!("{:?}", source),
                    fetched: 0,
                    error: Some(e.to_string()),
                });
                result.errors.push(msg);
            }
            Err(e) => {
                let msg = format!("Source task join error: {e}");
                warn!("{}", &msg);
                result.errors.push(msg);
            }
        }
    }
    result.perf_telemetry.search_ms = t_search.elapsed().as_millis() as u64;

    // Source-level dedupe before DB upsert to avoid repeatedly processing
    // the same paper returned by multiple providers.
    let t_dedup = std::time::Instant::now();
    result.papers_found_raw = all_papers.len();
    let mut seen = HashSet::new();
    all_papers.retain(|paper| {
        let key = paper
            .doi
            .as_ref()
            .map(|d| format!("doi:{}", d.trim().to_uppercase()))
            .or_else(|| paper.pmid.as_ref().map(|p| format!("pmid:{}", p.trim())))
            .unwrap_or_else(|| format!("title:{}", paper.title.trim().to_lowercase()));
        seen.insert(key)
    });

    result.papers_found = all_papers.len();
    result.perf_telemetry.dedup_ms = t_dedup.elapsed().as_millis() as u64;
    emit(
        "upsert",
        &format!("{} unique papers found, deduplicating…", all_papers.len()),
        {
            let mut p = prog_base.clone();
            p.papers_found = all_papers.len();
            p
        },
    );

    // ── 2. Upsert papers + chunk abstracts ───────────────────────────────────
    let chunker_cfg = ChunkerConfig::default();
    let t_upsert = std::time::Instant::now();
    let mut queued_new_papers: Vec<(crate::models::PaperMetadata, Uuid)> = Vec::new();
    for paper in &all_papers {
        let upsert = match repo.upsert_paper(paper).await {
            Ok(u) => u,
            Err(e) => {
                let id = paper
                    .pmid
                    .as_ref()
                    .or(paper.doi.as_ref())
                    .map(|s| s.as_str())
                    .unwrap_or("unknown");
                let msg = format!("paper upsert failed for {}: {e}", id);
                warn!("{}", &msg);
                result.errors.push(msg);
                continue;
            }
        };
        if !upsert.was_new {
            result.papers_duplicate += 1;
            continue;
        }
        result.papers_inserted += 1;
        queued_new_papers.push((paper.clone(), upsert.paper_id));
    }

    result.perf_telemetry.upsert_ms = t_upsert.elapsed().as_millis() as u64;

    let full_text_step_timeout =
        std::time::Duration::from_secs(job.full_text_step_timeout_secs.unwrap_or(15).clamp(5, 120));
    let prefetch_worker_limit = job
        .full_text_prefetch_workers
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get().clamp(2, 8))
                .unwrap_or(4)
        })
        .clamp(1, 32);
    let paper_worker_limit = resolve_paper_process_workers();
    let total_new_papers = queued_new_papers.len();
    let t_prefetch_and_process = std::time::Instant::now();

    emit(
        "process",
        &format!(
            "Processing {} new papers ({} prefetch workers, {} process workers)",
            total_new_papers, prefetch_worker_limit, paper_worker_limit
        ),
        {
            let mut p = prog_base.clone();
            p.papers_found = result.papers_found;
            p.papers_inserted = result.papers_inserted;
            p
        },
    );

    let (prefetch_tx, mut prefetch_rx) =
        mpsc::channel::<(crate::models::PaperMetadata, Uuid, Vec<DocumentSection>)>(
            total_new_papers.min(256).max(1),
        );
    let unpaywall_email = job.unpaywall_email.clone();
    let enable_scihub = job.enable_scihub_fallback;
    let full_text_enabled = job.full_text_enabled;
    let prefetch_input = queued_new_papers.clone();
    let prefetch_task = tokio::spawn(async move {
        if !full_text_enabled {
            for (paper, paper_id) in prefetch_input {
                let _ = prefetch_tx.send((paper, paper_id, Vec::new())).await;
            }
            return;
        }

        let mut set = tokio::task::JoinSet::new();
        let mut next_idx = 0usize;
        while next_idx < prefetch_input.len() || !set.is_empty() {
            while next_idx < prefetch_input.len() && set.len() < prefetch_worker_limit {
                let (paper, paper_id) = prefetch_input[next_idx].clone();
                let unpaywall_email = unpaywall_email.clone();
                set.spawn(async move {
                    let sections = fetch_full_text_sections_for_paper(
                        &paper,
                        unpaywall_email.as_deref(),
                        enable_scihub,
                        full_text_step_timeout,
                    )
                    .await
                    .unwrap_or_default();
                    (paper, paper_id, sections)
                });
                next_idx += 1;
            }

            if let Some(joined) = set.join_next().await {
                if let Ok(payload) = joined {
                    let _ = prefetch_tx.send(payload).await;
                }
            }
        }
    });

    let mut processing_set = tokio::task::JoinSet::new();
    let mut completed = 0usize;
    while prefetch_rx.is_closed() == false || !processing_set.is_empty() {
        while processing_set.len() < paper_worker_limit {
            let maybe_payload = prefetch_rx.recv().await;
            let Some((paper, paper_id, full_text_sections)) = maybe_payload else {
                break;
            };
            let repo_clone = repo.clone();
            let ner_clone = ner.clone();
            let chunker_cfg_clone = chunker_cfg.clone();
            let embed_client_clone = embed_client.clone();
            processing_set.spawn(async move {
                process_single_paper(
                    paper,
                    paper_id,
                    full_text_sections,
                    repo_clone,
                    ner_clone,
                    chunker_cfg_clone,
                    embed_client_clone,
                )
                .await
            });
        }

        if let Some(joined) = processing_set.join_next().await {
            match joined {
                Ok(outcome) => {
                    completed += 1;
                    result.chunks_inserted += outcome.chunks_inserted;
                    result.chunks_embedded += outcome.chunks_embedded;
                    result.errors.extend(outcome.errors);
                    if outcome.quality_gate_skipped {
                        result.perf_telemetry.quality_gate_skips += 1;
                    }
                    emit(
                        "progress",
                        &format!("Processed paper {}/{}", completed, total_new_papers),
                        {
                            let mut p = prog_base.clone();
                            p.papers_found = result.papers_found;
                            p.papers_inserted = result.papers_inserted;
                            p.chunks_inserted = result.chunks_inserted;
                            p
                        },
                    );
                }
                Err(e) => {
                    let msg = format!("paper worker join error: {e}");
                    warn!("{}", msg);
                    result.errors.push(msg);
                }
            }
        } else if prefetch_rx.is_closed() {
            break;
        }
    }
    let _ = prefetch_task.await;
    result.perf_telemetry.prefetch_ms = t_prefetch_and_process.elapsed().as_millis() as u64;
    result.perf_telemetry.process_ms = result.perf_telemetry.prefetch_ms;

    result.duration_ms = t0.elapsed().as_millis() as u64;
    let (pdf_hits, pdf_misses) = pdf_cache_counters();
    result.perf_telemetry.pdf_cache_hits = pdf_hits;
    result.perf_telemetry.pdf_cache_misses = pdf_misses;
    let cross_source_dedup_dropped = result.papers_found_raw.saturating_sub(result.papers_found);
    info!(
        papers_found_raw = result.papers_found_raw,
        papers_unique = result.papers_found,
        cross_source_dedup_dropped,
        papers_inserted = result.papers_inserted,
        papers_existing_duplicates = result.papers_duplicate,
        "Ingestion source/dedup telemetry summary"
    );
    for src in &result.source_telemetry {
        info!(
            source = %src.source,
            fetched = src.fetched,
            error = ?src.error,
            "Ingestion source telemetry"
        );
    }

    info!(
        job_id = %job_id,
        papers_found    = result.papers_found,
        papers_inserted = result.papers_inserted,
        papers_dup      = result.papers_duplicate,
        chunks          = result.chunks_inserted,
        duration_ms     = result.duration_ms,
        errors          = result.errors.len(),
        perf_search_ms  = result.perf_telemetry.search_ms,
        perf_upsert_ms  = result.perf_telemetry.upsert_ms,
        perf_process_ms = result.perf_telemetry.process_ms,
        pdf_cache_hits  = result.perf_telemetry.pdf_cache_hits,
        pdf_cache_misses = result.perf_telemetry.pdf_cache_misses,
        "Ingestion pipeline complete"
    );

    emit(
        "complete",
        &format!(
            "Done. {} new papers, {} chunks ({} embedded), {} duplicates skipped.",
            result.papers_inserted,
            result.chunks_inserted,
            result.chunks_embedded,
            result.papers_duplicate
        ),
        {
            let mut p = prog_base.clone();
            p.papers_found = result.papers_found;
            p.papers_inserted = result.papers_inserted;
            p.chunks_inserted = result.chunks_inserted;
            p
        },
    );

    result
}

// ── Query builder ─────────────────────────────────────────────────────────────

/// Build a PubMed/Europe PMC compatible search query.
pub fn build_query(job: &IngestionJob) -> String {
    // Portable query syntax for sources that do not support PubMed field tags.
    // Prefer broad recall here; source-specific builders can tighten semantics.
    let mut parts = vec![job.gene.clone(), job.cancer_type.clone()];
    if let Some(ref m) = job.mutation {
        if !m.trim().is_empty() {
            parts.push(m.clone());
        }
    }
    parts.join(" AND ")
}

fn resolve_paper_process_workers() -> usize {
    if let Ok(v) = std::env::var("FERRUMYX_PAPER_PROCESS_WORKERS") {
        if let Ok(n) = v.trim().parse::<usize>() {
            return n.clamp(1, 16);
        }
    }

    if let Some(n) = paper_process_workers_from_config() {
        return n.clamp(1, 16);
    }

    std::thread::available_parallelism()
        .map(|n| n.get().clamp(2, 8))
        .unwrap_or(4)
}

fn resolve_entity_insert_batch_size() -> usize {
    if let Ok(v) = std::env::var("FERRUMYX_INGESTION_ENTITY_BATCH_SIZE") {
        if let Ok(n) = v.trim().parse::<usize>() {
            return n.clamp(16, 2048);
        }
    }
    256
}

fn resolve_fact_insert_batch_size() -> usize {
    if let Ok(v) = std::env::var("FERRUMYX_INGESTION_FACT_BATCH_SIZE") {
        if let Ok(n) = v.trim().parse::<usize>() {
            return n.clamp(16, 4096);
        }
    }
    512
}

fn resolve_source_retries() -> usize {
    std::env::var("FERRUMYX_INGESTION_SOURCE_RETRIES")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(2)
        .clamp(0, 5)
}

fn resolve_source_max_inflight() -> usize {
    std::env::var("FERRUMYX_INGESTION_SOURCE_MAX_INFLIGHT")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(4)
        .clamp(1, 16)
}

fn paper_process_workers_from_config() -> Option<usize> {
    let path = std::env::var("FERRUMYX_CONFIG").unwrap_or_else(|_| "ferrumyx.toml".to_string());
    let content = std::fs::read_to_string(path).ok()?;
    content
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("paper_process_workers"))
        .and_then(|line| line.split('=').nth(1))
        .map(str::trim)
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v > 0)
}

fn build_pubmed_query(job: &IngestionJob) -> String {
    let mut parts = vec![
        format!("{}[tiab]", job.gene),
        format!("{}[tiab]", job.cancer_type),
    ];
    if let Some(ref m) = job.mutation {
        if !m.trim().is_empty() {
            parts.push(format!("{m}[tiab]"));
        }
    }
    parts.join(" AND ")
}

fn build_query_for_source(job: &IngestionJob, source: &IngestionSourceSpec) -> String {
    match source {
        IngestionSourceSpec::PubMed => build_pubmed_query(job),
        _ => build_query(job),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SourceSearchCacheEntry {
    cached_at_epoch_secs: u64,
    papers: Vec<crate::models::PaperMetadata>,
}

fn source_cache_ttl(source_cache_ttl_secs: Option<u64>) -> std::time::Duration {
    std::time::Duration::from_secs(source_cache_ttl_secs.unwrap_or(30 * 60).clamp(60, 86_400))
}

fn source_cache_dir() -> PathBuf {
    std::env::var("FERRUMYX_SOURCE_CACHE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data/cache/source_search"))
}

fn source_cache_path(source: &IngestionSourceSpec, query: &str, max_results: usize) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    query.hash(&mut hasher);
    max_results.hash(&mut hasher);
    let digest = hasher.finish();
    source_cache_dir().join(format!("{:?}_{digest:016x}.json", source))
}

fn load_source_cache(
    source: &IngestionSourceSpec,
    query: &str,
    max_results: usize,
    ttl: std::time::Duration,
) -> Option<Vec<crate::models::PaperMetadata>> {
    let path = source_cache_path(source, query, max_results);
    let content = std::fs::read_to_string(path).ok()?;
    let entry: SourceSearchCacheEntry = serde_json::from_str(&content).ok()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    if now.saturating_sub(entry.cached_at_epoch_secs) <= ttl.as_secs() {
        Some(entry.papers)
    } else {
        None
    }
}

fn save_source_cache(
    source: &IngestionSourceSpec,
    query: &str,
    max_results: usize,
    papers: &[crate::models::PaperMetadata],
) {
    let dir = source_cache_dir();
    let _ = std::fs::create_dir_all(&dir);
    let now = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(v) => v.as_secs(),
        Err(_) => return,
    };
    let entry = SourceSearchCacheEntry {
        cached_at_epoch_secs: now,
        papers: papers.to_vec(),
    };
    if let Ok(payload) = serde_json::to_string(&entry) {
        let _ = std::fs::write(source_cache_path(source, query, max_results), payload);
    }
}

async fn search_source_with_cache(
    source: IngestionSourceSpec,
    source_query: &str,
    max_results: usize,
    pubmed_api_key: Option<String>,
    semantic_scholar_api_key: Option<String>,
    source_cache_enabled: bool,
    source_cache_ttl_secs: Option<u64>,
) -> anyhow::Result<Vec<crate::models::PaperMetadata>> {
    if source_cache_enabled {
        if let Some(cached) = load_source_cache(
            &source,
            source_query,
            max_results,
            source_cache_ttl(source_cache_ttl_secs),
        ) {
            return Ok(cached);
        }
    }

    let retries = resolve_source_retries();
    let mut last_err = None;
    let mut papers = Vec::new();
    for attempt in 0..=retries {
        match search_source_once(
            &source,
            source_query,
            max_results,
            pubmed_api_key.clone(),
            semantic_scholar_api_key.clone(),
        )
        .await
        {
            Ok(found) => {
                papers = found;
                last_err = None;
                break;
            }
            Err(e) => {
                last_err = Some(e);
                if attempt < retries {
                    let backoff_ms = ((250_u64 << attempt).min(2_500))
                        + ((source_query.len() as u64 + attempt as u64 * 17) % 80);
                    sleep(std::time::Duration::from_millis(backoff_ms)).await;
                }
            }
        }
    }
    if let Some(err) = last_err {
        return Err(err);
    }

    if source_cache_enabled && !papers.is_empty() {
        save_source_cache(&source, source_query, max_results, &papers);
    }
    Ok(papers)
}

async fn search_source_once(
    source: &IngestionSourceSpec,
    source_query: &str,
    max_results: usize,
    pubmed_api_key: Option<String>,
    semantic_scholar_api_key: Option<String>,
) -> anyhow::Result<Vec<crate::models::PaperMetadata>> {
    match source {
        IngestionSourceSpec::PubMed => {
            let client = PubMedClient::new(pubmed_api_key);
            client.search(source_query, max_results).await
        }
        IngestionSourceSpec::EuropePmc => {
            let client = EuropePmcClient::new();
            client.search(source_query, max_results).await
        }
        IngestionSourceSpec::BioRxiv => {
            let client = BioRxivClient::new_biorxiv();
            client.search(source_query, max_results).await
        }
        IngestionSourceSpec::MedRxiv => {
            let client = BioRxivClient::new_medrxiv();
            client.search(source_query, max_results).await
        }
        IngestionSourceSpec::Arxiv => {
            let client = ArxivClient::new();
            client.search(source_query, max_results).await
        }
        IngestionSourceSpec::ClinicalTrials => {
            let client = ClinicalTrialsClient::new();
            client.search(source_query, max_results).await
        }
        IngestionSourceSpec::CrossRef => {
            let client = CrossRefClient::new();
            client.search(source_query, max_results).await
        }
        IngestionSourceSpec::SemanticScholar => {
            let client = SemanticScholarClient::new(semantic_scholar_api_key);
            client.search(source_query, max_results).await
        }
    }
}

// ── Section builder ───────────────────────────────────────────────────────────

/// Convert PaperMetadata abstract into document sections for chunking.
fn build_sections_from_abstract(paper: &crate::models::PaperMetadata) -> Vec<DocumentSection> {
    let mut sections = Vec::new();

    if let Some(ref abstract_text) = paper.abstract_text {
        if !abstract_text.trim().is_empty() {
            sections.push(DocumentSection {
                section_type: SectionType::Abstract,
                heading: Some("Abstract".to_string()),
                text: abstract_text.clone(),
                page_number: None,
            });
        }
    }

    // Title as a single mini-chunk (high signal for entity extraction)
    if !paper.title.is_empty() {
        sections.push(DocumentSection {
            section_type: SectionType::Introduction,
            heading: Some("Title".to_string()),
            text: paper.title.clone(),
            page_number: None,
        });
    }

    sections
}

fn map_ner_type(label: NerEntityType) -> DbEntityType {
    match label {
        NerEntityType::Gene => DbEntityType::Gene,
        NerEntityType::Disease => DbEntityType::Disease,
        NerEntityType::Chemical => DbEntityType::Chemical,
        NerEntityType::Mutation => DbEntityType::Mutation,
        NerEntityType::CancerType => DbEntityType::CancerType,
        NerEntityType::Pathway => DbEntityType::Pathway,
        NerEntityType::CellLine | NerEntityType::Other => DbEntityType::Disease,
    }
}

fn infer_object_type(predicate: &str, object: &str) -> DbEntityType {
    let p = predicate.to_lowercase();
    if p == "has_mutation" || p.contains("mutation") {
        return DbEntityType::Mutation;
    }
    let o = object.trim();
    let oncotree_like = !o.is_empty()
        && o.len() <= 8
        && o.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    if oncotree_like {
        return DbEntityType::CancerType;
    }
    let lc = o.to_lowercase();
    if lc.contains("cancer")
        || lc.contains("carcinoma")
        || lc.contains("sarcoma")
        || lc.contains("lymphoma")
        || lc.contains("leukemia")
        || lc.contains("tumor")
    {
        return DbEntityType::CancerType;
    }
    DbEntityType::Disease
}

fn canonical_key(entity_type: DbEntityType, name: &str) -> String {
    let mut normalized = name.trim().to_uppercase();
    normalized = normalized
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();
    while normalized.contains("__") {
        normalized = normalized.replace("__", "_");
    }
    format!("{}:{}", entity_type, normalized.trim_matches('_'))
}

async fn resolve_or_create_entity(
    repo: &EntityRepository,
    cache: &mut HashMap<String, Uuid>,
    entity_type: DbEntityType,
    display_name: &str,
) -> anyhow::Result<Uuid> {
    let key = canonical_key(entity_type, display_name);
    if let Some(id) = cache.get(&key) {
        return Ok(*id);
    }

    let external_id = format!("FERRUMYX:{}", key);
    if let Some(existing) = repo
        .find_by_external_id(&external_id)
        .await?
        .into_iter()
        .next()
    {
        cache.insert(key, existing.id);
        return Ok(existing.id);
    }

    let mut entity = DbEntity::new(
        entity_type,
        display_name.trim().to_string(),
        external_id.clone(),
        "ferrumyx".to_string(),
    );
    entity.canonical_name = Some(display_name.trim().to_string());
    if let Err(e) = repo.insert(&entity).await {
        // Another worker may have created this entity concurrently; retry lookup once.
        if let Some(existing) = repo
            .find_by_external_id(&external_id)
            .await?
            .into_iter()
            .next()
        {
            cache.insert(key, existing.id);
            return Ok(existing.id);
        }
        return Err(e.into());
    }
    cache.insert(key, entity.id);
    Ok(entity.id)
}

async fn process_single_paper(
    paper: crate::models::PaperMetadata,
    paper_id: Uuid,
    full_text_sections: Vec<DocumentSection>,
    repo: Arc<IngestionRepository>,
    ner: Arc<TrieNer>,
    chunker_cfg: ChunkerConfig,
    embed_client: Option<Arc<EmbeddingClient>>,
) -> PaperProcessingResult {
    let mut out = PaperProcessingResult::default();
    info!(paper_id = %paper_id, title = %paper.title, "Processing new paper");
    let _ = repo.set_parse_status(paper_id, "processing").await;

    let mut sections = build_sections_from_abstract(&paper);
    if !full_text_sections.is_empty() {
        info!(
            paper_id = %paper_id,
            n_sections = full_text_sections.len(),
            "Full-text parsed successfully"
        );
        sections.extend(full_text_sections);
        let _ = repo.set_full_text_status(paper_id, true).await;
    } else {
        debug!(
            paper_id = %paper_id,
            "Full-text PDF fetch/parse failed or unavailable, using abstract only"
        );
    }

    if sections.is_empty() {
        warn!(paper_id = %paper_id, "No sections (abstract/title) found for paper, skipping");
        return out;
    }

    let chunks = chunk_document(paper_id, sections, &chunker_cfg);
    let n_chunks = chunks.len();
    match repo.bulk_insert_chunks(&chunks).await {
        Ok(inserted) => {
            out.chunks_inserted += inserted;
            info!(
                paper_id = %paper_id,
                pmid = ?paper.pmid,
                n_chunks,
                "Paper ingested"
            );
        }
        Err(e) => {
            let id = paper
                .pmid
                .as_ref()
                .or(paper.doi.as_ref())
                .map(|s| s.as_str())
                .unwrap_or("unknown");
            let msg = format!("chunk insert failed for {}: {e}", id);
            warn!("{}", &msg);
            out.errors.push(msg);
            let _ = repo.set_parse_status(paper_id, "failed").await;
            return out;
        }
    }

    let min_ner_chars = std::env::var("FERRUMYX_INGESTION_MIN_NER_CHARS")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(500)
        .clamp(120, 5000);
    let total_chars: usize = chunks.iter().map(|c| c.content.len()).sum();
    if total_chars < min_ner_chars {
        out.quality_gate_skipped = true;
        let _ = repo.set_parse_status(paper_id, "parsed_light").await;
        if let Some(ref ec) = embed_client {
            match embed_pending_chunks(ec.as_ref(), repo.as_ref(), paper_id).await {
                Ok(n) => out.chunks_embedded += n,
                Err(e) => out
                    .errors
                    .push(format!("embed failed for {:?}: {e}", paper_id)),
            }
        }
        return out;
    }

    let entity_repo = EntityRepository::new(repo.db());
    let mut entity_id_cache: HashMap<String, Uuid> = HashMap::new();
    let cancer_normaliser = ner.cancers();
    let paper_subject_name = if let Some(ref journal) = paper.journal {
        format!("{} ({})", paper.title, journal)
    } else {
        paper.title.clone()
    };
    #[derive(Debug, Clone)]
    struct MentionFactSeed {
        entity_type: DbEntityType,
        object_name: String,
        confidence: f32,
    }

    #[derive(Debug, Clone)]
    struct RelationFactSeed {
        gene_symbol: String,
        predicate: String,
        object_name: String,
        confidence: f32,
    }

    let mut mention_seeds: Vec<MentionFactSeed> = Vec::new();
    let mut relation_seeds: Vec<RelationFactSeed> = Vec::new();
    let mut unique_candidates: HashMap<String, (DbEntityType, String)> = HashMap::new();

    for chunk in &chunks {
        let entities = ner.extract(&chunk.content);
        if !entities.is_empty() {
            info!(paper_id = %paper_id, count = entities.len(), "Entities extracted from chunk");
        }

        let mut chunk_seen_names: HashSet<String> = HashSet::new();
        let mut genes_for_relations: HashMap<String, f32> = HashMap::new();

        for entity in entities {
            let mut canon_subject = entity.text.clone();
            if entity.label == NerEntityType::Gene {
                if let Some(sym) = ner.hgnc().normalise_symbol(&entity.text) {
                    canon_subject = sym;
                }
            } else if entity.label == NerEntityType::CancerType {
                if let Some(code) = cancer_normaliser.normalise(&entity.text) {
                    canon_subject = code;
                }
            }

            if canon_subject.trim().is_empty() {
                continue;
            }

            if chunk_seen_names.insert(canon_subject.clone()) {
                let entity_db_type = map_ner_type(entity.label);
                mention_seeds.push(MentionFactSeed {
                    entity_type: entity_db_type,
                    object_name: canon_subject.clone(),
                    confidence: entity.confidence,
                });
                unique_candidates
                    .entry(canonical_key(entity_db_type, &canon_subject))
                    .or_insert((entity_db_type, canon_subject.clone()));
            }

            if entity.label == NerEntityType::Gene {
                genes_for_relations
                    .entry(canon_subject)
                    .or_insert(entity.confidence);
            }
        }

        for (gene_symbol, gene_confidence) in genes_for_relations {
            for mut fact in build_facts(&gene_symbol, &chunk.content) {
                if fact.fact_type != "has_mutation" {
                    if let Some(code) = cancer_normaliser.normalise(&fact.object) {
                        fact.object = code;
                    }
                }
                let object_type = infer_object_type(&fact.fact_type, &fact.object);
                unique_candidates
                    .entry(canonical_key(object_type, &fact.object))
                    .or_insert((object_type, fact.object.clone()));
                relation_seeds.push(RelationFactSeed {
                    gene_symbol: gene_symbol.clone(),
                    predicate: fact.fact_type.clone(),
                    object_name: fact.object.clone(),
                    confidence: gene_confidence,
                });
            }
        }
    }

    if !unique_candidates.is_empty() {
        let candidates: Vec<(DbEntityType, String)> = unique_candidates.into_values().collect();
        if let Err(err) = resolve_or_create_entities_bulk(
            &entity_repo,
            &mut entity_id_cache,
            &candidates,
            resolve_entity_insert_batch_size(),
        )
        .await
        {
            let msg = format!("entity bulk resolution failed for {:?}: {err}", paper_id);
            warn!("{msg}");
            out.errors.push(msg);
        }
    }

    let mut paper_facts: Vec<KgFact> = Vec::new();
    let mut dedup_mentions: HashSet<(Uuid, String)> = HashSet::new();
    for mention in mention_seeds {
        let key = canonical_key(mention.entity_type, &mention.object_name);
        let Some(entity_id) = entity_id_cache.get(&key).copied() else {
            continue;
        };
        if dedup_mentions.insert((entity_id, mention.object_name.clone())) {
            let mut fact = KgFact::new(
                paper_id,
                paper_id,
                paper_subject_name.clone(),
                "mentions".to_string(),
                entity_id,
                mention.object_name,
            );
            fact.confidence = mention.confidence;
            paper_facts.push(fact);
        }
    }

    for relation in relation_seeds {
        let gene_key = canonical_key(DbEntityType::Gene, &relation.gene_symbol);
        let object_type = infer_object_type(&relation.predicate, &relation.object_name);
        let object_key = canonical_key(object_type, &relation.object_name);
        let Some(gene_entity_id) = entity_id_cache.get(&gene_key).copied() else {
            continue;
        };
        let Some(object_id) = entity_id_cache.get(&object_key).copied() else {
            continue;
        };
        let mut db_fact = KgFact::new(
            paper_id,
            gene_entity_id,
            relation.gene_symbol.clone(),
            relation.predicate.clone(),
            object_id,
            relation.object_name.clone(),
        );
        db_fact.confidence = relation.confidence;
        paper_facts.push(db_fact);
    }

    if !paper_facts.is_empty() {
        let fact_batch_size = resolve_fact_insert_batch_size();
        for batch in paper_facts.chunks(fact_batch_size) {
            if let Err(e) = repo.bulk_insert_facts(batch).await {
                let msg = format!("bulk fact insert failed for {:?}: {e}", paper_id);
                warn!("{}", msg);
                out.errors.push(msg);
                for f in batch {
                    let _ = repo
                        .insert_fact(
                            f.paper_id,
                            f.subject_id,
                            &f.subject_name,
                            &f.predicate,
                            f.object_id,
                            &f.object_name,
                            f.confidence,
                        )
                        .await;
                }
            }
        }
    }

    let _ = repo.set_parse_status(paper_id, "parsed").await;

    if let Some(ref ec) = embed_client {
        match embed_pending_chunks(ec.as_ref(), repo.as_ref(), paper_id).await {
            Ok(n) => {
                out.chunks_embedded += n;
                debug!(paper_id = %paper_id, n, "Chunks embedded");
            }
            Err(e) => {
                let msg = format!("embed failed for {:?}: {e}", paper_id);
                warn!("{}", &msg);
                out.errors.push(msg);
            }
        }
    }

    out
}

async fn resolve_or_create_entities_bulk(
    repo: &EntityRepository,
    cache: &mut HashMap<String, Uuid>,
    candidates: &[(DbEntityType, String)],
    insert_batch_size: usize,
) -> anyhow::Result<()> {
    if candidates.is_empty() {
        return Ok(());
    }

    let mut missing: Vec<(String, DbEntity)> = Vec::new();
    for (entity_type, display_name) in candidates {
        let key = canonical_key(*entity_type, display_name);
        if cache.contains_key(&key) {
            continue;
        }

        let external_id = format!("FERRUMYX:{}", key);
        if let Some(existing) = repo
            .find_by_external_id(&external_id)
            .await?
            .into_iter()
            .next()
        {
            cache.insert(key, existing.id);
            continue;
        }

        let mut entity = DbEntity::new(
            *entity_type,
            display_name.trim().to_string(),
            external_id,
            "ferrumyx".to_string(),
        );
        entity.canonical_name = Some(display_name.trim().to_string());
        missing.push((key, entity));
    }

    if missing.is_empty() {
        return Ok(());
    }

    let batch_size = insert_batch_size.max(1);
    for batch in missing.chunks(batch_size) {
        let entities: Vec<DbEntity> = batch.iter().map(|(_, e)| e.clone()).collect();
        if repo.insert_batch(&entities).await.is_ok() {
            for (key, entity) in batch {
                cache.insert(key.clone(), entity.id);
            }
            continue;
        }

        for (key, entity) in batch {
            if repo.insert(entity).await.is_ok() {
                cache.insert(key.clone(), entity.id);
                continue;
            }
            if let Some(existing) = repo
                .find_by_external_id(&entity.external_id)
                .await?
                .into_iter()
                .next()
            {
                cache.insert(key.clone(), existing.id);
            }
        }
    }

    Ok(())
}

async fn fetch_full_text_sections_for_paper(
    paper: &crate::models::PaperMetadata,
    unpaywall_email: Option<&str>,
    enable_scihub_fallback: bool,
    step_timeout: std::time::Duration,
) -> anyhow::Result<Vec<DocumentSection>> {
    // Strategy 1: Explicit open-access URL
    if let Some(ref pdf_url) = paper.full_text_url {
        if let Ok(Ok(sections)) = timeout(step_timeout, fetch_and_parse_pdf(pdf_url)).await {
            if !sections.is_empty() {
                return Ok(sections);
            }
        }
    }

    // Strategy 2: PMC XML (preferred when PMCID is known)
    if let Some(ref pmcid) = paper.pmcid {
        let clean_pmcid = if pmcid.starts_with("PMC") {
            pmcid.clone()
        } else {
            format!("PMC{}", pmcid)
        };
        let epmc = EuropePmcClient::new();
        if let Ok(Ok(Some(xml))) = timeout(step_timeout, epmc.fetch_full_text(&clean_pmcid)).await {
            let sections = parse_pmc_xml_sections(&xml);
            if !sections.is_empty() {
                return Ok(sections);
            }
        }

        // Strategy 3: PMC direct PDF if XML path did not produce sections
        let epmc_url = format!(
            "https://europepmc.org/backend/ptpmcrender.fcgi?accid={}&blobtype=pdf",
            clean_pmcid
        );
        if let Ok(Ok(sections)) = timeout(step_timeout, fetch_and_parse_pdf(&epmc_url)).await {
            if !sections.is_empty() {
                return Ok(sections);
            }
        }
    }

    // Strategy 4: Unpaywall DOI resolution (if email configured)
    if let (Some(doi), Some(email)) = (paper.doi.as_deref(), unpaywall_email) {
        let email = email.trim();
        if !email.is_empty() {
            let unpaywall = UnpaywallClient::new(email);
            if let Ok(Ok(Some(pdf_url))) =
                timeout(step_timeout, unpaywall.resolve_pdf_url(doi)).await
            {
                if let Ok(Ok(sections)) = timeout(step_timeout, fetch_and_parse_pdf(&pdf_url)).await
                {
                    if !sections.is_empty() {
                        return Ok(sections);
                    }
                }
            }
        }
    }

    // Strategy 5: Sci-Hub fallback
    if enable_scihub_fallback {
        let identifier = paper.doi.as_ref().or(paper.pmid.as_ref());
        if let Some(id) = identifier {
            let scihub = crate::sources::scihub::SciHubClient::new();
            if let Ok(Ok(Some(pdf_bytes))) = timeout(step_timeout, scihub.download_pdf(id)).await {
                if let Ok(Ok(sections)) = timeout(step_timeout, parse_pdf_bytes(&pdf_bytes)).await {
                    if !sections.is_empty() {
                        return Ok(sections);
                    }
                }
            }
        }
    }

    Ok(Vec::new())
}

// ── Full-text PDF fetcher ─────────────────────────────────────────────────────

/// Download a PDF from URL and parse it with Ferrules.
/// Returns sections extracted from the PDF.
async fn fetch_and_parse_pdf(pdf_url: &str) -> anyhow::Result<Vec<DocumentSection>> {
    let client = PDF_HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .pool_max_idle_per_host(8)
            .connect_timeout(std::time::Duration::from_secs(8))
            .timeout(std::time::Duration::from_secs(60))
            .user_agent("Ferrumyx/0.1 (research)")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    });
    let _host_permit = acquire_pdf_host_permit(pdf_url).await;

    // Download PDF
    let resp = client.get(pdf_url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("PDF download failed: HTTP {}", resp.status());
    }

    let pdf_bytes = resp.bytes().await?;
    parse_pdf_bytes(&pdf_bytes).await
}

async fn parse_pdf_bytes(pdf_bytes: &[u8]) -> anyhow::Result<Vec<DocumentSection>> {
    if pdf_bytes.len() < 4 || &pdf_bytes[0..4] != b"%PDF" {
        anyhow::bail!("payload is not a PDF");
    }

    let cache_key = hash_bytes(pdf_bytes);
    if let Some(cached) = load_pdf_parse_cache(&cache_key) {
        if cached.parse_ok {
            return Ok(cached.sections);
        }
        return Ok(Vec::new());
    }

    let mut temp_file = NamedTempFile::new()?;
    std::io::Write::write_all(&mut temp_file, pdf_bytes)?;
    let temp_path = temp_file.path().to_path_buf();

    let parsed = tokio::task::spawn_blocking(move || parse_pdf_sections(&temp_path)).await??;

    info!(
        title = ?parsed.title,
        n_sections = parsed.sections.len(),
        page_count = parsed.page_count,
        "PDF parsed with Ferrules"
    );
    let sections = parsed.sections;
    save_pdf_parse_cache(
        &cache_key,
        &ParsedPdfCacheEntry {
            parse_ok: !sections.is_empty(),
            sections: sections.clone(),
        },
    );
    Ok(sections)
}

fn pdf_parse_cache_dir() -> PathBuf {
    std::env::var("FERRUMYX_PDF_PARSE_CACHE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data/cache/pdf_parse"))
}

fn hash_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for b in digest {
        let _ = std::fmt::Write::write_fmt(&mut out, format_args!("{:02x}", b));
    }
    out
}

fn load_pdf_parse_cache(key: &str) -> Option<ParsedPdfCacheEntry> {
    if std::env::var("FERRUMYX_PDF_PARSE_CACHE_ENABLED")
        .ok()
        .is_some_and(|v| v == "0" || v.eq_ignore_ascii_case("false"))
    {
        return None;
    }
    let path = pdf_parse_cache_dir().join(format!("{key}.json"));
    let payload = std::fs::read_to_string(path).ok()?;
    let parsed: ParsedPdfCacheEntry = serde_json::from_str(&payload).ok()?;
    if parsed.parse_ok {
        increment_pdf_cache_hits();
    } else {
        increment_pdf_cache_misses();
    }
    Some(parsed)
}

fn save_pdf_parse_cache(key: &str, entry: &ParsedPdfCacheEntry) {
    if std::env::var("FERRUMYX_PDF_PARSE_CACHE_ENABLED")
        .ok()
        .is_some_and(|v| v == "0" || v.eq_ignore_ascii_case("false"))
    {
        return;
    }
    let dir = pdf_parse_cache_dir();
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(payload) = serde_json::to_string(entry) {
        let _ = std::fs::write(dir.join(format!("{key}.json")), payload);
    }
    if entry.parse_ok {
        increment_pdf_cache_misses();
    }
}

async fn acquire_pdf_host_permit(url: &str) -> Option<tokio::sync::OwnedSemaphorePermit> {
    let parsed = url::Url::parse(url).ok()?;
    let host = parsed.host_str()?.to_string();
    let per_host_limit = std::env::var("FERRUMYX_PDF_HOST_CONCURRENCY")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(4)
        .clamp(1, 16);
    let map = PDF_HOST_LIMITS.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    let sem = {
        let mut guard = map.lock().ok()?;
        guard
            .entry(host)
            .or_insert_with(|| Arc::new(Semaphore::new(per_host_limit)))
            .clone()
    };
    sem.acquire_owned().await.ok()
}

static PDF_CACHE_HITS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static PDF_CACHE_MISSES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn increment_pdf_cache_hits() {
    PDF_CACHE_HITS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

fn increment_pdf_cache_misses() {
    PDF_CACHE_MISSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

fn pdf_cache_counters() -> (usize, usize) {
    (
        PDF_CACHE_HITS.load(std::sync::atomic::Ordering::Relaxed),
        PDF_CACHE_MISSES.load(std::sync::atomic::Ordering::Relaxed),
    )
}

fn parse_pmc_xml_sections(xml: &str) -> Vec<DocumentSection> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut sections = Vec::new();
    let mut in_abstract = false;
    let mut in_sec = false;
    let mut in_title = false;
    let mut in_p = false;
    let mut sec_heading: Option<String> = None;
    let mut sec_text = String::new();
    let mut abstract_text = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => match e.local_name().as_ref() {
                b"abstract" => in_abstract = true,
                b"sec" => {
                    in_sec = true;
                    sec_heading = None;
                    sec_text.clear();
                }
                b"title" => in_title = true,
                b"p" => in_p = true,
                _ => {}
            },
            Ok(Event::Text(t)) => {
                let text = t.unescape().map(|v| v.to_string()).unwrap_or_default();
                let text = text.trim();
                if text.is_empty() {
                    buf.clear();
                    continue;
                }
                if in_abstract && in_p {
                    if !abstract_text.is_empty() {
                        abstract_text.push(' ');
                    }
                    abstract_text.push_str(text);
                } else if in_sec && in_title {
                    if sec_heading.is_none() {
                        sec_heading = Some(text.to_string());
                    }
                } else if in_sec && in_p {
                    if !sec_text.is_empty() {
                        sec_text.push(' ');
                    }
                    sec_text.push_str(text);
                }
            }
            Ok(Event::End(e)) => match e.local_name().as_ref() {
                b"title" => in_title = false,
                b"p" => in_p = false,
                b"abstract" => in_abstract = false,
                b"sec" => {
                    in_sec = false;
                    if !sec_text.trim().is_empty() {
                        let heading = sec_heading.clone();
                        let section_type = heading
                            .as_deref()
                            .map(SectionType::from_heading)
                            .unwrap_or(SectionType::Other);
                        sections.push(DocumentSection {
                            section_type,
                            heading,
                            text: sec_text.trim().to_string(),
                            page_number: None,
                        });
                    }
                    sec_heading = None;
                    sec_text.clear();
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    if !abstract_text.trim().is_empty() {
        sections.insert(
            0,
            DocumentSection {
                section_type: SectionType::Abstract,
                heading: Some("Abstract".to_string()),
                text: abstract_text.trim().to_string(),
                page_number: None,
            },
        );
    }

    sections
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_query_with_mutation() {
        let job = IngestionJob {
            gene: "KRAS".to_string(),
            mutation: Some("G12D".to_string()),
            cancer_type: "pancreatic cancer".to_string(),
            ..Default::default()
        };
        let q = build_query(&job);
        assert!(q.contains("KRAS"));
        assert!(q.contains("G12D"));
        assert!(q.contains("pancreatic cancer"));
    }

    #[test]
    fn test_build_query_without_mutation() {
        let job = IngestionJob {
            gene: "TP53".to_string(),
            mutation: None,
            cancer_type: "lung cancer".to_string(),
            ..Default::default()
        };
        let q = build_query(&job);
        assert!(q.contains("TP53"));
        assert!(q.contains("lung cancer"));
        assert!(!q.contains("AND  AND")); // no empty mutation placeholder
    }

    #[test]
    fn test_build_sections_abstract_only() {
        use crate::models::{Author, IngestionSource};
        let paper = crate::models::PaperMetadata {
            doi: None,
            pmid: Some("1234".to_string()),
            pmcid: None,
            title: "KRAS G12D".to_string(),
            abstract_text: Some("Abstract content here.".to_string()),
            authors: vec![Author {
                name: "Smith J".to_string(),
                affiliation: None,
                orcid: None,
            }],
            journal: None,
            pub_date: None,
            source: IngestionSource::PubMed,
            open_access: false,
            full_text_url: None,
        };
        let sections = build_sections_from_abstract(&paper);
        assert!(sections
            .iter()
            .any(|s| s.section_type == SectionType::Abstract));
        assert!(sections
            .iter()
            .any(|s| s.heading.as_deref() == Some("Title")));
    }

    #[test]
    fn test_parse_pmc_xml_sections() {
        let xml = r#"<article><front><abstract><p>Abstract body text.</p></abstract></front><body><sec><title>Methods</title><p>Method A.</p><p>Method B.</p></sec><sec><title>Results</title><p>Result text.</p></sec></body></article>"#;
        let sections = parse_pmc_xml_sections(xml);
        assert!(sections
            .iter()
            .any(|s| s.section_type == SectionType::Abstract));
        assert!(sections
            .iter()
            .any(|s| s.heading.as_deref() == Some("Methods")));
        assert!(sections
            .iter()
            .any(|s| s.heading.as_deref() == Some("Results")));
    }
}
