//! ferrumyx-ranker — Target prioritization scoring engine.
//! Implements Phase 4 of ARCHITECTURE.md.

pub mod providers;
pub mod scorer;
pub mod normalise;
pub mod depmap_provider;
pub mod gtex_provider;
pub mod tcga_provider;
pub mod weights;

use std::sync::Arc;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use ferrumyx_db::Database;
use ferrumyx_db::papers::PaperRepository;
use ferrumyx_db::kg_facts::KgFactRepository;
use ferrumyx_db::kg_conflicts::KgConflictRepository;
use ferrumyx_db::target_scores::TargetScoreRepository;
use ferrumyx_db::{
    EntChemblTarget, EntGtexExpression, EntReactomeGene, EntStageRepository, EntTcgaSurvival,
    Phase4SignalRepository,
};
use ferrumyx_common::query::{QueryRequest, QueryResult, TargetMetrics};
use ferrumyx_ingestion::sources::ChemblClient;
use ferrumyx_ingestion::sources::DepMapCache;
use ferrumyx_ingestion::sources::GtexClient;
use ferrumyx_ingestion::sources::TcgaClient;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;

const PROVIDER_SIGNAL_TTL_DAYS: i64 = 14;

pub struct TargetQueryEngine {
    db: Arc<Database>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRefreshRequest {
    pub genes: Vec<String>,
    pub cancer_code: Option<String>,
    pub max_genes: usize,
    pub batch_size: usize,
    pub retries: u8,
}

impl Default for ProviderRefreshRequest {
    fn default() -> Self {
        Self {
            genes: Vec::new(),
            cancer_code: None,
            max_genes: 24,
            batch_size: 6,
            retries: 1,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderRefreshReport {
    pub genes_requested: usize,
    pub genes_processed: usize,
    pub gtex_attempted: usize,
    pub gtex_success: usize,
    pub gtex_failed: usize,
    pub tcga_attempted: usize,
    pub tcga_success: usize,
    pub tcga_failed: usize,
    pub tcga_skipped: usize,
    pub chembl_attempted: usize,
    pub chembl_success: usize,
    pub chembl_failed: usize,
    pub reactome_attempted: usize,
    pub reactome_success: usize,
    pub reactome_failed: usize,
    pub duration_ms: u64,
}

impl TargetQueryEngine {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn execute_query(&self, req: QueryRequest) -> anyhow::Result<Vec<QueryResult>> {
        let t0 = Instant::now();
        let kg_repo = KgFactRepository::new(self.db.clone());
        let conflict_repo = KgConflictRepository::new(self.db.clone());
        let score_repo = TargetScoreRepository::new(self.db.clone());

        // Fetch a bounded cohort of facts, then rank unique gene candidates.
        let fact_limit = (req.max_results.saturating_mul(250)).clamp(200, 5000);
        let facts = kg_repo.list(0, fact_limit).await.unwrap_or_default();
        let t_facts = Instant::now();
        let gene_filter = req.gene_symbol.as_deref().unwrap_or("").to_lowercase();
        let cancer_filter = req.cancer_code.as_deref().unwrap_or("").to_lowercase();
        let fact_ids: Vec<_> = facts.iter().map(|f| f.id).collect();
        let conflicts = conflict_repo
            .find_by_fact_ids(&fact_ids)
            .await
            .unwrap_or_default();
        let t_conflicts = Instant::now();
        let mut conflicts_by_fact: HashMap<uuid::Uuid, Vec<_>> = HashMap::new();
        for c in conflicts {
            conflicts_by_fact.entry(c.fact_a_id).or_default().push(c.clone());
            conflicts_by_fact.entry(c.fact_b_id).or_default().push(c);
        }

        let mut candidates: HashMap<uuid::Uuid, GeneCandidate> = HashMap::new();

        for f in facts {
            if f.predicate.eq_ignore_ascii_case("mentions") || !is_gene_like(&f.subject_name) {
                continue;
            }

            if !gene_filter.is_empty() && !f.subject_name.to_lowercase().contains(&gene_filter) {
                continue;
            }

            if !cancer_filter.is_empty() {
                let has_cancer_context = f.subject_name.to_lowercase().contains(&cancer_filter)
                    || f.object_name.to_lowercase().contains(&cancer_filter)
                    || f.predicate.to_lowercase().contains(&cancer_filter);
                if !has_cancer_context {
                    continue;
                }
            }

            let conflicts = conflicts_by_fact.remove(&f.id).unwrap_or_default();
            let mut include = true;
            let mut confidence_adj = f.confidence as f64;
            let mut disputed = false;

            for conflict in conflicts {
                let net = conflict.net_confidence as f64;
                if net < 0.30 {
                    include = false;
                    break;
                } else if net <= 0.60 {
                    disputed = true;
                    confidence_adj *= 0.70;
                }
            }

            if !include {
                continue;
            }

            let entry = candidates
                .entry(f.subject_id)
                .or_insert_with(|| GeneCandidate::new(f.subject_name.clone()));

            if disputed {
                entry.flags.insert("DISPUTED".to_string());
            }

            entry.fact_count += 1;
            entry.confidence_sum += confidence_adj.clamp(0.0, 1.0);
            entry.confidence_n += 1;
            entry.paper_ids.insert(f.paper_id);

            let predicate_lc = f.predicate.to_lowercase();
            if predicate_lc.contains("mutation") || predicate_lc == "has_mutation" {
                entry.mutation_mentions += 1;
            }
            if predicate_lc.contains("survival")
                || predicate_lc.contains("hazard")
                || predicate_lc.contains("mortality")
                || predicate_lc.contains("prognosis")
                || predicate_lc.contains("overall_survival")
                || predicate_lc.contains("progression_free")
            {
                entry.survival_mentions += 1;
                let survival_ctx = format!(
                    "{} {} {}",
                    f.subject_name.to_lowercase(),
                    f.object_name.to_lowercase(),
                    f.evidence.as_deref().unwrap_or("").to_lowercase()
                );
                if survival_ctx.contains("poor")
                    || survival_ctx.contains("worse")
                    || survival_ctx.contains("decreased")
                    || survival_ctx.contains("shorter")
                    || survival_ctx.contains("reduced")
                {
                    entry.survival_negative += 1;
                } else if survival_ctx.contains("improved")
                    || survival_ctx.contains("better")
                    || survival_ctx.contains("longer")
                    || survival_ctx.contains("favorable")
                    || survival_ctx.contains("increased")
                {
                    entry.survival_positive += 1;
                }
            }
            if predicate_lc.contains("expression")
                || predicate_lc.contains("overexpress")
                || predicate_lc.contains("underexpress")
                || predicate_lc.contains("upregulat")
                || predicate_lc.contains("downregulat")
            {
                let expr_ctx = format!(
                    "{} {} {}",
                    f.subject_name.to_lowercase(),
                    f.object_name.to_lowercase(),
                    f.evidence.as_deref().unwrap_or("").to_lowercase()
                );
                if expr_ctx.contains("normal")
                    || expr_ctx.contains("healthy")
                    || expr_ctx.contains("non-tumor")
                    || expr_ctx.contains("adjacent")
                {
                    entry.expression_normal_mentions += 1;
                } else {
                    entry.expression_tumor_mentions += 1;
                }
            }
            if predicate_lc.contains("inhibit")
                || predicate_lc.contains("bind")
                || predicate_lc.contains("target")
                || predicate_lc.contains("antagon")
            {
                entry.inhibitor_mentions += 1;
            }
            if predicate_lc.contains("pathway") || predicate_lc.contains("reactome") {
                entry.pathway_mentions += 1;
            }
            if predicate_lc.contains("pdb")
                || predicate_lc.contains("alphafold")
                || predicate_lc.contains("structure")
            {
                entry.structural_mentions += 1;
            }
            if predicate_lc.contains("pocket") || predicate_lc.contains("drugg") {
                entry.pocket_mentions += 1;
            }

            if is_cancer_like(&f.object_name) {
                entry.cancer_mentions += 1;
                *entry
                    .cancer_codes
                    .entry(f.object_name.to_uppercase())
                    .or_insert(0) += 1;
            }

            if let Some(sample_size) = f.sample_size {
                if sample_size > 0 {
                    entry.sample_size_sum += sample_size as f64;
                    entry.sample_obs += 1;
                }
            }
        }

        let gene_ids: Vec<uuid::Uuid> = candidates.keys().copied().collect();
        let persisted_scores = score_repo
            .find_current_by_gene_ids(&gene_ids, 150)
            .await
            .unwrap_or_default();
        let t_scores = Instant::now();
        let mut score_map_gene: HashMap<uuid::Uuid, f64> = HashMap::new();
        for s in persisted_scores {
            score_map_gene
                .entry(s.gene_id)
                .and_modify(|v| *v = v.max(s.confidence_adjusted_score))
                .or_insert(s.confidence_adjusted_score);
        }

        let ent_repo = EntStageRepository::new(self.db.clone());
        let signal_repo = Phase4SignalRepository::new(self.db.clone());
        let paper_repo = PaperRepository::new(self.db.clone());
        let symbol_list: Vec<String> = candidates
            .values()
            .map(|c| c.gene_symbol.clone())
            .collect();
        let enrichment_by_symbol = ent_repo
            .get_enrichment_by_symbol(&symbol_list)
            .await
            .unwrap_or_default();
        let all_paper_ids: Vec<uuid::Uuid> = candidates
            .values()
            .flat_map(|c| c.paper_ids.iter().copied())
            .collect();
        let paper_published_at = paper_repo
            .find_published_at_by_ids(&all_paper_ids)
            .await
            .unwrap_or_default();
        let t_enrich = Instant::now();

        let mut cohort_metrics = Vec::new();
        let mut by_gene_metrics: HashMap<uuid::Uuid, TargetMetrics> = HashMap::new();
        let mut component_sources_by_gene: HashMap<uuid::Uuid, BTreeMap<String, String>> =
            HashMap::new();
        for (gene_id, candidate) in &candidates {
            let mut metrics = candidate.to_target_metrics();
            let mut component_sources = default_component_sources();
            if let Some(enrich) = enrichment_by_symbol.get(&candidate.gene_symbol.to_uppercase()) {
                if enrich.mutation_count > 0 {
                    let source_mutation = (enrich.mutation_count as f64 / 20.0).clamp(0.0, 1.0);
                    metrics.mutation_freq = metrics.mutation_freq.max(source_mutation);
                    component_sources.insert("n1_mutation_freq".to_string(), "ent_stage".to_string());
                }
                if enrich.pdb_structure_count > 0 {
                    metrics.pdb_structure_count =
                        metrics.pdb_structure_count.max(enrich.pdb_structure_count);
                    component_sources.insert(
                        "n5_structural_tractability".to_string(),
                        "ent_stage".to_string(),
                    );
                }
                if let Some(plddt) = enrich.af_plddt_mean {
                    metrics.af_plddt_mean = metrics.af_plddt_mean.max(plddt);
                    component_sources.insert(
                        "n5_structural_tractability".to_string(),
                        "ent_stage".to_string(),
                    );
                }
                if let Some(fpocket) = enrich.fpocket_best_score {
                    metrics.fpocket_best_score = metrics.fpocket_best_score.max(fpocket);
                    component_sources.insert(
                        "n6_pocket_detectability".to_string(),
                        "ent_stage".to_string(),
                    );
                }
                if enrich.chembl_inhibitor_count > 0 {
                    metrics.chembl_inhibitor_count = metrics
                        .chembl_inhibitor_count
                        .max(enrich.chembl_inhibitor_count);
                    component_sources
                        .insert("n7_novelty_score".to_string(), "ent_stage".to_string());
                }
                if enrich.pathway_count > 0 {
                    metrics.reactome_escape_pathway_count = metrics
                        .reactome_escape_pathway_count
                        .max(enrich.pathway_count);
                    component_sources.insert(
                        "n8_pathway_independence".to_string(),
                        "ent_stage".to_string(),
                    );
                }
            }

            let inferred_cancer = candidate
                .infer_cancer_code()
                .or_else(|| req.cancer_code.clone())
                .unwrap_or_else(|| "UNK".to_string());

            if let Some(novelty) = candidate.source_backed_literature_novelty(&paper_published_at) {
                metrics.literature_novelty_velocity = novelty;
                component_sources.insert(
                    "n9_literature_novelty".to_string(),
                    "papers_metadata".to_string(),
                );
            }
            if candidate.survival_mentions > 0 {
                component_sources.insert(
                    "n3_survival_correlation".to_string(),
                    "kg_fact_semantic".to_string(),
                );
            }
            if candidate.expression_tumor_mentions + candidate.expression_normal_mentions > 0 {
                component_sources.insert(
                    "n4_expression_specificity".to_string(),
                    "kg_fact_semantic".to_string(),
                );
            }

            if let Some(depmap) = depmap_cache() {
                if let Some(ceres) = depmap.get_mean_ceres(&candidate.gene_symbol, &inferred_cancer) {
                    metrics.crispr_dependency = ceres;
                    component_sources.insert(
                        "n2_crispr_dependency".to_string(),
                        "depmap_cache".to_string(),
                    );
                }
            }

            // TCGA-backed survival proxy is enabled only for targeted/small cohorts
            // so we avoid broad-query network fanout.
            if should_fetch_tcga(candidates.len(), &req) {
                if let Some(tcga_survival_score) = get_cached_tcga_survival_score(
                    &signal_repo,
                    &candidate.gene_symbol,
                    &inferred_cancer,
                )
                .await
                {
                    metrics.survival_correlation = tcga_survival_score;
                    component_sources.insert(
                        "n3_survival_correlation".to_string(),
                        "tcga_table".to_string(),
                    );
                }
            }

            // GTEx-backed expression proxy is enabled only for targeted/small cohorts
            // to keep broad ranker queries fast and predictable.
            if should_fetch_gtex(candidates.len(), &req) {
                if let Some(gtex_expr_score) =
                    get_cached_gtex_expression_score(&signal_repo, &candidate.gene_symbol).await
                {
                    metrics.expression_specificity =
                        (1.0 + 4.0 * gtex_expr_score).clamp(0.5, 5.0);
                    component_sources.insert(
                        "n4_expression_specificity".to_string(),
                        "gtex_table".to_string(),
                    );
                }
            }

            if should_fetch_chembl(candidates.len(), &req) {
                if let Some(chembl_count) =
                    get_cached_chembl_inhibitor_count(&signal_repo, &candidate.gene_symbol).await
                {
                    metrics.chembl_inhibitor_count =
                        metrics.chembl_inhibitor_count.max(chembl_count);
                    component_sources.insert("n7_novelty_score".to_string(), "chembl_table".to_string());
                }
            }

            if should_fetch_reactome(candidates.len(), &req) {
                if let Some(reactome_count) =
                    get_cached_reactome_pathway_count(&signal_repo, &candidate.gene_symbol).await
                {
                    metrics.reactome_escape_pathway_count = metrics
                        .reactome_escape_pathway_count
                        .max(reactome_count);
                    component_sources.insert(
                        "n8_pathway_independence".to_string(),
                        "reactome_table".to_string(),
                    );
                }
            }

            cohort_metrics.push((*gene_id, metrics.clone()));
            by_gene_metrics.insert(*gene_id, metrics);
            component_sources_by_gene.insert(*gene_id, component_sources);
        }

        let cohort_scores = scorer::PrioritizationEngine::calculate_scores(&cohort_metrics);

        let mut results = Vec::new();

        for (gene_id, candidate) in &candidates {
            if let Some(score_res) = cohort_scores.get(gene_id) {
                let Some(metrics) = by_gene_metrics.get(gene_id) else {
                    continue;
                };

                let inferred_cancer = candidate
                    .infer_cancer_code()
                    .or_else(|| req.cancer_code.clone())
                    .unwrap_or_else(|| "UNK".to_string());

                let persisted_score = score_map_gene.get(gene_id).copied();
                let effective_score = persisted_score.unwrap_or(score_res.composite_score);
                let confidence_adj = (effective_score * candidate.mean_confidence()).clamp(0.0, 1.0);

                let penalties = scorer::PenaltyInputs {
                    chembl_inhibitor_count: metrics.chembl_inhibitor_count,
                    expression_ratio: metrics.expression_specificity,
                    has_pdb: metrics.pdb_structure_count > 0,
                    alphafold_plddt: Some(metrics.af_plddt_mean),
                };

                let tier = scorer::determine_shortlist_tier(
                    confidence_adj,
                    Some(metrics.mutation_freq),
                    score_res.n5_structural_tractability,
                    &penalties,
                    score_res.n7_novelty_score,
                );

                let shortlist_tier = match tier {
                    scorer::ShortlistTier::Primary => "primary".to_string(),
                    scorer::ShortlistTier::Secondary => "secondary".to_string(),
                    scorer::ShortlistTier::Excluded => "excluded".to_string(),
                };

                let mut flags: Vec<String> = candidate.flags.iter().cloned().collect();
                if metrics.expression_specificity < 1.20 {
                    flags.push("WARNING_LOW_TUMOR_SPECIFICITY".to_string());
                }
                if metrics.pdb_structure_count == 0 && metrics.af_plddt_mean < 50.0 {
                    flags.push("WARNING_STRUCTURALLY_UNRESOLVED".to_string());
                }
                if penalties.chembl_inhibitor_count > 50 && score_res.n7_novelty_score < 0.20 {
                    flags.push("HARD_EXCLUSION_SATURATED_TARGET".to_string());
                }
                if !enrichment_by_symbol.contains_key(&candidate.gene_symbol.to_uppercase()) {
                    flags.push("COVERAGE_PROXY_ONLY".to_string());
                }

                results.push(QueryResult {
                    rank: 0,
                    percentile: None,
                    gene_symbol: candidate.gene_symbol.clone(),
                    cancer_code: inferred_cancer,
                    composite_score: effective_score,
                    confidence_adj,
                    shortlist_tier,
                    flags,
                    component_sources: component_sources_by_gene.get(gene_id).cloned(),
                    component_breakdown: Some(score_res.clone()),
                    metrics: Some(metrics.clone()),
                });
            }
        }

        results.sort_by(|a, b| {
            b.composite_score
                .partial_cmp(&a.composite_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    b.confidence_adj
                        .partial_cmp(&a.confidence_adj)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });

        let total = results.len().max(1) as f64;
        for (i, res) in results.iter_mut().enumerate() {
            res.rank = i + 1;
            let p = 100.0 * (1.0 - (i as f64 / total));
            res.percentile = Some(p.clamp(0.0, 100.0));
        }
        results.truncate(req.max_results);

        if should_prewarm_large_cohort(candidates.len()) {
            let prewarm_genes: Vec<String> = results
                .iter()
                .take(20)
                .map(|r| r.gene_symbol.trim().to_uppercase())
                .filter(|s| !s.is_empty())
                .collect();
            if !prewarm_genes.is_empty() {
                let db = self.db.clone();
                let cancer_code = req.cancer_code.clone();
                tokio::spawn(async move {
                    prewarm_phase4_provider_signals(db, prewarm_genes, cancer_code).await;
                });
            }
        }

        info!(
            target: "ferrumyx_ranker_perf",
            facts_ms = (t_facts - t0).as_millis() as u64,
            conflicts_ms = (t_conflicts - t_facts).as_millis() as u64,
            scores_ms = (t_scores - t_conflicts).as_millis() as u64,
            enrich_ms = (t_enrich - t_scores).as_millis() as u64,
            rank_ms = (Instant::now() - t_enrich).as_millis() as u64,
            total_ms = (Instant::now() - t0).as_millis() as u64,
            candidate_count = candidates.len(),
            result_count = results.len(),
            "ranker query complete"
        );

        Ok(results)
    }

    pub async fn refresh_provider_signals(
        &self,
        mut request: ProviderRefreshRequest,
    ) -> anyhow::Result<ProviderRefreshReport> {
        let started = Instant::now();
        let mut uniq = HashSet::new();
        let max_genes = request.max_genes.clamp(1, 200);
        let batch_size = request.batch_size.clamp(1, 32);
        let retries = request.retries.min(3);

        let genes: Vec<String> = request
            .genes
            .drain(..)
            .map(|g| g.trim().to_uppercase())
            .filter(|g| !g.is_empty())
            .filter(|g| uniq.insert(g.clone()))
            .take(max_genes)
            .collect();

        let mut report = ProviderRefreshReport {
            genes_requested: genes.len(),
            ..ProviderRefreshReport::default()
        };

        if genes.is_empty() {
            report.duration_ms = started.elapsed().as_millis() as u64;
            return Ok(report);
        }

        let signal_repo = Phase4SignalRepository::new(self.db.clone());
        let cancer_code = request.cancer_code.map(|c| c.trim().to_uppercase());

        for batch in genes.chunks(batch_size) {
            for gene in batch {
                report.genes_processed += 1;

                report.gtex_attempted += 1;
                if retry_fetch_f64(retries, || get_cached_gtex_expression_score(&signal_repo, gene)).await {
                    report.gtex_success += 1;
                } else {
                    report.gtex_failed += 1;
                }

                report.chembl_attempted += 1;
                if retry_fetch_u32(retries, || {
                    get_cached_chembl_inhibitor_count(&signal_repo, gene)
                })
                .await
                {
                    report.chembl_success += 1;
                } else {
                    report.chembl_failed += 1;
                }

                report.reactome_attempted += 1;
                if retry_fetch_u32(retries, || {
                    get_cached_reactome_pathway_count(&signal_repo, gene)
                })
                .await
                {
                    report.reactome_success += 1;
                } else {
                    report.reactome_failed += 1;
                }

                if let Some(cc) = cancer_code.as_deref() {
                    report.tcga_attempted += 1;
                    if retry_fetch_f64(retries, || {
                        get_cached_tcga_survival_score(&signal_repo, gene, cc)
                    })
                    .await
                    {
                        report.tcga_success += 1;
                    } else {
                        report.tcga_failed += 1;
                    }
                } else {
                    report.tcga_skipped += 1;
                }
            }
        }

        report.duration_ms = started.elapsed().as_millis() as u64;
        info!(
            target: "ferrumyx_provider_refresh",
            genes_processed = report.genes_processed,
            gtex_success = report.gtex_success,
            gtex_failed = report.gtex_failed,
            tcga_success = report.tcga_success,
            tcga_failed = report.tcga_failed,
            chembl_success = report.chembl_success,
            chembl_failed = report.chembl_failed,
            reactome_success = report.reactome_success,
            reactome_failed = report.reactome_failed,
            duration_ms = report.duration_ms,
            "provider signal refresh complete"
        );

        Ok(report)
    }
}

fn depmap_cache() -> Option<&'static DepMapCache> {
    static CACHE: OnceLock<Option<DepMapCache>> = OnceLock::new();
    CACHE
        .get_or_init(|| DepMapCache::load_default().ok())
        .as_ref()
}

fn should_fetch_gtex(candidate_count: usize, req: &QueryRequest) -> bool {
    if candidate_count == 0 || candidate_count > 8 {
        return false;
    }
    let _ = req;
    true
}

fn should_fetch_tcga(candidate_count: usize, req: &QueryRequest) -> bool {
    if candidate_count == 0 || candidate_count > 8 {
        return false;
    }
    let cancer_present = req
        .cancer_code
        .as_deref()
        .map(str::trim)
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    cancer_present
}

fn should_fetch_chembl(candidate_count: usize, _req: &QueryRequest) -> bool {
    candidate_count > 0 && candidate_count <= 8
}

fn should_fetch_reactome(candidate_count: usize, _req: &QueryRequest) -> bool {
    candidate_count > 0 && candidate_count <= 8
}

fn should_prewarm_large_cohort(candidate_count: usize) -> bool {
    (9..=1000).contains(&candidate_count)
}

async fn prewarm_phase4_provider_signals(
    db: Arc<Database>,
    genes: Vec<String>,
    cancer_code: Option<String>,
) {
    let engine = TargetQueryEngine::new(db);
    let _ = engine
        .refresh_provider_signals(ProviderRefreshRequest {
            genes,
            cancer_code,
            max_genes: 24,
            batch_size: 6,
            retries: 1,
        })
        .await;
}

async fn retry_fetch_f64<F, Fut>(retries: u8, mut op: F) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Option<f64>>,
{
    for attempt in 0..=retries {
        if op().await.is_some() {
            return true;
        }
        if attempt < retries {
            tokio::time::sleep(std::time::Duration::from_millis(150 * (attempt as u64 + 1))).await;
        }
    }
    false
}

async fn retry_fetch_u32<F, Fut>(retries: u8, mut op: F) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Option<u32>>,
{
    for attempt in 0..=retries {
        if op().await.is_some() {
            return true;
        }
        if attempt < retries {
            tokio::time::sleep(std::time::Duration::from_millis(150 * (attempt as u64 + 1))).await;
        }
    }
    false
}

async fn get_cached_gtex_expression_score(
    signal_repo: &Phase4SignalRepository,
    gene_symbol: &str,
) -> Option<f64> {
    static GTEX_CACHE: OnceLock<Mutex<HashMap<String, Option<f64>>>> = OnceLock::new();
    let key = gene_symbol.trim().to_uppercase();
    if key.is_empty() {
        return None;
    }

    let cache = GTEX_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(v) = guard.get(&key) {
            return *v;
        }
    }

    if let Ok(Some(row)) = signal_repo
        .find_gtex_expression_fresh(&key, PROVIDER_SIGNAL_TTL_DAYS)
        .await
    {
        let value = Some(row.expression_score.clamp(0.0, 1.0));
        if let Ok(mut guard) = cache.lock() {
            guard.insert(key, value);
        }
        return value;
    }

    // Bounded network call with timeout so ranker latency stays controlled.
    let fetched = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        let client = GtexClient::new();
        let map = client.get_median_expression(&key).await.ok()?;
        if map.is_empty() {
            return None;
        }
        let baseline = map.values().copied().sum::<f64>() / map.len() as f64;
        // Lower normal baseline expression => higher therapeutic window proxy.
        Some((1.0 / (1.0 + baseline.ln_1p())).clamp(0.0, 1.0))
    })
    .await
    .ok()
    .flatten();

