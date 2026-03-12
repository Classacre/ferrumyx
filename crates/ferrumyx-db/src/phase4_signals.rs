//! Phase 4 provider-backed signal cache repository.
//!
//! Stores bounded external enrichments so ranker can avoid repeated API calls.

use crate::database::Database;
use crate::error::{DbError, Result};
use crate::schema::{
    EntChemblTarget, EntGtexExpression, EntReactomeGene, EntTcgaSurvival, TABLE_ENT_CHEMBL_TARGETS,
    TABLE_ENT_GTEX_EXPRESSION, TABLE_ENT_REACTOME_GENES, TABLE_ENT_TCGA_SURVIVAL,
};
use std::sync::Arc;

use arrow_array::{Array, Float64Array, Int64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use futures::StreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};

#[derive(Clone)]
pub struct Phase4SignalRepository {
    db: Arc<Database>,
}

impl Phase4SignalRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn find_tcga_survival(
        &self,
        gene_symbol: &str,
        cancer_code: &str,
    ) -> Result<Option<EntTcgaSurvival>> {
        let gene = normalize_symbol(gene_symbol);
        let cancer = normalize_symbol(cancer_code);
        if gene.is_empty() || cancer.is_empty() {
            return Ok(None);
        }

        let table = self
            .db
            .connection()
            .open_table(TABLE_ENT_TCGA_SURVIVAL)
            .execute()
            .await?;
        let filter = format!(
            "gene_symbol = '{}' AND cancer_code = '{}'",
            escape_sql(&gene),
            escape_sql(&cancer)
        );
        let mut stream = table.query().only_if(&filter).limit(1).execute().await?;
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            if batch.num_rows() > 0 {
                return Ok(Some(record_to_tcga_survival(&batch, 0)?));
            }
        }
        Ok(None)
    }

    pub async fn find_tcga_survival_fresh(
        &self,
        gene_symbol: &str,
        cancer_code: &str,
        max_age_days: i64,
    ) -> Result<Option<EntTcgaSurvival>> {
        let found = self.find_tcga_survival(gene_symbol, cancer_code).await?;
        Ok(found.filter(|v| is_fresh(v.fetched_at, max_age_days)))
    }

    pub async fn upsert_tcga_survival(&self, signal: &EntTcgaSurvival) -> Result<()> {
        let table = self
            .db
            .connection()
            .open_table(TABLE_ENT_TCGA_SURVIVAL)
            .execute()
            .await?;
        let record = tcga_survival_to_record(signal)?;
        let schema = record.schema();
        let iter = arrow_array::RecordBatchIterator::new(vec![Ok(record)], schema);
        let mut builder = table.merge_insert(&["gene_symbol", "cancer_code"]);
        builder.when_matched_update_all(None);
        builder.execute(Box::new(iter)).await?;
        Ok(())
    }

    pub async fn find_gtex_expression(
        &self,
        gene_symbol: &str,
    ) -> Result<Option<EntGtexExpression>> {
        let gene = normalize_symbol(gene_symbol);
        if gene.is_empty() {
            return Ok(None);
        }

        let table = self
            .db
            .connection()
            .open_table(TABLE_ENT_GTEX_EXPRESSION)
            .execute()
            .await?;
        let filter = format!("gene_symbol = '{}'", escape_sql(&gene));
        let mut stream = table.query().only_if(&filter).limit(1).execute().await?;
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            if batch.num_rows() > 0 {
                return Ok(Some(record_to_gtex_expression(&batch, 0)?));
            }
        }
        Ok(None)
    }

    pub async fn find_gtex_expression_fresh(
        &self,
        gene_symbol: &str,
        max_age_days: i64,
    ) -> Result<Option<EntGtexExpression>> {
        let found = self.find_gtex_expression(gene_symbol).await?;
        Ok(found.filter(|v| is_fresh(v.fetched_at, max_age_days)))
    }

    pub async fn upsert_gtex_expression(&self, signal: &EntGtexExpression) -> Result<()> {
        let table = self
            .db
            .connection()
            .open_table(TABLE_ENT_GTEX_EXPRESSION)
            .execute()
            .await?;
        let record = gtex_expression_to_record(signal)?;
        let schema = record.schema();
        let iter = arrow_array::RecordBatchIterator::new(vec![Ok(record)], schema);
        let mut builder = table.merge_insert(&["gene_symbol"]);
        builder.when_matched_update_all(None);
        builder.execute(Box::new(iter)).await?;
        Ok(())
    }

    pub async fn find_chembl_target(&self, gene_symbol: &str) -> Result<Option<EntChemblTarget>> {
        let gene = normalize_symbol(gene_symbol);
        if gene.is_empty() {
            return Ok(None);
        }
        let table = self
            .db
            .connection()
            .open_table(TABLE_ENT_CHEMBL_TARGETS)
            .execute()
            .await?;
        let filter = format!("gene_symbol = '{}'", escape_sql(&gene));
        let mut stream = table.query().only_if(&filter).limit(1).execute().await?;
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            if batch.num_rows() > 0 {
                return Ok(Some(record_to_chembl_target(&batch, 0)?));
            }
        }
        Ok(None)
    }

    pub async fn find_chembl_target_fresh(
        &self,
        gene_symbol: &str,
        max_age_days: i64,
    ) -> Result<Option<EntChemblTarget>> {
        let found = self.find_chembl_target(gene_symbol).await?;
        Ok(found.filter(|v| is_fresh(v.fetched_at, max_age_days)))
    }

    pub async fn upsert_chembl_target(&self, signal: &EntChemblTarget) -> Result<()> {
        let table = self
            .db
            .connection()
            .open_table(TABLE_ENT_CHEMBL_TARGETS)
            .execute()
            .await?;
        let record = chembl_target_to_record(signal)?;
        let schema = record.schema();
        let iter = arrow_array::RecordBatchIterator::new(vec![Ok(record)], schema);
        let mut builder = table.merge_insert(&["gene_symbol"]);
        builder.when_matched_update_all(None);
        builder.execute(Box::new(iter)).await?;
        Ok(())
    }

    pub async fn find_reactome_gene(&self, gene_symbol: &str) -> Result<Option<EntReactomeGene>> {
        let gene = normalize_symbol(gene_symbol);
        if gene.is_empty() {
            return Ok(None);
        }
        let table = self
            .db
            .connection()
            .open_table(TABLE_ENT_REACTOME_GENES)
            .execute()
            .await?;
        let filter = format!("gene_symbol = '{}'", escape_sql(&gene));
        let mut stream = table.query().only_if(&filter).limit(1).execute().await?;
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            if batch.num_rows() > 0 {
                return Ok(Some(record_to_reactome_gene(&batch, 0)?));
            }
        }
        Ok(None)
    }

    pub async fn find_reactome_gene_fresh(
        &self,
        gene_symbol: &str,
        max_age_days: i64,
    ) -> Result<Option<EntReactomeGene>> {
        let found = self.find_reactome_gene(gene_symbol).await?;
        Ok(found.filter(|v| is_fresh(v.fetched_at, max_age_days)))
    }

    pub async fn upsert_reactome_gene(&self, signal: &EntReactomeGene) -> Result<()> {
        let table = self
            .db
            .connection()
            .open_table(TABLE_ENT_REACTOME_GENES)
            .execute()
            .await?;
        let record = reactome_gene_to_record(signal)?;
        let schema = record.schema();
        let iter = arrow_array::RecordBatchIterator::new(vec![Ok(record)], schema);
        let mut builder = table.merge_insert(&["gene_symbol"]);
        builder.when_matched_update_all(None);
        builder.execute(Box::new(iter)).await?;
        Ok(())
    }
}

