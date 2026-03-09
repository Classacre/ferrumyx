//! Paper repository.
//!
//! Provides CRUD operations for paper metadata.

use crate::database::Database;
use crate::error::Result;
use crate::schema::Paper;
use crate::schema_arrow::{paper_to_record, record_to_paper};
use std::sync::Arc;
use std::collections::HashMap;
use futures::StreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use arrow_array::{Array, StringArray};

/// Repository for paper operations.
#[derive(Clone)]
pub struct PaperRepository {
    db: Arc<Database>,
}

impl PaperRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
    
    /// Insert a new paper.
    pub async fn insert(&self, paper: &Paper) -> Result<()> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_PAPERS)
            .execute()
            .await?;
        
        let record = paper_to_record(paper)?;
        let schema = record.schema();
        let iter = arrow_array::RecordBatchIterator::new(vec![Ok(record)], schema);
        
        table.add(iter).execute().await?;
        Ok(())
    }
    
    /// Insert multiple papers in bulk.
    pub async fn insert_batch(&self, papers: &[Paper]) -> Result<()> {
        if papers.is_empty() {
            return Ok(());
        }
        
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_PAPERS)
            .execute()
            .await?;
        
        let records: Vec<arrow_array::RecordBatch> = papers
            .iter()
            .map(paper_to_record)
            .collect::<Result<_>>()?;
        
        let schema = records[0].schema();
        let iter = arrow_array::RecordBatchIterator::new(
            records.into_iter().map(Ok),
            schema,
        );
        
        table.add(iter).execute().await?;
        Ok(())
    }
    
    /// Find a paper by ID.
    pub async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<Paper>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_PAPERS)
            .execute()
            .await?;
        
        let mut stream = table
            .query()
            .only_if(&format!("id = '{}'", id))
            .execute()
            .await?;
        
        if let Some(batch) = stream.next().await {
            let batch = batch?;
            if batch.num_rows() > 0 {
                return Ok(Some(record_to_paper(&batch, 0)?));
            }
        }
        
        Ok(None)
    }
    
    /// Find a paper by DOI.
    pub async fn find_by_doi(&self, doi: &str) -> Result<Option<Paper>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_PAPERS)
            .execute()
            .await?;
        
        // Escape single quotes in DOI
        let escaped = doi.replace('\'', "''");
        
        let mut stream = table
            .query()
            .only_if(&format!("doi = '{}'", escaped))
            .execute()
            .await?;
        
        if let Some(batch) = stream.next().await {
            let batch = batch?;
            if batch.num_rows() > 0 {
                return Ok(Some(record_to_paper(&batch, 0)?));
            }
        }
        
        Ok(None)
    }
    
    /// Find a paper by PMID.
    pub async fn find_by_pmid(&self, pmid: &str) -> Result<Option<Paper>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_PAPERS)
            .execute()
            .await?;
        
        let escaped = pmid.replace('\'', "''");
        
        let mut stream = table
            .query()
            .only_if(&format!("pmid = '{}'", escaped))
            .execute()
            .await?;
        
        if let Some(batch) = stream.next().await {
            let batch = batch?;
            if batch.num_rows() > 0 {
                return Ok(Some(record_to_paper(&batch, 0)?));
            }
        }
        
        Ok(None)
    }
    
    /// Find all papers from a specific source.
    pub async fn find_by_source(&self, source: &str) -> Result<Vec<Paper>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_PAPERS)
            .execute()
            .await?;
        
        let escaped = source.replace('\'', "''");
        
        let mut stream = table
            .query()
            .only_if(&format!("source = '{}'", escaped))
            .execute()
            .await?;
        
        let mut papers = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                papers.push(record_to_paper(&batch, i)?);
            }
        }
        
        Ok(papers)
    }
    
    /// Find papers with pending parse status.
    pub async fn find_pending(&self) -> Result<Vec<Paper>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_PAPERS)
            .execute()
            .await?;
        
        let mut stream = table
            .query()
            .only_if("parse_status = 'pending'")
            .execute()
            .await?;
        
        let mut papers = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                papers.push(record_to_paper(&batch, i)?);
            }
        }
        
        Ok(papers)
    }
    
    /// Update a paper.
    pub async fn update(&self, paper: &Paper) -> Result<()> {
        // LanceDB doesn't have direct update, so we use merge_insert
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_PAPERS)
            .execute()
            .await?;
        
        let record = paper_to_record(paper)?;
        let schema = record.schema();
        let iter = arrow_array::RecordBatchIterator::new(vec![Ok(record)], schema);
        
        let mut builder = table.merge_insert(&["id"]);
        builder.when_matched_update_all(None);
        builder.execute(Box::new(iter)).await?;
        
        Ok(())
    }
    
    /// Update parse status for a paper.
    pub async fn update_parse_status(&self, id: uuid::Uuid, status: &str) -> Result<()> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_PAPERS)
            .execute()
            .await?;
        
        // Use column() with a literal value expression
        let escaped = status.replace('\'', "''");
        table.update()
            .only_if(&format!("id = '{}'", id))
            .column("parse_status", format!("'{}'", escaped))
            .execute()
            .await?;
        
        Ok(())
    }
    
    /// Delete a paper by ID.
    pub async fn delete(&self, id: uuid::Uuid) -> Result<()> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_PAPERS)
            .execute()
            .await?;
        table.delete(&format!("id = '{}'", id)).await?;
        Ok(())
    }
    
    /// Count total papers.
    pub async fn count(&self) -> Result<u64> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_PAPERS)
            .execute()
            .await?;
        Ok(table.count_rows(None).await? as u64)
    }

    /// Resolve paper titles for a list of IDs in one query.
    pub async fn find_titles_by_ids(&self, ids: &[uuid::Uuid]) -> Result<HashMap<uuid::Uuid, String>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }

        let mut uniq: Vec<uuid::Uuid> = ids.to_vec();
        uniq.sort_unstable();
        uniq.dedup();

        let table = self.db.connection()
            .open_table(crate::schema::TABLE_PAPERS)
            .execute()
            .await?;

        let in_clause = uniq
            .iter()
            .map(|id| format!("'{}'", id))
            .collect::<Vec<_>>()
            .join(",");

        let mut stream = table
            .query()
            .only_if(&format!("id IN ({})", in_clause))
            .select(lancedb::query::Select::columns(&["id", "title"]))
            .execute()
            .await?;

        let mut out = HashMap::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            let schema = batch.schema();
            let id_idx = match schema.index_of("id") {
                Ok(i) => i,
                Err(_) => continue,
            };
            let title_idx = match schema.index_of("title") {
                Ok(i) => i,
                Err(_) => continue,
            };

            let ids_arr = match batch.column(id_idx).as_any().downcast_ref::<StringArray>() {
                Some(a) => a,
                None => continue,
            };
            let titles_arr = match batch.column(title_idx).as_any().downcast_ref::<StringArray>() {
                Some(a) => a,
                None => continue,
            };

            let row_count = batch.num_rows();
            for i in 0..row_count {
                if ids_arr.is_null(i) || titles_arr.is_null(i) {
                    continue;
                }

                if let Ok(id) = uuid::Uuid::parse_str(ids_arr.value(i)) {
                    out.insert(id, titles_arr.value(i).to_string());
                }
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

        let mut uniq: Vec<uuid::Uuid> = ids.to_vec();
        uniq.sort_unstable();
        uniq.dedup();

        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_PAPERS)
            .execute()
            .await?;

        let in_clause = uniq
            .iter()
            .map(|id| format!("'{}'", id))
            .collect::<Vec<_>>()
            .join(",");

        let mut stream = table
            .query()
            .only_if(&format!("id IN ({})", in_clause))
            .select(lancedb::query::Select::columns(&["id", "published_at"]))
            .execute()
            .await?;

        let mut out = HashMap::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            let schema = batch.schema();
            let id_idx = match schema.index_of("id") {
                Ok(i) => i,
                Err(_) => continue,
            };
            let published_idx = match schema.index_of("published_at") {
                Ok(i) => i,
                Err(_) => continue,
            };

            let ids_arr = match batch.column(id_idx).as_any().downcast_ref::<StringArray>() {
                Some(a) => a,
                None => continue,
            };
            let published_arr =
                match batch.column(published_idx).as_any().downcast_ref::<StringArray>() {
                    Some(a) => a,
                    None => continue,
                };

            let row_count = batch.num_rows();
            for i in 0..row_count {
                if ids_arr.is_null(i) || published_arr.is_null(i) {
                    continue;
                }

                let Ok(id) = uuid::Uuid::parse_str(ids_arr.value(i)) else {
                    continue;
                };
                let Ok(dt) = chrono::DateTime::parse_from_rfc3339(published_arr.value(i)) else {
                    continue;
                };

                out.insert(id, dt.with_timezone(&chrono::Utc));
            }
        }

        Ok(out)
    }
    
    /// Count papers by source.
    pub async fn count_by_source(&self, source: &str) -> Result<u64> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_PAPERS)
            .execute()
            .await?;
        let escaped = source.replace('\'', "''");
        let count = table.count_rows(Some(format!("source = '{}'", escaped))).await?;
        Ok(count as u64)
    }

    /// Count papers by parse status.
    pub async fn count_by_parse_status(&self, status: &str) -> Result<u64> {
        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_PAPERS)
            .execute()
            .await?;
        let escaped = status.replace('\'', "''");
        let count = table
            .count_rows(Some(format!("parse_status = '{}'", escaped)))
            .await?;
        Ok(count as u64)
    }
    
    /// List papers with pagination.
    pub async fn list(&self, offset: usize, limit: usize) -> Result<Vec<Paper>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_PAPERS)
            .execute()
            .await?;
        
        let mut stream = table
            .query()
            .limit(limit)
            .offset(offset)
            .execute()
            .await?;
        
        let mut papers = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                papers.push(record_to_paper(&batch, i)?);
            }
        }
        
        Ok(papers)
    }
}
