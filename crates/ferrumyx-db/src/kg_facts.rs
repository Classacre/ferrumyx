//! Knowledge graph facts repository.
//!
//! Provides CRUD operations for KG facts (subject-predicate-object triples).

use crate::database::Database;
use crate::error::Result;
use crate::schema::KgFact;
use std::collections::HashMap;
use std::sync::Arc;
use tokio_postgres::{Row, types::ToSql};

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
        let client = self.db.client();
        client.execute(
            "INSERT INTO kg_facts (id, paper_id, subject_id, subject_name, predicate, object_id, object_name, confidence, evidence, evidence_type, study_type, sample_size, valid_from, valid_until, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, NOW())",
            &[
                &fact.id,
                &fact.paper_id,
                &fact.subject_id,
                &fact.subject_name,
                &fact.predicate,
                &fact.object_id,
                &fact.object_name,
                &fact.confidence,
                &fact.evidence,
                &fact.evidence_type,
                &fact.study_type,
                &fact.sample_size,
                &fact.valid_from,
                &fact.valid_until,
            ],
        ).await?;
        Ok(())
    }

    /// Insert multiple facts in bulk.
    pub async fn insert_batch(&self, facts: &[KgFact]) -> Result<()> {
        if facts.is_empty() {
            return Ok(());
        }
        for fact in facts {
            self.insert(fact).await?;
        }
        Ok(())
    }

    /// Find a fact by ID.
    pub async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<KgFact>> {
        let client = self.db.client();
        let row = client.query_opt("SELECT * FROM kg_facts WHERE id = $1", &[&id]).await?;
        Ok(row.map(fact_from_row))
    }

    /// Find all facts for a paper.
    pub async fn find_by_paper_id(&self, paper_id: uuid::Uuid) -> Result<Vec<KgFact>> {
        let client = self.db.client();
        let rows = client.query("SELECT * FROM kg_facts WHERE paper_id = $1", &[&paper_id]).await?;
        Ok(rows.into_iter().map(fact_from_row).collect())
    }

    /// Find all facts where the entity is the subject.
    pub async fn find_by_subject(&self, subject_id: uuid::Uuid) -> Result<Vec<KgFact>> {
        let client = self.db.client();
        let rows = client.query("SELECT * FROM kg_facts WHERE subject_id = $1", &[&subject_id]).await?;
        Ok(rows.into_iter().map(fact_from_row).collect())
    }

    /// Find all facts where subject_id is within a bounded set.
    pub async fn find_by_subject_ids(
        &self,
        subject_ids: &[uuid::Uuid],
        chunk_size: usize,
    ) -> Result<Vec<KgFact>> {
        if subject_ids.is_empty() {
            return Ok(Vec::new());
        }
        let client = self.db.client();
        let mut facts = Vec::new();
        for group in subject_ids.chunks(chunk_size.max(1).min(200)) {
            if group.is_empty() { continue; }
            let placeholders = (1..=group.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(",");
            let sql = format!("SELECT * FROM kg_facts WHERE subject_id IN ({})", placeholders);
            let params: Vec<&(dyn ToSql + Sync)> = group.iter().map(|id| id as &(dyn ToSql + Sync)).collect();
            let rows = client.query(&sql, params.as_slice()).await?;
            for row in rows {
                facts.push(fact_from_row(row));
            }
        }
        Ok(facts)
    }

    /// Find all facts where the entity is the object.
    pub async fn find_by_object(&self, object_id: uuid::Uuid) -> Result<Vec<KgFact>> {
        let client = self.db.client();
        let rows = client.query("SELECT * FROM kg_facts WHERE object_id = $1", &[&object_id]).await?;
        Ok(rows.into_iter().map(fact_from_row).collect())
    }

    /// Find all facts with a specific predicate.
    pub async fn find_by_predicate(&self, predicate: &str) -> Result<Vec<KgFact>> {
        let client = self.db.client();
        let rows = client.query("SELECT * FROM kg_facts WHERE predicate = $1", &[&predicate]).await?;
        Ok(rows.into_iter().map(fact_from_row).collect())
    }

    /// Find all facts involving an entity (as subject or object).
    pub async fn find_by_entity(&self, entity_id: uuid::Uuid) -> Result<Vec<KgFact>> {
        let client = self.db.client();
        let sql = "SELECT * FROM kg_facts WHERE subject_id = $1 OR object_id = $1";
        let rows = client.query(sql, &[&entity_id]).await?;
        Ok(rows.into_iter().map(fact_from_row).collect())
    }

    /// Find facts by subject and predicate.
    pub async fn find_by_subject_and_predicate(
        &self,
        subject_id: uuid::Uuid,
        predicate: &str,
    ) -> Result<Vec<KgFact>> {
        let client = self.db.client();
        let rows = client.query(
            "SELECT * FROM kg_facts WHERE subject_id = $1 AND predicate = $2",
            &[&subject_id, &predicate],
        ).await?;
        Ok(rows.into_iter().map(fact_from_row).collect())
    }

    /// Delete a fact by ID.
    pub async fn delete(&self, id: uuid::Uuid) -> Result<()> {
        let client = self.db.client();
        client.execute("DELETE FROM kg_facts WHERE id = $1", &[&id]).await?;
        Ok(())
    }

    /// Delete all facts for a paper.
    pub async fn delete_by_paper_id(&self, paper_id: uuid::Uuid) -> Result<()> {
        let client = self.db.client();
        client.execute("DELETE FROM kg_facts WHERE paper_id = $1", &[&paper_id]).await?;
        Ok(())
    }

    /// Count total facts.
    pub async fn count(&self) -> Result<u64> {
        let client = self.db.client();
        let row = client.query_one("SELECT COUNT(*) FROM kg_facts", &[]).await?;
        Ok(row.get::<_, i64>(0) as u64)
    }

    /// List facts with pagination.
    pub async fn list(&self, _offset: usize, _limit: usize) -> Result<Vec<KgFact>> {
        let client = self.db.client();
        let rows = client.query("SELECT * FROM kg_facts ORDER BY created_at DESC", &[]).await?;
        Ok(rows.into_iter().map(fact_from_row).collect())
    }

    /// List facts with optional DB-side filtering for performance.
    pub async fn list_filtered(
        &self,
        gene_term: Option<&str>,
        query_term: Option<&str>,
        predicate: Option<&str>,
        limit: usize,
    ) -> Result<Vec<KgFact>> {
        let client = self.db.client();
        let mut conditions = Vec::new();
        if let Some(g) = gene_term.map(str::trim).filter(|s| !s.is_empty()) {
            conditions.push(format!("(subject_name ILIKE '%{}%' OR object_name ILIKE '%{}%' OR predicate ILIKE '%{}%')", g, g, g));
        }
        if let Some(q) = query_term.map(str::trim).filter(|s| !s.is_empty()) {
            conditions.push(format!("(subject_name ILIKE '%{}%' OR object_name ILIKE '%{}%' OR predicate ILIKE '%{}%')", q, q, q));
        }
        if let Some(p) = predicate.map(str::trim).filter(|s| !s.is_empty() && *s != "all") {
            conditions.push(format!("predicate = '{}'", p));
        }
        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            "WHERE ".to_owned() + &conditions.join(" AND ")
        };
        let sql = format!("SELECT * FROM kg_facts {} ORDER BY created_at DESC LIMIT {}", where_clause, limit);
        let rows = client.query(&sql, &[]).await?;
        Ok(rows.into_iter().map(fact_from_row).collect())
    }

    /// Get distinct predicates.
    pub async fn get_predicates(&self) -> Result<Vec<String>> {
        let client = self.db.client();
        let rows = client.query("SELECT DISTINCT predicate FROM kg_facts", &[]).await?;
        let mut predicates = rows.iter().map(|row| row.get::<_, String>("predicate")).collect::<Vec<_>>();
        predicates.sort();
        predicates.dedup();
        Ok(predicates)
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
        let client = self.db.client();
        let mut out = HashMap::new();
        for group in subject_ids.chunks(chunk_size.max(1).min(500)) {
            if group.is_empty() { continue; }
            let placeholders = (1..=group.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(",");
            let sql = format!("SELECT subject_id, COUNT(*) as cnt FROM kg_facts WHERE subject_id IN ({}) GROUP BY subject_id", placeholders);
            let params: Vec<&(dyn ToSql + Sync)> = group.iter().map(|id| id as &(dyn ToSql + Sync)).collect();
            let rows = client.query(&sql, params.as_slice()).await?;
            for row in rows {
                let subject_id: uuid::Uuid = row.get("subject_id");
                let cnt: i64 = row.get("cnt");
                out.insert(subject_id, cnt as u32);
            }
        }
        Ok(out)
    }
}

fn fact_from_row(row: Row) -> KgFact {
    KgFact {
        id: row.get("id"),
        paper_id: row.get("paper_id"),
        subject_id: row.get("subject_id"),
        subject_name: row.get("subject_name"),
        predicate: row.get("predicate"),
        object_id: row.get("object_id"),
        object_name: row.get("object_name"),
        confidence: row.get::<_, f32>("confidence"),
        evidence: row.get("evidence"),
        evidence_type: row.get("evidence_type"),
        study_type: row.get("study_type"),
        sample_size: row.get::<_, Option<i32>>("sample_size"),
        valid_from: row.get::<_, chrono::DateTime<chrono::Utc>>("valid_from"),
        valid_until: row.get::<_, Option<chrono::DateTime<chrono::Utc>>>("valid_until"),
        created_at: row.get::<_, chrono::DateTime<chrono::Utc>>("created_at"),
    }
}