    if let Some(expression_score) = fetched {
        let _ = signal_repo
            .upsert_gtex_expression(&EntGtexExpression {
                id: uuid::Uuid::new_v4(),
                gene_symbol: key.clone(),
                expression_score,
                source: "gtex_api".to_string(),
                fetched_at: chrono::Utc::now(),
            })
            .await;
    }

    if let Ok(mut guard) = cache.lock() {
        guard.insert(key, fetched);
    }
    fetched
}

fn to_tcga_project_id(cancer_code: &str) -> Option<String> {
    let code = cancer_code.trim().to_uppercase();
    if code.is_empty() {
        return None;
    }
    let mapped = match code.as_str() {
        "NSCLC" => "LUAD",
        "SKCM" | "PAAD" | "LUAD" | "LUSC" | "BRCA" | "COAD" | "READ" | "GBM" | "HNSC"
        | "OV" | "KIRC" | "KIRP" | "THCA" | "STAD" | "BLCA" | "UCEC" | "LIHC"
        | "PRAD" => code.as_str(),
        other if other.len() == 4 && other.chars().all(|c| c.is_ascii_uppercase()) => other,
        _ => return None,
    };
    Some(format!("TCGA-{}", mapped))
}

async fn get_cached_tcga_survival_score(
    signal_repo: &Phase4SignalRepository,
    gene_symbol: &str,
    cancer_code: &str,
) -> Option<f64> {
    static TCGA_CACHE: OnceLock<Mutex<HashMap<String, Option<f64>>>> = OnceLock::new();
    let gene = gene_symbol.trim().to_uppercase();
    let project = to_tcga_project_id(cancer_code)?;
    let normalized_cancer = cancer_code.trim().to_uppercase();
    let key = format!("{}|{}", gene, project);

    let cache = TCGA_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(v) = guard.get(&key) {
            return *v;
        }
    }

