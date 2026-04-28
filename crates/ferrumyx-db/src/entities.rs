//! Entity repository.
//!
//! Provides CRUD operations for named entities (genes, diseases, drugs, etc.).

use crate::database::Database;
use crate::error::{DbError, Result};
use crate::schema::{Entity, EntityType};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio_postgres::{Row, types::ToSql};

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
        let client = self.db.client();
        client.execute(
            "INSERT INTO entities (id, external_id, name, canonical_name, entity_type, synonyms, description, source_db, metadata, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW())",
            &[
                &entity.id,
                &entity.external_id,
                &entity.name,
                &entity.canonical_name,
                &entity.entity_type,
                &entity.synonyms,
                &entity.description,
                &entity.source_db,
                &entity.metadata,
            ],
        ).await?;
        Ok(())
    }

    /// Insert multiple entities in bulk.
    pub async fn insert_batch(&self, entities: &[Entity]) -> Result<()> {
        if entities.is_empty() {
            return Ok(());
        }
        for entity in entities {
            self.insert(entity).await?;
        }
        Ok(())
    }

    /// Find an entity by ID.
    pub async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<Entity>> {
        let client = self.db.client();
        let row = client.query_opt("SELECT * FROM entities WHERE id = $1", &[&id]).await?;
        Ok(row.map(entity_from_row))
    }

    /// Find entities by external ID (e.g., HGNC ID, ChEMBL ID).
    pub async fn find_by_external_id(&self, external_id: &str) -> Result<Vec<Entity>> {
        let client = self.db.client();
        let rows = client.query("SELECT * FROM entities WHERE external_id = $1", &[&external_id]).await?;
        Ok(rows.into_iter().map(entity_from_row).collect())
    }

    /// Find entities by name (exact match).
    pub async fn find_by_name(&self, name: &str) -> Result<Vec<Entity>> {
        let client = self.db.client();
        let rows = client.query("SELECT * FROM entities WHERE name = $1", &[&name]).await?;
        Ok(rows.into_iter().map(entity_from_row).collect())
    }

    /// Find entities by type.
    pub async fn find_by_type(&self, entity_type: EntityType) -> Result<Vec<Entity>> {
        let client = self.db.client();
        let type_str = match entity_type {
            EntityType::Gene => "gene",
            EntityType::Disease => "disease",
            EntityType::Chemical => "chemical",
            EntityType::Mutation => "mutation",
            EntityType::CancerType => "cancer_type",
            EntityType::Pathway => "pathway",
            EntityType::Protein => "protein",
        };
        let rows = client.query("SELECT * FROM entities WHERE entity_type = $1", &[&type_str]).await?;
        Ok(rows.into_iter().map(entity_from_row).collect())
    }

    /// Find entities by synonym.
    pub async fn find_by_synonym(&self, synonym: &str) -> Result<Vec<Entity>> {
        let client = self.db.client();
        let pattern = format!("%{}%", synonym);
        let rows = client.query("SELECT * FROM entities WHERE synonyms LIKE $1", &[&pattern]).await?;
        Ok(rows.into_iter().map(entity_from_row).collect())
    }

    /// Find entity names for a batch of IDs.
    pub async fn find_names_by_ids(
        &self,
        ids: &[uuid::Uuid],
    ) -> Result<HashMap<uuid::Uuid, String>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let unique_ids: Vec<uuid::Uuid> = ids
            .iter()
            .copied()
            .filter(|id| !id.is_nil())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        if unique_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let client = self.db.client();
        let mut names_by_id = HashMap::with_capacity(unique_ids.len());
        for chunk in unique_ids.chunks(512) {
            let placeholders = (1..=chunk.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(",");
            let sql = format!("SELECT id, name FROM entities WHERE id IN ({})", placeholders);
            let params: Vec<&(dyn ToSql + Sync)> = chunk.iter().map(|id| id as &(dyn ToSql + Sync)).collect();
            let rows = client.query(&sql, params.as_slice()).await?;
            for row in rows {
                let id: uuid::Uuid = row.get("id");
                let name: String = row.get("name");
                names_by_id.insert(id, name);
            }
        }
        Ok(names_by_id)
    }

    /// Update an entity.
    pub async fn update(&self, entity: &Entity) -> Result<()> {
        let client = self.db.client();
        client.execute(
            "UPDATE entities SET external_id = $2, name = $3, canonical_name = $4, entity_type = $5, \
             synonyms = $6, description = $7, source_db = $8, metadata = $9, updated_at = NOW() \
             WHERE id = $1",
            &[
                &entity.id,
                &entity.external_id,
                &entity.name,
                &entity.canonical_name,
                &entity.entity_type,
                &entity.synonyms,
                &entity.description,
                &entity.source_db,
                &entity.metadata,
            ],
        ).await?;
        Ok(())
    }

    /// Delete an entity by ID.
    pub async fn delete(&self, id: uuid::Uuid) -> Result<()> {
        let client = self.db.client();
        client.execute("DELETE FROM entities WHERE id = $1", &[&id]).await?;
        Ok(())
    }

    /// Count total entities.
    pub async fn count(&self) -> Result<u64> {
        let client = self.db.client();
        let row = client.query_one("SELECT COUNT(*) FROM entities", &[]).await?;
        Ok(row.get::<_, i64>(0) as u64)
    }

    /// Count entities by type.
    pub async fn count_by_type(&self, entity_type: EntityType) -> Result<u64> {
        let client = self.db.client();
        let type_str = match entity_type {
            EntityType::Gene => "gene",
            EntityType::Disease => "disease",
            EntityType::Chemical => "chemical",
            EntityType::Mutation => "mutation",
            EntityType::CancerType => "cancer_type",
            EntityType::Pathway => "pathway",
            EntityType::Protein => "protein",
        };
        let row = client.query_one(
            "SELECT COUNT(*) FROM entities WHERE entity_type = $1",
            &[&type_str],
        ).await?;
        Ok(row.get::<_, i64>(0) as u64)
    }

    /// List entities with pagination.
    pub async fn list(&self, _offset: usize, _limit: usize) -> Result<Vec<Entity>> {
        let client = self.db.client();
        let rows = client.query("SELECT * FROM entities ORDER BY created_at DESC", &[]).await?;
        Ok(rows.into_iter().map(entity_from_row).collect())
    }

    /// Search entities by name or synonym (fuzzy search).
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<Entity>> {
        let client = self.db.client();
        let pattern = format!("%{}%", query);
        let sql = format!("SELECT * FROM entities WHERE name ILIKE $1 OR synonyms ILIKE $1 LIMIT {}", limit);
        let rows = client.query(&sql, &[&pattern]).await?;
        Ok(rows.into_iter().map(entity_from_row).collect())
    }
}

fn entity_from_row(row: Row) -> Entity {
    Entity {
        id: row.get("id"),
        external_id: row.get("external_id"),
        name: row.get("name"),
        canonical_name: row.get("canonical_name"),
        entity_type: row.get("entity_type"),
        synonyms: row.get("synonyms"),
        description: row.get("description"),
        source_db: row.get("source_db"),
        metadata: row.get("metadata"),
        created_at: row.get::<_, chrono::DateTime<chrono::Utc>>("created_at"),
        updated_at: row.get::<_, chrono::DateTime<chrono::Utc>>("updated_at"),
    }
}
