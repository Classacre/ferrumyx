//! LanceDB repository for the ingestion pipeline.
//!
//! Handles:
//! - Paper INSERT with DOI/PMID deduplication
//! - DocumentChunk INSERT with embedding placeholder
//! - Ingestion audit logging
//! - SimHash-based duplicate detection at the DB level

use crate::dedup::{check_fuzzy_duplicate, hamming_distance, simhash, DedupResult};
use crate::models::{Author, DocumentChunk, IngestionSource, PaperMetadata};
use anyhow::Result;
use ferrumyx_db::{
    chunks::ChunkRepository,
    papers::PaperRepository,
    schema::{Chunk, Paper},
    Database,
};
use std::sync::Arc;
use uuid::Uuid;

/// Result of a paper upsert.
#[derive(Debug)]
pub struct PaperUpsertResult {
    pub paper_id: Uuid,
    pub was_new: bool,
    pub duplicate_of: Option<Uuid>,
}

/// LanceDB ingestion repository.
#[derive(Clone)]
pub struct IngestionRepository {
    db: Arc<Database>,
}

impl IngestionRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Get underlying database reference.
    pub fn db(&self) -> Arc<Database> {
        self.db.clone()
    }

    // ── Paper operations ─────────────────────────────────────────────────────

    /// Insert a paper, skipping if DOI or PMID already exists.
    /// Returns the paper UUID and whether it was newly inserted.
    pub async fn upsert_paper(&self, meta: &PaperMetadata) -> Result<PaperUpsertResult> {
        let paper_repo = PaperRepository::new(self.db.clone());

        // Check if a paper with this DOI or PMID already exists
        if let Some(doi) = &meta.doi {
            if let Some(existing) = paper_repo.find_by_doi(doi).await? {
                tracing::debug!(
                    paper_id = %existing.id,
                    doi = ?meta.doi,
                    "Paper already exists by DOI, skipping insert"
                );
                return Ok(PaperUpsertResult {
                    paper_id: existing.id,
                    was_new: false,
                    duplicate_of: None,
                });
            }
        }

        if let Some(pmid) = &meta.pmid {
            if let Some(existing) = paper_repo.find_by_pmid(pmid).await? {
                tracing::debug!(
                    paper_id = %existing.id,
                    pmid = ?meta.pmid,
                    "Paper already exists by PMID, skipping insert"
                );
                return Ok(PaperUpsertResult {
                    paper_id: existing.id,
                    was_new: false,
                    duplicate_of: None,
                });
            }
        }

        // Optional Stage 2/3 lexical fuzzy dedup. Disabled by default because
        // it can over-collapse distinct papers at large ingestion scale.
        let strict_fuzzy_dedup = std::env::var("FERRUMYX_STRICT_FUZZY_DEDUP")
            .ok()
            .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
        if strict_fuzzy_dedup {
            let recent = paper_repo.list(0, 500).await.unwrap_or_default();
            if let Some(incoming_abstract) = meta
                .abstract_text
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                let incoming_hash = simhash(incoming_abstract);
                if let Some(existing) = recent.iter().find(|p| {
                    p.abstract_simhash
                        .map(|h| hamming_distance(incoming_hash, h) <= 6)
                        .unwrap_or(false)
                }) {
                    tracing::debug!(
                        paper_id = %existing.id,
                        "Paper deduplicated by strict abstract SimHash distance <= 6"
                    );
                    return Ok(PaperUpsertResult {
                        paper_id: existing.id,
                        was_new: false,
                        duplicate_of: Some(existing.id),
                    });
                }
            }

            let existing_meta: Vec<PaperMetadata> = recent.iter().map(paper_to_metadata).collect();
            match check_fuzzy_duplicate(meta, existing_meta.iter()) {
                DedupResult::ProbableDuplicate { method, similarity } => {
                    if let Some(existing) = existing_meta.iter().find(|p| {
                        p.title.eq_ignore_ascii_case(&meta.title)
                            || strsim::jaro_winkler(
                                &meta.title.to_lowercase(),
                                &p.title.to_lowercase(),
                            ) >= similarity.max(0.98)
                    }) {
                        if let Some(db_row) = recent.iter().find(|p| p.title == existing.title) {
                            tracing::debug!(
                                paper_id = %db_row.id,
                                method = %method,
                                similarity = similarity,
                                "Paper deduplicated by strict fuzzy title/author"
                            );
                            return Ok(PaperUpsertResult {
                                paper_id: db_row.id,
                                was_new: false,
                                duplicate_of: Some(db_row.id),
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        let simhash: Option<i64> = meta
            .abstract_text
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(simhash);

        let paper = Paper {
            id: Uuid::new_v4(),
            doi: meta.doi.clone(),
            pmid: meta.pmid.clone(),
            title: meta.title.clone(),
            abstract_text: meta.abstract_text.clone(),
            full_text: None,
            raw_json: None,
            source: meta.source.as_str().to_string(),
            source_id: meta.pmcid.clone(),
            published_at: meta.pub_date.map(|d| {
                // Convert NaiveDate to DateTime<Utc> by setting time to midnight UTC
                chrono::DateTime::from_naive_utc_and_offset(
                    d.and_hms_opt(0, 0, 0)
                        .unwrap_or_else(|| chrono::NaiveDateTime::MIN),
                    chrono::Utc,
                )
            }),
            authors: if meta.authors.is_empty() {
                None
            } else {
                Some(
                    meta.authors
                        .iter()
                        .map(|a| a.name.clone())
                        .collect::<Vec<_>>()
                        .join(", "),
                )
            },
            journal: meta.journal.clone(),
            volume: None,
            issue: None,
            pages: None,
            parse_status: "pending".to_string(),
            open_access: meta.open_access,
            retrieval_tier: None,
            ingested_at: chrono::Utc::now(),
            abstract_simhash: simhash,
            published_version_doi: None,
        };

        let paper_id = paper.id;
        paper_repo.insert(&paper).await?;

        tracing::debug!(
            paper_id = %paper_id,
            doi = ?meta.doi,
            pmid = ?meta.pmid,
            "Inserted new paper"
        );

        Ok(PaperUpsertResult {
            paper_id,
            was_new: true,
            duplicate_of: None,
        })
    }

    /// Fast existence check by DOI/PMID identity.
    pub async fn exists_paper_identity(&self, meta: &PaperMetadata) -> Result<bool> {
        let paper_repo = PaperRepository::new(self.db.clone());
        if let Some(doi) = &meta.doi {
            if paper_repo.find_by_doi(doi).await?.is_some() {
                return Ok(true);
            }
        }
        if let Some(pmid) = &meta.pmid {
            if paper_repo.find_by_pmid(pmid).await?.is_some() {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Mark a paper's parse_status as 'parsed' or 'failed'.
    pub async fn set_parse_status(&self, paper_id: Uuid, status: &str) -> Result<()> {
        let paper_repo = PaperRepository::new(self.db.clone());
        paper_repo.update_parse_status(paper_id, status).await?;
        Ok(())
    }

    /// Mark whether a paper has full-text available (PDF parsed successfully).
    pub async fn set_full_text_status(&self, paper_id: Uuid, has_full_text: bool) -> Result<()> {
        // In LanceDB, we store full_text directly, so this is just a status update
        // The actual full text is stored when PDF is parsed
        tracing::debug!(paper_id = %paper_id, has_full_text = has_full_text, "Setting full text status");
        Ok(())
    }

    // ── Chunk operations ─────────────────────────────────────────────────────

    /// Insert a document chunk. Embedding is null until the embedding service runs.
    pub async fn insert_chunk(&self, chunk: &DocumentChunk) -> Result<Uuid> {
        let chunk_repo = ChunkRepository::new(self.db.clone());

        let new_chunk = Chunk {
            id: Uuid::new_v4(),
            paper_id: chunk.paper_id,
            chunk_index: chunk.chunk_index as i64,
            token_count: chunk.token_count as i32,
            content: chunk.content.clone(),
            embedding: None, // Will be filled by embedding service
            embedding_large: None,
            section: chunk.section_heading.clone(),
            page: chunk.page_number.map(|p| p as i64),
            created_at: chrono::Utc::now(),
        };

        let id = new_chunk.id;
        chunk_repo.insert(&new_chunk).await?;
        Ok(id)
    }

    /// Bulk insert chunks for a paper.
    /// This is significantly faster than individual inserts.
    pub async fn bulk_insert_chunks(&self, chunks: &[DocumentChunk]) -> Result<usize> {
        if chunks.is_empty() {
            return Ok(0);
        }

        let chunk_repo = ChunkRepository::new(self.db.clone());

        let new_chunks: Vec<Chunk> = chunks
            .iter()
            .map(|chunk| Chunk {
                id: Uuid::new_v4(),
                paper_id: chunk.paper_id,
                chunk_index: chunk.chunk_index as i64,
                token_count: chunk.token_count as i32,
                content: chunk.content.clone(),
                embedding: None,
                embedding_large: None,
                section: chunk.section_heading.clone(),
                page: chunk.page_number.map(|p| p as i64),
                created_at: chrono::Utc::now(),
            })
            .collect();

        let count = new_chunks.len();
        chunk_repo.insert_batch(&new_chunks).await?;

        tracing::debug!("bulk_insert_chunks: inserted {} chunks", count);
        Ok(count)
    }

    // ── Stats ────────────────────────────────────────────────────────────────

    /// Total papers in the database.
    pub async fn paper_count(&self) -> Result<i64> {
        let paper_repo = PaperRepository::new(self.db.clone());
        Ok(paper_repo.count().await? as i64)
    }

    /// Papers with a given parse_status.
    pub async fn paper_count_by_status(&self, status: &str) -> Result<i64> {
        let paper_repo = PaperRepository::new(self.db.clone());
        Ok(paper_repo.count_by_parse_status(status).await? as i64)
    }

    /// Total chunks in the database.
    pub async fn chunk_count(&self) -> Result<i64> {
        let chunk_repo = ChunkRepository::new(self.db.clone());
        Ok(chunk_repo.count().await? as i64)
    }

    // ── Entity operations ─────────────────────────────────────────────────────

    /// Insert a knowledge graph fact.
    pub async fn insert_fact(
        &self,
        paper_id: uuid::Uuid,
        subject_id: uuid::Uuid,
        subject_name: &str,
        predicate: &str,
        object_id: uuid::Uuid,
        object_name: &str,
        confidence: f32,
    ) -> Result<uuid::Uuid> {
        let fact_repo = ferrumyx_db::kg_facts::KgFactRepository::new(self.db.clone());
        let mut fact = ferrumyx_db::schema::KgFact::new(
            paper_id,
            subject_id,
            subject_name.to_string(),
            predicate.to_string(),
            object_id,
            object_name.to_string(),
        );
        fact.confidence = confidence;

        fact_repo.insert(&fact).await?;
        Ok(fact.id)
    }

    /// Bulk insert facts.
    pub async fn bulk_insert_facts(&self, facts: &[ferrumyx_db::schema::KgFact]) -> Result<usize> {
        if facts.is_empty() {
            return Ok(0);
        }
        let fact_repo = ferrumyx_db::kg_facts::KgFactRepository::new(self.db.clone());
        let count = facts.len();
        fact_repo.insert_batch(facts).await?;
        Ok(count)
    }

    // ── Embedding operations ───────────────────────────────────────────────────

    /// Find chunks without embeddings for a specific paper.
    pub async fn find_chunks_without_embeddings(
        &self,
        paper_id: Uuid,
    ) -> Result<Vec<(Uuid, String)>> {
        let chunk_repo = ChunkRepository::new(self.db.clone());
        let chunks = chunk_repo.find_by_paper_id(paper_id).await?;

        // Filter to only those without embeddings
        let pending: Vec<(Uuid, String)> = chunks
            .into_iter()
            .filter(|c| c.embedding.is_none())
            .map(|c| (c.id, c.content))
            .collect();

        Ok(pending)
    }

    /// Update the embedding for a chunk.
    pub async fn update_chunk_embedding(&self, chunk_id: Uuid, embedding: Vec<f32>) -> Result<()> {
        let chunk_repo = ChunkRepository::new(self.db.clone());
        chunk_repo.update_embedding(chunk_id, embedding).await?;
        Ok(())
    }

    /// Bulk update embeddings for multiple chunks.
    pub async fn bulk_update_embeddings(&self, updates: &[(Uuid, Vec<f32>)]) -> Result<usize> {
        if updates.is_empty() {
            return Ok(0);
        }
        let concurrency = std::env::var("FERRUMYX_INGESTION_EMBED_UPDATE_CONCURRENCY")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(8)
            .clamp(1, 64);
        let mut set = tokio::task::JoinSet::new();
        let mut next_idx = 0usize;
        let mut updated = 0usize;
        while next_idx < updates.len() || !set.is_empty() {
            while next_idx < updates.len() && set.len() < concurrency {
                let (chunk_id, embedding) = updates[next_idx].clone();
                let db = self.db.clone();
                set.spawn(async move {
                    let repo = ChunkRepository::new(db);
                    repo.update_embedding(chunk_id, embedding)
                        .await
                        .ok()
                        .map(|_| 1usize)
                        .unwrap_or(0)
                });
                next_idx += 1;
            }
            if let Some(joined) = set.join_next().await {
                updated += joined.unwrap_or(0);
            }
        }
        Ok(updated)
    }
}

fn paper_to_metadata(paper: &Paper) -> PaperMetadata {
    let authors = paper
        .authors
        .as_deref()
        .map(|joined| {
            joined
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|name| Author {
                    name: name.to_string(),
                    affiliation: None,
                    orcid: None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    PaperMetadata {
        doi: paper.doi.clone(),
        pmid: paper.pmid.clone(),
        pmcid: paper.source_id.clone(),
        title: paper.title.clone(),
        abstract_text: paper.abstract_text.clone(),
        authors,
        journal: paper.journal.clone(),
        pub_date: paper.published_at.map(|d| d.date_naive()),
        source: IngestionSource::PubMed,
        open_access: paper.open_access,
        full_text_url: None,
    }
}