    if let Ok(Some(row)) = signal_repo
        .find_tcga_survival_fresh(&gene, &normalized_cancer, PROVIDER_SIGNAL_TTL_DAYS)
        .await
    {
        let value = Some(row.survival_score.clamp(0.0, 1.0));
        if let Ok(mut guard) = cache.lock() {
            guard.insert(key, value);
        }
        return value;
    }

    let fetched = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        let client = TcgaClient::new();
        let corr = client
            .get_survival_correlation(&gene, &project)
            .await
            .ok()
            .flatten()?;
        Some(((corr + 1.0) / 2.0).clamp(0.0, 1.0))
    })
    .await
    .ok()
    .flatten();

    if let Some(survival_score) = fetched {
        let _ = signal_repo
            .upsert_tcga_survival(&EntTcgaSurvival {
                id: uuid::Uuid::new_v4(),
                gene_symbol: gene.clone(),
                cancer_code: normalized_cancer,
                tcga_project_id: project.clone(),
                survival_score,
                source: "tcga_api".to_string(),
                fetched_at: chrono::Utc::now(),
            })
            .await;
    }

    if let Ok(mut guard) = cache.lock() {
        guard.insert(key, fetched);
    }
    fetched
}

#[derive(Debug, Deserialize)]
struct ReactomeProjectionResponse {
    #[serde(rename = "pathwaysFound")]
    pathways_found: Option<u64>,
    pathways: Option<Vec<serde_json::Value>>,
}

