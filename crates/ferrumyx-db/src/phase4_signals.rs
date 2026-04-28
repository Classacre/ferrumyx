//! Phase 4 provider-backed signal cache repository.
//!
//! Stores bounded external enrichments so ranker can avoid repeated API calls.

use crate::database::Database;
use crate::error::{DbError, Result};
use crate::schema::*;
use std::sync::Arc;
use tokio_postgres::Row;

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
        let client = self.db.client();
        let row = client.query_opt(
            "SELECT * FROM ent_tcga_survival WHERE gene_symbol = $1 AND cancer_code = $2 LIMIT 1",
            &[&gene_symbol, &cancer_code],
        ).await?;
        Ok(row.map(row_to_tcga_survival))
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
        let client = self.db.client();
        client.execute(
            "INSERT INTO ent_tcga_survival (id, gene_symbol, cancer_code, tcga_project_id, survival_score, source, fetched_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT (gene_symbol, cancer_code) DO UPDATE SET \
             tcga_project_id = EXCLUDED.tcga_project_id, \
             survival_score = EXCLUDED.survival_score, \
             source = EXCLUDED.source, \
             fetched_at = EXCLUDED.fetched_at",
            &[
                &signal.id, &signal.gene_symbol, &signal.cancer_code,
                &signal.tcga_project_id, &signal.survival_score,
                &signal.source, &signal.fetched_at,
            ],
        ).await?;
        Ok(())
    }

    pub async fn find_cbio_mutation_frequency(
        &self,
        gene_symbol: &str,
        cancer_code: &str,
    ) -> Result<Option<EntCbioMutationFrequency>> {
        let client = self.db.client();
        let row = client.query_opt(
            "SELECT * FROM ent_cbio_mutation_frequency WHERE gene_symbol = $1 AND cancer_code = $2 LIMIT 1",
            &[&gene_symbol, &cancer_code],
        ).await?;
        Ok(row.map(row_to_cbio_mutation_frequency))
    }

    pub async fn find_cbio_mutation_frequency_fresh(
        &self,
        gene_symbol: &str,
        cancer_code: &str,
        max_age_days: i64,
    ) -> Result<Option<EntCbioMutationFrequency>> {
        let found = self.find_cbio_mutation_frequency(gene_symbol, cancer_code).await?;
        Ok(found.filter(|v| is_fresh(v.fetched_at, max_age_days)))
    }

    pub async fn upsert_cbio_mutation_frequency(
        &self,
        signal: &EntCbioMutationFrequency,
    ) -> Result<()> {
        let client = self.db.client();
        client.execute(
            "INSERT INTO ent_cbio_mutation_frequency (id, gene_symbol, cancer_code, study_id, molecular_profile_id, sample_list_id, mutated_sample_count, profiled_sample_count, mutation_frequency, source, fetched_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
             ON CONFLICT (gene_symbol, cancer_code) DO UPDATE SET \
             study_id = EXCLUDED.study_id, molecular_profile_id = EXCLUDED.molecular_profile_id, \
             sample_list_id = EXCLUDED.sample_list_id, mutated_sample_count = EXCLUDED.mutated_sample_count, \
             profiled_sample_count = EXCLUDED.profiled_sample_count, mutation_frequency = EXCLUDED.mutation_frequency, \
             source = EXCLUDED.source, fetched_at = EXCLUDED.fetched_at",
            &[
                &signal.id, &signal.gene_symbol, &signal.cancer_code,
                &signal.study_id, &signal.molecular_profile_id, &signal.sample_list_id,
                &signal.mutated_sample_count, &signal.profiled_sample_count,
                &signal.mutation_frequency, &signal.source, &signal.fetched_at,
            ],
        ).await?;
        Ok(())
    }

    pub async fn find_cosmic_mutation_frequency(
        &self,
        gene_symbol: &str,
        cancer_code: &str,
    ) -> Result<Option<EntCosmicMutationFrequency>> {
        let client = self.db.client();
        let row = client.query_opt(
            "SELECT * FROM ent_cosmic_mutation_frequency WHERE gene_symbol = $1 AND cancer_code = $2 LIMIT 1",
            &[&gene_symbol, &cancer_code],
        ).await?;
        Ok(row.map(row_to_cosmic_mutation_frequency))
    }

    pub async fn find_cosmic_mutation_frequency_fresh(
        &self,
        gene_symbol: &str,
        cancer_code: &str,
        max_age_days: i64,
    ) -> Result<Option<EntCosmicMutationFrequency>> {
        let found = self.find_cosmic_mutation_frequency(gene_symbol, cancer_code).await?;
        Ok(found.filter(|v| is_fresh(v.fetched_at, max_age_days)))
    }

    pub async fn find_cosmic_mutation_frequency_any_cancer(
        &self,
        gene_symbol: &str,
    ) -> Result<Option<EntCosmicMutationFrequency>> {
        let client = self.db.client();
        let rows = client.query(
            "SELECT * FROM ent_cosmic_mutation_frequency WHERE gene_symbol = $1 ORDER BY profiled_sample_count DESC, mutation_frequency DESC LIMIT 1",
            &[&gene_symbol],
        ).await?;
        Ok(rows.into_iter().next().map(row_to_cosmic_mutation_frequency))
    }

    pub async fn upsert_cosmic_mutation_frequency(
        &self,
        signal: &EntCosmicMutationFrequency,
    ) -> Result<()> {
        let client = self.db.client();
        client.execute(
            "INSERT INTO ent_cosmic_mutation_frequency (id, gene_symbol, cancer_code, mutated_sample_count, profiled_sample_count, mutation_frequency, source, fetched_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
             ON CONFLICT (gene_symbol, cancer_code) DO UPDATE SET \
             mutated_sample_count = EXCLUDED.mutated_sample_count, \
             profiled_sample_count = EXCLUDED.profiled_sample_count, \
             mutation_frequency = EXCLUDED.mutation_frequency, \
             source = EXCLUDED.source, fetched_at = EXCLUDED.fetched_at",
            &[
                &signal.id, &signal.gene_symbol, &signal.cancer_code,
                &signal.mutated_sample_count, &signal.profiled_sample_count,
                &signal.mutation_frequency, &signal.source, &signal.fetched_at,
            ],
        ).await?;
        Ok(())
    }

    pub async fn find_gtex_expression(
        &self,
        gene_symbol: &str,
    ) -> Result<Option<EntGtexExpression>> {
        let client = self.db.client();
        let row = client.query_opt(
            "SELECT * FROM ent_gtex_expression WHERE gene_symbol = $1 LIMIT 1",
            &[&gene_symbol],
        ).await?;
        Ok(row.map(row_to_gtex_expression))
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
        let client = self.db.client();
        client.execute(
            "INSERT INTO ent_gtex_expression (id, gene_symbol, expression_score, source, fetched_at) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (gene_symbol) DO UPDATE SET \
             expression_score = EXCLUDED.expression_score, source = EXCLUDED.source, fetched_at = EXCLUDED.fetched_at",
            &[&signal.id, &signal.gene_symbol, &signal.expression_score, &signal.source, &signal.fetched_at],
        ).await?;
        Ok(())
    }

    pub async fn find_chembl_target(&self, gene_symbol: &str) -> Result<Option<EntChemblTarget>> {
        let client = self.db.client();
        let row = client.query_opt(
            "SELECT * FROM ent_chembl_targets WHERE gene_symbol = $1 LIMIT 1",
            &[&gene_symbol],
        ).await?;
        Ok(row.map(row_to_chembl_target))
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
        let client = self.db.client();
        client.execute(
            "INSERT INTO ent_chembl_targets (id, gene_symbol, inhibitor_count, source, fetched_at) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (gene_symbol) DO UPDATE SET \
             inhibitor_count = EXCLUDED.inhibitor_count, source = EXCLUDED.source, fetched_at = EXCLUDED.fetched_at",
            &[&signal.id, &signal.gene_symbol, &signal.inhibitor_count, &signal.source, &signal.fetched_at],
        ).await?;
        Ok(())
    }

    pub async fn find_reactome_gene(&self, gene_symbol: &str) -> Result<Option<EntReactomeGene>> {
        let client = self.db.client();
        let row = client.query_opt(
            "SELECT * FROM ent_reactome_genes WHERE gene_symbol = $1 LIMIT 1",
            &[&gene_symbol],
        ).await?;
        Ok(row.map(row_to_reactome_gene))
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
        let client = self.db.client();
        client.execute(
            "INSERT INTO ent_reactome_genes (id, gene_symbol, pathway_count, source, fetched_at) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (gene_symbol) DO UPDATE SET \
             pathway_count = EXCLUDED.pathway_count, source = EXCLUDED.source, fetched_at = EXCLUDED.fetched_at",
            &[&signal.id, &signal.gene_symbol, &signal.pathway_count, &signal.source, &signal.fetched_at],
        ).await?;
        Ok(())
    }

    pub async fn find_cbio_mutation_frequency_any_cancer(
        &self,
        gene_symbol: &str,
    ) -> Result<Option<EntCbioMutationFrequency>> {
        let client = self.db.client();
        let rows = client.query(
            "SELECT * FROM ent_cbio_mutation_frequency WHERE gene_symbol = $1 ORDER BY profiled_sample_count DESC LIMIT 1",
            &[&gene_symbol],
        ).await?;
        Ok(rows.into_iter().next().map(row_to_cbio_mutation_frequency))
    }

    pub async fn append_provider_refresh_run(&self, run: &EntProviderRefreshRun) -> Result<()> {
        let client = self.db.client();
        client.execute(
            "INSERT INTO ent_provider_refresh_runs (id, provider, started_at, finished_at, genes_requested, genes_processed, attempted, success, failed, skipped, duration_ms, error_rate, cadence_interval_secs, trigger_reason) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
            &[
                &run.id, &run.provider, &run.started_at, &run.finished_at,
                &run.genes_requested, &run.genes_processed, &run.attempted,
                &run.success, &run.failed, &run.skipped, &run.duration_ms,
                &run.error_rate, &run.cadence_interval_secs, &run.trigger_reason,
            ],
        ).await?;
        Ok(())
    }

    pub async fn list_provider_refresh_runs(
        &self,
        provider: &str,
        limit: usize,
    ) -> Result<Vec<EntProviderRefreshRun>> {
        let client = self.db.client();
        let sql = "SELECT * FROM ent_provider_refresh_runs WHERE provider = $1 ORDER BY finished_at DESC LIMIT $2";
        let rows = client.query(sql, &[&provider, &(limit as i64)]).await?;
        let mut rows = rows.into_iter().map(row_to_provider_refresh_run).collect::<Vec<_>>();
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

fn row_to_tcga_survival(row: Row) -> EntTcgaSurvival {
    let id = row.get::<_, uuid::Uuid>("id");
    let gene_symbol = row.get::<_, String>("gene_symbol");
    let cancer_code = row.get::<_, String>("cancer_code");
    let tcga_project_id = row.get::<_, String>("tcga_project_id");
    let survival_score = row.get::<_, f64>("survival_score");
    let source = row.get::<_, String>("source");
    let fetched_at = row.get::<_, chrono::DateTime<chrono::Utc>>("fetched_at");
    EntTcgaSurvival { id, gene_symbol, cancer_code, tcga_project_id, survival_score, source, fetched_at }
}

fn row_to_cbio_mutation_frequency(row: Row) -> EntCbioMutationFrequency {
    EntCbioMutationFrequency {
        id: row.get("id"),
        gene_symbol: row.get("gene_symbol"),
        cancer_code: row.get("cancer_code"),
        study_id: row.get("study_id"),
        molecular_profile_id: row.get("molecular_profile_id"),
        sample_list_id: row.get("sample_list_id"),
        mutated_sample_count: row.get::<_, i64>("mutated_sample_count") as i64,
        profiled_sample_count: row.get::<_, i64>("profiled_sample_count") as i64,
        mutation_frequency: row.get::<_, f64>("mutation_frequency"),
        source: row.get("source"),
        fetched_at: row.get::<_, chrono::DateTime<chrono::Utc>>("fetched_at"),
    }
}

fn row_to_cosmic_mutation_frequency(row: Row) -> EntCosmicMutationFrequency {
    EntCosmicMutationFrequency {
        id: row.get("id"),
        gene_symbol: row.get("gene_symbol"),
        cancer_code: row.get("cancer_code"),
        mutated_sample_count: row.get::<_, i64>("mutated_sample_count") as i64,
        profiled_sample_count: row.get::<_, i64>("profiled_sample_count") as i64,
        mutation_frequency: row.get::<_, f64>("mutation_frequency"),
        source: row.get("source"),
        fetched_at: row.get::<_, chrono::DateTime<chrono::Utc>>("fetched_at"),
    }
}

fn row_to_gtex_expression(row: Row) -> EntGtexExpression {
    EntGtexExpression {
        id: row.get("id"),
        gene_symbol: row.get("gene_symbol"),
        expression_score: row.get::<_, f64>("expression_score"),
        source: row.get("source"),
        fetched_at: row.get::<_, chrono::DateTime<chrono::Utc>>("fetched_at"),
    }
}

fn row_to_chembl_target(row: Row) -> EntChemblTarget {
    EntChemblTarget {
        id: row.get("id"),
        gene_symbol: row.get("gene_symbol"),
        inhibitor_count: row.get::<_, i64>("inhibitor_count") as i64,
        source: row.get("source"),
        fetched_at: row.get::<_, chrono::DateTime<chrono::Utc>>("fetched_at"),
    }
}

fn row_to_reactome_gene(row: Row) -> EntReactomeGene {
    EntReactomeGene {
        id: row.get("id"),
        gene_symbol: row.get("gene_symbol"),
        pathway_count: row.get::<_, i64>("pathway_count") as i64,
        source: row.get("source"),
        fetched_at: row.get::<_, chrono::DateTime<chrono::Utc>>("fetched_at"),
    }
}

fn row_to_provider_refresh_run(row: Row) -> EntProviderRefreshRun {
    EntProviderRefreshRun {
        id: row.get("id"),
        provider: row.get("provider"),
        started_at: row.get::<_, chrono::DateTime<chrono::Utc>>("started_at"),
        finished_at: row.get::<_, chrono::DateTime<chrono::Utc>>("finished_at"),
        genes_requested: row.get::<_, i64>("genes_requested") as i64,
        genes_processed: row.get::<_, i64>("genes_processed") as i64,
        attempted: row.get::<_, i64>("attempted") as i64,
        success: row.get::<_, i64>("success") as i64,
        failed: row.get::<_, i64>("failed") as i64,
        skipped: row.get::<_, i64>("skipped") as i64,
        duration_ms: row.get::<_, i64>("duration_ms") as i64,
        error_rate: row.get::<_, f64>("error_rate"),
        cadence_interval_secs: row.get::<_, i64>("cadence_interval_secs") as i64,
        trigger_reason: row.get("trigger_reason"),
    }
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
