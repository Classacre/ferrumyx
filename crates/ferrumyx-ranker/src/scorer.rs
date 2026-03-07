//! Composite target score computation.
//! Implements S(g, c) formula from ARCHITECTURE.md §4.1

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use ferrumyx_common::query::{TargetMetrics, TargetScoreResult};
use crate::weights::WeightVector;
use crate::depmap_provider::DepMapProvider;
use crate::tcga_provider::TcgaProvider;
use crate::gtex_provider::GtexProvider;
use crate::normalise::normalise_ceres;

/// Raw component scores for a (gene, cancer) pair.
/// All values should be in their natural units before normalisation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentScoresRaw {
    pub mutation_freq: Option<f64>,          // 0.0–1.0 (fraction of tumours)
    pub crispr_dependency: Option<f64>,      // CERES score (typically -2.0 to 0)
    pub survival_correlation: Option<f64>,   // hazard ratio or log-rank p-value
    pub expression_specificity: Option<f64>, // tumour_tpm / normal_tpm
    pub structural_tractability: Option<f64>,// 0.0–1.0
    pub pocket_detectability: Option<f64>,   // fpocket score 0.0–1.0
    pub novelty_score: Option<f64>,          // 1 / (1 + inhibitor_count)
    pub pathway_independence: Option<f64>,   // 1 / (1 + escape_pathway_count)
    pub literature_novelty: Option<f64>,     // underexplored ratio
}

/// Normalised component scores (all in [0, 1]).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentScoresNormed {
    pub mutation_freq: f64,
    pub crispr_dependency: f64,
    pub survival_correlation: f64,
    pub expression_specificity: f64,
    pub structural_tractability: f64,
    pub pocket_detectability: f64,
    pub novelty_score: f64,
    pub pathway_independence: f64,
    pub literature_novelty: f64,
}

impl ComponentScoresNormed {
    pub fn as_array(&self) -> [f64; 9] {
        [
            self.mutation_freq,
            self.crispr_dependency,
            self.survival_correlation,
            self.expression_specificity,
            self.structural_tractability,
            self.pocket_detectability,
            self.novelty_score,
            self.pathway_independence,
            self.literature_novelty,
        ]
    }
}

/// Penalty term inputs.
#[derive(Debug, Clone)]
pub struct PenaltyInputs {
    pub chembl_inhibitor_count: u32,
    pub expression_ratio: f64,
    pub has_pdb: bool,
    pub alphafold_plddt: Option<f64>,
}

/// Compute penalty term P(g, c).
/// See ARCHITECTURE.md §4.1
pub fn compute_penalty(inputs: &PenaltyInputs) -> f64 {
    let mut penalty = 0.0;

    // Inhibitor saturation penalty
    if inputs.chembl_inhibitor_count > 50 {
        penalty += 0.15;
    }

    // Low expression specificity penalty
    if inputs.expression_ratio < 1.5 {
        penalty += 0.10;
    }

    // Structural void penalty
    if !inputs.has_pdb {
        if let Some(plddt) = inputs.alphafold_plddt {
            if plddt < 50.0 {
                penalty += 0.08;
            }
        } else {
            penalty += 0.08; // No structure at all
        }
    }

    penalty
}

/// Final scored target result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetScore {
    pub gene_id: Uuid,
    pub cancer_id: Uuid,
    pub composite_score: f64,
    pub confidence_adjusted_score: f64,
    pub component_scores_raw: serde_json::Value,
    pub component_scores_normed: serde_json::Value,
    pub penalty: f64,
    pub mean_confidence: f64,
}

/// Compute the composite score S(g, c) given normalised components,
/// weights, penalty, and mean KG confidence.
///
/// S(g,c) = Σ(w_i × n_i) − P(g,c)
/// S_adj(g,c) = S(g,c) × C(g,c)
///
/// See ARCHITECTURE.md §4.1
pub fn compute_composite_score(
    normed: &ComponentScoresNormed,
    weights: &WeightVector,
    penalty: f64,
    mean_confidence: f64,
) -> (f64, f64) {
    let components = normed.as_array();
    let weight_arr = weights.as_array();

    let weighted_sum: f64 = components
        .iter()
        .zip(weight_arr.iter())
        .map(|(n, w)| n * w)
        .sum();

    let composite = (weighted_sum - penalty).clamp(0.0, 1.0);
    let adjusted  = (composite * mean_confidence).clamp(0.0, 1.0);

    (composite, adjusted)
}