async fn get_cached_chembl_inhibitor_count(
    signal_repo: &Phase4SignalRepository,
    gene_symbol: &str,
) -> Option<u32> {
    static CHEMBL_CACHE: OnceLock<Mutex<HashMap<String, Option<u32>>>> = OnceLock::new();
    let key = gene_symbol.trim().to_uppercase();
    if key.is_empty() {
        return None;
    }
    let cache = CHEMBL_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(v) = guard.get(&key) {
            return *v;
        }
    }

    if let Ok(Some(row)) = signal_repo
        .find_chembl_target_fresh(&key, PROVIDER_SIGNAL_TTL_DAYS)
        .await
    {
        let value = Some(row.inhibitor_count.max(0) as u32);
        if let Ok(mut guard) = cache.lock() {
            guard.insert(key, value);
        }
        return value;
    }

    let key_for_fetch = key.clone();
    let fetched = tokio::time::timeout(std::time::Duration::from_secs(4), async move {
        let client = ChemblClient::new();
        let targets = client.search_targets_by_gene(&key_for_fetch).await.ok()?;
        if targets.is_empty() {
            return Some(0u32);
        }

        let mut unique_compounds: HashSet<String> = HashSet::new();
        for target in targets.iter().take(3) {
            let acts = client
                .fetch_target_activities(&target.chembl_id, None, 250)
                .await
                .ok()?;
            for act in acts {
                if !act.compound_id.trim().is_empty() {
                    unique_compounds.insert(act.compound_id);
                }
            }
            if unique_compounds.len() >= 1000 {
                break;
            }
        }
        Some(unique_compounds.len() as u32)
    })
    .await
    .ok()
    .flatten();

    if let Some(inhibitor_count) = fetched {
        let _ = signal_repo
            .upsert_chembl_target(&EntChemblTarget {
                id: uuid::Uuid::new_v4(),
                gene_symbol: key.clone(),
                inhibitor_count: inhibitor_count as i64,
                source: "chembl_api".to_string(),
                fetched_at: chrono::Utc::now(),
            })
            .await;
    }

    if let Ok(mut guard) = cache.lock() {
        guard.insert(key, fetched);
    }
    fetched
}

