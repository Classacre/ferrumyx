//! ADMET prediction for molecules.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use crate::ligand::Molecule;

/// ADMET properties for a molecule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdmetProperties {
    pub ro5_violations: u32,
    pub qed_estimate: f64,
    pub mw: f64,
    pub logp: f64,
}

/// Predictor for ADMET properties.
pub struct AdmetPredictor {}

impl AdmetPredictor {
    /// Create a new AdmetPredictor.
    pub fn new() -> Self {
        Self {}
    }

    /// Predict ADMET properties for a given molecule.
    pub async fn predict(&self, molecule: &Molecule) -> Result<AdmetProperties> {
        let mw = molecule.mw.unwrap_or(400.0);
        let logp = molecule.logp.unwrap_or(3.0);
        
        let mut violations = 0;
        if mw > 500.0 { violations += 1; }
        if logp > 5.0 { violations += 1; }
        if let Some(hbd) = molecule.hbd { if hbd > 5 { violations += 1; } }
        if let Some(hba) = molecule.hba { if hba > 10 { violations += 1; } }

        let qed = 1.0 - (violations as f64 * 0.2).min(0.8);

        Ok(AdmetProperties {
            ro5_violations: violations,
            qed_estimate: qed,
            mw,
            logp,
        })
    }
}
