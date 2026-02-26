//! Ligand generation and retrieval.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use ferrumyx_common::sandbox::SandboxClient as Client;
use tracing::{info, debug, warn};

/// A generated or retrieved molecule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Molecule {
    pub id: Uuid,
    pub smiles: String,
    pub inchi_key: Option<String>,
    pub chembl_id: Option<String>,
    pub name: Option<String>,
    pub mw: Option<f64>,
    pub logp: Option<f64>,
    pub hbd: Option<i32>,
    pub hba: Option<i32>,
    pub tpsa: Option<f64>,
    pub sa_score: Option<f64>,
    pub source: String,
    pub parent_id: Option<Uuid>,
}

impl Molecule {
    /// Create a new molecule from SMILES.
    pub fn new(smiles: &str, source: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            smiles: smiles.to_string(),
            inchi_key: None,
            chembl_id: None,
            name: None,
            mw: None,
            logp: None,
            hbd: None,
            hba: None,
            tpsa: None,
            sa_score: None,
            source: source.to_string(),
            parent_id: None,
        }
    }
}

/// Generator for potential ligands.
pub struct LigandGenerator {
    client: Client,
}

#[derive(Deserialize)]
struct ChemblTargetResponse {
    target_components: Vec<ChemblTargetComponent>,
}

#[derive(Deserialize)]
struct ChemblTargetComponent {
    targets: Vec<ChemblTargetData>,
}

#[derive(Deserialize)]
struct ChemblTargetData {
    target_chembl_id: String,
}

#[derive(Deserialize)]
struct ChemblMechanismResponse {
    mechanisms: Vec<ChemblMechanism>,
}

#[derive(Deserialize)]
struct ChemblMechanism {
    molecule_chembl_id: String,
}

#[derive(Deserialize)]
struct ChemblMoleculeResponse {
    molecules: Vec<ChemblMoleculeData>,
}

#[derive(Deserialize)]
struct ChemblMoleculeData {
    molecule_chembl_id: String,
    pref_name: Option<String>,
    molecule_structures: Option<ChemblMoleculeStructures>,
    molecule_properties: Option<ChemblMoleculeProperties>,
}

#[derive(Deserialize)]
struct ChemblMoleculeStructures {
    canonical_smiles: Option<String>,
    standard_inchi_key: Option<String>,
}

#[derive(Deserialize)]
struct ChemblMoleculeProperties {
    full_mwt: Option<String>,
    alogp: Option<String>,
    num_ro5_violations: Option<u32>,
    psa: Option<String>,
}

impl LigandGenerator {
    /// Create a new LigandGenerator.
    pub fn new() -> Self {
        Self {
            client: Client::new().unwrap(),
        }
    }

    /// Generate ligands for a given target pocket / protein.
    /// In this implementation we fetch known inhibitors from ChEMBL for the target.
    pub async fn generate(&self, target_uniprot_id: &str) -> Result<Vec<Molecule>> {
        info!("Fetching ChEMBL ligands for target: {}", target_uniprot_id);
        
        let mut results = Vec::new();
        
        // 1. Get Target ChEMBL ID from UniProt
        let target_url = format!(
            "https://www.ebi.ac.uk/chembl/api/data/target?target_components__accession={}&format=json",
            target_uniprot_id
        );
        let resp = match self.client.get(&target_url)?.send().await {
            Ok(r) => r,
            Err(e) => {
                warn!("Failed to fetch target from ChEMBL: {}", e);
                return Ok(vec![Molecule::new("CC(=O)OC1=CC=CC=C1C(=O)O", "fallback_dummy")]);
            }
        };
        
        let targets_body: serde_json::Value = resp.json().await.unwrap_or_default();
        let target_chembl_id = targets_body["targets"].as_array().and_then(|targets| {
            targets.first().and_then(|t| t["target_chembl_id"].as_str())
        }).map(|s| s.to_string());

        let target_chembl_id = match target_chembl_id {
            Some(id) => id,
            None => {
                warn!("No ChEMBL target found for {}", target_uniprot_id);
                results.push(Molecule::new("CC(=O)OC1=CC=CC=C1C(=O)O", "fallback_dummy"));
                return Ok(results);
            }
        };

        debug!("Resolved target ChEMBL ID: {}", target_chembl_id);

        // 2. Fetch known mechanisms (inhibitors) for this target
        let mech_url = format!(
            "https://www.ebi.ac.uk/chembl/api/data/mechanism?target_chembl_id={}&format=json&limit=5",
            target_chembl_id
        );
        let mech_resp = match self.client.get(&mech_url)?.send().await {
            Ok(r) => r,
            Err(_) => return Ok(results),
        };
        
        let mech_body: ChemblMechanismResponse = match mech_resp.json().await {
            Ok(m) => m,
            Err(_) => return Ok(results),
        };

        if mech_body.mechanisms.is_empty() {
             results.push(Molecule::new("CC(=O)OC1=CC=CC=C1C(=O)O", "fallback_dummy"));
             return Ok(results);
        }

        // 3. Fetch molecule details
        for mech in mech_body.mechanisms.iter().take(5) {
            let mol_url = format!(
                "https://www.ebi.ac.uk/chembl/api/data/molecule?molecule_chembl_id={}&format=json",
                mech.molecule_chembl_id
            );
            
            if let Ok(resp) = self.client.get(&mol_url)?.send().await {
                if let Ok(mol_body) = resp.json::<ChemblMoleculeResponse>().await {
                    if let Some(mol_data) = mol_body.molecules.first() {
                        if let Some(structs) = &mol_data.molecule_structures {
                            if let Some(smiles) = &structs.canonical_smiles {
                                let mut m = Molecule::new(smiles, "chembl");
                                m.chembl_id = Some(mol_data.molecule_chembl_id.clone());
                                m.name = mol_data.pref_name.clone();
                                m.inchi_key = structs.standard_inchi_key.clone();
                                
                                if let Some(props) = &mol_data.molecule_properties {
                                    m.mw = props.full_mwt.as_ref().and_then(|s| s.parse().ok());
                                    m.logp = props.alogp.as_ref().and_then(|s| s.parse().ok());
                                    m.tpsa = props.psa.as_ref().and_then(|s| s.parse().ok());
                                }
                                
                                results.push(m);
                            }
                        }
                    }
                }
            }
        }
        
        if results.is_empty() {
            results.push(Molecule::new("CC(=O)OC1=CC=CC=C1C(=O)O", "fallback_dummy"));
        }

        Ok(results)
    }
}
