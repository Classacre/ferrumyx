//! Target score repository.
//!
//! Provides CRUD operations for persisted target prioritization scores.

use crate::database::Database;
use crate::error::{DbError, Result};
use crate::schema::TargetScore;
use std::collections::HashMap;
use std::sync::Arc;

use arrow_array::{Array, BooleanArray, Float64Array, Int64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use futures::StreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};

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
        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_TARGET_SCORES)
            .execute()
            .await?;

        let has_versioning = table_has_versioning(&table).await?;
        let mut score = score.clone();
        if has_versioning {
            score.score_version = self.next_score_version(&table, score.gene_id, score.cancer_id).await?;
            score.is_current = true;
            self.mark_pair_not_current(&table, score.gene_id, score.cancer_id).await?;
        }

        let record = target_score_to_record(&score, has_versioning)?;
        let schema = record.schema();
        let iter = arrow_array::RecordBatchIterator::new(vec![Ok(record)], schema);
        table.add(iter).execute().await?;
        Ok(())
    }

    /// Upsert a score using (gene_id, cancer_id) as logical key.
    pub async fn upsert(&self, score: &TargetScore) -> Result<()> {
        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_TARGET_SCORES)
            .execute()
            .await?;

        let has_versioning = table_has_versioning(&table).await?;
        let mut score = score.clone();
        if has_versioning {
            score.score_version = self.next_score_version(&table, score.gene_id, score.cancer_id).await?;
            score.is_current = true;
            self.mark_pair_not_current(&table, score.gene_id, score.cancer_id).await?;

            let record = target_score_to_record(&score, true)?;
            let schema = record.schema();
            let iter = arrow_array::RecordBatchIterator::new(vec![Ok(record)], schema);
            table.add(iter).execute().await?;
        } else {
            let record = target_score_to_record(&score, false)?;
            let schema = record.schema();
            let iter = arrow_array::RecordBatchIterator::new(vec![Ok(record)], schema);

            let mut builder = table.merge_insert(&["gene_id", "cancer_id"]);
            builder.when_matched_update_all(None);
            builder.execute(Box::new(iter)).await?;
        }
        Ok(())
    }

    /// Upsert a batch of scores.
    pub async fn upsert_batch(&self, scores: &[TargetScore]) -> Result<usize> {
        if scores.is_empty() {
            return Ok(0);
        }
        for score in scores {
            self.upsert(score).await?;
        }
        Ok(scores.len())
    }

    /// List all target scores.
    pub async fn list(&self, offset: usize, limit: usize) -> Result<Vec<TargetScore>> {
        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_TARGET_SCORES)
            .execute()
            .await?;

        let has_versioning = table_has_versioning(&table).await?;
        let mut stream = if has_versioning {
            table.query()
                .only_if("is_current = true")
                .offset(offset)
                .limit(limit)
                .execute()
                .await?
        } else {
            table.query().offset(offset).limit(limit).execute().await?
        };
        let mut out = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for row in 0..batch.num_rows() {
                out.push(record_to_target_score(&batch, row)?);
            }
        }
        if !has_versioning {
            out = dedupe_latest(out);
            if offset > 0 || limit < out.len() {
                out = out.into_iter().skip(offset).take(limit).collect();
            }
        }
        Ok(out)
    }

    /// List all scores for a given gene.
    pub async fn find_by_gene(&self, gene_id: uuid::Uuid) -> Result<Vec<TargetScore>> {
        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_TARGET_SCORES)
            .execute()
            .await?;

        let has_versioning = table_has_versioning(&table).await?;
        let filter = if has_versioning {
            format!("gene_id = '{}' AND is_current = true", gene_id)
        } else {
            format!("gene_id = '{}'", gene_id)
        };
        let mut stream = table.query().only_if(&filter).execute().await?;
        let mut out = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for row in 0..batch.num_rows() {
                out.push(record_to_target_score(&batch, row)?);
            }
        }
        if !has_versioning {
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
        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_TARGET_SCORES)
            .execute()
            .await?;
        let has_versioning = table_has_versioning(&table).await?;

        let mut out = Vec::new();
        let chunk = chunk_size.max(1).min(500);
        for ids in gene_ids.chunks(chunk) {
            let id_filter = ids
                .iter()
                .map(|id| format!("gene_id = '{}'", id))
                .collect::<Vec<_>>()
                .join(" OR ");
            let filter = if has_versioning {
                format!("({}) AND is_current = true", id_filter)
            } else {
                format!("({})", id_filter)
            };

            let mut stream = table.query().only_if(&filter).execute().await?;
            while let Some(batch) = stream.next().await {
                let batch = batch?;
                for row in 0..batch.num_rows() {
                    out.push(record_to_target_score(&batch, row)?);
                }
            }
        }

        if !has_versioning {
            out = dedupe_latest(out);
        }

        Ok(out)
    }

    /// Count rows in target_scores table.
    pub async fn count(&self) -> Result<u64> {
        let table = self
            .db
            .connection()
            .open_table(crate::schema::TABLE_TARGET_SCORES)
            .execute()
            .await?;
        let has_versioning = table_has_versioning(&table).await?;
        if has_versioning {
            Ok(table.count_rows(Some("is_current = true".to_string())).await? as u64)
        } else {
            let all = self.list(0, usize::MAX).await?;
            Ok(all.len() as u64)
        }
    }

    async fn next_score_version(
        &self,
        table: &lancedb::table::Table,
        gene_id: uuid::Uuid,
        cancer_id: uuid::Uuid,
    ) -> Result<i64> {
        let filter = format!("gene_id = '{}' AND cancer_id = '{}'", gene_id, cancer_id);
        let mut stream = table.query().only_if(&filter).execute().await?;
        let mut max_version = 0_i64;
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for row in 0..batch.num_rows() {
                let score = record_to_target_score(&batch, row)?;
                max_version = max_version.max(score.score_version);
            }
        }
        Ok(max_version + 1)
    }

    async fn mark_pair_not_current(
        &self,
        table: &lancedb::table::Table,
        gene_id: uuid::Uuid,
        cancer_id: uuid::Uuid,
    ) -> Result<()> {
        let filter = format!(
            "gene_id = '{}' AND cancer_id = '{}' AND is_current = true",
            gene_id, cancer_id
        );
        table
            .update()
            .only_if(filter)
            .column("is_current", "false")
            .execute()
            .await?;
        Ok(())
    }
}