async fn get_cached_reactome_pathway_count(
    signal_repo: &Phase4SignalRepository,
    gene_symbol: &str,
) -> Option<u32> {
    static REACTOME_CACHE: OnceLock<Mutex<HashMap<String, Option<u32>>>> = OnceLock::new();
    let key = gene_symbol.trim().to_uppercase();
    if key.is_empty() {
        return None;
    }
    let cache = REACTOME_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(v) = guard.get(&key) {
            return *v;
        }
    }

    if let Ok(Some(row)) = signal_repo
        .find_reactome_gene_fresh(&key, PROVIDER_SIGNAL_TTL_DAYS)
        .await
    {
        let value = Some(row.pathway_count.max(0) as u32);
        if let Ok(mut guard) = cache.lock() {
            guard.insert(key, value);
        }
        return value;
    }

    let key_for_fetch = key.clone();
    let fetched = tokio::time::timeout(std::time::Duration::from_secs(4), async move {
        let client = Client::new();
        let resp = client
            .post("https://reactome.org/AnalysisService/identifiers/projection?pageSize=200&page=1")
            .header("Content-Type", "text/plain")
            .body(key_for_fetch)
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let body = resp.json::<ReactomeProjectionResponse>().await.ok()?;
        if let Some(pathways) = body.pathways {
            return Some(pathways.len() as u32);
        }
        body.pathways_found.map(|v| v as u32)
    })
    .await
    .ok()
    .flatten();

    if let Some(pathway_count) = fetched {
        let _ = signal_repo
            .upsert_reactome_gene(&EntReactomeGene {
                id: uuid::Uuid::new_v4(),
                gene_symbol: key.clone(),
                pathway_count: pathway_count as i64,
                source: "reactome_api".to_string(),
                fetched_at: chrono::Utc::now(),
            })
            .await;
    }

    if let Ok(mut guard) = cache.lock() {
        guard.insert(key, fetched);
    }
    fetched
}

