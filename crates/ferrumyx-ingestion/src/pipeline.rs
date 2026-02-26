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

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, info, warn, instrument};
use uuid::Uuid;
use tempfile::NamedTempFile;

use crate::sources::pubmed::PubMedClient;
use crate::sources::europepmc::EuropePmcClient;
use crate::sources::biorxiv::BioRxivClient;
use crate::sources::clinicaltrials::ClinicalTrialsClient;
use crate::sources::crossref::CrossRefClient;
use crate::sources::LiteratureSource;
use crate::chunker::{chunk_document, ChunkerConfig, DocumentSection};
use crate::repository::IngestionRepository;
use crate::embedding::{EmbeddingClient, EmbeddingConfig, embed_pending_chunks};
use crate::pdf_parser::parse_pdf_sections;
use crate::models::SectionType;
use ferrumyx_ner::trie_ner::TrieNer;

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
    /// If provided, chunks are embedded immediately after insert.
    /// If None, a separate embed pass is needed (e.g. scheduled background job).
    pub embedding_cfg: Option<EmbeddingConfig>,
    /// Whether to attempt downloading paywalled PDFs via Sci-Hub.
    /// WARNING: Use at your own risk. Disabled by default.
    pub enable_scihub_fallback: bool,
}

/// Which literature sources to search.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IngestionSourceSpec {
    PubMed,
    EuropePmc,
    BioRxiv,
    MedRxiv,
    ClinicalTrials,
    CrossRef,
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
                IngestionSourceSpec::ClinicalTrials,
                IngestionSourceSpec::CrossRef,
            ],
            pubmed_api_key: None,
            embedding_cfg: None,
            enable_scihub_fallback: false,
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
pub struct IngestionResult {
    pub job_id: Uuid,
    pub query: String,
    pub papers_found: usize,
    pub papers_inserted: usize,
    pub papers_duplicate: usize,
    pub chunks_inserted: usize,
    pub chunks_embedded: usize,
    pub errors: Vec<String>,
    pub duration_ms: u64,
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

    // Build embedding client once if configured
    let embed_client = job.embedding_cfg.as_ref().map(|cfg| {
        info!("Embedding enabled: {:?} / {}", cfg.backend, cfg.model);
        Arc::new(EmbeddingClient::new(cfg.clone()))
    });

    // Initialize NER once for the entire pipeline (loads real databases)
    info!("Initializing NER with complete biomedical databases...");
    let ner = match tokio::task::spawn_blocking(|| TrieNer::with_complete_databases()).await.unwrap() {
        Ok(ner) => ner,
        Err(e) => {
            warn!("Failed to load complete databases, falling back to embedded subset: {}", e);
            tokio::task::spawn_blocking(|| TrieNer::with_embedded_subset()).await.unwrap()
        }
    };
    info!("NER initialized: {}patterns loaded", ner.stats().total_patterns);

    let mut result = IngestionResult {
        job_id,
        query: query.clone(),
        papers_found: 0,
        papers_inserted: 0,
        papers_duplicate: 0,
        chunks_inserted: 0,
        chunks_embedded: 0,
        errors: Vec::new(),
        duration_ms: 0,
    };

let prog_base = IngestionProgress::new(job_id, "search", "");
    emit("search", &format!("Searching with query: {query}"), prog_base.clone());

    // ── 1. Collect papers from all enabled sources ────────────────────────────
    let mut all_papers = Vec::new();

