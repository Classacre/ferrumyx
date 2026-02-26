//! Knowledge graph facts repository.
//!
//! Provides CRUD operations for KG facts (subject-predicate-object triples).

use crate::database::Database;
use crate::error::Result;
use crate::schema::KgFact;
use crate::schema_arrow::{kg_fact_to_record, record_to_kg_fact};
use std::sync::Arc;
use futures::StreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use arrow_array::Array;

/// Repository for knowledge graph fact operations.
#[derive(Clone)]
pub struct KgFactRepository {
    db: Arc<Database>,
}

impl KgFactRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
    
    /// Insert a new fact.
    pub async fn insert(&self, fact: &KgFact) -> Result<()> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;
        
        let record = kg_fact_to_record(fact)?;
        let schema = record.schema();
        let iter = arrow_array::RecordBatchIterator::new(vec![Ok(record)], schema);
        
        table.add(iter).execute().await?;

        Ok(())
    }
    
    /// Insert multiple facts in bulk.
    pub async fn insert_batch(&self, facts: &[KgFact]) -> Result<()> {
        if facts.is_empty() {
            return Ok(());
        }
        
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;
        
        let records: Vec<arrow_array::RecordBatch> = facts
            .iter()
            .map(kg_fact_to_record)
            .collect::<Result<_>>()?;
        
        let schema = records[0].schema();
        let iter = arrow_array::RecordBatchIterator::new(
            records.into_iter().map(Ok),
            schema,
        );
        
        table.add(iter).execute().await?;

        Ok(())
    }
    
    /// Find a fact by ID.
    pub async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<KgFact>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
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
                return Ok(Some(record_to_kg_fact(&batch, 0)?));
            }
        }
        
        Ok(None)
    }
    
    /// Find all facts for a paper.
    pub async fn find_by_paper_id(&self, paper_id: uuid::Uuid) -> Result<Vec<KgFact>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;
        
        let mut stream = table
            .query()
            .only_if(&format!("paper_id = '{}'", paper_id))
            .execute()
            .await?;
        
        let mut facts = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                facts.push(record_to_kg_fact(&batch, i)?);
            }
        }
        
        Ok(facts)
    }
    
    /// Find all facts where the entity is the subject.
    pub async fn find_by_subject(&self, subject_id: uuid::Uuid) -> Result<Vec<KgFact>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;
        
        let mut stream = table
            .query()
            .only_if(&format!("subject_id = '{}'", subject_id))
            .execute()
            .await?;
        
        let mut facts = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                facts.push(record_to_kg_fact(&batch, i)?);
            }
        }
        
        Ok(facts)
    }
    
    /// Find all facts where the entity is the object.
    pub async fn find_by_object(&self, object_id: uuid::Uuid) -> Result<Vec<KgFact>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;
        
        let mut stream = table
            .query()
            .only_if(&format!("object_id = '{}'", object_id))
            .execute()
            .await?;
        
        let mut facts = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                facts.push(record_to_kg_fact(&batch, i)?);
            }
        }
        
        Ok(facts)
    }
    
    /// Find all facts with a specific predicate.
    pub async fn find_by_predicate(&self, predicate: &str) -> Result<Vec<KgFact>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;
        
        let escaped = predicate.replace('\'', "''");
        
        let mut stream = table
            .query()
            .only_if(&format!("predicate = '{}'", escaped))
            .execute()
            .await?;
        
        let mut facts = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                facts.push(record_to_kg_fact(&batch, i)?);
            }
        }
        
        Ok(facts)
    }
    
    /// Find all facts involving an entity (as subject or object).
    pub async fn find_by_entity(&self, entity_id: uuid::Uuid) -> Result<Vec<KgFact>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;
        
        let mut stream = table
            .query()
            .only_if(&format!("subject_id = '{}' OR object_id = '{}'", entity_id, entity_id))
            .execute()
            .await?;
        
        let mut facts = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                facts.push(record_to_kg_fact(&batch, i)?);
            }
        }
        
        Ok(facts)
    }
    
    /// Find facts by subject and predicate.
    pub async fn find_by_subject_and_predicate(
        &self,
        subject_id: uuid::Uuid,
        predicate: &str,
    ) -> Result<Vec<KgFact>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;
        
        let escaped = predicate.replace('\'', "''");
        let filter = format!(
            "subject_id = '{}' AND predicate = '{}'",
            subject_id, escaped
        );
        
        let mut stream = table
            .query()
            .only_if(&filter)
            .execute()
            .await?;
        
        let mut facts = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                facts.push(record_to_kg_fact(&batch, i)?);
            }
        }
        
        Ok(facts)
    }
    
    /// Delete a fact by ID.
    pub async fn delete(&self, id: uuid::Uuid) -> Result<()> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;
        table.delete(&format!("id = '{}'", id)).await?;
        Ok(())
    }
    
    /// Delete all facts for a paper.
    pub async fn delete_by_paper_id(&self, paper_id: uuid::Uuid) -> Result<()> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;
        table.delete(&format!("paper_id = '{}'", paper_id)).await?;
        Ok(())
    }
    
    /// Count total facts.
    pub async fn count(&self) -> Result<u64> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;
        Ok(table.count_rows(None).await? as u64)
    }
    
    /// List facts with pagination.
    pub async fn list(&self, offset: usize, limit: usize) -> Result<Vec<KgFact>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;
        
        let mut stream = table
            .query()
            .limit(limit)
            .offset(offset)
            .execute()
            .await?;
        
        let mut facts = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                facts.push(record_to_kg_fact(&batch, i)?);
            }
        }
        
        Ok(facts)
    }
    
    /// Get distinct predicates.
    pub async fn get_predicates(&self) -> Result<Vec<String>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;
        
        let mut stream = table
            .query()
            .select(lancedb::query::Select::columns(&["predicate"]))
            .execute()
            .await?;
        
        let mut predicates = std::collections::HashSet::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            let schema = batch.schema();
            if let Ok(idx) = schema.index_of("predicate") {
                let arr = batch.column(idx).as_any().downcast_ref::<arrow_array::StringArray>().unwrap();
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        predicates.insert(arr.value(i).to_string());
                    }
                }
            }
        }
        
        Ok(predicates.into_iter().collect())
    }
}