fn default_component_sources() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    out.insert("n1_mutation_freq".to_string(), "proxy_kg".to_string());
    out.insert("n2_crispr_dependency".to_string(), "proxy_kg".to_string());
    out.insert("n3_survival_correlation".to_string(), "proxy_kg".to_string());
    out.insert("n4_expression_specificity".to_string(), "proxy_kg".to_string());
    out.insert("n5_structural_tractability".to_string(), "proxy_kg".to_string());
    out.insert("n6_pocket_detectability".to_string(), "proxy_kg".to_string());
    out.insert("n7_novelty_score".to_string(), "proxy_kg".to_string());
    out.insert("n8_pathway_independence".to_string(), "proxy_kg".to_string());
    out.insert("n9_literature_novelty".to_string(), "proxy_kg".to_string());
    out
}

#[derive(Debug, Clone)]
struct GeneCandidate {
    gene_symbol: String,
    flags: HashSet<String>,
    paper_ids: HashSet<uuid::Uuid>,
    fact_count: u32,
    confidence_sum: f64,
    confidence_n: u32,
    mutation_mentions: u32,
    survival_mentions: u32,
    survival_positive: u32,
    survival_negative: u32,
    expression_tumor_mentions: u32,
    expression_normal_mentions: u32,
    cancer_mentions: u32,
    inhibitor_mentions: u32,
    pathway_mentions: u32,
    structural_mentions: u32,
    pocket_mentions: u32,
    sample_size_sum: f64,
    sample_obs: u32,
    cancer_codes: HashMap<String, u32>,
}

