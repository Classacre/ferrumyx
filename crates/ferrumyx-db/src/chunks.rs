//! Chunk repository.
//!
//! Provides CRUD operations for text chunks with vector search.

use crate::database::Database;
use crate::error::Result;
use crate::schema::Chunk;
use pgvector::Vector;
use std::collections::HashMap;
use std::sync::Arc;
use tokio_postgres::{Row, types::ToSql};

/// Repository for chunk operations.
#[derive(Clone)]
pub struct ChunkRepository {
    db: Arc<Database>,
}

impl ChunkRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Insert a new chunk.
    pub async fn insert(&self, chunk: &Chunk) -> Result<()> {
        let client = self.db.client();
        let embedding = chunk.embedding.as_ref().map(|v| Vector::from(v.clone()));
        client.execute(
            "INSERT INTO chunks (id, paper_id, chunk_index, token_count, content, section, page, created_at, embedding) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), $8)",
            &[
                &chunk.id,
                &chunk.paper_id,
                &chunk.chunk_index,
                &chunk.token_count,
                &chunk.content,
                &chunk.section,
                &chunk.page,
                &embedding,
            ],
        ).await?;
        Ok(())
    }

    /// Insert multiple chunks in bulk.
    pub async fn insert_batch(&self, chunks: &[Chunk]) -> Result<()> {
        if chunks.is_empty() {
            return Ok(());
        }
        for chunk in chunks {
            self.insert(chunk).await?;
        }
        Ok(())
    }

    /// Find a chunk by ID.
    pub async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<Chunk>> {
        let client = self.db.client();
        let row = client.query_opt("SELECT * FROM chunks WHERE id = $1", &[&id]).await?;
        Ok(row.map(chunk_from_row))
    }

    /// Find all chunks for a paper.
    pub async fn find_by_paper_id(&self, paper_id: uuid::Uuid) -> Result<Vec<Chunk>> {
        let client = self.db.client();
        let rows = client.query("SELECT * FROM chunks WHERE paper_id = $1", &[&paper_id]).await?;
        Ok(rows.into_iter().map(chunk_from_row).collect())
    }

    /// Find chunks by IDs in one query.
    pub async fn find_by_ids(&self, ids: &[uuid::Uuid]) -> Result<HashMap<uuid::Uuid, Chunk>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut uniq = ids.to_vec();
        uniq.sort_unstable();
        uniq.dedup();
        let client = self.db.client();
        let placeholders = (1..=uniq.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(",");
        let sql = format!("SELECT * FROM chunks WHERE id IN ({})", placeholders);
        let params: Vec<&(dyn ToSql + Sync)> = uniq.iter().map(|id| id as &(dyn ToSql + Sync)).collect();
        let rows = client.query(&sql, params.as_slice()).await?;
        let mut out = HashMap::with_capacity(rows.len());
        for row in rows {
            let chunk = chunk_from_row(row);
            out.insert(chunk.id, chunk);
        }
        Ok(out)
    }

    /// Delete all chunks for a paper.
    pub async fn delete_by_paper_id(&self, paper_id: uuid::Uuid) -> Result<()> {
        let client = self.db.client();
        client.execute("DELETE FROM chunks WHERE paper_id = $1", &[&paper_id]).await?;
        Ok(())
    }

    /// Delete a chunk by ID.
    pub async fn delete(&self, id: uuid::Uuid) -> Result<()> {
        let client = self.db.client();
        client.execute("DELETE FROM chunks WHERE id = $1", &[&id]).await?;
        Ok(())
    }

    /// Count total chunks.
    pub async fn count(&self) -> Result<u64> {
        let client = self.db.client();
        let row = client.query_one("SELECT COUNT(*) FROM chunks", &[]).await?;
        Ok(row.get::<_, i64>(0) as u64)
    }

    /// Count chunks for a paper.
    pub async fn count_by_paper_id(&self, paper_id: uuid::Uuid) -> Result<u64> {
        let client = self.db.client();
        let row = client.query_one(
            "SELECT COUNT(*) FROM chunks WHERE paper_id = $1",
            &[&paper_id],
        ).await?;
        Ok(row.get::<_, i64>(0) as u64)
    }

    /// List chunks with pagination.
    pub async fn list(&self, _offset: usize, _limit: usize) -> Result<Vec<Chunk>> {
        let client = self.db.client();
        let rows = client.query("SELECT * FROM chunks ORDER BY created_at DESC", &[]).await?;
        Ok(rows.into_iter().map(chunk_from_row).collect())
    }

    /// Search for similar chunks using vector similarity.
    ///
    /// Returns the top-k most similar chunks to the given query vector.
    pub async fn search_similar(&self, query_vector: &[f32], k: usize) -> Result<Vec<Chunk>> {
        let client = self.db.client();
        let query_vec = Vector::from(query_vector.to_vec());
        let sql = format!("SELECT * FROM chunks ORDER BY embedding <=> $1 LIMIT {}", k);
        let rows = client.query(&sql, &[&query_vec]).await?;
        Ok(rows.into_iter().map(chunk_from_row).collect())
    }

    /// Search for similar chunks with a filter.
    ///
    /// Returns the top-k most similar chunks that match the filter.
    pub async fn search_similar_filtered(
        &self,
        query_vector: &[f32],
        k: usize,
        filter: &str,
    ) -> Result<Vec<Chunk>> {
        let client = self.db.client();
        let query_vec = Vector::from(query_vector.to_vec());
        let sql = format!("SELECT * FROM chunks WHERE {} ORDER BY embedding <=> $1 LIMIT {}", filter, k);
        let rows = client.query(&sql, &[&query_vec]).await?;
        Ok(rows.into_iter().map(chunk_from_row).collect())
    }

    /// Update the embedding for a chunk.
    ///
    /// This is done by deleting the old chunk and inserting a new one with the embedding.
    pub async fn update_embedding(&self, chunk_id: uuid::Uuid, embedding: Vec<f32>) -> Result<()> {
        let existing = self.find_by_id(chunk_id).await?.ok_or_else(|| {
            crate::error::DbError::NotFound(format!("Chunk {} not found", chunk_id))
        })?;
        self.delete(chunk_id).await?;
        let updated = Chunk {
            id: existing.id,
            paper_id: existing.paper_id,
            chunk_index: existing.chunk_index,
            token_count: existing.token_count,
            content: existing.content,
            embedding: Some(embedding),
            embedding_large: None,
            section: existing.section,
            page: existing.page,
            created_at: existing.created_at,
        };
        self.insert(&updated).await?;
        Ok(())
    }

    /// Batch-update chunk embeddings using a single merge-upsert operation.
    pub async fn update_embeddings_batch(
        &self,
        updates: &[(uuid::Uuid, Vec<f32>)],
    ) -> Result<usize> {
        if updates.is_empty() {
            return Ok(0);
        }
        let mut latest_by_id: HashMap<uuid::Uuid, Vec<f32>> = HashMap::with_capacity(updates.len());
        for (id, embedding) in updates {
            latest_by_id.insert(*id, embedding.clone());
        }
        let ids: Vec<uuid::Uuid> = latest_by_id.keys().copied().collect();
        let existing = self.find_by_ids(&ids).await?;
        if existing.is_empty() {
            return Ok(0);
        }
        let mut rows = Vec::with_capacity(existing.len());
        for (id, embedding) in latest_by_id {
            if let Some(chunk) = existing.get(&id) {
                let updated = Chunk {
                    id: chunk.id,
                    paper_id: chunk.paper_id,
                    chunk_index: chunk.chunk_index,
                    token_count: chunk.token_count,
                    content: chunk.content.clone(),
                    embedding: Some(embedding),
                    embedding_large: None,
                    section: chunk.section.clone(),
                    page: chunk.page,
                    created_at: chunk.created_at,
                };
                rows.push(updated);
            }
        }
        if rows.is_empty() {
            return Ok(0);
        }
        for row in &rows {
            self.insert(row).await?;
        }
        Ok(rows.len())
    }
}

fn chunk_from_row(row: Row) -> Chunk {
    Chunk {
        id: row.get("id"),
        paper_id: row.get("paper_id"),
        chunk_index: row.get("chunk_index"),
        token_count: row.get("token_count"),
        content: row.get("content"),
        section: row.get("section"),
        page: row.get("page"),
        created_at: row.get::<_, chrono::DateTime<chrono::Utc>>("created_at"),
        embedding: row.get("embedding"),
        embedding_large: None,
    }
}
