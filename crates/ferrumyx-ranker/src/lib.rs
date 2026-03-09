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
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use ferrumyx_db::Database;
use ferrumyx_db::kg_facts::KgFactRepository;
use ferrumyx_db::kg_conflicts::KgConflictRepository;
use ferrumyx_db::target_scores::TargetScoreRepository;
use ferrumyx_db::EntStageRepository;
use ferrumyx_common::query::{QueryRequest, QueryResult, TargetMetrics};
use tracing::info;

pub struct TargetQueryEngine {
    db: Arc<Database>,
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

            let predicate_lc = f.predicate.to_lowercase();
            if predicate_lc.contains("mutation") || predicate_lc == "has_mutation" {
                entry.mutation_mentions += 1;
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
        let symbol_list: Vec<String> = candidates
            .values()
            .map(|c| c.gene_symbol.clone())
            .collect();
        let enrichment_by_symbol = ent_repo
            .get_enrichment_by_symbol(&symbol_list)
            .await
            .unwrap_or_default();
        let t_enrich = Instant::now();

        let mut cohort_metrics = Vec::new();
        let mut by_gene_metrics: HashMap<uuid::Uuid, TargetMetrics> = HashMap::new();
        for (gene_id, candidate) in &candidates {
            let mut metrics = candidate.to_target_metrics();
            if let Some(enrich) = enrichment_by_symbol.get(&candidate.gene_symbol.to_uppercase()) {
                if enrich.mutation_count > 0 {
                    let source_mutation = (enrich.mutation_count as f64 / 20.0).clamp(0.0, 1.0);
                    metrics.mutation_freq = metrics.mutation_freq.max(source_mutation);
                }
                if enrich.pdb_structure_count > 0 {
                    metrics.pdb_structure_count =
                        metrics.pdb_structure_count.max(enrich.pdb_structure_count);
                }
                if let Some(plddt) = enrich.af_plddt_mean {
                    metrics.af_plddt_mean = metrics.af_plddt_mean.max(plddt);
                }
                if let Some(fpocket) = enrich.fpocket_best_score {
                    metrics.fpocket_best_score = metrics.fpocket_best_score.max(fpocket);
                }
                if enrich.chembl_inhibitor_count > 0 {
                    metrics.chembl_inhibitor_count = metrics
                        .chembl_inhibitor_count
                        .max(enrich.chembl_inhibitor_count);
                }
                if enrich.pathway_count > 0 {
                    metrics.reactome_escape_pathway_count = metrics
                        .reactome_escape_pathway_count
                        .max(enrich.pathway_count);
                }
            }
            cohort_metrics.push((*gene_id, metrics.clone()));
            by_gene_metrics.insert(*gene_id, metrics);
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
}

#[derive(Debug, Clone)]
struct GeneCandidate {
    gene_symbol: String,
    flags: HashSet<String>,
    fact_count: u32,
    confidence_sum: f64,
    confidence_n: u32,
    mutation_mentions: u32,
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
            fact_count: 0,
            confidence_sum: 0.0,
            confidence_n: 0,
            mutation_mentions: 0,
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

        // Proxy until TCGA/GTEx direct tables are integrated end-to-end.
        let survival_correlation = if self.sample_obs > 0 {
            (self.sample_size_sum / self.sample_obs as f64).ln_1p().min(10.0) / 10.0
        } else {
            confidence_mean
        };

        let expression_specificity =
            (1.0 + 4.0 * (self.cancer_mentions as f64 / evidence)).clamp(0.5, 5.0);

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