impl GeneCandidate {
    fn new(gene_symbol: String) -> Self {
        Self {
            gene_symbol,
            flags: HashSet::new(),
            paper_ids: HashSet::new(),
            fact_count: 0,
            confidence_sum: 0.0,
            confidence_n: 0,
            mutation_mentions: 0,
            survival_mentions: 0,
            survival_positive: 0,
            survival_negative: 0,
            expression_tumor_mentions: 0,
            expression_normal_mentions: 0,
            cancer_mentions: 0,
            inhibitor_mentions: 0,
            pathway_mentions: 0,
            structural_mentions: 0,
            pocket_mentions: 0,
            sample_size_sum: 0.0,
            sample_obs: 0,
            cancer_codes: HashMap::new(),
        }
    }

    fn mean_confidence(&self) -> f64 {
        if self.confidence_n == 0 {
            return 0.5;
        }
        (self.confidence_sum / self.confidence_n as f64).clamp(0.0, 1.0)
    }

    fn infer_cancer_code(&self) -> Option<String> {
        self.cancer_codes
            .iter()
            .max_by_key(|(_, cnt)| **cnt)
            .map(|(code, _)| code.clone())
    }

    fn to_target_metrics(&self) -> TargetMetrics {
        let evidence = self.fact_count.max(1) as f64;
        let confidence_mean = self.mean_confidence();

        let mutation_freq = (self.mutation_mentions as f64 / evidence).clamp(0.0, 1.0);
        let crispr_dependency = (-2.0 * confidence_mean).clamp(-2.0, 0.0);

        // Source-derived from survival semantics when present; confidence/sample fallback otherwise.
        let survival_correlation = if self.survival_mentions > 0 {
            let signed = (self.survival_positive as f64 - self.survival_negative as f64)
                / self.survival_mentions as f64;
            ((signed + 1.0) / 2.0).clamp(0.0, 1.0)
        } else if self.sample_obs > 0 {
            (self.sample_size_sum / self.sample_obs as f64).ln_1p().min(10.0) / 10.0
        } else {
            confidence_mean
        };

        // Source-derived from expression predicates (tumor/normal ratio) when present.
        let expression_specificity = if self.expression_tumor_mentions + self.expression_normal_mentions > 0 {
            ((self.expression_tumor_mentions as f64 + 1.0)
                / (self.expression_normal_mentions as f64 + 1.0))
                .clamp(0.5, 5.0)
        } else {
            (1.0 + 4.0 * (self.cancer_mentions as f64 / evidence)).clamp(0.5, 5.0)
        };

        let pdb_structure_count = self.structural_mentions;
        let af_plddt_mean = if pdb_structure_count > 0 {
            (70.0 + 25.0 * confidence_mean).clamp(0.0, 100.0)
        } else {
            (45.0 + 20.0 * confidence_mean).clamp(0.0, 100.0)
        };

        let fpocket_best_score =
            (0.2 + 0.8 * (self.pocket_mentions as f64 / evidence)).clamp(0.0, 1.0);

        let chembl_inhibitor_count = self.inhibitor_mentions;
        let reactome_escape_pathway_count = self.pathway_mentions;

        // Inverted evidence velocity proxy: underexplored genes score higher.
        let literature_novelty_velocity = 1.0 / (1.0 + evidence.ln_1p());

        TargetMetrics {
            mutation_freq,
            crispr_dependency,
            survival_correlation,
            expression_specificity,
            pdb_structure_count,
            af_plddt_mean,
            fpocket_best_score,
            chembl_inhibitor_count,
            reactome_escape_pathway_count,
            literature_novelty_velocity,
        }
    }