    for source in &job.sources {
        let source_result = match source {
            IngestionSourceSpec::PubMed => {
                let client = PubMedClient::new(job.pubmed_api_key.clone());
                client.search(&query, job.max_results).await
            }
            IngestionSourceSpec::EuropePmc => {
                let client = EuropePmcClient::new();
                client.search(&query, job.max_results).await
            }
            IngestionSourceSpec::BioRxiv => {
                let client = BioRxivClient::new_biorxiv();
                client.search(&query, job.max_results).await
            }
            IngestionSourceSpec::MedRxiv => {
                let client = BioRxivClient::new_medrxiv();
                client.search(&query, job.max_results).await
            }
            IngestionSourceSpec::ClinicalTrials => {
                let client = ClinicalTrialsClient::new();
                client.search(&query, job.max_results).await
            }
            IngestionSourceSpec::CrossRef => {
                let client = CrossRefClient::new();
                client.search(&query, job.max_results).await
            }
        };

        match source_result {
            Ok(papers) => {
                info!(source = ?source, n = papers.len(), "Papers retrieved");
                all_papers.extend(papers);
            }
            Err(e) => {
                let msg = format!("Source {:?} error: {e}", source);
                warn!("{}", &msg);
                result.errors.push(msg);
            }
        }
    }

    result.papers_found = all_papers.len();
    emit("upsert", &format!("{} papers found, deduplicating…", all_papers.len()), {
        let mut p = prog_base.clone();
        p.papers_found = all_papers.len();
        p
    });

    // ── 2. Upsert papers + chunk abstracts ───────────────────────────────────
    let chunker_cfg = ChunkerConfig::default();

