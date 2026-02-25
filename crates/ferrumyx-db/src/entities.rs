//! Entity repository.
//!
//! Provides CRUD operations for named entities (genes, diseases, drugs, etc.).

use crate::database::Database;
use crate::error::Result;
use crate::schema::{Entity, EntityType};
use crate::schema_arrow::{entity_to_record, record_to_entity};
use std::sync::Arc;
use futures::StreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};

/// Repository for entity operations.
#[derive(Clone)]
pub struct EntityRepository {
    db: Arc<Database>,
}

impl EntityRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
    
    /// Insert a new entity.
    pub async fn insert(&self, entity: &Entity) -> Result<()> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITIES)
            .execute()
            .await?;
        
        let record = entity_to_record(entity)?;
        let schema = record.schema();
        let iter = arrow_array::RecordBatchIterator::new(vec![Ok(record)], schema);
        
        table.add(iter).execute().await?;
        Ok(())
    }
    
    /// Insert multiple entities in bulk.
    pub async fn insert_batch(&self, entities: &[Entity]) -> Result<()> {
        if entities.is_empty() {
            return Ok(());
        }
        
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITIES)
            .execute()
            .await?;
        
        let records: Vec<arrow_array::RecordBatch> = entities
            .iter()
            .map(entity_to_record)
            .collect::<Result<_>>()?;
        
        let schema = records[0].schema();
        let iter = arrow_array::RecordBatchIterator::new(
            records.into_iter().map(Ok),
            schema,
        );
        
        table.add(iter).execute().await?;
        Ok(())
    }
    
    /// Find an entity by ID.
    pub async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<Entity>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITIES)
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
                return Ok(Some(record_to_entity(&batch, 0)?));
            }
        }
        
        Ok(None)
    }
    
    /// Find entities by external ID (e.g., HGNC ID, ChEMBL ID).
    pub async fn find_by_external_id(&self, external_id: &str) -> Result<Vec<Entity>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITIES)
            .execute()
            .await?;
        
        let escaped = external_id.replace('\'', "''");
        
        let mut stream = table
            .query()
            .only_if(&format!("external_id = '{}'", escaped))
            .execute()
            .await?;
        
        let mut entities = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                entities.push(record_to_entity(&batch, i)?);
            }
        }
        
        Ok(entities)
    }
    
    /// Find entities by name (exact match).
    pub async fn find_by_name(&self, name: &str) -> Result<Vec<Entity>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITIES)
            .execute()
            .await?;
        
        let escaped = name.replace('\'', "''");
        
        let mut stream = table
            .query()
            .only_if(&format!("name = '{}'", escaped))
            .execute()
            .await?;
        
        let mut entities = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                entities.push(record_to_entity(&batch, i)?);
            }
        }
        
        Ok(entities)
    }
    
    /// Find entities by type.
    pub async fn find_by_type(&self, entity_type: EntityType) -> Result<Vec<Entity>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITIES)
            .execute()
            .await?;
        
        let type_str = match entity_type {
            EntityType::Gene => "gene",
            EntityType::Disease => "disease",
            EntityType::Chemical => "chemical",
            EntityType::Mutation => "mutation",
            EntityType::CancerType => "cancer_type",
            EntityType::Pathway => "pathway",
            EntityType::Protein => "protein",
        };
        
        let mut stream = table
            .query()
            .only_if(&format!("entity_type = '{}'", type_str))
            .execute()
            .await?;
        
        let mut entities = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                entities.push(record_to_entity(&batch, i)?);
            }
        }
        
        Ok(entities)
    }
    
    /// Find entities by synonym.
    pub async fn find_by_synonym(&self, synonym: &str) -> Result<Vec<Entity>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITIES)
            .execute()
            .await?;
        
        let escaped = synonym.replace('\'', "''");
        
        // Search in the synonyms column (array_contains for list columns)
        let mut stream = table
            .query()
            .only_if(&format!("array_contains(synonyms, '{}')", escaped))
            .execute()
            .await?;
        
        let mut entities = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                entities.push(record_to_entity(&batch, i)?);
            }
        }
        
        Ok(entities)
    }
    
    /// Update an entity.
    pub async fn update(&self, entity: &Entity) -> Result<()> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITIES)
            .execute()
            .await?;
        
        let record = entity_to_record(entity)?;
        let schema = record.schema();
        let iter = arrow_array::RecordBatchIterator::new(vec![Ok(record)], schema);
        
        let mut builder = table.merge_insert(&["id"]);
        builder.when_matched_update_all(None);
        builder.execute(Box::new(iter)).await?;
        
        Ok(())
    }
    
    /// Delete an entity by ID.
    pub async fn delete(&self, id: uuid::Uuid) -> Result<()> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITIES)
            .execute()
            .await?;
        table.delete(&format!("id = '{}'", id)).await?;
        Ok(())
    }
    
    /// Count total entities.
    pub async fn count(&self) -> Result<u64> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITIES)
            .execute()
            .await?;
        Ok(table.count_rows(None).await? as u64)
    }
    
    /// Count entities by type.
    pub async fn count_by_type(&self, entity_type: EntityType) -> Result<u64> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITIES)
            .execute()
            .await?;
        
        let type_str = match entity_type {
            EntityType::Gene => "gene",
            EntityType::Disease => "disease",
            EntityType::Chemical => "chemical",
            EntityType::Mutation => "mutation",
            EntityType::CancerType => "cancer_type",
            EntityType::Pathway => "pathway",
            EntityType::Protein => "protein",
        };
        
        let count = table
            .count_rows(Some(format!("entity_type = '{}'", type_str)))
            .await?;
        Ok(count as u64)
    }
    
    /// List entities with pagination.
    pub async fn list(&self, offset: usize, limit: usize) -> Result<Vec<Entity>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITIES)
            .execute()
            .await?;
        
        let mut stream = table
            .query()
            .limit(limit)
            .offset(offset)
            .execute()
            .await?;
        
        let mut entities = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                entities.push(record_to_entity(&batch, i)?);
            }
        }
        
        Ok(entities)
    }
    
    /// Search entities by name or synonym (fuzzy search).
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<Entity>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_ENTITIES)
            .execute()
            .await?;
        
        let escaped = query.replace('\'', "''");
        
        // Use LIKE for simple text search
        let filter = format!(
            "name LIKE '%{}%' OR array_contains(synonyms, '{}')",
            escaped, escaped
        );
        
        let mut stream = table
            .query()
            .only_if(&filter)
            .limit(limit)
            .execute()
            .await?;
        
        let mut entities = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                entities.push(record_to_entity(&batch, i)?);
            }
        }
        
        Ok(entities)
    }
}