/// Shortlisting threshold check.
/// See ARCHITECTURE.md §4.5
#[derive(Debug, Clone, PartialEq)]
pub enum ShortlistTier {
    Primary,
    Secondary,
    Excluded,
}

pub struct PrioritizationEngine;

impl PrioritizationEngine {
    pub fn calculate_scores(cohort: &[(Uuid, TargetMetrics)]) -> std::collections::HashMap<Uuid, TargetScoreResult> {
        let n = cohort.len() as f64;
        let mut results = std::collections::HashMap::new();

        if cohort.is_empty() {
            return results;
        }

        let mutation_freq_ranks = Self::rank_cohort(cohort, |m| m.mutation_freq);
        let survival_corr_ranks = Self::rank_cohort(cohort, |m| m.survival_correlation);
        let expr_spec_ranks = Self::rank_cohort(cohort, |m| m.expression_specificity);
        let lit_novelty_ranks = Self::rank_cohort(cohort, |m| m.literature_novelty_velocity);

        for (id, metrics) in cohort {
            let n1 = mutation_freq_ranks.get(id).unwrap_or(&0.0) / n;
            let n3 = survival_corr_ranks.get(id).unwrap_or(&0.0) / n;
            let n4 = expr_spec_ranks.get(id).unwrap_or(&0.0) / n;
            let n9 = lit_novelty_ranks.get(id).unwrap_or(&0.0) / n;
            
            let ceres_normalized = (metrics.crispr_dependency + 2.0) / 2.0;
            let n2 = 1.0 - ceres_normalized.clamp(0.0, 1.0);

            let pdb_cov = (metrics.pdb_structure_count as f64 / 5.0).min(1.0);
            let n5 = if metrics.pdb_structure_count > 0 {
                0.40 * pdb_cov + 0.25 * metrics.fpocket_best_score.clamp(0.0, 1.0)
            } else {
                0.35 * (metrics.af_plddt_mean / 100.0) + 0.25 * metrics.fpocket_best_score.clamp(0.0, 1.0)
            };

            let n6 = metrics.fpocket_best_score.clamp(0.0, 1.0);
            let n7 = 1.0 / (1.0 + metrics.chembl_inhibitor_count as f64);
            let n8 = 1.0 / (1.0 + metrics.reactome_escape_pathway_count as f64);

            let mut penalty = 0.0;
            if metrics.chembl_inhibitor_count > 50 { penalty += 0.15; }
            if metrics.expression_specificity < 1.5 { penalty += 0.10; }
            if metrics.pdb_structure_count == 0 && metrics.af_plddt_mean < 50.0 { penalty += 0.08; }

            let mut composite = (
                0.20 * n1 + 0.18 * n2 + 0.15 * n3 + 0.12 * n4 + 0.12 * n5 + 0.08 * n6 + 0.07 * n7 + 0.05 * n8 + 0.03 * n9
            ) - penalty;

            composite = composite.clamp(0.0, 1.0);

            results.insert(*id, TargetScoreResult {
                n1_mutation_freq: n1,
                n2_crispr_dependency: n2,
                n3_survival_correlation: n3,
                n4_expression_specificity: n4,
                n5_structural_tractability: n5,
                n6_pocket_detectability: n6,
                n7_novelty_score: n7,
                n8_pathway_independence: n8,
                n9_literature_novelty: n9,
                penalty,
                composite_score: composite,
                is_disputed: false,
            });
        }

        results
    }

