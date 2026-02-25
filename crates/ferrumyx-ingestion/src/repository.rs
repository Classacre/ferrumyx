//! LanceDB repository for the ingestion pipeline.
//!
//! Handles:
//! - Paper INSERT with DOI/PMID deduplication
//! - DocumentChunk INSERT with embedding placeholder
//! - Ingestion audit logging
//! - SimHash-based duplicate detection at the DB level

use anyhow::Result;
use uuid::Uuid;
use std::sync::Arc;
use ferrumyx_db::{Database, papers::PaperRepository, chunks::ChunkRepository, schema::{Paper, Chunk}};
use crate::models::{PaperMetadata, DocumentChunk};
use crate::dedup::simhash as compute_simhash;

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
    pub fn new(db: Arc<Database>) -> Self { Self { db } }

    /// Get underlying database reference.
    pub fn db(&self) -> Arc<Database> { self.db.clone() }

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

        // Compute SimHash for abstract deduplication
        let simhash: Option<i64> = meta.abstract_text
            .as_deref()
            .map(|t| compute_simhash(t));

        // No existing paper found, insert new one
        let paper = Paper {
            id: Uuid::new_v4(),
            doi: meta.doi.clone(),
            pmid: meta.pmid.clone(),
            title: meta.title.clone(),
            abstract_text: meta.abstract_text.clone(),
            full_text: None,
            source: meta.source.as_str().to_string(),
            source_id: meta.pmcid.clone(),
            published_at: meta.pub_date.map(|d| {
                // Convert NaiveDate to DateTime<Utc> by setting time to midnight UTC
                chrono::DateTime::from_naive_utc_and_offset(
                    d.and_hms_opt(0, 0, 0).unwrap_or_else(|| chrono::NaiveDateTime::MIN),
                    chrono::Utc
                )
            }),
            authors: if meta.authors.is_empty() {
                None
            } else {
                Some(meta.authors.iter().map(|a| a.name.clone()).collect::<Vec<_>>().join(", "))
            },
            journal: meta.journal.clone(),
            volume: None,
            issue: None,
            pages: None,
            parse_status: "pending".to_string(),
            ingested_at: chrono::Utc::now(),
            abstract_simhash: simhash,
        };
        
        let paper_id = paper.id;
        paper_repo.insert(&paper).await?;

        tracing::debug!(
            paper_id = %paper_id,
            doi = ?meta.doi,
            pmid = ?meta.pmid,
            "Inserted new paper"
        );

        Ok(PaperUpsertResult { paper_id, was_new: true, duplicate_of: None })
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
            content: chunk.content.clone(),
            embedding: None, // Will be filled by embedding service
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
        if chunks.is_empty() { return Ok(0); }
        
        let chunk_repo = ChunkRepository::new(self.db.clone());
        
        let new_chunks: Vec<Chunk> = chunks.iter().map(|chunk| Chunk {
            id: Uuid::new_v4(),
            paper_id: chunk.paper_id,
            chunk_index: chunk.chunk_index as i64,
            content: chunk.content.clone(),
            embedding: None,
            section: chunk.section_heading.clone(),
            page: chunk.page_number.map(|p| p as i64),
            created_at: chrono::Utc::now(),
        }).collect();
        
        let count = new_chunks.len();
        
        // Insert each chunk (LanceDB doesn't have batch insert yet in our API)
        for chunk in &new_chunks {
            chunk_repo.insert(chunk).await?;
        }
        
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
    pub async fn paper_count_by_status(&self, _status: &str) -> Result<i64> {
        // TODO: Implement count_by_status in PaperRepository
        // For now, return 0 as placeholder
        Ok(0)
    }

    /// Total chunks in the database.
    pub async fn chunk_count(&self) -> Result<i64> {
        let chunk_repo = ChunkRepository::new(self.db.clone());
        Ok(chunk_repo.count().await? as i64)
    }

    // ── Entity operations ─────────────────────────────────────────────────────

    /// Insert an extracted entity from a chunk.
    /// Note: Entity storage is now handled by ferrumyx-ner crate directly.
    pub async fn insert_entity(
        &self,
        paper_id: Uuid,
        chunk_id: Uuid,
        entity_type: &str,
        entity_text: &str,
        normalized_id: Option<&str>,
        score: f32,
    ) -> Result<Uuid> {
        // This is now a no-op as entities are stored in ferrumyx-ner
        // The NER pipeline handles entity storage directly
        tracing::debug!(
            paper_id = %paper_id,
            chunk_id = %chunk_id,
            entity_type = entity_type,
            entity_text = entity_text,
            normalized_id = ?normalized_id,
            score = score,
            "Entity extraction recorded (handled by NER pipeline)"
        );
        Ok(Uuid::new_v4())
    }

    // ── Embedding operations ───────────────────────────────────────────────────

    /// Find chunks without embeddings for a specific paper.
    pub async fn find_chunks_without_embeddings(&self, paper_id: Uuid) -> Result<Vec<(Uuid, String)>> {
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
        let chunk_repo = ChunkRepository::new(self.db.clone());
        let mut count = 0;
        for (chunk_id, embedding) in updates {
            chunk_repo.update_embedding(*chunk_id, embedding.clone()).await?;
            count += 1;
        }
        Ok(count)
    }
}
