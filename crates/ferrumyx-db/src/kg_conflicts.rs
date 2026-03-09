//! Knowledge graph conflicts repository.
//!
//! Provides operations for KG conflicts.

use crate::database::Database;
use crate::error::Result;
use crate::schema::KgConflict;
use crate::schema_arrow::record_to_kg_conflict;
use std::collections::HashSet;
use std::sync::Arc;
use futures::StreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};

/// Repository for knowledge graph conflict operations.
#[derive(Clone)]
pub struct KgConflictRepository {
    db: Arc<Database>,
}

impl KgConflictRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
    
    /// Find conflict by fact_a_id or fact_b_id.
    pub async fn find_by_fact_id(&self, fact_id: uuid::Uuid) -> Result<Vec<KgConflict>> {
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_KG_CONFLICTS)
            .execute()
            .await?;
        
        let mut stream = table
            .query()
            .only_if(&format!("fact_a_id = '{}' OR fact_b_id = '{}'", fact_id, fact_id))
            .execute()
            .await?;
        
        let mut conflicts = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                conflicts.push(record_to_kg_conflict(&batch, i)?);
            }
        }
        
        Ok(conflicts)
    }
    
    /// Get all conflicts involving any of the provided fact IDs.
    pub async fn find_by_fact_ids(&self, fact_ids: &[uuid::Uuid]) -> Result<Vec<KgConflict>> {
        if fact_ids.is_empty() {
            return Ok(vec![]);
        }
        
        let table = self.db.connection()
            .open_table(crate::schema::TABLE_KG_CONFLICTS)
            .execute()
            .await?;
            
        let mut conflicts = Vec::new();
        let mut seen = HashSet::new();

        // Bound filter complexity to avoid deep parser recursion/stack overflow
        // in large OR expressions.
        let chunk_size = 16usize;
        for chunk in fact_ids.chunks(chunk_size) {
            let filters: Vec<String> = chunk
                .iter()
                .map(|id| format!("(fact_a_id = '{}' OR fact_b_id = '{}')", id, id))
                .collect();
            let filter = filters.join(" OR ");

            let mut stream = table
                .query()
                .only_if(&filter)
                .execute()
                .await?;

            while let Some(batch) = stream.next().await {
                let batch = batch?;
                for i in 0..batch.num_rows() {
                    let conflict = record_to_kg_conflict(&batch, i)?;
                    if seen.insert(conflict.id) {
                        conflicts.push(conflict);
                    }
                }
            }
        }
        
        Ok(conflicts)
    }
}
