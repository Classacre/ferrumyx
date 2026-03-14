//! Phase 4 provider-backed signal cache repository.
//!
//! Stores bounded external enrichments so ranker can avoid repeated API calls.

use crate::database::Database;
use crate::error::{DbError, Result};
use crate::schema::{
    EntCbioMutationFrequency, EntChemblTarget, EntCosmicMutationFrequency, EntGtexExpression,
    EntProviderRefreshRun, EntReactomeGene, EntTcgaSurvival, TABLE_ENT_CBIO_MUTATION_FREQUENCY,
    TABLE_ENT_CHEMBL_TARGETS, TABLE_ENT_COSMIC_MUTATION_FREQUENCY, TABLE_ENT_GTEX_EXPRESSION,
    TABLE_ENT_PROVIDER_REFRESH_RUNS, TABLE_ENT_REACTOME_GENES, TABLE_ENT_TCGA_SURVIVAL,
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

    pub async fn find_cbio_mutation_frequency(
        &self,
        gene_symbol: &str,
        cancer_code: &str,
    ) -> Result<Option<EntCbioMutationFrequency>> {
        let gene = normalize_symbol(gene_symbol);
        let cancer = normalize_symbol(cancer_code);
        if gene.is_empty() || cancer.is_empty() {
            return Ok(None);
        }

        let table = self
            .db
            .connection()
            .open_table(TABLE_ENT_CBIO_MUTATION_FREQUENCY)
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
                return Ok(Some(record_to_cbio_mutation_frequency(&batch, 0)?));
            }
        }
        Ok(None)
    }

    pub async fn find_cbio_mutation_frequency_fresh(
        &self,
        gene_symbol: &str,
        cancer_code: &str,
        max_age_days: i64,
    ) -> Result<Option<EntCbioMutationFrequency>> {
        let found = self
            .find_cbio_mutation_frequency(gene_symbol, cancer_code)
            .await?;
        Ok(found.filter(|v| is_fresh(v.fetched_at, max_age_days)))
    }

    pub async fn upsert_cbio_mutation_frequency(
        &self,
        signal: &EntCbioMutationFrequency,
    ) -> Result<()> {
        let table = self
            .db
            .connection()
            .open_table(TABLE_ENT_CBIO_MUTATION_FREQUENCY)
            .execute()
            .await?;
        let record = cbio_mutation_frequency_to_record(signal)?;
        let schema = record.schema();
        let iter = arrow_array::RecordBatchIterator::new(vec![Ok(record)], schema);
        let mut builder = table.merge_insert(&["gene_symbol", "cancer_code"]);
        builder.when_matched_update_all(None);
        builder.execute(Box::new(iter)).await?;
        Ok(())
    }

    pub async fn find_cosmic_mutation_frequency(
        &self,
        gene_symbol: &str,
        cancer_code: &str,
    ) -> Result<Option<EntCosmicMutationFrequency>> {
        let gene = normalize_symbol(gene_symbol);
        let cancer = normalize_symbol(cancer_code);
        if gene.is_empty() || cancer.is_empty() {
            return Ok(None);
        }

        let table = self
            .db
            .connection()
            .open_table(TABLE_ENT_COSMIC_MUTATION_FREQUENCY)
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
                return Ok(Some(record_to_cosmic_mutation_frequency(&batch, 0)?));
            }
        }
        Ok(None)
    }

    pub async fn find_cosmic_mutation_frequency_fresh(
        &self,
        gene_symbol: &str,
        cancer_code: &str,
        max_age_days: i64,
    ) -> Result<Option<EntCosmicMutationFrequency>> {
        let found = self
            .find_cosmic_mutation_frequency(gene_symbol, cancer_code)
            .await?;
        Ok(found.filter(|v| is_fresh(v.fetched_at, max_age_days)))
    }

    pub async fn find_cosmic_mutation_frequency_any_cancer(
        &self,
        gene_symbol: &str,
    ) -> Result<Option<EntCosmicMutationFrequency>> {
        let gene = normalize_symbol(gene_symbol);
        if gene.is_empty() {
            return Ok(None);
        }

        let table = self
            .db
            .connection()
            .open_table(TABLE_ENT_COSMIC_MUTATION_FREQUENCY)
            .execute()
            .await?;
        let filter = format!("gene_symbol = '{}'", escape_sql(&gene));
        let mut stream = table.query().only_if(&filter).limit(64).execute().await?;

        let mut best: Option<EntCosmicMutationFrequency> = None;
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for row in 0..batch.num_rows() {
                let candidate = record_to_cosmic_mutation_frequency(&batch, row)?;
                best = Some(match best.take() {
                    Some(cur) => {
                        if candidate.profiled_sample_count > cur.profiled_sample_count
                            || (candidate.profiled_sample_count == cur.profiled_sample_count
                                && candidate.mutation_frequency > cur.mutation_frequency)
                        {
                            candidate
                        } else {
                            cur
                        }
                    }
                    None => candidate,
                });
            }
        }
        Ok(best)
    }

    pub async fn upsert_cosmic_mutation_frequency(
        &self,
        signal: &EntCosmicMutationFrequency,
    ) -> Result<()> {
        let table = self
            .db
            .connection()
            .open_table(TABLE_ENT_COSMIC_MUTATION_FREQUENCY)
            .execute()
            .await?;
        let record = cosmic_mutation_frequency_to_record(signal)?;
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

    pub async fn find_cbio_mutation_frequency_any_cancer(
        &self,
        gene_symbol: &str,
    ) -> Result<Option<EntCbioMutationFrequency>> {
        let gene = normalize_symbol(gene_symbol);
        if gene.is_empty() {
            return Ok(None);
        }

        let table = self
            .db
            .connection()
            .open_table(TABLE_ENT_CBIO_MUTATION_FREQUENCY)
            .execute()
            .await?;
        let filter = format!("gene_symbol = '{}'", escape_sql(&gene));
        let mut stream = table.query().only_if(&filter).limit(64).execute().await?;

        let mut best: Option<EntCbioMutationFrequency> = None;
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for row in 0..batch.num_rows() {
                let candidate = record_to_cbio_mutation_frequency(&batch, row)?;
                best = Some(match best.take() {
                    Some(cur) => {
                        if candidate.profiled_sample_count > cur.profiled_sample_count
                            || (candidate.profiled_sample_count == cur.profiled_sample_count
                                && candidate.mutation_frequency > cur.mutation_frequency)
                        {
                            candidate
                        } else {
                            cur
                        }
                    }
                    None => candidate,
                });
            }
        }
        Ok(best)
    }

    pub async fn append_provider_refresh_run(&self, run: &EntProviderRefreshRun) -> Result<()> {
        let table = self
            .db
            .connection()
            .open_table(TABLE_ENT_PROVIDER_REFRESH_RUNS)
            .execute()
            .await?;
        let record = provider_refresh_run_to_record(run)?;
        let schema = record.schema();
        let iter = arrow_array::RecordBatchIterator::new(vec![Ok(record)], schema);
        table.add(Box::new(iter)).execute().await?;
        Ok(())
    }

    pub async fn list_provider_refresh_runs(
        &self,
        provider: &str,
        limit: usize,
    ) -> Result<Vec<EntProviderRefreshRun>> {
        let provider_norm = provider.trim().to_lowercase();
        if provider_norm.is_empty() {
            return Ok(Vec::new());
        }

        let table = self
            .db
            .connection()
            .open_table(TABLE_ENT_PROVIDER_REFRESH_RUNS)
            .execute()
            .await?;
        let filter = format!("provider = '{}'", escape_sql(&provider_norm));
        let mut stream = table
            .query()
            .only_if(&filter)
            .limit(limit.clamp(1, 256))
            .execute()
            .await?;

        let mut rows = Vec::new();
        while let Some(batch) = stream.next().await {
            let batch = batch?;
            for row in 0..batch.num_rows() {
                rows.push(record_to_provider_refresh_run(&batch, row)?);
            }
        }
        rows.sort_by(|a, b| b.finished_at.cmp(&a.finished_at));
        Ok(rows)
    }

    pub async fn latest_provider_refresh_run(
        &self,
        provider: &str,
    ) -> Result<Option<EntProviderRefreshRun>> {
        let rows = self.list_provider_refresh_runs(provider, 32).await?;
        Ok(rows.into_iter().next())
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

fn cbio_mutation_frequency_to_record(signal: &EntCbioMutationFrequency) -> Result<RecordBatch> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("gene_symbol", DataType::Utf8, false),
        Field::new("cancer_code", DataType::Utf8, false),
        Field::new("study_id", DataType::Utf8, false),
        Field::new("molecular_profile_id", DataType::Utf8, false),
        Field::new("sample_list_id", DataType::Utf8, false),
        Field::new("mutated_sample_count", DataType::Int64, false),
        Field::new("profiled_sample_count", DataType::Int64, false),
        Field::new("mutation_frequency", DataType::Float64, false),
        Field::new("source", DataType::Utf8, false),
        Field::new("fetched_at", DataType::Utf8, false),
    ]));

    let cols: Vec<Arc<dyn Array>> = vec![
        Arc::new(StringArray::from(vec![signal.id.to_string()])),
        Arc::new(StringArray::from(vec![signal.gene_symbol.clone()])),
        Arc::new(StringArray::from(vec![signal.cancer_code.clone()])),
        Arc::new(StringArray::from(vec![signal.study_id.clone()])),
        Arc::new(StringArray::from(vec![signal.molecular_profile_id.clone()])),
        Arc::new(StringArray::from(vec![signal.sample_list_id.clone()])),
        Arc::new(Int64Array::from(vec![signal.mutated_sample_count])),
        Arc::new(Int64Array::from(vec![signal.profiled_sample_count])),
        Arc::new(Float64Array::from(vec![signal.mutation_frequency])),
        Arc::new(StringArray::from(vec![signal.source.clone()])),
        Arc::new(StringArray::from(vec![signal.fetched_at.to_rfc3339()])),
    ];
    Ok(RecordBatch::try_new(schema, cols)?)
}