    fn rank_cohort<F>(cohort: &[(Uuid, TargetMetrics)], key_fn: F) -> std::collections::HashMap<Uuid, f64> 
    where F: Fn(&TargetMetrics) -> f64 {
        let mut sortable: Vec<(&Uuid, f64)> = cohort.iter().map(|(id, m)| (id, key_fn(m))).collect();
        sortable.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        
        let mut ranks = std::collections::HashMap::new();
        for (i, (id, _)) in sortable.iter().enumerate() {
            ranks.insert(**id, (i + 1) as f64);
        }
        ranks
    }
}

pub fn determine_shortlist_tier(
    score_adjusted: f64,
    mutation_freq_raw: Option<f64>,
    structural_tractability: f64,
    penalty_inputs: &PenaltyInputs,
) -> ShortlistTier {
    // Hard exclusion: saturated + low novelty
    if penalty_inputs.chembl_inhibitor_count > 50 {
        return ShortlistTier::Excluded;
    }

    // Primary shortlist
    if score_adjusted > 0.60
        && mutation_freq_raw.unwrap_or(0.0) > 0.05
        && structural_tractability > 0.40
    {
        return ShortlistTier::Primary;
    }

    // Secondary shortlist
    if score_adjusted > 0.45 {
        return ShortlistTier::Secondary;
    }

    ShortlistTier::Excluded
}

/// Compute CRISPR dependency component score from DepMap data.
///
/// This function:
/// 1. Queries DepMap for CERES scores
/// 2. Normalizes the score using `normalise_ceres()`
/// 3. Returns the normalized component score (0.0–1.0)
///
/// Returns None if no DepMap data is available for this gene-cancer pair.
pub fn compute_crispr_component(
    gene: &str,
    cancer_type: &str,
    depmap: &dyn DepMapProvider,
) -> Option<f64> {
    // Try mean CERES first
    let ceres = depmap.get_mean_ceres(gene, cancer_type)?;
    
    // Normalize: more essential (more negative) → higher score
    Some(normalise_ceres(ceres))
}

/// Compute TCGA survival correlation component score.
pub fn compute_survival_component(
    gene: &str,
    cancer_type: &str,
    tcga: &dyn TcgaProvider,
) -> Option<f64> {
    let correlation = tcga.get_survival_correlation(gene, cancer_type)?;
    // the simulation returns correlation -1.0 to 1.0, map to 0.0-1.0
    Some((correlation + 1.0) / 2.0)
}

/// Compute expression specificity from GTEx.
pub fn compute_expression_component(
    gene: &str,
    tumour_tpm: Option<f64>,
    gtex: &dyn GtexProvider,
) -> Option<f64> {
    let t_tpm = tumour_tpm?;
    let median_exprs = gtex.get_median_expression(gene)?;
    
    // Average normal expression across all GTEx tissues
    let sum: f64 = median_exprs.values().sum();
    let count = median_exprs.len() as f64;
    
    if count == 0.0 {
        return None;
    }
    let baseline = sum / count;
    
    if baseline == 0.0 {
        return Some(1.0); // Extremely tumor specific
    }
    
    let ratio = t_tpm / baseline;
    // Cap at 10x ratio
    Some((ratio / 10.0).clamp(0.0, 1.0))
}