fn target_score_to_record(score: &TargetScore, include_versioning: bool) -> Result<RecordBatch> {
    let mut fields = vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("gene_id", DataType::Utf8, false),
        Field::new("cancer_id", DataType::Utf8, false),
    ];
    if include_versioning {
        fields.push(Field::new("score_version", DataType::Int64, false));
        fields.push(Field::new("is_current", DataType::Boolean, false));
    }
    fields.extend([
        Field::new("composite_score", DataType::Float64, false),
        Field::new("confidence_adjusted_score", DataType::Float64, false),
        Field::new("penalty_score", DataType::Float64, false),
        Field::new("shortlist_tier", DataType::Utf8, false),
        Field::new("components_raw", DataType::Utf8, false),
        Field::new("components_normed", DataType::Utf8, false),
        Field::new("created_at", DataType::Utf8, false),
    ]);
    let schema = Arc::new(Schema::new(fields));

    let mut cols: Vec<Arc<dyn arrow_array::Array>> = vec![
        Arc::new(StringArray::from(vec![score.id.to_string()])),
        Arc::new(StringArray::from(vec![score.gene_id.to_string()])),
        Arc::new(StringArray::from(vec![score.cancer_id.to_string()])),
    ];
    if include_versioning {
        cols.push(Arc::new(Int64Array::from(vec![score.score_version])));
        cols.push(Arc::new(BooleanArray::from(vec![score.is_current])));
    }
    cols.extend([
        Arc::new(Float64Array::from(vec![score.composite_score])) as Arc<dyn arrow_array::Array>,
        Arc::new(Float64Array::from(vec![score.confidence_adjusted_score])),
        Arc::new(Float64Array::from(vec![score.penalty_score])),
        Arc::new(StringArray::from(vec![score.shortlist_tier.clone()])),
        Arc::new(StringArray::from(vec![score.components_raw.clone()])),
        Arc::new(StringArray::from(vec![score.components_normed.clone()])),
        Arc::new(StringArray::from(vec![score.created_at.to_rfc3339()])),
    ]);

    Ok(RecordBatch::try_new(schema, cols)?)
}