fn tcga_survival_to_record(signal: &EntTcgaSurvival) -> Result<RecordBatch> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("gene_symbol", DataType::Utf8, false),
        Field::new("cancer_code", DataType::Utf8, false),
        Field::new("tcga_project_id", DataType::Utf8, false),
        Field::new("survival_score", DataType::Float64, false),
        Field::new("source", DataType::Utf8, false),
        Field::new("fetched_at", DataType::Utf8, false),
    ]));

    let cols: Vec<Arc<dyn Array>> = vec![
        Arc::new(StringArray::from(vec![signal.id.to_string()])),
        Arc::new(StringArray::from(vec![signal.gene_symbol.clone()])),
        Arc::new(StringArray::from(vec![signal.cancer_code.clone()])),
        Arc::new(StringArray::from(vec![signal.tcga_project_id.clone()])),
        Arc::new(Float64Array::from(vec![signal.survival_score])),
        Arc::new(StringArray::from(vec![signal.source.clone()])),
        Arc::new(StringArray::from(vec![signal.fetched_at.to_rfc3339()])),
    ];
    Ok(RecordBatch::try_new(schema, cols)?)
}

fn gtex_expression_to_record(signal: &EntGtexExpression) -> Result<RecordBatch> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("gene_symbol", DataType::Utf8, false),
        Field::new("expression_score", DataType::Float64, false),
        Field::new("source", DataType::Utf8, false),
        Field::new("fetched_at", DataType::Utf8, false),
    ]));

    let cols: Vec<Arc<dyn Array>> = vec![
        Arc::new(StringArray::from(vec![signal.id.to_string()])),
        Arc::new(StringArray::from(vec![signal.gene_symbol.clone()])),
        Arc::new(Float64Array::from(vec![signal.expression_score])),
        Arc::new(StringArray::from(vec![signal.source.clone()])),
        Arc::new(StringArray::from(vec![signal.fetched_at.to_rfc3339()])),
    ];
    Ok(RecordBatch::try_new(schema, cols)?)
}

