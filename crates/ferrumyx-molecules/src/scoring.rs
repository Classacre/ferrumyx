//! Scoring and ranking of generated molecules.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use crate::ligand::Molecule;
use crate::admet::AdmetProperties;

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
    pub fn score(&self, molecule: Molecule, docking_score: f64, admet_properties: AdmetProperties) -> ScoredMolecule {
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
        molecules.sort_by(|a, b| b.composite_score.partial_cmp(&a.composite_score).unwrap_or(std::cmp::Ordering::Equal));
        molecules
    }
}
