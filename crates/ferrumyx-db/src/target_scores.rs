//! Target score repository.
//!
//! Provides CRUD operations for persisted target prioritization scores.

use crate::database::Database;
use crate::error::{DbError, Result};
use crate::schema::TargetScore;
use std::collections::HashMap;
use std::sync::Arc;
use tokio_postgres::{Row, types::ToSql};

/// Repository for target score operations.
#[derive(Clone)]
pub struct TargetScoreRepository {
    db: Arc<Database>,
}

impl TargetScoreRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Insert a new target score.
    pub async fn insert(&self, score: &TargetScore) -> Result<()> {
        let client = self.db.client();
        let has_versioning = self.table_has_versioning(client).await?;
        let mut score = score.clone();
        if has_versioning {
            score.score_version = self.next_score_version(client, score.gene_id, score.cancer_id).await?;
            score.is_current = true;
            self.mark_pair_not_current(client, score.gene_id, score.cancer_id).await?;
        }
        if has_versioning {
            client.execute(
                "INSERT INTO target_scores (id, gene_id, cancer_id, score_version, is_current, composite_score, confidence_adjusted_score, penalty_score, shortlist_tier, components_raw, components_normed, created_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())",
                &[
                    &score.id, &score.gene_id, &score.cancer_id,
                    &score.score_version, &score.is_current,
                    &score.composite_score, &score.confidence_adjusted_score, &score.penalty_score,
                    &score.shortlist_tier, &score.components_raw, &score.components_normed,
                ],
            ).await?;
        } else {
            client.execute(
                "INSERT INTO target_scores (id, gene_id, cancer_id, composite_score, confidence_adjusted_score, penalty_score, shortlist_tier, components_raw, components_normed, created_at) \
                 VALUES ($1, $2, $3, $6, $7, $8, $9, $10, $11, NOW())",
                &[
                    &score.id, &score.gene_id, &score.cancer_id,
                    &score.composite_score, &score.confidence_adjusted_score, &score.penalty_score,
                    &score.shortlist_tier, &score.components_raw, &score.components_normed,
                ],
            ).await?;
        }
        Ok(())
    }

    /// Upsert a score using (gene_id, cancer_id) as logical key.
    pub async fn upsert(&self, score: &TargetScore) -> Result<()> {
        let client = self.db.client();
        let has_versioning = self.table_has_versioning(client).await?;
        let mut score = score.clone();
        if has_versioning {
            score.score_version = self.next_score_version(client, score.gene_id, score.cancer_id).await?;
            score.is_current = true;
            self.mark_pair_not_current(client, score.gene_id, score.cancer_id).await?;
            self.insert(&score).await?;
        } else {
            client.execute(
                "INSERT INTO target_scores (id, gene_id, cancer_id, composite_score, confidence_adjusted_score, penalty_score, shortlist_tier, components_raw, components_normed, created_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW()) \
                 ON CONFLICT (gene_id, cancer_id) DO UPDATE SET \
                 composite_score = EXCLUDED.composite_score, \
                 confidence_adjusted_score = EXCLUDED.confidence_adjusted_score, \
                 penalty_score = EXCLUDED.penalty_score, \
                 shortlist_tier = EXCLUDED.shortlist_tier, \
                 components_raw = EXCLUDED.components_raw, \
                 components_normed = EXCLUDED.components_normed, \
                 created_at = EXCLUDED.created_at",
                &[
                    &score.id, &score.gene_id, &score.cancer_id,
                    &score.composite_score, &score.confidence_adjusted_score, &score.penalty_score,
                    &score.shortlist_tier, &score.components_raw, &score.components_normed,
                ],
            ).await?;
        }
        Ok(())
    }

    /// Upsert a batch of scores.
    pub async fn upsert_batch(&self, scores: &[TargetScore]) -> Result<usize> {
        if scores.is_empty() {
            return Ok(0);
        }
        let client = self.db.client();
        let has_versioning = self.table_has_versioning(client).await?;

        if has_versioning {
            let mut next_versions: HashMap<(uuid::Uuid, uuid::Uuid), i64> = HashMap::new();
            let chunk = 20usize;
            let mut dedup_pairs: Vec<(uuid::Uuid, uuid::Uuid)> = scores.iter().map(|s| (s.gene_id, s.cancer_id)).collect();
            dedup_pairs.sort_unstable();
            dedup_pairs.dedup();

            for pairs in dedup_pairs.chunks(chunk) {
                for (g, c) in pairs {
                    let row = client.query_opt(
                        "SELECT MAX(score_version) as max_ver FROM target_scores WHERE gene_id = $1 AND cancer_id = $2",
                        &[g, c],
                    ).await?;
                    let max_ver: Option<i64> = row.and_then(|r| r.get("max_ver"));
                    next_versions.insert((*g, *c), max_ver.unwrap_or(0));
                }
            }

            for pair in &dedup_pairs {
                let _ = next_versions.entry(*pair).or_insert(0);
            }

            for pairs in dedup_pairs.chunks(chunk) {
                for (g, c) in pairs {
                    client.execute(
                        "UPDATE target_scores SET is_current = false WHERE gene_id = $1 AND cancer_id = $2 AND is_current = true",
                        &[g, c],
                    ).await?;
                }
            }

            let mut versioned_rows = Vec::with_capacity(scores.len());
            for score in scores {
                let key = (score.gene_id, score.cancer_id);
                let base = next_versions.get(&key).copied().unwrap_or(0);
                let mut updated = score.clone();
                updated.score_version = base + 1;
                updated.is_current = true;
                next_versions.insert(key, updated.score_version);
                versioned_rows.push(updated);
            }
            for row in &versioned_rows {
                self.insert(row).await?;
            }
        } else {
            for score in scores {
                self.upsert(score).await?;
            }
        }
        Ok(scores.len())
    }

    /// List all target scores.
    pub async fn list(&self, offset: usize, limit: usize) -> Result<Vec<TargetScore>> {
        let client = self.db.client();
        let sql = "SELECT * FROM target_scores ORDER BY created_at DESC LIMIT $1 OFFSET $2";
        let rows = client.query(sql, &[&(limit as i64), &(offset as i64)]).await?;
        let mut out = rows.into_iter().map(score_from_row).collect::<Vec<_>>();
        if self.table_has_versioning(client).await? {
            Ok(out)
        } else {
            out = dedupe_latest(out);
            if offset > 0 || limit < out.len() {
                out = out.into_iter().skip(offset).take(limit).collect();
            }
            Ok(out)
        }
    }

    /// List all scores for a given gene.
    pub async fn find_by_gene(&self, gene_id: uuid::Uuid) -> Result<Vec<TargetScore>> {
        let client = self.db.client();
        let filter = if self.table_has_versioning(client).await? {
            "gene_id = $1 AND is_current = true"
        } else {
            "gene_id = $1"
        };
        let sql = format!("SELECT * FROM target_scores WHERE {}", filter);
        let rows = client.query(&sql, &[&gene_id]).await?;
        let mut out = rows.into_iter().map(score_from_row).collect::<Vec<_>>();
        if !self.table_has_versioning(client).await? {
            out = dedupe_latest(out);
        }
        Ok(out)
    }

    /// Fetch current scores for a set of genes in a bounded number of queries.
    pub async fn find_current_by_gene_ids(
        &self,
        gene_ids: &[uuid::Uuid],
        chunk_size: usize,
    ) -> Result<Vec<TargetScore>> {
        if gene_ids.is_empty() {
            return Ok(Vec::new());
        }
        let client = self.db.client();
        let mut out = Vec::new();
        let chunk = chunk_size.max(1).min(500);
        for ids in gene_ids.chunks(chunk) {
            if ids.is_empty() { continue; }
            let placeholders = (1..=ids.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(",");
            let filter = if self.table_has_versioning(client).await? {
                format!("gene_id IN ({}) AND is_current = true", placeholders)
            } else {
                format!("gene_id IN ({})", placeholders)
            };
            let sql = format!("SELECT * FROM target_scores WHERE {}", filter);
            let params: Vec<&(dyn ToSql + Sync)> = ids.iter().map(|id| id as &(dyn ToSql + Sync)).collect();
            let rows = client.query(&sql, params.as_slice()).await?;
            out.extend(rows.into_iter().map(score_from_row));
        }
        if !self.table_has_versioning(client).await? {
            out = dedupe_latest(out);
        }
        Ok(out)
    }

    /// Delete all score rows for a bounded set of genes.
    /// This is used by incremental recompute to prevent stale rows for impacted genes.
    pub async fn delete_by_gene_ids(
        &self,
        gene_ids: &[uuid::Uuid],
        chunk_size: usize,
    ) -> Result<u64> {
        if gene_ids.is_empty() {
            return Ok(0);
        }
        let client = self.db.client();
        let mut uniq = gene_ids.to_vec();
        uniq.sort_unstable();
        uniq.dedup();
        let chunk = chunk_size.max(1).min(500);
        let mut _total_deleted = 0u64;
        for ids in uniq.chunks(chunk) {
            if ids.is_empty() { continue; }
            let placeholders = (1..=ids.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(",");
            let where_clause = format!("gene_id IN ({})", placeholders);
            let params: Vec<&(dyn ToSql + Sync)> = ids.iter().map(|id| id as &(dyn ToSql + Sync)).collect();
            let affected = client.execute(&format!("DELETE FROM target_scores WHERE {}", where_clause), params.as_slice()).await?;
            _total_deleted += affected as u64;
        }
        Ok(0)
    }

    /// Count rows in target_scores table.
    pub async fn count(&self) -> Result<u64> {
        let client = self.db.client();
        if self.table_has_versioning(client).await? {
            let row = client.query_one("SELECT COUNT(*) FROM target_scores WHERE is_current = true", &[]).await?;
            Ok(row.get::<_, i64>(0) as u64)
        } else {
            let all = self.list(0, usize::MAX).await?;
            Ok(all.len() as u64)
        }
    }

    async fn next_score_version(&self, client: &tokio_postgres::Client, gene_id: uuid::Uuid, cancer_id: uuid::Uuid) -> Result<i64> {
        let row = client.query_one(
            "SELECT COALESCE(MAX(score_version), 0) + 1 FROM target_scores WHERE gene_id = $1 AND cancer_id = $2",
            &[&gene_id, &cancer_id],
        ).await?;
        Ok(row.get::<_, i64>(0))
    }

    async fn mark_pair_not_current(&self, client: &tokio_postgres::Client, gene_id: uuid::Uuid, cancer_id: uuid::Uuid) -> Result<()> {
        client.execute(
            "UPDATE target_scores SET is_current = false WHERE gene_id = $1 AND cancer_id = $2",
            &[&gene_id, &cancer_id],
        ).await?;
        Ok(())
    }

    async fn table_has_versioning(&self, client: &tokio_postgres::Client) -> Result<bool> {
        let row = client.query_opt(
            "SELECT column_name FROM information_schema.columns WHERE table_name = 'target_scores' AND column_name = 'score_version'",
            &[]
        ).await?;
        Ok(row.is_some())
    }
}

fn score_from_row(row: Row) -> TargetScore {
    TargetScore {
        id: row.get("id"),
        gene_id: row.get("gene_id"),
        cancer_id: row.get("cancer_id"),
        score_version: row.get::<_, i64>("score_version") as i64,
        is_current: row.get::<_, bool>("is_current"),
        composite_score: row.get::<_, f64>("composite_score"),
        confidence_adjusted_score: row.get::<_, f64>("confidence_adjusted_score"),
        penalty_score: row.get::<_, f64>("penalty_score"),
        shortlist_tier: row.get("shortlist_tier"),
        components_raw: row.get("components_raw"),
        components_normed: row.get("components_normed"),
        created_at: row.get::<_, chrono::DateTime<chrono::Utc>>("created_at"),
    }
}

fn dedupe_latest(scores: Vec<TargetScore>) -> Vec<TargetScore> {
    let mut latest: HashMap<(uuid::Uuid, uuid::Uuid), TargetScore> = HashMap::new();
    for score in scores {
        let key = (score.gene_id, score.cancer_id);
        if let Some(existing) = latest.get(&key) {
            if (score.score_version, score.created_at) > (existing.score_version, existing.created_at) {
                latest.insert(key, score);
            }
        } else {
            latest.insert(key, score);
        }
    }
    latest.into_values().collect()
}
