//! ChEMBL API client.
//!
//! ChEMBL is a database of bioactive molecules with drug-like properties.
//! It provides:
//!   - Compound structures and bioactivities
//!   - Target associations and binding data
//!   - Drug mechanisms and indications
//!   - ADMET data
//!
//! API docs: https://chembl.gitbook.io/chembl-interface-documentation/web-resources/chembl-api
//! Endpoint: https://www.ebi.ac.uk/chembl/api/data
//!
//! Returns CompoundRecord and TargetRecord with:
//!   - compound: ChEMBL compound ID, SMILES, properties
//!   - target: Protein target with organism and type
//!   - activity: IC50, Ki, etc. with assay details

use async_trait::async_trait;
use ferrumyx_common::sandbox::SandboxClient as Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use super::LiteratureSource;
use crate::models::PaperMetadata;

const CHEMBL_API_URL: &str = "https://www.ebi.ac.uk/chembl/api/data";

/// Compound record from ChEMBL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompoundRecord {
    pub chembl_id: String,
    pub name: Option<String>,
    pub smiles: Option<String>,
    pub inchi_key: Option<String>,
    pub molecular_weight: Option<f64>,
    pub alogp: Option<f64>,
    pub max_phase: Option<i32>, // Clinical trial phase (0-4)
    pub indication_class: Option<String>,
}

/// Target record from ChEMBL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetRecord {
    pub chembl_id: String,
    pub name: String,
    pub organism: Option<String>,
    pub target_type: String,
    pub gene_names: Vec<String>,
    pub uniprot_id: Option<String>,
}

/// Activity record from ChEMBL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityRecord {
    pub compound_id: String,
    pub target_id: String,
    pub standard_type: String, // IC50, Ki, EC50, etc.
    pub standard_value: f64,
    pub standard_units: String,
    pub assay_type: String,
    pub assay_organism: Option<String>,
    pub pchembl_value: Option<f64>, // -log10(M) normalized value
}

/// ChEMBL client for compound and target data.
pub struct ChemblClient {
    client: Client,
}

impl ChemblClient {
    pub fn new() -> Self {
        Self { client: Client::new().unwrap() }
    }

    /// Fetch compound by ChEMBL ID.
    #[instrument(skip(self))]
    pub async fn fetch_compound(&self, chembl_id: &str) -> anyhow::Result<Option<CompoundRecord>> {
        let url = format!("{}/molecule/{}.json", CHEMBL_API_URL, chembl_id);
        
        debug!(chembl_id = chembl_id, "Fetching ChEMBL compound");
        
        let resp = self.client
            .get(&url)?
            .send()
            .await?;

        if !resp.status().is_success() {
            return Ok(None);
        }

        let json: serde_json::Value = resp.json().await?;
        
        Ok(Some(CompoundRecord {
            chembl_id: json["molecule_chembl_id"].as_str().unwrap_or("").to_string(),
            name: json["pref_name"].as_str().map(String::from),
            smiles: json["molecule_structures"]["canonical_smiles"]
                .as_str().map(String::from),
            inchi_key: json["molecule_structures"]["standard_inchi_key"]
                .as_str().map(String::from),
            molecular_weight: json["molecule_properties"]["mw_freebase"]
                .as_f64(),
            alogp: json["molecule_properties"]["alogp"].as_f64(),
            max_phase: json["max_phase"].as_i64().map(|n| n as i32),
            indication_class: None, // Would need separate query
        }))
    }

    /// Search compounds by SMILES structure similarity.
    #[instrument(skip(self))]
    pub async fn search_compounds_by_similarity(
        &self,
        smiles: &str,
        similarity_threshold: f64,
        max_results: usize,
    ) -> anyhow::Result<Vec<CompoundRecord>> {
        // ChEMBL supports similarity search via the API
        // GET /molecule?molecule_structures__canonical_smiles__flexmatch=<SMILES>
        
        debug!(
            smiles = smiles,
            threshold = similarity_threshold,
            "Searching similar compounds"
        );

        // TODO: Implement similarity search
        // Would use ChEMBL's structure search endpoint
        
        Ok(Vec::new())
    }

    /// Fetch target by ChEMBL ID.
    #[instrument(skip(self))]
    pub async fn fetch_target(&self, chembl_id: &str) -> anyhow::Result<Option<TargetRecord>> {
        let url = format!("{}/target/{}.json", CHEMBL_API_URL, chembl_id);
        
        debug!(chembl_id = chembl_id, "Fetching ChEMBL target");
        
        let resp = self.client
            .get(&url)?
            .send()
            .await?;

        if !resp.status().is_success() {
            return Ok(None);
        }

        let json: serde_json::Value = resp.json().await?;
        
        Ok(Some(TargetRecord {
            chembl_id: json["target_chembl_id"].as_str().unwrap_or("").to_string(),
            name: json["pref_name"].as_str().unwrap_or("").to_string(),
            organism: json["organism"].as_str().map(String::from),
            target_type: json["target_type"].as_str().unwrap_or("").to_string(),
            gene_names: json["target_components"]
                .as_array()
                .map(|arr| arr.iter()
                    .filter_map(|c| c["target_component_synonym"].as_str().map(String::from))
                    .collect())
                .unwrap_or_default(),
            uniprot_id: None, // Would need to parse from target_components
        }))
    }