/// Compute component scores with DepMap, TCGA, and GTEx integration.
///
/// This is a convenience function that wires in the providers
/// for the automated components.
pub fn compute_component_scores_with_providers(
    gene: &str,
    cancer_type: &str,
    tumour_tpm: Option<f64>,
    depmap: &dyn DepMapProvider,
    tcga: &dyn TcgaProvider,
    gtex: &dyn GtexProvider,
    mutation_freq: Option<f64>,
    structural_tractability: Option<f64>,
    pocket_detectability: Option<f64>,
    novelty_score: Option<f64>,
    pathway_independence: Option<f64>,
    literature_novelty: Option<f64>,
) -> ComponentScoresRaw {
    let crispr_dependency = depmap.get_mean_ceres(gene, cancer_type);
    let survival_correlation = tcga.get_survival_correlation(gene, cancer_type);
    
    let expression_specificity = match tumour_tpm {
        Some(tpm) => compute_expression_component(gene, Some(tpm), gtex),
        None => None
    };
    
    ComponentScoresRaw {
        mutation_freq,
        crispr_dependency,
        survival_correlation,
        expression_specificity,
        structural_tractability,
        pocket_detectability,
        novelty_score,
        pathway_independence,
        literature_novelty,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::depmap_provider::MockDepMapProvider;
    use crate::tcga_provider::MockTcgaProvider;
    use crate::gtex_provider::MockGtexProvider;

    #[test]
    fn test_composite_score_range() {
        let normed = ComponentScoresNormed {
            mutation_freq: 0.8,
            crispr_dependency: 0.9,
            survival_correlation: 0.7,
            expression_specificity: 0.8,
            structural_tractability: 0.7,
            pocket_detectability: 0.6,
            novelty_score: 0.9,
            pathway_independence: 0.7,
            literature_novelty: 0.8,
        };
        let weights = WeightVector::default();
        let penalty = 0.0;
        let confidence = 0.85;

        let (composite, adjusted) = compute_composite_score(&normed, &weights, penalty, confidence);
        assert!((0.0..=1.0).contains(&composite));
        assert!((0.0..=1.0).contains(&adjusted));
        assert!(adjusted <= composite);
    }

    #[test]
    fn test_penalty_reduces_score() {
        let normed = ComponentScoresNormed {
            mutation_freq: 0.5, crispr_dependency: 0.5, survival_correlation: 0.5,
            expression_specificity: 0.5, structural_tractability: 0.5,
            pocket_detectability: 0.5, novelty_score: 0.5,
            pathway_independence: 0.5, literature_novelty: 0.5,
        };
        let weights = WeightVector::default();
        let (no_pen, _)   = compute_composite_score(&normed, &weights, 0.0,  1.0);
        let (with_pen, _) = compute_composite_score(&normed, &weights, 0.15, 1.0);
        assert!(no_pen >= with_pen);
    }

    #[test]
    fn test_crispr_component_normalized() {
        let provider = MockDepMapProvider::new()
            .with("KRAS", "PAAD", -1.2);  // Strongly essential
        
        let score = compute_crispr_component("KRAS", "PAAD", &provider);
        
        // -1.2 should normalize to ~0.6 (moderate-high)
        assert!(score.is_some());
        let s = score.unwrap();
        assert!(s > 0.5 && s < 0.7, "Expected ~0.6, got {}", s);
    }

    #[test]
    fn test_crispr_component_missing_gene() {
        let provider = MockDepMapProvider::new()
            .with("KRAS", "PAAD", -1.0);
        
        let score = compute_crispr_component("TP53", "PAAD", &provider);
        assert!(score.is_none());
    }

    #[test]
    fn test_crispr_component_missing_cancer() {
        let provider = MockDepMapProvider::new()
            .with("KRAS", "PAAD", -1.0);
        
        let score = compute_crispr_component("KRAS", "LUAD", &provider);
        assert!(score.is_none());
    }

    #[test]
    fn test_compute_survival_component() {
        let tcga = MockTcgaProvider::new()
            .with("TP53", "BRCA", -0.5); // Better survival -> negative correlation
        
        let score = compute_survival_component("TP53", "BRCA", &tcga);
        assert!(score.is_some());
        // -0.5 mapped to 0.0-1.0 is (-0.5 + 1.0) / 2.0 = 0.25
        assert_eq!(score.unwrap(), 0.25);
    }

    #[test]
    fn test_compute_expression_component() {
        let gtex = MockGtexProvider::new()
            .with("HER2", "Breast", 5.0)
            .with("HER2", "Lung", 2.0); // Mean = 3.5
        
        // Tumour TPM = 35.0 (10x ratio)
        let score_max = compute_expression_component("HER2", Some(35.0), &gtex);
        assert!(score_max.is_some());
        assert_eq!(score_max.unwrap(), 1.0); // 10x ratio clamped to 1.0

        // Tumour TPM = 17.5 (5x ratio)
        let score_mid = compute_expression_component("HER2", Some(17.5), &gtex);
        assert_eq!(score_mid.unwrap(), 0.5); // 5x ratio -> 0.5
    }
}
