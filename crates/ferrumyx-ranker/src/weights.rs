//! Weight vector for target prioritization scoring.
//! See ARCHITECTURE.md §4.1 — Initial weight vector W.

use serde::{Deserialize, Serialize};

/// The 9-component weight vector W.
/// Weights sum to 1.0.
/// See ARCHITECTURE.md §4.1 for biological justification of each value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightVector {
    /// Mutation frequency in cancer type (COSMIC/cBioPortal)
    pub mutation_freq: f64,
    /// CRISPR dependency score (DepMap CERES, inverted)
    pub crispr_dependency: f64,
    /// Survival correlation (TCGA Kaplan-Meier)
    pub survival_correlation: f64,
    /// Expression specificity (tumour/normal ratio)
    pub expression_specificity: f64,
    /// Structural tractability (PDB + AlphaFold + pocket)
    pub structural_tractability: f64,
    /// Binding pocket detectability (fpocket/DoGSiteScorer)
    pub pocket_detectability: f64,
    /// Novelty score (inverse ChEMBL inhibitor density)
    pub novelty_score: f64,
    /// Pathway independence (inverse Reactome escape routes)
    pub pathway_independence: f64,
    /// Literature novelty (underexplored ratio)
    pub literature_novelty: f64,
}

impl Default for WeightVector {
    /// Initial expert prior weights from ARCHITECTURE.md §4.1
    fn default() -> Self {
        Self {
            mutation_freq:          0.20,
            crispr_dependency:      0.18,
            survival_correlation:   0.15,
            expression_specificity: 0.12,
            structural_tractability:0.12,
            pocket_detectability:   0.08,
            novelty_score:          0.07,
            pathway_independence:   0.05,
            literature_novelty:     0.03,
        }
    }
}

impl WeightVector {
    /// Validate that all weights sum to ~1.0
    pub fn validate(&self) -> bool {
        let sum = self.mutation_freq
            + self.crispr_dependency
            + self.survival_correlation
            + self.expression_specificity
            + self.structural_tractability
            + self.pocket_detectability
            + self.novelty_score
            + self.pathway_independence
            + self.literature_novelty;
        (sum - 1.0).abs() < 1e-6
    }

    /// Renormalise weights so they sum to 1.0
    pub fn normalise(&mut self) {
        let sum = self.mutation_freq
            + self.crispr_dependency
            + self.survival_correlation
            + self.expression_specificity
            + self.structural_tractability
            + self.pocket_detectability
            + self.novelty_score
            + self.pathway_independence
            + self.literature_novelty;
        if sum > 0.0 {
            self.mutation_freq           /= sum;
            self.crispr_dependency       /= sum;
            self.survival_correlation    /= sum;
            self.expression_specificity  /= sum;
            self.structural_tractability /= sum;
            self.pocket_detectability    /= sum;
            self.novelty_score           /= sum;
            self.pathway_independence    /= sum;
            self.literature_novelty      /= sum;
        }
    }

    /// Convert to array for iteration.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_weights_sum_to_one() {
        let w = WeightVector::default();
        assert!(w.validate(), "Default weights must sum to 1.0");
    }

    #[test]
    fn test_normalise_restores_sum() {
        let mut w = WeightVector::default();
        w.mutation_freq += 0.10; // deliberately break sum
        assert!(!w.validate());
        w.normalise();
        assert!(w.validate());
    }
}
