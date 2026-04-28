//! Paper repository.
//!
//! Provides CRUD operations for paper metadata.

use crate::database::Database;
use crate::error::Result;
use crate::schema::Paper;
use std::collections::HashMap;
use std::sync::Arc;
use tokio_postgres::{Row, types::ToSql};

/// Repository for paper operations.
#[derive(Clone)]
pub struct PaperRepository {
    db: Arc<Database>,
}

#[derive(Debug, Clone, Default)]
pub struct PaperNoveltySignal {
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
    pub citation_count: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct PaperReference {
    pub title: String,
    pub doi: Option<String>,
    pub pmid: Option<String>,
    pub source: Option<String>,
    pub source_id: Option<String>,
    pub published_version_doi: Option<String>,
}

impl PaperRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Insert a new paper.
    pub async fn insert(&self, paper: &Paper) -> Result<()> {
        let client = self.db.client();
        client.execute(
            "INSERT INTO papers (id, doi, pmid, title, abstract, authors, journal, pub_date, source, open_access, full_text_url, ingested_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())",
            &[
                &paper.id,
                &paper.doi,
                &paper.pmid,
                &paper.title,
                &paper.abstract_text,
                &paper.authors,
                &paper.journal,
                &paper.published_at,
                &paper.source,
                &paper.open_access,
                &paper.full_text,
            ],
        ).await?;
        Ok(())
    }

    /// Insert multiple papers in bulk.
    pub async fn insert_batch(&self, papers: &[Paper]) -> Result<()> {
        if papers.is_empty() {
            return Ok(());
        }
        for paper in papers {
            self.insert(paper).await?;
        }
        Ok(())
    }

    /// Find a paper by ID.
    pub async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<Paper>> {
        let client = self.db.client();
        let row = client.query_opt("SELECT * FROM papers WHERE id = $1", &[&id]).await?;
        Ok(row.map(paper_from_row))
    }

    /// Find a paper by DOI.
    pub async fn find_by_doi(&self, doi: &str) -> Result<Option<Paper>> {
        let client = self.db.client();
        let row = client.query_opt("SELECT * FROM papers WHERE doi = $1", &[&doi]).await?;
        Ok(row.map(paper_from_row))
    }

    /// Find a paper by PMID.
    pub async fn find_by_pmid(&self, pmid: &str) -> Result<Option<Paper>> {
        let client = self.db.client();
        let row = client.query_opt("SELECT * FROM papers WHERE pmid = $1", &[&pmid]).await?;
        Ok(row.map(paper_from_row))
    }

    /// Find all papers from a specific source.
    pub async fn find_by_source(&self, source: &str) -> Result<Vec<Paper>> {
        let client = self.db.client();
        let rows = client.query("SELECT * FROM papers WHERE source = $1", &[&source]).await?;
        Ok(rows.into_iter().map(paper_from_row).collect())
    }

    /// Find papers with pending parse status.
    pub async fn find_pending(&self) -> Result<Vec<Paper>> {
        let client = self.db.client();
        let rows = client.query("SELECT * FROM papers WHERE parse_status = 'pending'", &[]).await?;
        Ok(rows.into_iter().map(paper_from_row).collect())
    }

    /// Update a paper.
    pub async fn update(&self, paper: &Paper) -> Result<()> {
        let client = self.db.client();
        client.execute(
            "UPDATE papers SET doi = $2, pmid = $3, title = $4, abstract = $5, authors = $6, \
             journal = $7, pub_date = $8, source = $9, open_access = $10, full_text_url = $11, ingested_at = NOW() \
             WHERE id = $1",
            &[
                &paper.id,
                &paper.doi,
                &paper.pmid,
                &paper.title,
                &paper.abstract_text,
                &paper.authors,
                &paper.journal,
                &paper.published_at,
                &paper.source,
                &paper.open_access,
                &paper.full_text,
            ],
        ).await?;
        Ok(())
    }

    /// Update parse status for a paper.
    pub async fn update_parse_status(&self, id: uuid::Uuid, status: &str) -> Result<()> {
        let client = self.db.client();
        client.execute(
            "UPDATE papers SET parse_status = $1 WHERE id = $2",
            &[&status, &id],
        ).await?;
        Ok(())
    }

    /// Delete a paper by ID.
    pub async fn delete(&self, id: uuid::Uuid) -> Result<()> {
        let client = self.db.client();
        client.execute("DELETE FROM papers WHERE id = $1", &[&id]).await?;
        Ok(())
    }

    /// Count total papers.
    pub async fn count(&self) -> Result<u64> {
        let client = self.db.client();
        let row = client.query_one("SELECT COUNT(*) FROM papers", &[]).await?;
        Ok(row.get::<_, i64>(0) as u64)
    }

