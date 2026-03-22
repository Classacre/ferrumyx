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
use arrow_array::{RecordBatch, RecordBatchIterator, StringArray};
use arrow_schema::{DataType, Field, Schema};
use ferrumyx_db::{
    chunks::ChunkRepository,
    papers::PaperRepository,
    schema::{Chunk, Paper, TABLE_CHUNKS, TABLE_INGESTION_AUDIT},
    schema_arrow::record_to_chunk,
    Database,
};
use futures::StreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use serde_json::{json, Value};
use std::collections::HashSet;
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
                self.record_duplicate_audit(
                    existing.id,
                    "doi",
                    json!({
                        "method": "doi",
                        "matched_paper_id": existing.id,
                        "doi": truncate_audit_detail(doi, 160),
                    }),
                )
                .await;
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
                self.record_duplicate_audit(
                    existing.id,
                    "pmid",
                    json!({
                        "method": "pmid",
                        "matched_paper_id": existing.id,
                        "pmid": truncate_audit_detail(pmid, 64),
                    }),
                )
                .await;
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

        // When identifier metadata is missing, use a normalized title-key
        // guard to suppress repeated cross-source copies of the same paper.
        if meta.doi.is_none() && meta.pmid.is_none() {
            if let Some(incoming_title_key) = canonical_title_identity(&meta.title) {
                let recent = paper_repo.list(0, 1_500).await.unwrap_or_default();
                if let Some(existing) = recent.iter().find(|p| {
                    canonical_title_identity(&p.title)
                        .is_some_and(|key| title_identity_match(&incoming_title_key, &key))
                }) {
                    self.record_duplicate_audit(
                        existing.id,
                        "title",
                        json!({
                            "method": "title",
                            "matched_paper_id": existing.id,
                            "title_key": truncate_audit_detail(&incoming_title_key, 160),
                        }),
                    )
                    .await;
                    tracing::debug!(
                        paper_id = %existing.id,
                        title = %meta.title,
                        "Paper deduplicated by canonical title identity"
                    );
                    return Ok(PaperUpsertResult {
                        paper_id: existing.id,
                        was_new: false,
                        duplicate_of: Some(existing.id),
                    });
                }
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
                    self.record_duplicate_audit(
                        existing.id,
                        "strict_fuzzy",
                        json!({
                            "method": "strict_simhash",
                            "matched_paper_id": existing.id,
                            "distance_max": 6,
                        }),
                    )
                    .await;
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
                            self.record_duplicate_audit(
                                db_row.id,
                                "strict_fuzzy",
                                json!({
                                    "method": "strict_fuzzy",
                                    "matched_paper_id": db_row.id,
                                    "rule": truncate_audit_detail(&method, 64),
                                    "similarity": round_similarity(similarity),
                                }),
                            )
                            .await;
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
        let table = self
            .db
            .connection()
            .open_table(TABLE_CHUNKS)
            .execute()
            .await?;
        let missing_filter = format!("paper_id = '{}' AND embedding IS NULL", paper_id);
        match table.query().only_if(&missing_filter).execute().await {
            Ok(mut stream) => {
                let mut pending = Vec::new();
                while let Some(batch) = stream.next().await {
                    let batch = batch?;
                    for i in 0..batch.num_rows() {
                        let chunk = record_to_chunk(&batch, i)?;
                        pending.push((chunk.id, chunk.content));
                    }
                }
                return Ok(pending);
            }
            Err(err) => {
                tracing::debug!(
                    paper_id = %paper_id,
                    error = %err,
                    "Chunk embedding-null DB filter unsupported; falling back to client-side filtering"
                );
            }
        }

        let chunks = chunk_repo.find_by_paper_id(paper_id).await?;
        Ok(chunks
            .into_iter()
            .filter(|c| c.embedding.is_none())
            .map(|c| (c.id, c.content))
            .collect())
    }

    /// Find paper IDs that still have chunks without embeddings, using a bounded
    /// paper scan as a fallback for manual backfill jobs.
    pub async fn pending_embedding_paper_ids(&self, scan_limit: usize) -> Result<Vec<Uuid>> {
        if scan_limit == 0 {
            return Ok(Vec::new());
        }

        let table = self
            .db
            .connection()
            .open_table(TABLE_CHUNKS)
            .execute()
            .await?;
        let max_chunk_scan = scan_limit.saturating_mul(128).clamp(64, 20_000);
        if let Ok(mut stream) = table
            .query()
            .only_if("embedding IS NULL")
            .limit(max_chunk_scan)
            .execute()
            .await
        {
            let mut seen = HashSet::new();
            let mut out = Vec::new();
            while let Some(batch) = stream.next().await {
                let batch = batch?;
                for i in 0..batch.num_rows() {
                    let chunk = record_to_chunk(&batch, i)?;
                    if seen.insert(chunk.paper_id) {
                        out.push(chunk.paper_id);
                        if out.len() >= scan_limit {
                            return Ok(out);
                        }
                    }
                }
            }
            if !out.is_empty() {
                return Ok(out);
            }
        }

        // Fallback for backends that cannot execute the `embedding IS NULL`
        // predicate reliably.
        let paper_repo = PaperRepository::new(self.db.clone());
        let papers = paper_repo.list(0, scan_limit).await?;
        let mut out = Vec::new();

        for paper in papers {
            if !self.find_chunks_without_embeddings(paper.id).await?.is_empty() {
                out.push(paper.id);
            }
        }

        Ok(out)
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

        let batch_size = std::env::var("FERRUMYX_INGESTION_EMBED_UPDATE_BATCH_SIZE")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(256)
            .clamp(1, 2_048);

        let chunk_repo = ChunkRepository::new(self.db.clone());
        let mut updated = 0usize;
        for batch in updates.chunks(batch_size) {
            updated += chunk_repo.update_embeddings_batch(batch).await?;
        }
        Ok(updated)
    }

    async fn record_duplicate_audit(&self, paper_id: Uuid, method: &str, detail: Value) {
        let action = "deduplicated";
        let payload = json!({
            "method": method,
            "detail": detail,
        });
        if let Err(err) = self
            .write_ingestion_audit(action, payload.to_string(), Some(paper_id))
            .await
        {
            tracing::warn!(
                paper_id = %paper_id,
                method,
                error = %err,
                "Failed to persist ingestion_audit dedup event"
            );
        }
        tracing::debug!(
            paper_id = %paper_id,
            method,
            detail = %payload,
            "Ingestion dedup audit event"
        );
    }

    async fn write_ingestion_audit(
        &self,
        action: &str,
        detail: String,
        paper_id: Option<Uuid>,
    ) -> Result<()> {
        let table = self
            .db
            .connection()
            .open_table(TABLE_INGESTION_AUDIT)
            .execute()
            .await?;

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("job_id", DataType::Utf8, true),
            Field::new("paper_id", DataType::Utf8, true),
            Field::new("action", DataType::Utf8, false),
            Field::new("detail", DataType::Utf8, false),
            Field::new("created_at", DataType::Utf8, false),
        ]));

        let now = chrono::Utc::now().to_rfc3339();
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(StringArray::from(vec![Uuid::new_v4().to_string()])),
                Arc::new(StringArray::from(vec![Option::<String>::None])),
                Arc::new(StringArray::from(vec![paper_id.map(|v| v.to_string())])),
                Arc::new(StringArray::from(vec![action.to_string()])),
                Arc::new(StringArray::from(vec![detail])),
                Arc::new(StringArray::from(vec![now])),
            ],
        )?;
        let iter = RecordBatchIterator::new(vec![Ok(batch)], schema);
        table.add(iter).execute().await?;
        Ok(())
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

fn canonical_title_identity(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut normalized = String::with_capacity(trimmed.len());
    let mut prev_space = false;
    for ch in trimmed.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            ' '
        };
        if mapped == ' ' {
            if prev_space {
                continue;
            }
            prev_space = true;
            normalized.push(' ');
        } else {
            prev_space = false;
            normalized.push(mapped);
        }
    }
    let stop_words = [
        "a", "an", "and", "the", "of", "for", "to", "in", "on", "by", "with", "from", "against",
        "at", "as", "into",
    ];
    let compact: Vec<&str> = normalized
        .split_whitespace()
        .filter(|t| t.len() > 1 && !stop_words.contains(t))
        .take(36)
        .collect();
    if compact.is_empty() {
        None
    } else {
        Some(compact.join(" "))
    }
}

fn title_identity_match(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    if a.len() >= 42 && b.len() >= 42 && (a.starts_with(b) || b.starts_with(a)) {
        return true;
    }
    strsim::jaro_winkler(a, b) >= 0.992
}

fn truncate_audit_detail(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    trimmed.chars().take(max_chars).collect()
}

fn round_similarity(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}
