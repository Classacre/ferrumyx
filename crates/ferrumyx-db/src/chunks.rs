//! Chunk repository.
//!
//! Provides CRUD operations for text chunks with vector search.

use crate::database::Database;
use crate::error::Result;
use crate::schema::Chunk;
use crate::schema_arrow::{chunk_to_record, record_to_chunk};
use std::sync::Arc;
use futures::StreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};

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
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_CHUNKS)
            .execute()
            .await?;
        
        let record = chunk_to_record(chunk)?;
        let schema = record.schema();
        let iter = arrow_array::RecordBatchIterator::new(vec![Ok(record)], schema);
        
        table.add(iter).execute().await?;
        Ok(())
    }
    
    /// Insert multiple chunks in bulk.
    pub async fn insert_batch(&self, chunks: &[Chunk]) -> Result<()> {
        if chunks.is_empty() {
            return Ok(());
        }
        
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_CHUNKS)
            .execute()
            .await?;
        
        let records: Vec<arrow_array::RecordBatch> = chunks
            .iter()
            .map(chunk_to_record)
            .collect::<Result<_>>()?;
        
        let schema = records[0].schema();
        let iter = arrow_array::RecordBatchIterator::new(
            records.into_iter().map(Ok),
            schema,
        );
        
        table.add(iter).execute().await?;
        Ok(())
    }
    
    /// Find a chunk by ID.
    pub async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<Chunk>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_CHUNKS)
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
                return Ok(Some(record_to_chunk(&batch, 0)?));
            }
        }
        
        Ok(None)
    }
    
    /// Find all chunks for a paper.
    pub async fn find_by_paper_id(&self, paper_id: uuid::Uuid) -> Result<Vec<Chunk>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_CHUNKS)
            .execute()
            .await?;
        
        let mut stream = table
            .query()
            .only_if(&format!("paper_id = '{}'", paper_id))
            .execute()
            .await?;
        
        let mut chunks = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                chunks.push(record_to_chunk(&batch, i)?);
            }
        }
        
        Ok(chunks)
    }
    
    /// Delete all chunks for a paper.
    pub async fn delete_by_paper_id(&self, paper_id: uuid::Uuid) -> Result<()> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_CHUNKS)
            .execute()
            .await?;
        table.delete(&format!("paper_id = '{}'", paper_id)).await?;
        Ok(())
    }
    
    /// Delete a chunk by ID.
    pub async fn delete(&self, id: uuid::Uuid) -> Result<()> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_CHUNKS)
            .execute()
            .await?;
        table.delete(&format!("id = '{}'", id)).await?;
        Ok(())
    }
    
    /// Count total chunks.
    pub async fn count(&self) -> Result<u64> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_CHUNKS)
            .execute()
            .await?;
        Ok(table.count_rows(None).await? as u64)
    }
    
    /// Count chunks for a paper.
    pub async fn count_by_paper_id(&self, paper_id: uuid::Uuid) -> Result<u64> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_CHUNKS)
            .execute()
            .await?;
        let count = table
            .count_rows(Some(format!("paper_id = '{}'", paper_id)))
            .await?;
        Ok(count as u64)
    }
    
    /// List chunks with pagination.
    pub async fn list(&self, offset: usize, limit: usize) -> Result<Vec<Chunk>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_CHUNKS)
            .execute()
            .await?;
        
        let mut stream = table
            .query()
            .limit(limit)
            .offset(offset)
            .execute()
            .await?;
        
        let mut chunks = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                chunks.push(record_to_chunk(&batch, i)?);
            }
        }
        
        Ok(chunks)
    }
    
    /// Search for similar chunks using vector similarity.
    /// 
    /// Returns the top-k most similar chunks to the given query vector.
    pub async fn search_similar(
        &self,
        query_vector: &[f32],
        k: usize,
    ) -> Result<Vec<Chunk>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_CHUNKS)
            .execute()
            .await?;
        
        let mut stream = table
            .vector_search(query_vector.to_vec())?
            .limit(k)
            .execute()
            .await?;
        
        let mut chunks = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                chunks.push(record_to_chunk(&batch, i)?);
            }
        }
        
        Ok(chunks)
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
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_CHUNKS)
            .execute()
            .await?;
        
        let mut stream = table
            .vector_search(query_vector.to_vec())?
            .only_if(filter)
            .limit(k)
            .execute()
            .await?;
        
        let mut chunks = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                chunks.push(record_to_chunk(&batch, i)?);
            }
        }
        
        Ok(chunks)
    }
    
    /// Update the embedding for a chunk.
    /// 
    /// This is done by deleting the old chunk and inserting a new one with the embedding.
    pub async fn update_embedding(&self, chunk_id: uuid::Uuid, embedding: Vec<f32>) -> Result<()> {
        // First, get the existing chunk
        let existing = self.find_by_id(chunk_id).await?
            .ok_or_else(|| crate::error::DbError::NotFound(format!("Chunk {} not found", chunk_id)))?;
        
        // Delete the old chunk
        self.delete(chunk_id).await?;
        
        // Insert with new embedding
        let updated = Chunk {
            id: existing.id,
            paper_id: existing.paper_id,
            chunk_index: existing.chunk_index,
            content: existing.content,
            embedding: Some(embedding),
            section: existing.section,
            page: existing.page,
            created_at: existing.created_at,
        };
        
        self.insert(&updated).await?;
        Ok(())
    }
}