fn chembl_target_to_record(signal: &EntChemblTarget) -> Result<RecordBatch> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("gene_symbol", DataType::Utf8, false),
        Field::new("inhibitor_count", DataType::Int64, false),
        Field::new("source", DataType::Utf8, false),
        Field::new("fetched_at", DataType::Utf8, false),
    ]));

    let cols: Vec<Arc<dyn Array>> = vec![
        Arc::new(StringArray::from(vec![signal.id.to_string()])),
        Arc::new(StringArray::from(vec![signal.gene_symbol.clone()])),
        Arc::new(Int64Array::from(vec![signal.inhibitor_count])),
        Arc::new(StringArray::from(vec![signal.source.clone()])),
        Arc::new(StringArray::from(vec![signal.fetched_at.to_rfc3339()])),
    ];
    Ok(RecordBatch::try_new(schema, cols)?)
}

fn reactome_gene_to_record(signal: &EntReactomeGene) -> Result<RecordBatch> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("gene_symbol", DataType::Utf8, false),
        Field::new("pathway_count", DataType::Int64, false),
        Field::new("source", DataType::Utf8, false),
        Field::new("fetched_at", DataType::Utf8, false),
    ]));

    let cols: Vec<Arc<dyn Array>> = vec![
        Arc::new(StringArray::from(vec![signal.id.to_string()])),
        Arc::new(StringArray::from(vec![signal.gene_symbol.clone()])),
        Arc::new(Int64Array::from(vec![signal.pathway_count])),
        Arc::new(StringArray::from(vec![signal.source.clone()])),
        Arc::new(StringArray::from(vec![signal.fetched_at.to_rfc3339()])),
    ];
    Ok(RecordBatch::try_new(schema, cols)?)
}

fn record_to_tcga_survival(batch: &RecordBatch, row: usize) -> Result<EntTcgaSurvival> {
    let get_s = |col: &str| -> Result<String> {
        let idx = batch
            .schema()
            .index_of(col)
            .map_err(|e| DbError::Arrow(e.to_string()))?;
        let arr = batch
            .column(idx)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| DbError::Arrow(format!("{col} is not StringArray")))?;
        if arr.is_null(row) {
            Ok(String::new())
        } else {
            Ok(arr.value(row).to_string())
        }
    };
    let get_f = |col: &str| -> Result<f64> {
        let idx = batch
            .schema()
            .index_of(col)
            .map_err(|e| DbError::Arrow(e.to_string()))?;
        let arr = batch
            .column(idx)
            .as_any()
            .downcast_ref::<Float64Array>()
            .ok_or_else(|| DbError::Arrow(format!("{col} is not Float64Array")))?;
        if arr.is_null(row) {
            Ok(0.0)
        } else {
            Ok(arr.value(row))
        }
    };

    let id =
        uuid::Uuid::parse_str(&get_s("id")?).map_err(|e| DbError::InvalidQuery(e.to_string()))?;
    let fetched_at = chrono::DateTime::parse_from_rfc3339(&get_s("fetched_at")?)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());

    Ok(EntTcgaSurvival {
        id,
        gene_symbol: get_s("gene_symbol")?,
        cancer_code: get_s("cancer_code")?,
        tcga_project_id: get_s("tcga_project_id")?,
        survival_score: get_f("survival_score")?,
        source: get_s("source")?,
        fetched_at,
    })
}