    fn source_backed_literature_novelty(
        &self,
        published_at_by_paper: &HashMap<uuid::Uuid, chrono::DateTime<chrono::Utc>>,
    ) -> Option<f64> {
        if self.paper_ids.is_empty() {
            return None;
        }

        let now = chrono::Utc::now();
        let mut total_with_dates = 0usize;
        let mut recent_2y = 0usize;
        for paper_id in &self.paper_ids {
            let Some(ts) = published_at_by_paper.get(paper_id) else {
                continue;
            };
            total_with_dates += 1;
            let age_days = (now - *ts).num_days();
            if age_days <= 365 * 2 {
                recent_2y += 1;
            }
        }

        if total_with_dates == 0 {
            return None;
        }

        let velocity = recent_2y as f64 / total_with_dates as f64;
        Some((1.0 - velocity).clamp(0.0, 1.0))
    }
}

fn is_gene_like(name: &str) -> bool {
    let n = name.trim();
    if n.is_empty() || n.len() > 16 || n.contains(' ') {
        return false;
    }
    n.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && n.chars().any(|c| c.is_ascii_uppercase())
}

fn is_cancer_like(name: &str) -> bool {
    let n = name.trim();
    if n.is_empty() {
        return false;
    }
    let code_like =
        n.len() <= 8 && n.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    if code_like {
        return true;
    }
    let lc = n.to_lowercase();
    lc.contains("cancer")
        || lc.contains("carcinoma")
        || lc.contains("sarcoma")
        || lc.contains("lymphoma")
        || lc.contains("leukemia")
        || lc.contains("tumor")
}
