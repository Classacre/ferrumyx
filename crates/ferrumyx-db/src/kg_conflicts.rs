//! Knowledge graph conflicts repository.
//!
//! Provides operations for KG conflicts.

use crate::database::Database;
use crate::error::Result;
use crate::schema::KgConflict;
use std::collections::HashSet;
use std::sync::Arc;
use tokio_postgres::{Row, types::ToSql};

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
        let client = self.db.client();
        let sql = "SELECT * FROM kg_conflicts WHERE fact_a_id = $1 OR fact_b_id = $1";
        let rows = client.query(sql, &[&fact_id]).await?;
        Ok(rows.into_iter().map(conflict_from_row).collect())
    }

    /// Get all conflicts involving any of the provided fact IDs.
    pub async fn find_by_fact_ids(&self, fact_ids: &[uuid::Uuid]) -> Result<Vec<KgConflict>> {
        if fact_ids.is_empty() {
            return Ok(vec![]);
        }
        let client = self.db.client();
        let mut conflicts = Vec::new();
        let mut seen = HashSet::new();
        let chunk_size = 16usize;
        for chunk in fact_ids.chunks(chunk_size) {
            if chunk.is_empty() { continue; }
            let mut params: Vec<&(dyn ToSql + Sync)> = Vec::with_capacity(chunk.len() * 2);
            for id in chunk {
                params.push(id);
                params.push(id);
            }
            let placeholders = (1..=params.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(",");
            let sql = format!("SELECT * FROM kg_conflicts WHERE fact_a_id IN ({}) OR fact_b_id IN ({})", placeholders, placeholders);
            let rows = client.query(&sql, params.as_slice()).await?;
            for row in rows {
                let conflict = conflict_from_row(row);
                if seen.insert(conflict.id) {
                    conflicts.push(conflict);
                }
            }
        }
        Ok(conflicts)
    }
}

fn conflict_from_row(row: Row) -> KgConflict {
    KgConflict {
        id: row.get("id"),
        fact_a_id: row.get("fact_a_id"),
        fact_b_id: row.get("fact_b_id"),
        conflict_type: row.get("conflict_type"),
        net_confidence: row.get::<_, f32>("net_confidence"),
        resolution: row.get("resolution"),
        detected_at: row.get::<_, chrono::DateTime<chrono::Utc>>("detected_at"),
    }
}