fn record_to_gtex_expression(batch: &RecordBatch, row: usize) -> Result<EntGtexExpression> {
    let get_s = |col: &str| -> Result<String> {
        let idx = batch
            .schema()
            .index_of(col)
            .map_err(|e| DbError::Arrow(e.to_string()))?;
        let arr = batch
            .column(idx)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| DbError::Arrow(format!("{col} is not StringArray")))?;
        if arr.is_null(row) {
            Ok(String::new())
        } else {
            Ok(arr.value(row).to_string())
        }
    };
    let get_f = |col: &str| -> Result<f64> {
        let idx = batch
            .schema()
            .index_of(col)
            .map_err(|e| DbError::Arrow(e.to_string()))?;
        let arr = batch
            .column(idx)
            .as_any()
            .downcast_ref::<Float64Array>()
            .ok_or_else(|| DbError::Arrow(format!("{col} is not Float64Array")))?;
        if arr.is_null(row) {
            Ok(0.0)
        } else {
            Ok(arr.value(row))
        }
    };

    let id =
        uuid::Uuid::parse_str(&get_s("id")?).map_err(|e| DbError::InvalidQuery(e.to_string()))?;
    let fetched_at = chrono::DateTime::parse_from_rfc3339(&get_s("fetched_at")?)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());

    Ok(EntGtexExpression {
        id,
        gene_symbol: get_s("gene_symbol")?,
        expression_score: get_f("expression_score")?,
        source: get_s("source")?,
        fetched_at,
    })
}

fn record_to_chembl_target(batch: &RecordBatch, row: usize) -> Result<EntChemblTarget> {
    let get_s = |col: &str| -> Result<String> {
        let idx = batch
            .schema()
            .index_of(col)
            .map_err(|e| DbError::Arrow(e.to_string()))?;
        let arr = batch
            .column(idx)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| DbError::Arrow(format!("{col} is not StringArray")))?;
        if arr.is_null(row) {
            Ok(String::new())
        } else {
            Ok(arr.value(row).to_string())
        }
    };
    let get_i = |col: &str| -> Result<i64> {
        let idx = batch
            .schema()
            .index_of(col)
            .map_err(|e| DbError::Arrow(e.to_string()))?;
        let arr = batch
            .column(idx)
            .as_any()
            .downcast_ref::<Int64Array>()
            .ok_or_else(|| DbError::Arrow(format!("{col} is not Int64Array")))?;
        if arr.is_null(row) {
            Ok(0)
        } else {
            Ok(arr.value(row))
        }
    };

    let id =
        uuid::Uuid::parse_str(&get_s("id")?).map_err(|e| DbError::InvalidQuery(e.to_string()))?;
    let fetched_at = chrono::DateTime::parse_from_rfc3339(&get_s("fetched_at")?)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());

    Ok(EntChemblTarget {
        id,
        gene_symbol: get_s("gene_symbol")?,
        inhibitor_count: get_i("inhibitor_count")?,
        source: get_s("source")?,
        fetched_at,
    })
}

fn record_to_reactome_gene(batch: &RecordBatch, row: usize) -> Result<EntReactomeGene> {
    let get_s = |col: &str| -> Result<String> {
        let idx = batch
            .schema()
            .index_of(col)
            .map_err(|e| DbError::Arrow(e.to_string()))?;
        let arr = batch
            .column(idx)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| DbError::Arrow(format!("{col} is not StringArray")))?;
        if arr.is_null(row) {
            Ok(String::new())
        } else {
            Ok(arr.value(row).to_string())
        }
    };
    let get_i = |col: &str| -> Result<i64> {
        let idx = batch
            .schema()
            .index_of(col)
            .map_err(|e| DbError::Arrow(e.to_string()))?;
        let arr = batch
            .column(idx)
            .as_any()
            .downcast_ref::<Int64Array>()
            .ok_or_else(|| DbError::Arrow(format!("{col} is not Int64Array")))?;
        if arr.is_null(row) {
            Ok(0)
        } else {
            Ok(arr.value(row))
        }
    };

    let id =
        uuid::Uuid::parse_str(&get_s("id")?).map_err(|e| DbError::InvalidQuery(e.to_string()))?;
    let fetched_at = chrono::DateTime::parse_from_rfc3339(&get_s("fetched_at")?)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());

    Ok(EntReactomeGene {
        id,
        gene_symbol: get_s("gene_symbol")?,
        pathway_count: get_i("pathway_count")?,
        source: get_s("source")?,
        fetched_at,
    })
}

fn escape_sql(input: &str) -> String {
    input.replace('\'', "''")
}

fn normalize_symbol(input: &str) -> String {
    input.trim().to_uppercase()
}

fn is_fresh(fetched_at: chrono::DateTime<chrono::Utc>, max_age_days: i64) -> bool {
    if max_age_days <= 0 {
        return true;
    }
    let age = chrono::Utc::now() - fetched_at;
    age.num_days() <= max_age_days
}
