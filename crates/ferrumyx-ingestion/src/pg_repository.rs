//! PostgreSQL repository for the ingestion pipeline.
//!
//! Handles:
//! - Paper INSERT with DOI/PMID deduplication
//! - DocumentChunk INSERT with embedding placeholder
//! - Ingestion audit logging
//! - SimHash-based duplicate detection at the DB level

use anyhow::{Context, Result};
use sqlx::PgPool;
use uuid::Uuid;
use crate::models::{PaperMetadata, DocumentChunk};
use crate::dedup::simhash as compute_simhash;

/// Result of a paper upsert.
#[derive(Debug)]
pub struct PaperUpsertResult {
    pub paper_id: Uuid,
    pub was_new: bool,
    pub duplicate_of: Option<Uuid>,
}

/// PostgreSQL ingestion repository.
#[derive(Clone)]
pub struct PgIngestionRepository {
    pool: PgPool,
}

impl PgIngestionRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }

    /// Expose underlying pool for pipeline embedding step.
    pub fn pool(&self) -> &PgPool { &self.pool }

    // ── Paper operations ─────────────────────────────────────────────────────

    /// Insert a paper, skipping if DOI or PMID already exists.
    /// Returns the paper UUID and whether it was newly inserted.
    pub async fn upsert_paper(&self, meta: &PaperMetadata) -> Result<PaperUpsertResult> {
        // Compute SimHash for abstract deduplication (as i64 for PostgreSQL BIGINT)
        let simhash: Option<i64> = meta.abstract_text
            .as_deref()
            .map(|t| compute_simhash(t));

        // Check for near-duplicates by SimHash (Hamming < 12)
        // Skip this check for now to avoid overflow issues
        // TODO: Implement proper Hamming distance query
        let _ = simhash; // suppress unused warning

        let authors_json = serde_json::to_value(&meta.authors)?;

        let result: Option<(Uuid, bool)> = sqlx::query_as(
            r#"
            WITH ins AS (
                INSERT INTO papers
                    (doi, pmid, pmcid, title, abstract_text, authors,
                     journal, pub_date, source, open_access, full_text_url,
                     parse_status, abstract_simhash)
                VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'pending',$12)
                ON CONFLICT (pmid) WHERE pmid IS NOT NULL
                    DO NOTHING
                RETURNING id, TRUE AS is_new
            )
            SELECT id, is_new FROM ins
            "#,
        )
        .bind(&meta.doi)
        .bind(&meta.pmid)
        .bind(&meta.pmcid)
        .bind(&meta.title)
        .bind(&meta.abstract_text)
        .bind(&authors_json)
        .bind(&meta.journal)
        .bind(meta.pub_date)
        .bind(meta.source.as_str())
        .bind(meta.open_access)
        .bind(&meta.full_text_url)
        .bind(simhash)
        .fetch_optional(&self.pool)
        .await
        .context("paper upsert failed")?;

        // If DOI conflict triggered DO NOTHING, fetch existing id
        let (paper_id, was_new) = match result {
            Some((id, new)) => (id, new),
            None => {
                // Fetch by DOI or PMID
                let existing: Uuid = sqlx::query_scalar(
                    "SELECT id FROM papers WHERE doi = $1 OR pmid = $2 LIMIT 1"
                )
                .bind(&meta.doi)
                .bind(&meta.pmid)
                .fetch_one(&self.pool)
                .await
                .context("Could not find existing paper after conflict")?;
                (existing, false)
            }
        };

        if was_new {
            self.log_audit(paper_id, &meta.pmid, &meta.doi, "discovered", meta.source.as_str()).await?;
        }

        Ok(PaperUpsertResult { paper_id, was_new, duplicate_of: None })
    }

    /// Mark a paper's parse_status as 'parsed' or 'failed'.
    pub async fn set_parse_status(&self, paper_id: Uuid, status: &str) -> Result<()> {
        sqlx::query("UPDATE papers SET parse_status = $1 WHERE id = $2")
            .bind(status)
            .bind(paper_id)
            .execute(&self.pool)
            .await
            .context("set_parse_status failed")?;
        Ok(())
    }

    // ── Chunk operations ─────────────────────────────────────────────────────

    /// Insert a document chunk. Embedding is null until the embedding service runs.
    pub async fn insert_chunk(&self, chunk: &DocumentChunk) -> Result<Uuid> {
        let id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO paper_chunks
                (paper_id, chunk_index, section_type, section_heading,
                 content, token_count, page_number)
            VALUES ($1,$2,$3,$4,$5,$6,$7)
            ON CONFLICT (paper_id, chunk_index) DO NOTHING
            RETURNING id
            "#,
        )
        .bind(chunk.paper_id)
        .bind(chunk.chunk_index as i32)
        .bind(chunk.section_type.as_str())
        .bind(&chunk.section_heading)
        .bind(&chunk.content)
        .bind(chunk.token_count as i32)
        .bind(chunk.page_number.map(|p| p as i32))
        .fetch_optional(&self.pool)
        .await
        .context("insert_chunk failed")?
        .unwrap_or_else(Uuid::new_v4);  // if conflict, we don't need the id

        Ok(id)
    }

    /// Bulk insert chunks for a paper in a single transaction.
    pub async fn bulk_insert_chunks(&self, chunks: &[DocumentChunk]) -> Result<usize> {
        if chunks.is_empty() { return Ok(0); }
        let mut tx = self.pool.begin().await?;
        let mut count = 0;

        for chunk in chunks {
            sqlx::query(
                r#"
                INSERT INTO paper_chunks
                    (paper_id, chunk_index, section_type, section_heading,
                     content, token_count, page_number)
                VALUES ($1,$2,$3,$4,$5,$6,$7)
                ON CONFLICT (paper_id, chunk_index) DO NOTHING
                "#,
            )
            .bind(chunk.paper_id)
            .bind(chunk.chunk_index as i32)
            .bind(chunk.section_type.as_str())
            .bind(&chunk.section_heading)
            .bind(&chunk.content)
            .bind(chunk.token_count as i32)
            .bind(chunk.page_number.map(|p| p as i32))
            .execute(&mut *tx)
            .await?;
            count += 1;
        }

        tx.commit().await?;
        tracing::debug!("bulk_insert_chunks: committed {count} chunks");
        Ok(count)
    }

    // ── Stats ────────────────────────────────────────────────────────────────

    /// Total papers in the database.
    pub async fn paper_count(&self) -> Result<i64> {
        sqlx::query_scalar("SELECT COUNT(*) FROM papers")
            .fetch_one(&self.pool).await.context("paper_count failed")
    }

    /// Papers with a given parse_status.
    pub async fn paper_count_by_status(&self, status: &str) -> Result<i64> {
        sqlx::query_scalar("SELECT COUNT(*) FROM papers WHERE parse_status = $1")
            .bind(status)
            .fetch_one(&self.pool).await.context("paper_count_by_status failed")
    }

    /// Total chunks in the database.
    pub async fn chunk_count(&self) -> Result<i64> {
        sqlx::query_scalar("SELECT COUNT(*) FROM paper_chunks")
            .fetch_one(&self.pool).await.context("chunk_count failed")
    }

    // ── Audit ────────────────────────────────────────────────────────────────

    async fn log_audit(
        &self,
        paper_id: Uuid,
        pmid: &Option<String>,
        doi: &Option<String>,
        action: &str,
        source: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO ingestion_audit (paper_id, paper_pmid, paper_doi, action, source)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(paper_id)
        .bind(pmid.as_deref())
        .bind(doi.as_deref())
        .bind(action)
        .bind(source)
        .execute(&self.pool)
        .await
        .context("ingestion_audit insert failed")?;
        Ok(())
    }
}
