//! Test fixtures and data generation utilities

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Test data for oncology research
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OncologyTestData {
    pub papers: Vec<PaperFixture>,
    pub targets: Vec<TargetFixture>,
    pub clinical_trials: Vec<ClinicalTrialFixture>,
}

/// Mock paper data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperFixture {
    pub id: Uuid,
    pub title: String,
    pub abstract_text: String,
    pub authors: Vec<String>,
    pub publication_date: DateTime<Utc>,
    pub doi: Option<String>,
    pub pmc_id: Option<String>,
}

/// Mock drug target data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetFixture {
    pub id: Uuid,
    pub gene_symbol: String,
    pub protein_name: String,
    pub cancer_type: String,
    pub confidence_score: f64,
}

/// Mock clinical trial data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClinicalTrialFixture {
    pub id: Uuid,
    pub nct_id: String,
    pub title: String,
    pub phase: String,
    pub status: String,
    pub conditions: Vec<String>,
}

impl Default for OncologyTestData {
    fn default() -> Self {
        Self {
            papers: vec![
                PaperFixture {
                    id: Uuid::new_v4(),
                    title: "KRAS G12D Mutations in Pancreatic Cancer".to_string(),
                    abstract_text: "This study investigates KRAS G12D mutations...".to_string(),
                    authors: vec!["Dr. Smith".to_string(), "Dr. Johnson".to_string()],
                    publication_date: Utc::now(),
                    doi: Some("10.1234/test".to_string()),
                    pmc_id: Some("PMC123456".to_string()),
                }
            ],
            targets: vec![
                TargetFixture {
                    id: Uuid::new_v4(),
                    gene_symbol: "KRAS".to_string(),
                    protein_name: "K-Ras".to_string(),
                    cancer_type: "Pancreatic Adenocarcinoma".to_string(),
                    confidence_score: 0.95,
                }
            ],
            clinical_trials: vec![
                ClinicalTrialFixture {
                    id: Uuid::new_v4(),
                    nct_id: "NCT01234567".to_string(),
                    title: "Phase II Study of KRAS Inhibitor".to_string(),
                    phase: "Phase 2".to_string(),
                    status: "Recruiting".to_string(),
                    conditions: vec!["Pancreatic Cancer".to_string()],
                }
            ],
        }
    }
}

/// Test fixture manager
pub struct TestFixtureManager {
    pub oncology: OncologyTestData,
}

impl TestFixtureManager {
    pub fn new() -> Self {
        Self {
            oncology: OncologyTestData::default(),
        }
    }

    pub fn with_custom_oncology_data(mut self, data: OncologyTestData) -> Self {
        self.oncology = data;
        self
    }
}

impl Default for TestFixtureManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixture_manager_creation() {
        let manager = TestFixtureManager::new();
        assert!(!manager.oncology.papers.is_empty());
        assert!(!manager.oncology.targets.is_empty());
    }
}