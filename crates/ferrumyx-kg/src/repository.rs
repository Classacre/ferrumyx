//! KG repository - database access layer.
//! Uses LanceDB for KG fact storage and retrieval.
//! See ARCHITECTURE.md §3.7 for query patterns.

use async_trait::async_trait;
use uuid::Uuid;
use std::sync::Arc;
use anyhow::Result;
use ferrumyx_db::Database;
use ferrumyx_db::kg_facts::KgFactRepository;
use ferrumyx_db::schema::{KgFact, KgConflict};
use lancedb::query::{ExecutableQuery, QueryBase};
use futures::StreamExt;

/// Knowledge Graph repository trait.
#[async_trait]
pub trait KgRepositoryTrait: Send + Sync {
    /// Insert a new KG fact (append-only).
    async fn insert_fact(&self, fact: &KgFact) -> Result<Uuid>;

    /// Get all current facts for a (subject, predicate) pair.
    async fn get_facts(
        &self,
        subject_id: Uuid,
        predicate: &str,
    ) -> Result<Vec<KgFact>>;

    /// Supersede an existing fact (set valid_until = now).
    async fn supersede_fact(&self, fact_id: Uuid) -> Result<()>;

    /// Get synthetic lethality partners for a gene in a cancer context.
    async fn get_synthetic_lethality_partners(
        &self,
        gene_id: Uuid,
        cancer_id: Uuid,
        min_confidence: f64,
    ) -> Result<Vec<SyntheticLethalityResult>>;
}

/// LanceDB-backed KG repository.
#[derive(Clone)]
pub struct KgRepository {
    db: Arc<Database>,
    event_queue: Option<tokio::sync::mpsc::UnboundedSender<crate::update::KgUpdateTrigger>>,
}

impl KgRepository {
    pub fn new(db: Arc<Database>) -> Self { Self { db, event_queue: None } }
    
    pub fn with_event_queue(mut self, tx: tokio::sync::mpsc::UnboundedSender<crate::update::KgUpdateTrigger>) -> Self {
        self.event_queue = Some(tx);
        self
    }

    /// Get underlying database reference.
    pub fn db(&self) -> Arc<Database> { self.db.clone() }
    
    fn fact_repo(&self) -> KgFactRepository {
        KgFactRepository::new(self.db.clone())
    }

    /// Insert a new KG fact.
    pub async fn insert_fact(&self, fact: &KgFact) -> Result<()> {
        let fact_repo = self.fact_repo();
        fact_repo.insert(fact).await?;
        self.handle_post_insert(fact).await?;
        Ok(())
    }

    /// Bulk insert facts.
    pub async fn insert_facts(&self, facts: &[KgFact]) -> Result<()> {
        let fact_repo = self.fact_repo();
        fact_repo.insert_batch(facts).await?;
        for fact in facts {
            self.handle_post_insert(fact).await?;
        }
        Ok(())
    }

    async fn handle_post_insert(&self, fact: &KgFact) -> Result<()> {
        if let Some(tx) = &self.event_queue {
            let trigger = crate::update::KgUpdateTrigger::NewFact {
                subject_id: fact.subject_id,
                predicate: fact.predicate.clone(),
                object_id: fact.object_id,
                new_confidence: fact.confidence.unwrap_or(0.0) as f64,
            };

            let _ = tx.send(trigger);
        }

        // Detect conflicts with existing facts
        let existing = self.db.connection()
            .open_table(ferrumyx_db::schema::TABLE_KG_FACTS)
            .execute()
            .await?
            .query()
            .only_if(&format!("subject_id = '{}' AND object_id = '{}'", fact.subject_id, fact.object_id))
            .execute()
            .await?;

        let mut stream = existing;
        while let Some(batch_result) = stream.next().await {
            let batch: arrow_array::RecordBatch = batch_result?;
            for i in 0..batch.num_rows() {
                let existing_fact = ferrumyx_db::schema_arrow::record_to_kg_fact(&batch, i)?;
                
                // Compare fact predicates for directional opposites
                let opposite = (fact.predicate.contains("inhibits") && existing_fact.predicate.contains("activates")) ||
                               (fact.predicate.contains("activates") && existing_fact.predicate.contains("inhibits"));

                if let Some(conflict) = crate::conflict::evaluate_conflict(
                    fact.confidence.unwrap_or(0.0) as f64, 
                    existing_fact.confidence.unwrap_or(0.0) as f64, 
                    opposite
                ) {
                    let conflict_record = KgConflict::new(
                        fact.id,
                        existing_fact.id,
                        format!("{:?}", conflict.conflict_type),
                        conflict.net_confidence as f32,
                        format!("{:?}", conflict.resolution)
                    );

                    let conflict_table = self.db.connection()
                        .open_table(ferrumyx_db::schema::TABLE_KG_CONFLICTS)
                        .execute()
                        .await?;

                    let record = ferrumyx_db::schema_arrow::kg_conflict_to_record(&conflict_record)?;
                    let schema = record.schema();
                    let iter = arrow_array::RecordBatchIterator::new(vec![Ok::<_, arrow_schema::ArrowError>(record)], schema);
                    conflict_table.add(iter).execute().await?;
                }
            }
        }

        Ok(())
    }

    /// Find facts by subject entity.
    pub async fn find_by_subject(&self, subject_id: Uuid) -> Result<Vec<KgFact>> {
        let fact_repo = self.fact_repo();
        Ok(fact_repo.find_by_subject(subject_id).await?)
    }

    /// Find facts by object entity.
    pub async fn find_by_object(&self, object_id: Uuid) -> Result<Vec<KgFact>> {
        let fact_repo = self.fact_repo();
        Ok(fact_repo.find_by_object(object_id).await?)
    }

    /// Find facts by predicate (relationship type).
    pub async fn find_by_predicate(&self, predicate: &str) -> Result<Vec<KgFact>> {
        let fact_repo = self.fact_repo();
        Ok(fact_repo.find_by_predicate(predicate).await?)
    }

    /// Find facts by paper.
    pub async fn find_by_paper(&self, paper_id: Uuid) -> Result<Vec<KgFact>> {
        let fact_repo = self.fact_repo();
        Ok(fact_repo.find_by_paper_id(paper_id).await?)
    }

    /// Count total facts.
    pub async fn fact_count(&self) -> Result<u64> {
        let fact_repo = self.fact_repo();
        Ok(fact_repo.count().await?)
    }
    
    /// Find all facts involving an entity (as subject or object).
    pub async fn find_by_entity(&self, entity_id: Uuid) -> Result<Vec<KgFact>> {
        let fact_repo = self.fact_repo();
        Ok(fact_repo.find_by_entity(entity_id).await?)
    }
    
    /// Find facts by subject and predicate.
    pub async fn find_by_subject_and_predicate(
        &self,
        subject_id: Uuid,
        predicate: &str,
    ) -> Result<Vec<KgFact>> {
        let fact_repo = self.fact_repo();
        Ok(fact_repo.find_by_subject_and_predicate(subject_id, predicate).await?)
    }
}

#[derive(Debug, Clone)]
pub struct SyntheticLethalityResult {
    pub partner_gene_id: Uuid,
    pub partner_symbol: String,
    pub effect_size: Option<f64>,
    pub confidence: f64,
    pub evidence_type: String,
    pub source_db: Option<String>,
}