    /// Search targets by gene name.
    #[instrument(skip(self))]
    pub async fn search_targets_by_gene(
        &self,
        gene_symbol: &str,
    ) -> anyhow::Result<Vec<TargetRecord>> {
        let url = format!("{}/target/search.json", CHEMBL_API_URL);
        
        debug!(gene = gene_symbol, "Searching ChEMBL targets");
        
        let resp = self.client
            .get(&url)?
            .query(&[("q", gene_symbol)])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let json: serde_json::Value = resp.json().await?;
        
        let targets = json["targets"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|t| {
                Some(TargetRecord {
                    chembl_id: t["target_chembl_id"].as_str()?.to_string(),
                    name: t["pref_name"].as_str().unwrap_or("").to_string(),
                    organism: t["organism"].as_str().map(String::from),
                    target_type: t["target_type"].as_str().unwrap_or("").to_string(),
                    gene_names: Vec::new(),
                    uniprot_id: None,
                })
            }).collect())
            .unwrap_or_default();

        Ok(targets)
    }

    /// Fetch activities for a target (compounds tested against it).
    #[instrument(skip(self))]
    pub async fn fetch_target_activities(
        &self,
        target_chembl_id: &str,
        activity_type: Option<&str>, // IC50, Ki, etc.
        max_results: usize,
    ) -> anyhow::Result<Vec<ActivityRecord>> {
        let url = format!("{}/activity.json", CHEMBL_API_URL);
        
        debug!(
            target = target_chembl_id,
            activity_type = activity_type,
            "Fetching target activities"
        );

        let limit_str = max_results.to_string();
        let mut params: Vec<(&str, &str)> = vec![
            ("target_chembl_id", target_chembl_id),
            ("limit", &limit_str),
        ];
        
        if let Some(at) = activity_type {
            params.push(("standard_type", at));
        }

        let resp = self.client
            .get(&url)?
            .query(&params)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let json: serde_json::Value = resp.json().await?;
        
        let activities = json["activities"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|a| {
                Some(ActivityRecord {
                    compound_id: a["molecule_chembl_id"].as_str()?.to_string(),
                    target_id: a["target_chembl_id"].as_str().unwrap_or("").to_string(),
                    standard_type: a["standard_type"].as_str().unwrap_or("").to_string(),
                    standard_value: a["standard_value"].as_f64()?,
                    standard_units: a["standard_units"].as_str().unwrap_or("").to_string(),
                    assay_type: a["assay_type"].as_str().unwrap_or("").to_string(),
                    assay_organism: a["assay_organism"].as_str().map(String::from),
                    pchembl_value: a["pchembl_value"].as_f64(),
                })
            }).collect())
            .unwrap_or_default();

        Ok(activities)
    }

    /// Get approved drugs for a target.
    pub async fn get_approved_drugs(
        &self,
        target_chembl_id: &str,
    ) -> anyhow::Result<Vec<CompoundRecord>> {
        let activities = self.fetch_target_activities(target_chembl_id, None, 1000).await?;
        
        let mut approved = Vec::new();
        for activity in activities {
            if let Some(compound) = self.fetch_compound(&activity.compound_id).await? {
                if compound.max_phase.unwrap_or(0) >= 4 {
                    approved.push(compound);
                }
            }
        }

        Ok(approved)
    }
}

impl Default for ChemblClient {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl LiteratureSource for ChemblClient {
    async fn search(&self, _query: &str, _max_results: usize) -> anyhow::Result<Vec<PaperMetadata>> {
        Ok(Vec::new())
    }

    async fn fetch_full_text(&self, _paper_id: &str) -> anyhow::Result<Option<String>> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chembl_client_new() {
        let client = ChemblClient::new();
        assert!(client.client.get("https://example.com").unwrap().build().is_ok());
    }

    #[test]
    fn test_compound_record_serialization() {
        let compound = CompoundRecord {
            chembl_id: "CHEMBL1201496".to_string(),
            name: Some("Gefitinib".to_string()),
            smiles: Some("COC1=C(C=C2C(=C1)N=CN=C2NC3=CC(=C(C=C3)F)Cl)OCCCN4CCOCC4".to_string()),
            inchi_key: Some("XGALLCUGACUPHD-UHFFFAOYSA-N".to_string()),
            molecular_weight: Some(446.9),
            alogp: Some(3.3),
            max_phase: Some(4),
            indication_class: Some("Antineoplastic".to_string()),
        };
        let json = serde_json::to_string(&compound).unwrap();
        assert!(json.contains("Gefitinib"));
        assert!(json.contains("CHEMBL1201496"));
    }

    #[test]
    fn test_target_record_serialization() {
        let target = TargetRecord {
            chembl_id: "CHEMBL240".to_string(),
            name: "Epidermal growth factor receptor".to_string(),
            organism: Some("Homo sapiens".to_string()),
            target_type: "SINGLE PROTEIN".to_string(),
            gene_names: vec!["EGFR".to_string()],
            uniprot_id: Some("P00533".to_string()),
        };
        let json = serde_json::to_string(&target).unwrap();
        assert!(json.contains("EGFR"));
        assert!(json.contains("Epidermal growth factor receptor"));
    }

    #[test]
    fn test_activity_record_serialization() {
        let activity = ActivityRecord {
            compound_id: "CHEMBL1201496".to_string(),
            target_id: "CHEMBL240".to_string(),
            standard_type: "IC50".to_string(),
            standard_value: 3.0,
            standard_units: "nM".to_string(),
            assay_type: "B".to_string(),
            assay_organism: Some("Homo sapiens".to_string()),
            pchembl_value: Some(8.52),
        };
        let json = serde_json::to_string(&activity).unwrap();
        assert!(json.contains("IC50"));
        assert!(json.contains("8.52"));
    }
}