fn cosmic_mutation_frequency_to_record(signal: &EntCosmicMutationFrequency) -> Result<RecordBatch> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("gene_symbol", DataType::Utf8, false),
        Field::new("cancer_code", DataType::Utf8, false),
        Field::new("mutated_sample_count", DataType::Int64, false),
        Field::new("profiled_sample_count", DataType::Int64, false),
        Field::new("mutation_frequency", DataType::Float64, false),
        Field::new("source", DataType::Utf8, false),
        Field::new("fetched_at", DataType::Utf8, false),
    ]));

    let cols: Vec<Arc<dyn Array>> = vec![
        Arc::new(StringArray::from(vec![signal.id.to_string()])),
        Arc::new(StringArray::from(vec![signal.gene_symbol.clone()])),
        Arc::new(StringArray::from(vec![signal.cancer_code.clone()])),
        Arc::new(Int64Array::from(vec![signal.mutated_sample_count])),
        Arc::new(Int64Array::from(vec![signal.profiled_sample_count])),
        Arc::new(Float64Array::from(vec![signal.mutation_frequency])),
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

fn provider_refresh_run_to_record(signal: &EntProviderRefreshRun) -> Result<RecordBatch> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("provider", DataType::Utf8, false),
        Field::new("started_at", DataType::Utf8, false),
        Field::new("finished_at", DataType::Utf8, false),
        Field::new("genes_requested", DataType::Int64, false),
        Field::new("genes_processed", DataType::Int64, false),
        Field::new("attempted", DataType::Int64, false),
        Field::new("success", DataType::Int64, false),
        Field::new("failed", DataType::Int64, false),
        Field::new("skipped", DataType::Int64, false),
        Field::new("duration_ms", DataType::Int64, false),
        Field::new("error_rate", DataType::Float64, false),
        Field::new("cadence_interval_secs", DataType::Int64, false),
        Field::new("trigger_reason", DataType::Utf8, false),
    ]));

    let cols: Vec<Arc<dyn Array>> = vec![
        Arc::new(StringArray::from(vec![signal.id.to_string()])),
        Arc::new(StringArray::from(vec![signal.provider.to_lowercase()])),
        Arc::new(StringArray::from(vec![signal.started_at.to_rfc3339()])),
        Arc::new(StringArray::from(vec![signal.finished_at.to_rfc3339()])),
        Arc::new(Int64Array::from(vec![signal.genes_requested])),
        Arc::new(Int64Array::from(vec![signal.genes_processed])),
        Arc::new(Int64Array::from(vec![signal.attempted])),
        Arc::new(Int64Array::from(vec![signal.success])),
        Arc::new(Int64Array::from(vec![signal.failed])),
        Arc::new(Int64Array::from(vec![signal.skipped])),
        Arc::new(Int64Array::from(vec![signal.duration_ms])),
        Arc::new(Float64Array::from(vec![signal.error_rate])),
        Arc::new(Int64Array::from(vec![signal.cadence_interval_secs])),
        Arc::new(StringArray::from(vec![signal.trigger_reason.clone()])),
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

fn record_to_cbio_mutation_frequency(
    batch: &RecordBatch,
    row: usize,
) -> Result<EntCbioMutationFrequency> {
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

    Ok(EntCbioMutationFrequency {
        id,
        gene_symbol: get_s("gene_symbol")?,
        cancer_code: get_s("cancer_code")?,
        study_id: get_s("study_id")?,
        molecular_profile_id: get_s("molecular_profile_id")?,
        sample_list_id: get_s("sample_list_id")?,
        mutated_sample_count: get_i("mutated_sample_count")?,
        profiled_sample_count: get_i("profiled_sample_count")?,
        mutation_frequency: get_f("mutation_frequency")?,
        source: get_s("source")?,
        fetched_at,
    })
}

fn record_to_cosmic_mutation_frequency(
    batch: &RecordBatch,
    row: usize,
) -> Result<EntCosmicMutationFrequency> {
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

    Ok(EntCosmicMutationFrequency {
        id,
        gene_symbol: get_s("gene_symbol")?,
        cancer_code: get_s("cancer_code")?,
        mutated_sample_count: get_i("mutated_sample_count")?,
        profiled_sample_count: get_i("profiled_sample_count")?,
        mutation_frequency: get_f("mutation_frequency")?,
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

fn record_to_provider_refresh_run(
    batch: &RecordBatch,
    row: usize,
) -> Result<EntProviderRefreshRun> {
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
    let started_at = chrono::DateTime::parse_from_rfc3339(&get_s("started_at")?)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());
    let finished_at = chrono::DateTime::parse_from_rfc3339(&get_s("finished_at")?)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());

    Ok(EntProviderRefreshRun {
        id,
        provider: get_s("provider")?.to_lowercase(),
        started_at,
        finished_at,
        genes_requested: get_i("genes_requested")?,
        genes_processed: get_i("genes_processed")?,
        attempted: get_i("attempted")?,
        success: get_i("success")?,
        failed: get_i("failed")?,
        skipped: get_i("skipped")?,
        duration_ms: get_i("duration_ms")?,
        error_rate: get_f("error_rate")?,
        cadence_interval_secs: get_i("cadence_interval_secs")?,
        trigger_reason: get_s("trigger_reason")?,
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
