//! Composite target score computation.
//! Implements S(g, c) formula from ARCHITECTURE.md §4.1

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::weights::WeightVector;

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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(composite >= 0.0 && composite <= 1.0);
        assert!(adjusted >= 0.0 && adjusted <= 1.0);
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
}
