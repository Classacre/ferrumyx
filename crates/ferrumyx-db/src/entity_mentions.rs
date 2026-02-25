//! Entity mention repository.
//!
//! Tracks where entities are mentioned in chunks (for provenance).

use crate::database::Database;
use crate::error::Result;
use crate::schema::EntityMention;
use crate::schema_arrow::{entity_mention_to_record, record_to_entity_mention};
use std::sync::Arc;
use futures::StreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use arrow_array::Array;

/// Repository for entity mention operations.
#[derive(Clone)]
pub struct EntityMentionRepository {
    db: Arc<Database>,
}

impl EntityMentionRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
    
    /// Insert a new entity mention.
    pub async fn insert(&self, mention: &EntityMention) -> Result<()> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITY_MENTIONS)
            .execute()
            .await?;
        
        let record = entity_mention_to_record(mention)?;
        let schema = record.schema();
        let iter = arrow_array::RecordBatchIterator::new(vec![Ok(record)], schema);
        
        table.add(iter).execute().await?;
        Ok(())
    }
    
    /// Insert multiple mentions in bulk.
    pub async fn insert_batch(&self, mentions: &[EntityMention]) -> Result<()> {
        if mentions.is_empty() {
            return Ok(());
        }
        
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITY_MENTIONS)
            .execute()
            .await?;
        
        let records: Vec<arrow_array::RecordBatch> = mentions
            .iter()
            .map(entity_mention_to_record)
            .collect::<Result<_>>()?;
        
        let schema = records[0].schema();
        let iter = arrow_array::RecordBatchIterator::new(
            records.into_iter().map(Ok),
            schema,
        );
        
        table.add(iter).execute().await?;
        Ok(())
    }
    
    /// Find a mention by ID.
    pub async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<EntityMention>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITY_MENTIONS)
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
                return Ok(Some(record_to_entity_mention(&batch, 0)?));
            }
        }
        
        Ok(None)
    }
    
    /// Find all mentions for a chunk.
    pub async fn find_by_chunk_id(&self, chunk_id: uuid::Uuid) -> Result<Vec<EntityMention>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITY_MENTIONS)
            .execute()
            .await?;
        
        let mut stream = table
            .query()
            .only_if(&format!("chunk_id = '{}'", chunk_id))
            .execute()
            .await?;
        
        let mut mentions = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                mentions.push(record_to_entity_mention(&batch, i)?);
            }
        }
        
        Ok(mentions)
    }
    
    /// Find all mentions for an entity.
    pub async fn find_by_entity_id(&self, entity_id: uuid::Uuid) -> Result<Vec<EntityMention>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITY_MENTIONS)
            .execute()
            .await?;
        
        let mut stream = table
            .query()
            .only_if(&format!("entity_id = '{}'", entity_id))
            .execute()
            .await?;
        
        let mut mentions = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                mentions.push(record_to_entity_mention(&batch, i)?);
            }
        }
        
        Ok(mentions)
    }
    
    /// Find all mentions for a paper (via chunks).
    pub async fn find_by_paper_id(&self, paper_id: uuid::Uuid) -> Result<Vec<EntityMention>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITY_MENTIONS)
            .execute()
            .await?;
        
        let mut stream = table
            .query()
            .only_if(&format!("paper_id = '{}'", paper_id))
            .execute()
            .await?;
        
        let mut mentions = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                mentions.push(record_to_entity_mention(&batch, i)?);
            }
        }
        
        Ok(mentions)
    }
    
    /// Delete all mentions for a chunk.
    pub async fn delete_by_chunk_id(&self, chunk_id: uuid::Uuid) -> Result<()> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITY_MENTIONS)
            .execute()
            .await?;
        table.delete(&format!("chunk_id = '{}'", chunk_id)).await?;
        Ok(())
    }
    
    /// Delete all mentions for a paper.
    pub async fn delete_by_paper_id(&self, paper_id: uuid::Uuid) -> Result<()> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITY_MENTIONS)
            .execute()
            .await?;
        table.delete(&format!("paper_id = '{}'", paper_id)).await?;
        Ok(())
    }
    
    /// Delete a mention by ID.
    pub async fn delete(&self, id: uuid::Uuid) -> Result<()> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITY_MENTIONS)
            .execute()
            .await?;
        table.delete(&format!("id = '{}'", id)).await?;
        Ok(())
    }
    
    /// Count total mentions.
    pub async fn count(&self) -> Result<u64> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITY_MENTIONS)
            .execute()
            .await?;
        Ok(table.count_rows(None).await? as u64)
    }
    
    /// Count mentions for an entity.
    pub async fn count_by_entity_id(&self, entity_id: uuid::Uuid) -> Result<u64> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITY_MENTIONS)
            .execute()
            .await?;
        let count = table
            .count_rows(Some(format!("entity_id = '{}'", entity_id)))
            .await?;
        Ok(count as u64)
    }
    
    /// Count mentions for a paper.
    pub async fn count_by_paper_id(&self, paper_id: uuid::Uuid) -> Result<u64> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITY_MENTIONS)
            .execute()
            .await?;
        let count = table
            .count_rows(Some(format!("paper_id = '{}'", paper_id)))
            .await?;
        Ok(count as u64)
    }
    
    /// List mentions with pagination.
    pub async fn list(&self, offset: usize, limit: usize) -> Result<Vec<EntityMention>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITY_MENTIONS)
            .execute()
            .await?;
        
        let mut stream = table
            .query()
            .limit(limit)
            .offset(offset)
            .execute()
            .await?;
        
        let mut mentions = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                mentions.push(record_to_entity_mention(&batch, i)?);
            }
        }
        
        Ok(mentions)
    }
    
    /// Get entity co-occurrence counts (entities mentioned in the same chunk).
    pub async fn get_cooccurrences(
        &self,
        entity_id: uuid::Uuid,
        limit: usize,
    ) -> Result<Vec<(uuid::Uuid, u64)>> {
        // First get all chunks where this entity is mentioned
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITY_MENTIONS)
            .execute()
            .await?;
        
        let mut stream = table
            .query()
            .only_if(&format!("entity_id = '{}'", entity_id))
            .select(lancedb::query::Select::columns(&["chunk_id"]))
            .execute()
            .await?;
        
        let mut chunk_ids = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            let schema = batch.schema();
            if let Ok(idx) = schema.index_of("chunk_id") {
                let arr = batch.column(idx).as_any().downcast_ref::<arrow_array::StringArray>().unwrap();
                for i in 0..arr.len() {
                    if let Ok(id) = uuid::Uuid::parse_str(arr.value(i)) {
                        chunk_ids.push(id);
                    }
                }
            }
        }
        
        // Now find other entities in those chunks
        let mut cooccurrence: std::collections::HashMap<uuid::Uuid, u64> = std::collections::HashMap::new();
        
        for chunk_id in chunk_ids {
            let mut stream = table
                .query()
                .only_if(&format!("chunk_id = '{}'", chunk_id))
                .select(lancedb::query::Select::columns(&["entity_id"]))
                .execute()
                .await?;
            
            while let Some(batch) = stream.next().await {
                let batch = batch?;
                let schema = batch.schema();
                if let Ok(idx) = schema.index_of("entity_id") {
                    let arr = batch.column(idx).as_any().downcast_ref::<arrow_array::StringArray>().unwrap();
                    for i in 0..arr.len() {
                        if let Ok(other_id) = uuid::Uuid::parse_str(arr.value(i)) {
                            if other_id != entity_id {
                                *cooccurrence.entry(other_id).or_insert(0) += 1;
                            }
                        }
                    }
                }
            }
        }
        
        // Sort by count and take top N
        let mut sorted: Vec<(uuid::Uuid, u64)> = cooccurrence.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(limit);
        
        Ok(sorted)
    }
}