fn record_to_target_score(batch: &RecordBatch, row: usize) -> Result<TargetScore> {
    let schema = batch.schema();
    let col_idx = |name: &str| -> Option<usize> { schema.index_of(name).ok() };

    let get_s = |idx: usize| -> Result<String> {
        let arr = batch
            .column(idx)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| DbError::Arrow(format!("column {idx} is not StringArray")))?;
        if arr.is_null(row) {
            Ok(String::new())
        } else {
            Ok(arr.value(row).to_string())
        }
    };
    let get_f = |idx: usize| -> Result<f64> {
        let arr = batch
            .column(idx)
            .as_any()
            .downcast_ref::<Float64Array>()
            .ok_or_else(|| DbError::Arrow(format!("column {idx} is not Float64Array")))?;
        if arr.is_null(row) { Ok(0.0) } else { Ok(arr.value(row)) }
    };
    let get_i = |idx: usize| -> Result<i64> {
        let arr = batch
            .column(idx)
            .as_any()
            .downcast_ref::<Int64Array>()
            .ok_or_else(|| DbError::Arrow(format!("column {idx} is not Int64Array")))?;
        if arr.is_null(row) { Ok(0) } else { Ok(arr.value(row)) }
    };
    let get_b = |idx: usize| -> Result<bool> {
        let arr = batch
            .column(idx)
            .as_any()
            .downcast_ref::<BooleanArray>()
            .ok_or_else(|| DbError::Arrow(format!("column {idx} is not BooleanArray")))?;
        if arr.is_null(row) { Ok(false) } else { Ok(arr.value(row)) }
    };

    let id_idx = col_idx("id").unwrap_or(0);
    let gene_idx = col_idx("gene_id").unwrap_or(1);
    let cancer_idx = col_idx("cancer_id").unwrap_or(2);
    let score_version_idx = col_idx("score_version");
    let is_current_idx = col_idx("is_current");
    let composite_idx = col_idx("composite_score").unwrap_or(3);
    let conf_adj_idx = col_idx("confidence_adjusted_score").unwrap_or(4);
    let penalty_idx = col_idx("penalty_score").unwrap_or(5);
    let tier_idx = col_idx("shortlist_tier").unwrap_or(6);
    let raw_idx = col_idx("components_raw").unwrap_or(7);
    let norm_idx = col_idx("components_normed").unwrap_or(8);
    let created_idx = col_idx("created_at").unwrap_or(9);

    let id = uuid::Uuid::parse_str(&get_s(id_idx)?)
        .map_err(|e| DbError::Serialization(serde_json::Error::io(std::io::Error::other(e.to_string()))))?;
    let gene_id = uuid::Uuid::parse_str(&get_s(gene_idx)?)
        .map_err(|e| DbError::Serialization(serde_json::Error::io(std::io::Error::other(e.to_string()))))?;
    let cancer_id = uuid::Uuid::parse_str(&get_s(cancer_idx)?)
        .map_err(|e| DbError::Serialization(serde_json::Error::io(std::io::Error::other(e.to_string()))))?;

    let created_at = chrono::DateTime::parse_from_rfc3339(&get_s(created_idx)?)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());

    Ok(TargetScore {
        id,
        gene_id,
        cancer_id,
        score_version: score_version_idx.map(|idx| get_i(idx)).transpose()?.unwrap_or(1),
        is_current: is_current_idx.map(|idx| get_b(idx)).transpose()?.unwrap_or(true),
        composite_score: get_f(composite_idx)?,
        confidence_adjusted_score: get_f(conf_adj_idx)?,
        penalty_score: get_f(penalty_idx)?,
        shortlist_tier: get_s(tier_idx)?,
        components_raw: get_s(raw_idx)?,
        components_normed: get_s(norm_idx)?,
        created_at,
    })
}

async fn table_has_versioning(table: &lancedb::table::Table) -> Result<bool> {
    let schema = table.schema().await?;
    let names: HashMap<_, _> = schema
        .fields()
        .iter()
        .map(|f| (f.name().as_str(), true))
        .collect();
    Ok(names.contains_key("score_version") && names.contains_key("is_current"))
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
