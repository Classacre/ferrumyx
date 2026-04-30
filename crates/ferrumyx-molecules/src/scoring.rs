//! Scoring and ranking of generated molecules.

use crate::admet::AdmetProperties;
use crate::ligand::Molecule;
use serde::{Deserialize, Serialize};

/// A scored molecule with its docking and ADMET results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredMolecule {
    pub molecule: Molecule,
    pub docking_score: f64,
    pub admet_properties: AdmetProperties,
    pub composite_score: f64,
}

/// Scorer for evaluating and ranking molecules.
pub struct MoleculeScorer {}

impl MoleculeScorer {
    /// Create a new MoleculeScorer.
    pub fn new() -> Self {
        Self {}
    }

    /// Score a molecule based on its docking score and ADMET properties.
    pub fn score(
        &self,
        molecule: Molecule,
        docking_score: f64,
        admet_properties: AdmetProperties,
    ) -> ScoredMolecule {
        let base_score = docking_score * -1.0;
        let penalty = admet_properties.ro5_violations as f64 * 2.5;
        let composite_score = (base_score * admet_properties.qed_estimate) - penalty;

        ScoredMolecule {
            molecule,
            docking_score,
            admet_properties,
            composite_score,
        }
    }

    /// Rank a list of scored molecules by their composite score.
    pub fn rank(&self, mut molecules: Vec<ScoredMolecule>) -> Vec<ScoredMolecule> {
        molecules.sort_by(|a, b| {
            b.composite_score
                .partial_cmp(&a.composite_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        molecules
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admet::AdmetProperties;
    use crate::ligand::Molecule;
    use uuid::Uuid;

    fn create_test_molecule() -> Molecule {
        Molecule::new("CCO", "test")
    }

    fn create_test_admet_properties() -> AdmetProperties {
        AdmetProperties {
            ro5_violations: 0,
            qed_estimate: 0.8,
            mw: 46.07,
            logp: -0.14,
        }
    }

    #[test]
    fn test_molecule_scorer_creation() {
        let scorer = MoleculeScorer::new();
        // Just verify it can be created
        assert!(true);
    }

    #[test]
    fn test_score_molecule_basic() {
        let scorer = MoleculeScorer::new();
        let molecule = create_test_molecule();
        let admet_props = create_test_admet_properties();
        let docking_score = -6.5;

        let scored = scorer.score(molecule.clone(), docking_score, admet_props.clone());

        assert_eq!(scored.molecule.id, molecule.id);
        assert_eq!(scored.docking_score, docking_score);
        assert_eq!(scored.admet_properties.ro5_violations, admet_props.ro5_violations);
        assert_eq!(scored.admet_properties.qed_estimate, admet_props.qed_estimate);

        // Check composite score calculation: (docking_score * -1.0) * qed_estimate - penalty
        // (-6.5 * -1.0) * 0.8 - (0 * 2.5) = 6.5 * 0.8 = 5.2
        assert_eq!(scored.composite_score, 5.2);
    }

    #[test]
    fn test_score_molecule_with_violations() {
        let scorer = MoleculeScorer::new();
        let molecule = create_test_molecule();
        let mut admet_props = create_test_admet_properties();
        admet_props.ro5_violations = 2; // Add violations
        let docking_score = -5.0;

        let scored = scorer.score(molecule, docking_score, admet_props);

        // (-5.0 * -1.0) * 0.8 - (2 * 2.5) = 5.0 * 0.8 - 5.0 = 4.0 - 5.0 = -1.0
        assert_eq!(scored.composite_score, -1.0);
    }

    #[test]
    fn test_rank_molecules() {
        let scorer = MoleculeScorer::new();

        let molecule1 = create_test_molecule();
        let molecule2 = Molecule::new("CCCC", "test");
        let molecule3 = Molecule::new("CCCCC", "test");

        let admet_props1 = create_test_admet_properties();
        let mut admet_props2 = create_test_admet_properties();
        admet_props2.qed_estimate = 0.9; // Higher QED
        let admet_props3 = create_test_admet_properties();

        let scored1 = scorer.score(molecule1, -6.0, admet_props1); // Score: 6.0 * 0.8 = 4.8
        let scored2 = scorer.score(molecule2, -7.0, admet_props2); // Score: 7.0 * 0.9 = 6.3
        let scored3 = scorer.score(molecule3, -5.0, admet_props3); // Score: 5.0 * 0.8 = 4.0

        let mut molecules = vec![scored1, scored2, scored3];
        let ranked = scorer.rank(molecules);

        assert_eq!(ranked.len(), 3);
        // Should be ordered by composite score: scored2 (6.3), scored1 (4.8), scored3 (4.0)
        assert_eq!(ranked[0].composite_score, 6.3);
        assert_eq!(ranked[1].composite_score, 4.8);
        assert_eq!(ranked[2].composite_score, 4.0);
    }

    #[test]
    fn test_rank_empty_list() {
        let scorer = MoleculeScorer::new();
        let molecules: Vec<ScoredMolecule> = vec![];
        let ranked = scorer.rank(molecules);

        assert!(ranked.is_empty());
    }

    #[test]
    fn test_rank_single_molecule() {
        let scorer = MoleculeScorer::new();
        let molecule = create_test_molecule();
        let admet_props = create_test_admet_properties();
        let scored = scorer.score(molecule, -6.0, admet_props);

        let molecules = vec![scored.clone()];
        let ranked = scorer.rank(molecules);

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].composite_score, scored.composite_score);
    }

    #[test]
    fn test_scored_molecule_serialization() {
        let molecule = create_test_molecule();
        let admet_props = create_test_admet_properties();
        let scored = ScoredMolecule {
            molecule: molecule.clone(),
            docking_score: -7.2,
            admet_properties: admet_props.clone(),
            composite_score: 5.76,
        };

        let json = serde_json::to_string(&scored).unwrap();
        assert!(json.contains("CCO"));
        assert!(json.contains("-7.2"));
        assert!(json.contains("5.76"));

        let deserialized: ScoredMolecule = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.molecule.smiles, molecule.smiles);
        assert_eq!(deserialized.docking_score, -7.2);
        assert_eq!(deserialized.composite_score, 5.76);
    }
}
