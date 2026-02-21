//! PostgreSQL implementation of KgRepository.
//!
//! All writes are append-only: facts are never deleted, only superseded
//! (valid_until = NOW()). This preserves the full evidence audit trail.

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use anyhow::{Context, Result};
use ferrumyx_common::entities::{KgFact, EvidenceType};
use crate::repository::{KgRepository, SyntheticLethalityResult};

/// PostgreSQL-backed KG repository.
#[derive(Clone)]
pub struct PgKgRepository {
    pool: PgPool,
}

impl PgKgRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[async_trait]
impl KgRepository for PgKgRepository {
    async fn insert_fact(&self, fact: &KgFact) -> Result<Uuid> {
        let id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO kg_facts
                (id, subject_id, predicate, object_id, confidence,
                 evidence_type, evidence_weight, source_pmid, source_doi,
                 source_db, sample_size, study_type,
                 valid_from, valid_until)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,NOW(),NULL)
            ON CONFLICT DO NOTHING
            RETURNING id
            "#,
        )
        .bind(fact.id)
        .bind(fact.subject_id)
        .bind(&fact.predicate)
        .bind(fact.object_id)
        .bind(fact.confidence)
        .bind(fact.evidence_type.as_str())
        .bind(fact.evidence_weight)
        .bind(&fact.source_pmid)
        .bind(&fact.source_doi)
        .bind(&fact.source_db)
        .bind(fact.sample_size)
        .bind(&fact.study_type)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to insert KG fact")?
        .unwrap_or(fact.id); // if ON CONFLICT DO NOTHING, return existing id

        Ok(id)
    }

    async fn get_facts(&self, subject_id: Uuid, predicate: &str) -> Result<Vec<KgFact>> {
        let rows = sqlx::query_as::<_, KgFactRow>(
            r#"
            SELECT id, subject_id, predicate, object_id,
                   confidence, evidence_type, evidence_weight,
                   source_pmid, source_doi, source_db,
                   sample_size, study_type,
                   created_at, valid_from, valid_until
            FROM kg_facts
            WHERE subject_id = $1
              AND predicate   = $2
              AND valid_until IS NULL
            ORDER BY confidence DESC
            "#,
        )
        .bind(subject_id)
        .bind(predicate)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch KG facts")?;

        Ok(rows.into_iter().map(KgFact::from).collect())
    }

    async fn supersede_fact(&self, fact_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE kg_facts SET valid_until = NOW() WHERE id = $1")
            .bind(fact_id)
            .execute(&self.pool)
            .await
            .context("Failed to supersede KG fact")?;
        Ok(())
    }

    async fn get_synthetic_lethality_partners(
        &self,
        gene_id: Uuid,
        cancer_id: Uuid,
        min_confidence: f64,
    ) -> Result<Vec<SyntheticLethalityResult>> {
        let rows: Vec<(Uuid, String, Option<f64>, f64, String, Option<String>)> =
            sqlx::query_as(
                r#"
                SELECT
                    CASE WHEN sl.gene1_id = $1 THEN sl.gene2_id ELSE sl.gene1_id END,
                    COALESCE(eg.symbol, e.name),
                    sl.effect_size,
                    sl.confidence,
                    sl.evidence_type,
                    sl.source_db
                FROM ent_synthetic_lethality sl
                JOIN entities e
                    ON e.id = CASE WHEN sl.gene1_id = $1 THEN sl.gene2_id ELSE sl.gene1_id END
                LEFT JOIN ent_genes eg ON eg.id = e.id
                WHERE (sl.gene1_id = $1 OR sl.gene2_id = $1)
                  AND (sl.cancer_id = $2 OR sl.cancer_id IS NULL)
                  AND sl.confidence >= $3
                ORDER BY sl.confidence DESC
                "#,
            )
            .bind(gene_id)
            .bind(cancer_id)
            .bind(min_confidence)
            .fetch_all(&self.pool)
            .await
            .context("Failed to fetch synthetic lethality partners")?;

        Ok(rows.into_iter().map(|(partner_gene_id, partner_symbol, effect_size, confidence, evidence_type, source_db)| {
            SyntheticLethalityResult { partner_gene_id, partner_symbol, effect_size, confidence, evidence_type, source_db }
        }).collect())
    }
}

// ── Bulk operations ──────────────────────────────────────────────────────────

