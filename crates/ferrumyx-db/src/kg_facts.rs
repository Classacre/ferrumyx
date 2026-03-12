//! Knowledge graph facts repository.
//!
//! Provides CRUD operations for KG facts (subject-predicate-object triples).

use crate::database::Database;
use crate::error::Result;
use crate::schema::KgFact;
use crate::schema_arrow::{kg_fact_to_record, record_to_kg_fact};
use arrow_array::Array;
use futures::StreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use std::collections::HashMap;
use std::sync::Arc;

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
        let table = self
            .db
            .connection()
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

        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;

        let records: Vec<arrow_array::RecordBatch> =
            facts.iter().map(kg_fact_to_record).collect::<Result<_>>()?;

        let schema = records[0].schema();
        let iter = arrow_array::RecordBatchIterator::new(records.into_iter().map(Ok), schema);

        table.add(iter).execute().await?;

        Ok(())
    }

    /// Find a fact by ID.
    pub async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<KgFact>> {
        let table = self
            .db
            .connection()
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
        let table = self
            .db
            .connection()
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
        let table = self
            .db
            .connection()
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
        let table = self
            .db
            .connection()
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
        let table = self
            .db
            .connection()
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
        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;

        let mut stream = table
            .query()
            .only_if(&format!(
                "subject_id = '{}' OR object_id = '{}'",
                entity_id, entity_id
            ))
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
        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;

        let escaped = predicate.replace('\'', "''");
        let filter = format!(
            "subject_id = '{}' AND predicate = '{}'",
            subject_id, escaped
        );

        let mut stream = table.query().only_if(&filter).execute().await?;

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
        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;
        table.delete(&format!("id = '{}'", id)).await?;
        Ok(())
    }

    /// Delete all facts for a paper.
    pub async fn delete_by_paper_id(&self, paper_id: uuid::Uuid) -> Result<()> {
        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;
        table.delete(&format!("paper_id = '{}'", paper_id)).await?;
        Ok(())
    }

    /// Count total facts.
    pub async fn count(&self) -> Result<u64> {
        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;
        Ok(table.count_rows(None).await? as u64)
    }

    /// List facts with pagination.
    pub async fn list(&self, offset: usize, limit: usize) -> Result<Vec<KgFact>> {
        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;

        let mut stream = table.query().limit(limit).offset(offset).execute().await?;

        let mut facts = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for i in 0..batch.num_rows() {
                facts.push(record_to_kg_fact(&batch, i)?);
            }
        }

        Ok(facts)
    }

    /// List facts with optional DB-side filtering for performance.
    pub async fn list_filtered(
        &self,
        gene_term: Option<&str>,
        query_term: Option<&str>,
        predicate: Option<&str>,
        limit: usize,
    ) -> Result<Vec<KgFact>> {
        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;

        let mut conditions = Vec::new();
        if let Some(g) = gene_term.map(str::trim).filter(|s| !s.is_empty()) {
            let esc = g.replace('\'', "''");
            conditions.push(format!(
                "(subject_name LIKE '%{esc}%' OR object_name LIKE '%{esc}%' OR predicate LIKE '%{esc}%')"
            ));
        }

        if let Some(q) = query_term.map(str::trim).filter(|s| !s.is_empty()) {
            let esc = q.replace('\'', "''");
            conditions.push(format!(
                "(subject_name LIKE '%{esc}%' OR object_name LIKE '%{esc}%' OR predicate LIKE '%{esc}%')"
            ));
        }

        if let Some(p) = predicate
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != "all")
        {
            let esc = p.replace('\'', "''");
            conditions.push(format!("predicate = '{esc}'"));
        }

        let mut query = table.query().limit(limit);
        if !conditions.is_empty() {
            query = query.only_if(&conditions.join(" AND "));
        }

        let mut stream = query.execute().await?;
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
        let table = self
            .db
            .connection()
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
                let arr = batch
                    .column(idx)
                    .as_any()
                    .downcast_ref::<arrow_array::StringArray>()
                    .unwrap();
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        predicates.insert(arr.value(i).to_string());
                    }
                }
            }
        }

        Ok(predicates.into_iter().collect())
    }

    /// Count facts per subject_id for a bounded set of subjects.
    pub async fn count_by_subject_ids(
        &self,
        subject_ids: &[uuid::Uuid],
        chunk_size: usize,
    ) -> Result<HashMap<uuid::Uuid, u32>> {
        if subject_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_KG_FACTS)
            .execute()
            .await?;

        let mut uniq = subject_ids.to_vec();
        uniq.sort_unstable();
        uniq.dedup();
        let chunk = chunk_size.max(1).min(500);

        let mut out: HashMap<uuid::Uuid, u32> = HashMap::new();
        for group in uniq.chunks(chunk) {
            let filter = group
                .iter()
                .map(|id| format!("subject_id = '{}'", id))
                .collect::<Vec<_>>()
                .join(" OR ");
            if filter.is_empty() {
                continue;
            }
            let mut stream = table
                .query()
                .only_if(&format!("({filter})"))
                .select(lancedb::query::Select::columns(&["subject_id"]))
                .execute()
                .await?;

            while let Some(batch) = stream.next().await {
                let batch = batch?;
                let schema = batch.schema();
                let subj_idx = match schema.index_of("subject_id") {
                    Ok(i) => i,
                    Err(_) => continue,
                };
                let subj_arr = match batch
                    .column(subj_idx)
                    .as_any()
                    .downcast_ref::<arrow_array::StringArray>()
                {
                    Some(a) => a,
                    None => continue,
                };
                for row in 0..batch.num_rows() {
                    if subj_arr.is_null(row) {
                        continue;
                    }
                    if let Ok(id) = uuid::Uuid::parse_str(subj_arr.value(row)) {
                        *out.entry(id).or_insert(0) += 1;
                    }
                }
            }
        }
        Ok(out)
    }
}
