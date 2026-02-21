//! KG repository — database access layer.
//! Wraps sqlx queries for KG fact storage and retrieval.
//! See ARCHITECTURE.md §3.7 for query patterns.

// NOTE: Full implementation requires a live PostgreSQL connection.
// This module defines the repository interface (trait) and
// stub implementations. Real implementations will be added
// in Phase 1 Month 2 when the DB schema is deployed.

use async_trait::async_trait;
use uuid::Uuid;
use ferrumyx_common::entities::KgFact;

#[async_trait]
pub trait KgRepository: Send + Sync {
    /// Insert a new KG fact (append-only).
    async fn insert_fact(&self, fact: &KgFact) -> anyhow::Result<Uuid>;

    /// Get all current facts for a (subject, predicate) pair.
    async fn get_facts(
        &self,
        subject_id: Uuid,
        predicate: &str,
    ) -> anyhow::Result<Vec<KgFact>>;

    /// Supersede an existing fact (set valid_until = now).
    async fn supersede_fact(&self, fact_id: Uuid) -> anyhow::Result<()>;

    /// Get synthetic lethality partners for a gene in a cancer context.
    async fn get_synthetic_lethality_partners(
        &self,
        gene_id: Uuid,
        cancer_id: Uuid,
        min_confidence: f64,
    ) -> anyhow::Result<Vec<SyntheticLethalityResult>>;
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
