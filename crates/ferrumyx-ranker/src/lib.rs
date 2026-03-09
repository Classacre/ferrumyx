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
use ferrumyx_db::Database;
use ferrumyx_db::kg_facts::KgFactRepository;
use ferrumyx_db::kg_conflicts::KgConflictRepository;
use ferrumyx_common::query::{QueryRequest, QueryResult, TargetMetrics, TargetScoreResult};

pub struct TargetQueryEngine {
    db: Arc<Database>,
}

impl TargetQueryEngine {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn execute_query(&self, req: QueryRequest) -> anyhow::Result<Vec<QueryResult>> {
        let kg_repo = KgFactRepository::new(self.db.clone());
        let conflict_repo = KgConflictRepository::new(self.db.clone());
        
        // Fetch facts from KG
        let facts = kg_repo.list(0, req.max_results * 5).await.unwrap_or_default();
        let gene_filter = req.gene_symbol.as_deref().unwrap_or("");
        
        let mut cohort_metrics = Vec::new();
        let mut candidate_facts = Vec::new();
        
        for f in facts {
            if !gene_filter.is_empty() && !f.subject_name.contains(gene_filter) {
                continue;
            }
            
            let conflicts = conflict_repo.find_by_fact_id(f.id).await.unwrap_or_default();
            let mut include = true;
            let mut confidence_adj = f.confidence as f64;
            let mut flags: Vec<String> = Vec::new();
            
            for conflict in conflicts {
                let net = conflict.net_confidence as f64;
                if net < 0.30 {
                    include = false;
                    break;
                } else if net <= 0.60 {
                    flags.push("DISPUTED".to_string());
                    confidence_adj *= 0.70;
                }
            }
            
            if !include { continue; }

            // Metrics simulation/mock for now (as in original query crate)
            let hash_seed = (f.id.as_u128() % 1000) as f64 / 1000.0;
            let mut metrics = TargetMetrics::default();
            metrics.mutation_freq = hash_seed * 0.1; 
            metrics.crispr_dependency = -1.0 - (hash_seed * 1.5); 
            metrics.survival_correlation = hash_seed;
            metrics.expression_specificity = 1.0 + (hash_seed * 2.0);
            metrics.pdb_structure_count = (f.id.as_u128() % 10) as u32;
            metrics.fpocket_best_score = 1.0 - hash_seed;
            metrics.chembl_inhibitor_count = (f.id.as_u128() % 20) as u32;
            
            cohort_metrics.push((f.id, metrics));
            candidate_facts.push((f, confidence_adj, flags));
        }

        // Scoring
        let cohort_scores = scorer::PrioritizationEngine::calculate_scores(&cohort_metrics);
        
        let mut results = Vec::new();
        let cancer_code = req.cancer_code.clone().unwrap_or_else(|| "PAAD".to_string());

        for (f, conf_adj, mut flags) in candidate_facts {
            if let Some(score_res) = cohort_scores.get(&f.id) {
                if score_res.is_disputed {
                    flags.push("DISPUTED".to_string());
                }

                let tier = if score_res.composite_score > 0.60 {
                    "primary".to_string()
                } else if score_res.composite_score > 0.45 {
                    "secondary".to_string()
                } else {
                    "excluded".to_string()
                };

                results.push(QueryResult {
                    rank: 0, // Assigned after sorting
                    gene_symbol: f.subject_name.clone(),
                    cancer_code: cancer_code.clone(),
                    composite_score: score_res.composite_score,
                    confidence_adj: score_res.composite_score * conf_adj,
                    shortlist_tier: tier,
                    flags,
                    metrics: cohort_metrics.iter().find(|(id, _)| *id == f.id).map(|(_, m): &(uuid::Uuid, TargetMetrics)| m.clone()),
                });
            }
        }

        // Sort by composite score
        results.sort_by(|a, b| b.composite_score.partial_cmp(&a.composite_score).unwrap_or(std::cmp::Ordering::Equal));
        
        // Assign ranks and take max_results
        for (i, res) in results.iter_mut().enumerate() {
            res.rank = i + 1;
        }
        results.truncate(req.max_results);
            
        Ok(results)
    }
}