impl PgKgRepository {
    /// Insert multiple facts in a single transaction.
    pub async fn bulk_upsert_facts(&self, facts: &[KgFact]) -> Result<usize> {
        if facts.is_empty() { return Ok(0); }
        let mut tx = self.pool.begin().await?;
        let mut count = 0usize;

        for fact in facts {
            sqlx::query(
                r#"
                INSERT INTO kg_facts
                    (id, subject_id, predicate, object_id, confidence,
                     evidence_type, evidence_weight, source_pmid, source_doi,
                     source_db, sample_size, study_type, valid_from, valid_until)
                VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,NOW(),NULL)
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(fact.id)
            .bind(fact.subject_id)
            .bind(&fact.predicate)
            .bind(fact.object_id)
            .bind(fact.confidence)
            .bind(fact.evidence_type.as_str())
            .bind(fact.evidence_weight)
            .bind(&fact.source_pmid)
            .bind(&fact.source_doi)
            .bind(&fact.source_db)
            .bind(fact.sample_size)
            .bind(&fact.study_type)
            .execute(&mut *tx)
            .await?;
            count += 1;
        }

        tx.commit().await?;
        tracing::debug!("bulk_upsert_facts: committed {count} facts");
        Ok(count)
    }

    /// Resolve a gene symbol → entity UUID.
    pub async fn resolve_gene_id(&self, symbol: &str) -> Result<Option<Uuid>> {
        sqlx::query_scalar(
            "SELECT e.id FROM entities e \
             JOIN ent_genes eg ON eg.id = e.id \
             WHERE eg.symbol = $1 LIMIT 1"
        )
        .bind(symbol)
        .fetch_optional(&self.pool)
        .await
        .context("resolve_gene_id query failed")
    }

    /// Resolve an OncoTree code → entity UUID.
    pub async fn resolve_cancer_id(&self, oncotree_code: &str) -> Result<Option<Uuid>> {
        sqlx::query_scalar(
            "SELECT e.id FROM entities e \
             JOIN ent_cancer_types ec ON ec.id = e.id \
             WHERE ec.oncotree_code = $1 LIMIT 1"
        )
        .bind(oncotree_code)
        .fetch_optional(&self.pool)
        .await
        .context("resolve_cancer_id query failed")
    }

    /// Count current facts for a gene (literature support score component).
    pub async fn fact_count_for_gene(&self, gene_id: Uuid) -> Result<i64> {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM kg_facts \
             WHERE subject_id = $1 AND valid_until IS NULL"
        )
        .bind(gene_id)
        .fetch_one(&self.pool)
        .await
        .context("fact_count_for_gene failed")
    }

    /// Average confidence across current facts for a gene.
    pub async fn avg_confidence_for_gene(&self, gene_id: Uuid) -> Result<f64> {
        let avg: Option<f64> = sqlx::query_scalar(
            "SELECT AVG(confidence) FROM kg_facts \
             WHERE subject_id = $1 AND valid_until IS NULL"
        )
        .bind(gene_id)
        .fetch_one(&self.pool)
        .await
        .context("avg_confidence_for_gene failed")?;
        Ok(avg.unwrap_or(0.0))
    }
}

// ── Internal sqlx row mapping ────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct KgFactRow {
    id: Uuid,
    subject_id: Uuid,
    predicate: String,
    object_id: Uuid,
    confidence: f64,
    evidence_type: String,
    evidence_weight: f64,
    source_pmid: Option<String>,
    source_doi: Option<String>,
    source_db: Option<String>,
    sample_size: Option<i32>,
    study_type: Option<String>,
    created_at: DateTime<Utc>,
    valid_from: DateTime<Utc>,
    valid_until: Option<DateTime<Utc>>,
}

impl From<KgFactRow> for KgFact {
    fn from(r: KgFactRow) -> Self {
        KgFact {
            id: r.id,
            subject_id: r.subject_id,
            predicate: r.predicate,
            object_id: r.object_id,
            confidence: r.confidence,
            evidence_type: EvidenceType::from_str(&r.evidence_type),
            evidence_weight: r.evidence_weight,
            source_pmid: r.source_pmid,
            source_doi: r.source_doi,
            source_db: r.source_db,
            sample_size: r.sample_size,
            study_type: r.study_type,
            created_at: r.created_at,
            valid_from: r.valid_from,
            valid_until: r.valid_until,
        }
    }
}