    /// Resolve paper titles for a list of IDs in one query.
    pub async fn find_titles_by_ids(
        &self,
        ids: &[uuid::Uuid],
    ) -> Result<HashMap<uuid::Uuid, String>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let client = self.db.client();
        let placeholders = (1..=ids.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(",");
        let sql = format!("SELECT id, title FROM papers WHERE id IN ({})", placeholders);
        let params: Vec<&(dyn ToSql + Sync)> = ids.iter().map(|id| id as &(dyn ToSql + Sync)).collect();
        let rows = client.query(&sql, params.as_slice()).await?;
        let mut out = HashMap::with_capacity(rows.len());
        for row in rows {
            let id: uuid::Uuid = row.get("id");
            let title: String = row.get("title");
            out.insert(id, title);
        }
        Ok(out)
    }

    /// Resolve paper title + identifier metadata for a list of IDs in one query.
    pub async fn find_references_by_ids(
        &self,
        ids: &[uuid::Uuid],
    ) -> Result<HashMap<uuid::Uuid, PaperReference>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let client = self.db.client();
        let placeholders = (1..=ids.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT id, title, doi, pmid, source, source_id, published_version_doi FROM papers WHERE id IN ({})",
            placeholders
        );
        let params: Vec<&(dyn ToSql + Sync)> = ids.iter().map(|id| id as &(dyn ToSql + Sync)).collect();
        let rows = client.query(&sql, params.as_slice()).await?;
        let mut out = HashMap::with_capacity(rows.len());
        for row in rows {
            let id: uuid::Uuid = row.get("id");
            out.insert(
                id,
                PaperReference {
                    title: row.get("title"),
                    doi: row.get("doi"),
                    pmid: row.get("pmid"),
                    source: row.get("source"),
                    source_id: row.get("source_id"),
                    published_version_doi: row.get("published_version_doi"),
                },
            );
        }
        Ok(out)
    }

    /// Resolve existing paper IDs for DOI values in bounded query chunks.
    pub async fn find_ids_by_dois(
        &self,
        dois: &[String],
        chunk_size: usize,
    ) -> Result<HashMap<String, uuid::Uuid>> {
        self.find_ids_by_identity_values("doi", dois, chunk_size).await
    }

    /// Resolve existing paper IDs for PMID values in bounded query chunks.
    pub async fn find_ids_by_pmids(
        &self,
        pmids: &[String],
        chunk_size: usize,
    ) -> Result<HashMap<String, uuid::Uuid>> {
        self.find_ids_by_identity_values("pmid", pmids, chunk_size).await
    }

    async fn find_ids_by_identity_values(
        &self,
        field: &str,
        values: &[String],
        chunk_size: usize,
    ) -> Result<HashMap<String, uuid::Uuid>> {
        if values.is_empty() {
            return Ok(HashMap::new());
        }
        let mut uniq = values.iter().map(|v| v.trim().to_string()).filter(|v| !v.is_empty()).collect::<Vec<_>>();
        uniq.sort_unstable();
        uniq.dedup();
        if uniq.is_empty() {
            return Ok(HashMap::new());
        }
        let client = self.db.client();
        let mut out = HashMap::new();
        let chunk = chunk_size.max(1).min(300);
        for group in uniq.chunks(chunk) {
            let placeholders = (1..=group.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(",");
            let sql = format!("SELECT id, {} FROM papers WHERE {} IN ({})", field, field, placeholders);
            let params: Vec<&(dyn ToSql + Sync)> = group.iter().map(|s| s as &(dyn ToSql + Sync)).collect();
            let rows = client.query(&sql, params.as_slice()).await?;
            for row in rows {
                let id: uuid::Uuid = row.get("id");
                let val: String = row.get(field);
                out.insert(val, id);
            }
        }
        Ok(out)
    }

    /// Resolve paper publication timestamps for a list of IDs in one query.
    pub async fn find_published_at_by_ids(
        &self,
        ids: &[uuid::Uuid],
    ) -> Result<HashMap<uuid::Uuid, chrono::DateTime<chrono::Utc>>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let client = self.db.client();
        let placeholders = (1..=ids.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(",");
        let sql = format!("SELECT id, published_at FROM papers WHERE id IN ({})", placeholders);
        let params: Vec<&(dyn ToSql + Sync)> = ids.iter().map(|id| id as &(dyn ToSql + Sync)).collect();
        let rows = client.query(&sql, params.as_slice()).await?;
        let mut out = HashMap::with_capacity(rows.len());
        for row in rows {
            let id: uuid::Uuid = row.get("id");
            if let Some(pa) = row.get::<_, Option<chrono::DateTime<chrono::Utc>>>("published_at") {
                out.insert(id, pa);
            }
        }
        Ok(out)
    }

    /// Resolve publication + citation metadata for novelty scoring.
    pub async fn find_novelty_signals_by_ids(
        &self,
        ids: &[uuid::Uuid],
    ) -> Result<HashMap<uuid::Uuid, PaperNoveltySignal>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let client = self.db.client();
        let placeholders = (1..=ids.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(",");
        let sql = format!("SELECT id, published_at, raw_json FROM papers WHERE id IN ({})", placeholders);
        let params: Vec<&(dyn ToSql + Sync)> = ids.iter().map(|id| id as &(dyn ToSql + Sync)).collect();
        let rows = client.query(&sql, params.as_slice()).await?;
        let mut out = HashMap::with_capacity(rows.len());
        for row in rows {
            let id: uuid::Uuid = row.get("id");
            let published_at: Option<chrono::DateTime<chrono::Utc>> = row.get("published_at");
            let raw_json: Option<String> = row.get("raw_json");
            let citation_count = raw_json.and_then(|rj| extract_citation_count(&rj));
            out.insert(
                id,
                PaperNoveltySignal {
                    published_at,
                    citation_count,
                },
            );
        }
        Ok(out)
    }

    /// Count papers by source.
    pub async fn count_by_source(&self, source: &str) -> Result<u64> {
        let client = self.db.client();
        let row = client.query_one(
            "SELECT COUNT(*) FROM papers WHERE source = $1",
            &[&source],
        ).await?;
        Ok(row.get::<_, i64>(0) as u64)
    }

    /// Count papers by parse status.
    pub async fn count_by_parse_status(&self, status: &str) -> Result<u64> {
        let client = self.db.client();
        let row = client.query_one(
            "SELECT COUNT(*) FROM papers WHERE parse_status = $1",
            &[&status],
        ).await?;
        Ok(row.get::<_, i64>(0) as u64)
    }

    /// List papers with pagination.
    pub async fn list(&self, _offset: usize, _limit: usize) -> Result<Vec<Paper>> {
        let client = self.db.client();
        let rows = client.query("SELECT * FROM papers ORDER BY ingested_at DESC", &[]).await?;
        Ok(rows.into_iter().map(paper_from_row).collect())
    }
}