    for paper in &all_papers {
        // Upsert
        let upsert = match repo.upsert_paper(paper).await {
            Ok(u) => u,
            Err(e) => {
                // Use DOI if PMID is not available (e.g., ClinicalTrials)
                let id = paper.pmid.as_ref()
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

        // Build sections from abstract (always available)
        let mut sections = build_sections_from_abstract(paper);
        
        // ── 3. Fetch full-text PDF if available ───────────────────────────────
        if let Some(ref pdf_url) = paper.full_text_url {
            match fetch_and_parse_pdf(pdf_url).await {
                Ok(pdf_sections) => {
                    if !pdf_sections.is_empty() {
                        info!(
                            paper_id = %upsert.paper_id,
                            n_sections = pdf_sections.len(),
                            "Full-text PDF parsed successfully"
                        );
                        sections.extend(pdf_sections);
                        // Mark as full-text available
                        let _ = repo.set_full_text_status(upsert.paper_id, true).await;
                    }
                }
                Err(e) => {
                    debug!(
                        paper_id = %upsert.paper_id,
                        error = %e,
                        "Full-text PDF fetch/parse failed, using abstract only"
                    );
                    // Non-fatal: continue with abstract-only
                }
            }
        }

        if sections.is_empty() {
            continue;
        }

        let chunks = chunk_document(upsert.paper_id, sections, &chunker_cfg);
        let n_chunks = chunks.len();

        // Extract entities from chunks using the pre-initialized NER
        for chunk in &chunks {
            let entities = ner.extract(&chunk.content);
            if !entities.is_empty() {
                // Store entities in database (linking to chunk)
                for entity in entities {
                    if let Err(e) = repo.insert_entity(
                        upsert.paper_id,
                        chunk.chunk_id,
                        entity.label.as_str(),
                        &entity.text,
                        None, // normalized_id - not available in ExtractedEntity
                        entity.confidence,
                    ).await {
                        warn!("Failed to insert entity: {}", e);
                    }
                }
            }
        }

        match repo.bulk_insert_chunks(&chunks).await {
            Ok(inserted) => {
                result.chunks_inserted += inserted;
                info!(
                    paper_id = %upsert.paper_id,
                    pmid = ?paper.pmid,
                    n_chunks,
                    "Paper ingested"
                );
            }
            Err(e) => {
                // Use DOI if PMID is not available (e.g., ClinicalTrials)
                let id = paper.pmid.as_ref()
                    .or(paper.doi.as_ref())
                    .map(|s| s.as_str())
                    .unwrap_or("unknown");
                let msg = format!("chunk insert failed for {}: {e}", id);
                warn!("{}", &msg);
                result.errors.push(msg);
                // Mark paper as failed so it can be retried
                let _ = repo.set_parse_status(upsert.paper_id, "failed").await;
            }
        }

        // Mark parsed
        let _ = repo.set_parse_status(upsert.paper_id, "parsed").await;

        // Embed chunks immediately if embedding is configured
        if let Some(ref ec) = embed_client {
            match embed_pending_chunks(ec.as_ref(), repo.as_ref(), upsert.paper_id).await {
                Ok(n) => {
                    result.chunks_embedded += n;
                    debug!(paper_id = %upsert.paper_id, n, "Chunks embedded");
                }
                Err(e) => {
                    let msg = format!("embed failed for {:?}: {e}", upsert.paper_id);
                    warn!("{}", &msg);
                    result.errors.push(msg);
                    // Non-fatal: paper is still ingested, just without vectors
                }
            }
        }
    }

    result.duration_ms = t0.elapsed().as_millis() as u64;

    info!(
        job_id = %job_id,
        papers_found    = result.papers_found,
        papers_inserted = result.papers_inserted,
        papers_dup      = result.papers_duplicate,
        chunks          = result.chunks_inserted,
        duration_ms     = result.duration_ms,
        errors          = result.errors.len(),
        "Ingestion pipeline complete"
    );

    emit("complete", &format!(
        "Done. {} new papers, {} chunks ({} embedded), {} duplicates skipped.",
        result.papers_inserted, result.chunks_inserted, result.chunks_embedded, result.papers_duplicate
    ), {
        let mut p = prog_base.clone();
        p.papers_found    = result.papers_found;
        p.papers_inserted = result.papers_inserted;
        p.chunks_inserted = result.chunks_inserted;
        p
    });

    result
}

// ── Query builder ─────────────────────────────────────────────────────────────

/// Build a PubMed/Europe PMC compatible search query.
pub fn build_query(job: &IngestionJob) -> String {
    let mut parts = vec![
        format!("{}[tiab]", job.gene),
        format!("{}[tiab]", job.cancer_type),
    ];
    if let Some(ref m) = job.mutation {
        parts.push(format!("{m}[tiab]"));
    }
    parts.join(" AND ")
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

// ── Full-text PDF fetcher ─────────────────────────────────────────────────────

/// Download a PDF from URL and parse it with Ferrules.
/// Returns sections extracted from the PDF.
async fn fetch_and_parse_pdf(pdf_url: &str) -> anyhow::Result<Vec<DocumentSection>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .user_agent("Ferrumyx/0.1 (research)")
        .build()?;

    // Download PDF
    let resp = client.get(pdf_url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("PDF download failed: HTTP {}", resp.status());
    }

    let pdf_bytes = resp.bytes().await?;
    
    // Write to temp file (lopdf requires file path)
    let mut temp_file = NamedTempFile::new()?;
    std::io::Write::write_all(&mut temp_file, &pdf_bytes)?;
    let temp_path = temp_file.path().to_path_buf();

    // Parse with Ferrules
    let parsed = tokio::task::spawn_blocking(move || {
        parse_pdf_sections(&temp_path)
    }).await??;

    info!(
        title = ?parsed.title,
        n_sections = parsed.sections.len(),
        page_count = parsed.page_count,
        "PDF parsed with Ferrules"
    );

    Ok(parsed.sections)
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
        assert!(q.contains("KRAS[tiab]"));
        assert!(q.contains("G12D[tiab]"));
        assert!(q.contains("pancreatic cancer[tiab]"));
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
        assert!(q.contains("TP53[tiab]"));
        assert!(!q.contains("[tiab] AND [tiab]")); // no empty mutation
    }

    #[test]
    fn test_build_sections_abstract_only() {
        use crate::models::{Author, IngestionSource};
        let paper = crate::models::PaperMetadata {
            doi: None, pmid: Some("1234".to_string()), pmcid: None,
            title: "KRAS G12D".to_string(),
            abstract_text: Some("Abstract content here.".to_string()),
            authors: vec![Author { name: "Smith J".to_string(), affiliation: None, orcid: None }],
            journal: None, pub_date: None,
            source: IngestionSource::PubMed,
            open_access: false, full_text_url: None,
        };
        let sections = build_sections_from_abstract(&paper);
        assert!(sections.iter().any(|s| s.section_type == SectionType::Abstract));
        assert!(sections.iter().any(|s| s.heading.as_deref() == Some("Title")));
    }
}
