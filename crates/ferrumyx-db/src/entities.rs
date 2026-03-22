//! Entity repository.
//!
//! Provides CRUD operations for named entities (genes, diseases, drugs, etc.).

use crate::database::Database;
use crate::error::DbError;
use crate::error::Result;
use crate::schema::{Entity, EntityType};
use crate::schema_arrow::{entity_to_record, record_to_entity};
use arrow_array::StringArray;
use futures::StreamExt;
use lancedb::query::{ExecutableQuery, QueryBase, Select};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

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
        let table = self
            .db
            .connection()
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

        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_ENTITIES)
            .execute()
            .await?;

        let records: Vec<arrow_array::RecordBatch> = entities
            .iter()
            .map(entity_to_record)
            .collect::<Result<_>>()?;

        let schema = records[0].schema();
        let iter = arrow_array::RecordBatchIterator::new(records.into_iter().map(Ok), schema);

        table.add(iter).execute().await?;
        Ok(())
    }

    /// Find an entity by ID.
    pub async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<Entity>> {
        let table = self
            .db
            .connection()
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
        let table = self
            .db
            .connection()
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
        let table = self
            .db
            .connection()
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
        let table = self
            .db
            .connection()
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
        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_ENTITIES)
            .execute()
            .await?;

        let mut stream = table
            .query()
            .only_if(&synonym_exact_filter(synonym))
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

    /// Find entity names for a batch of IDs.
    pub async fn find_names_by_ids(
        &self,
        ids: &[uuid::Uuid],
    ) -> Result<HashMap<uuid::Uuid, String>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }

        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_ENTITIES)
            .execute()
            .await?;

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

        let mut names_by_id = HashMap::with_capacity(unique_ids.len());
        for chunk in unique_ids.chunks(512) {
            let filter = format!(
                "id IN ({})",
                chunk
                    .iter()
                    .map(|id| format!("'{}'", id))
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            let mut stream = table
                .query()
                .select(Select::columns(&["id", "name"]))
                .only_if(&filter)
                .execute()
                .await?;

            while let Some(batch) = stream.next().await {
                let batch = batch?;
                let ids = batch
                    .column(0)
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .ok_or_else(|| DbError::Arrow("entities.id column was not Utf8".to_string()))?;
                let names = batch
                    .column(1)
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .ok_or_else(|| {
                        DbError::Arrow("entities.name column was not Utf8".to_string())
                    })?;

                for row in 0..batch.num_rows() {
                    if let Ok(id) = uuid::Uuid::parse_str(ids.value(row)) {
                        names_by_id.insert(id, names.value(row).to_string());
                    }
                }
            }
        }

        Ok(names_by_id)
    }

    /// Update an entity.
    pub async fn update(&self, entity: &Entity) -> Result<()> {
        let table = self
            .db
            .connection()
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
        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_ENTITIES)
            .execute()
            .await?;
        table.delete(&format!("id = '{}'", id)).await?;
        Ok(())
    }

    /// Count total entities.
    pub async fn count(&self) -> Result<u64> {
        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_ENTITIES)
            .execute()
            .await?;
        Ok(table.count_rows(None).await? as u64)
    }

    /// Count entities by type.
    pub async fn count_by_type(&self, entity_type: EntityType) -> Result<u64> {
        let table = self
            .db
            .connection()
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
        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_ENTITIES)
            .execute()
            .await?;

        let mut stream = table.query().limit(limit).offset(offset).execute().await?;

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
        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_ENTITIES)
            .execute()
            .await?;

        let mut stream = table
            .query()
            .only_if(&synonym_search_filter(query))
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

fn escape_sql_literal(value: &str) -> String {
    value.replace('\'', "''")
}

fn synonym_exact_filter(value: &str) -> String {
    let escaped = escape_sql_literal(value);
    let quoted_escaped = escape_sql_literal(&format!("\"{}\"", value));

    format!("synonyms = '{escaped}' OR synonyms LIKE '%{quoted_escaped}%'")
}

fn synonym_search_filter(value: &str) -> String {
    let escaped = escape_sql_literal(value);
    let quoted_escaped = escape_sql_literal(&format!("\"{}\"", value));

    format!(
        "name LIKE '%{escaped}%' OR synonyms LIKE '%{escaped}%' OR synonyms LIKE '%{quoted_escaped}%'"
    )
}