fn paper_from_row(row: Row) -> Paper {
    Paper {
        id: row.get("id"),
        doi: row.get("doi"),
        pmid: row.get("pmid"),
        title: row.get("title"),
        abstract_text: row.get("abstract"),
        full_text: row.get("full_text"),
        raw_json: row.get("raw_json"),
        source: row.get("source"),
        source_id: row.get("source_id"),
        published_at: row.get("pub_date"),
        authors: row.get("authors"),
        journal: row.get("journal"),
        volume: None,
        issue: None,
        pages: None,
        parse_status: row.get("parse_status"),
        open_access: row.get("open_access"),
        retrieval_tier: None,
        ingested_at: row.get::<_, chrono::DateTime<chrono::Utc>>("ingested_at"),
        abstract_simhash: None,
        published_version_doi: row.get("published_version_doi"),
    }
}

fn extract_citation_count(raw_json: &str) -> Option<u32> {
    let parsed: serde_json::Value = serde_json::from_str(raw_json).ok()?;

    fn as_u32(v: &serde_json::Value) -> Option<u32> {
        if let Some(n) = v.as_u64() {
            return Some(n.min(u32::MAX as u64) as u32);
        }
        if let Some(s) = v.as_str() {
            return s.trim().parse::<u32>().ok();
        }
        None
    }

    as_u32(&parsed["citation_count"])
        .or_else(|| as_u32(&parsed["citationCount"]))
        .or_else(|| as_u32(&parsed["cited_by_count"]))
        .or_else(|| as_u32(&parsed["citation_count_total"]))
        .or_else(|| as_u32(&parsed["metrics"]["citation_count"]))
        .or_else(|| as_u32(&parsed["metrics"]["citationCount"]))
        .or_else(|| as_u32(&parsed["external"]["semantic_scholar"]["citationCount"]))
}

#[cfg(test)]
mod tests {
    use super::extract_citation_count;

    #[test]
    fn citation_extract_supports_common_keys() {
        assert_eq!(extract_citation_count(r#"{"citationCount": 42}"#), Some(42));
        assert_eq!(
            extract_citation_count(r#"{"metrics": {"citation_count": 17}}"#),
            Some(17)
        );
        assert_eq!(
            extract_citation_count(r#"{"external": {"semantic_scholar": {"citationCount": 89}}}"#),
            Some(89)
        );
    }

    #[test]
    fn citation_extract_handles_missing_or_invalid_payload() {
        assert_eq!(extract_citation_count("{}"), None);
        assert_eq!(extract_citation_count("{not-json"), None);
    }
}
