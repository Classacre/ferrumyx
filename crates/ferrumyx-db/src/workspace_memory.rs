//! Workspace memory repository.
//!
//! Provides CRUD operations for workspace memory.

use crate::database::Database;
use crate::error::{DbError, Result};
use crate::schema::WorkspaceMemory;
use std::sync::Arc;
use tokio_postgres::Row;

/// Repository for workspace memory operations.
#[derive(Clone)]
pub struct WorkspaceMemoryRepository {
    db: Arc<Database>,
}

impl WorkspaceMemoryRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Insert a new workspace memory record.
    pub async fn insert(&self, memory: &WorkspaceMemory) -> Result<()> {
        let client = self.db.client();
        client.execute(
            "INSERT INTO workspace_memory (id, scope, content, created_at) \
             VALUES ($1, $2, $3, $4)",
            &[
                &memory.id,
                &memory.scope,
                &memory.content,
                &memory.created_at,
            ],
        ).await?;
        Ok(())
    }

    /// Get all memory records by scope.
    pub async fn find_by_scope(&self, scope: &str) -> Result<Vec<WorkspaceMemory>> {
        let client = self.db.client();
        let rows = client.query(
            "SELECT id, scope, content, created_at FROM workspace_memory WHERE scope = $1 ORDER BY created_at DESC",
            &[&scope],
        ).await?;
        Ok(rows.iter().map(|row| Self::row_to_memory(row)).collect())
    }

    /// Get the latest memory record by scope.
    pub async fn find_latest_by_scope(&self, scope: &str) -> Result<Option<WorkspaceMemory>> {
        let client = self.db.client();
        let row = client.query_opt(
            "SELECT id, scope, content, created_at FROM workspace_memory WHERE scope = $1 ORDER BY created_at DESC LIMIT 1",
            &[&scope],
        ).await?;
        Ok(row.map(|r| Self::row_to_memory(&r)))
    }

    /// Delete memory records by scope.
    pub async fn delete_by_scope(&self, scope: &str) -> Result<u64> {
        let client = self.db.client();
        Ok(client.execute(
            "DELETE FROM workspace_memory WHERE scope = $1",
            &[&scope],
        ).await?)
    }

    fn row_to_memory(row: &Row) -> WorkspaceMemory {
        WorkspaceMemory {
            id: row.get(0),
            scope: row.get(1),
            content: row.get(2),
            created_at: row.get(3),
        }
    }
}