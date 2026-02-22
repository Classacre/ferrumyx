//! Target configuration for dynamic research targets.
//!
//! Users can define custom targets via YAML/JSON config or web GUI.
//! This replaces the hardcoded "KRAS G12D Pancreatic Cancer" MVP.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete target investigation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetConfig {
    /// Target gene/mutation specification
    pub target: TargetSpec,
    
    /// Constraints for filtering results
    #[serde(default)]
    pub constraints: Constraints,
    
    /// Data sources to query
    #[serde(default)]
    pub data_sources: DataSourceConfig,
    
    /// Scoring weights
    #[serde(default)]
    pub scoring: ScoringConfig,
    
    /// Output options
    #[serde(default)]
    pub output: OutputConfig,
    
    /// Execution options
    #[serde(default)]
    pub execution: ExecutionConfig,
}

impl Default for TargetConfig {
    fn default() -> Self {
        Self {
            target: TargetSpec::default(),
            constraints: Constraints::default(),
            data_sources: DataSourceConfig::default(),
            scoring: ScoringConfig::default(),
            output: OutputConfig::default(),
            execution: ExecutionConfig::default(),
        }
    }
}

// ── Target Specification ─────────────────────────────────────────────────────

/// Target gene/mutation specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetSpec {
    /// Primary gene symbol (e.g., "KRAS")
    pub gene: String,
    
    /// Optional mutation (e.g., "G12D")
    pub mutation: Option<String>,
    
    /// Cancer type name (e.g., "Pancreatic Adenocarcinoma")
    pub cancer_type: String,
    
    /// Cancer type code for DepMap (e.g., "PAAD")
    pub cancer_code: Option<String>,
    
    /// Alternative: multiple genes for synthetic lethality
    #[serde(default)]
    pub genes: Vec<GeneSpec>,
    
    /// Alternative: pathway-based targeting
    pub pathway: Option<String>,
}

impl Default for TargetSpec {
    fn default() -> Self {
        Self {
            gene: "KRAS".to_string(),
            mutation: Some("G12D".to_string()),
            cancer_type: "Pancreatic Adenocarcinoma".to_string(),
            cancer_code: Some("PAAD".to_string()),
            genes: vec![],
            pathway: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneSpec {
    pub gene: String,
    pub mutation: Option<String>,
}

// ── Constraints ───────────────────────────────────────────────────────────────

/// Constraints for filtering target results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraints {
    /// Minimum druggability score (0.0 - 1.0)
    #[serde(default = "default_min_druggability")]
    pub min_druggability_score: f32,
    
    /// Require protein structure (PDB or AlphaFold)
    #[serde(default)]
    pub require_structure: bool,
    
    /// Minimum CERES score from DepMap (more negative = more essential)
    #[serde(default = "default_min_ceres")]
    pub min_ceres_score: f32,
    
    /// Minimum cancer specificity ratio
    #[serde(default)]
    pub min_cancer_specificity: f32,
    
    /// Minimum supporting papers
    #[serde(default = "default_min_papers")]
    pub min_papers: usize,
    
    /// Maximum publication age in years
    #[serde(default = "default_max_age")]
    pub max_publication_age_years: usize,
    
    /// Exclude targets at these clinical stages
    #[serde(default)]
    pub exclude_clinical_stage: Vec<String>,
}

fn default_min_druggability() -> f32 { 0.5 }
fn default_min_ceres() -> f32 { -0.5 }
fn default_min_papers() -> usize { 10 }
fn default_max_age() -> usize { 5 }

impl Default for Constraints {
    fn default() -> Self {
        Self {
            min_druggability_score: default_min_druggability(),
            require_structure: false,
            min_ceres_score: default_min_ceres(),
            min_cancer_specificity: 0.3,
            min_papers: default_min_papers(),
            max_publication_age_years: default_max_age(),
            exclude_clinical_stage: vec![],
        }
    }
}

// ── Data Sources ──────────────────────────────────────────────────────────────

/// Data source configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceConfig {
    /// Enable PubMed
    #[serde(default = "default_true")]
    pub pubmed: bool,
    
    /// Enable Europe PMC
    #[serde(default = "default_true")]
    pub europe_pmc: bool,
    
    /// Enable bioRxiv/medRxiv
    #[serde(default = "default_true")]
    pub biorxiv: bool,
    
    /// Enable ClinicalTrials.gov
    #[serde(default)]
    pub clinical_trials: bool,
    
    /// Enable DepMap
    #[serde(default = "default_true")]
    pub depmap: bool,
    
    /// Enable COSMIC
    #[serde(default = "default_true")]
    pub cosmic: bool,
    
    /// Enable ChEMBL
    #[serde(default = "default_true")]
    pub chembl: bool,
    
    /// Custom data sources (user-provided files)
    #[serde(default)]
    pub custom: Vec<CustomDataSource>,
}

fn default_true() -> bool { true }

impl Default for DataSourceConfig {
    fn default() -> Self {
        Self {
            pubmed: true,
            europe_pmc: true,
            biorxiv: true,
            clinical_trials: false,
            depmap: true,
            cosmic: true,
            chembl: true,
            custom: vec![],
        }
    }
}

/// Custom user-provided data source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomDataSource {
    /// Name for reference
    pub name: String,
    
    /// File type (csv, json, tsv)
    #[serde(rename = "type")]
    pub file_type: String,
    
    /// Path to file
    pub path: String,
    
    /// Column mapping for the file
    pub mapping: HashMap<String, String>,
}

// ── Scoring Configuration ─────────────────────────────────────────────────────

/// Scoring weights for target prioritization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringConfig {
    /// Weight for CRISPR dependency score (DepMap)
    #[serde(default = "default_crispr_weight")]
    pub crispr_dependency: f32,
    
    /// Weight for mutation frequency (COSMIC)
    #[serde(default = "default_mutation_weight")]
    pub mutation_frequency: f32,
    
    /// Weight for expression dysregulation
    #[serde(default = "default_expression_weight")]
    pub expression_dysregulation: f32,
    
    /// Weight for literature evidence
    #[serde(default = "default_literature_weight")]
    pub literature_evidence: f32,
    
    /// Weight for druggability assessment
    #[serde(default = "default_druggability_weight")]
    pub druggability: f32,
    
    /// Weight for pathway position
    #[serde(default = "default_pathway_weight")]
    pub pathway_position: f32,
    
    /// Weight for clinical status
    #[serde(default = "default_clinical_weight")]
    pub clinical_status: f32,
    
    /// Path to custom scorer plugin (WASM or Rust)
    pub custom_scorer: Option<String>,
}

fn default_crispr_weight() -> f32 { 0.25 }
fn default_mutation_weight() -> f32 { 0.15 }
fn default_expression_weight() -> f32 { 0.15 }
fn default_literature_weight() -> f32 { 0.15 }
fn default_druggability_weight() -> f32 { 0.15 }
fn default_pathway_weight() -> f32 { 0.10 }
fn default_clinical_weight() -> f32 { 0.05 }

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            crispr_dependency: default_crispr_weight(),
            mutation_frequency: default_mutation_weight(),
            expression_dysregulation: default_expression_weight(),
            literature_evidence: default_literature_weight(),
            druggability: default_druggability_weight(),
            pathway_position: default_pathway_weight(),
            clinical_status: default_clinical_weight(),
            custom_scorer: None,
        }
    }
}

impl ScoringConfig {
    /// Validate weights sum to 1.0
    pub fn validate(&self) -> bool {
        let sum = self.crispr_dependency
            + self.mutation_frequency
            + self.expression_dysregulation
            + self.literature_evidence
            + self.druggability
            + self.pathway_position
            + self.clinical_status;
        (sum - 1.0).abs() < 0.01
    }
    
    /// Normalize weights to sum to 1.0
    pub fn normalize(&mut self) {
        let sum = self.crispr_dependency
            + self.mutation_frequency
            + self.expression_dysregulation
            + self.literature_evidence
            + self.druggability
            + self.pathway_position
            + self.clinical_status;
        
        if sum > 0.0 {
            self.crispr_dependency /= sum;
            self.mutation_frequency /= sum;
            self.expression_dysregulation /= sum;
            self.literature_evidence /= sum;
            self.druggability /= sum;
            self.pathway_position /= sum;
            self.clinical_status /= sum;
        }
    }
}

// ── Output Configuration ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Output format (json, csv, html)
    #[serde(default = "default_format")]
    pub format: String,
    
    /// Number of top results to return
    #[serde(default = "default_top_n")]
    pub top_n: usize,
    
    /// Include evidence links
    #[serde(default = "default_true")]
    pub include_evidence: bool,
    
    /// Generate PDF report
    #[serde(default)]
    pub generate_report: bool,
}

fn default_format() -> String { "json".to_string() }
fn default_top_n() -> usize { 50 }

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            format: default_format(),
            top_n: default_top_n(),
            include_evidence: true,
            generate_report: false,
        }
    }
}

// ── Execution Configuration ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Execution mode (full, quick, custom)
    #[serde(default = "default_mode")]
    pub mode: String,
    
    /// Sources for quick mode
    #[serde(default = "default_quick_sources")]
    pub quick_mode_sources: Vec<String>,
    
    /// Maximum runtime in minutes
    #[serde(default = "default_max_runtime")]
    pub max_runtime_minutes: usize,
    
    /// Number of parallel workers
    #[serde(default = "default_workers")]
    pub parallel_workers: usize,
}

fn default_mode() -> String { "full".to_string() }
fn default_quick_sources() -> Vec<String> { vec!["pubmed".to_string(), "depmap".to_string()] }
fn default_max_runtime() -> usize { 60 }
fn default_workers() -> usize { 4 }

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            quick_mode_sources: default_quick_sources(),
            max_runtime_minutes: default_max_runtime(),
            parallel_workers: default_workers(),
        }
    }
}

// ── Helper Methods ─────────────────────────────────────────────────────────────

impl TargetConfig {
    /// Load from YAML file
    pub fn from_yaml(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)?;
        Ok(config)
    }
    
    /// Load from JSON file
    pub fn from_json(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }
    
    /// Save to YAML file
    pub fn to_yaml(&self, path: &str) -> anyhow::Result<()> {
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// Build search query for literature sources
    pub fn build_search_query(&self) -> String {
        let mut parts = vec![
            format!("{}[tiab]", self.target.gene),
            format!("{}[tiab]", self.target.cancer_type),
        ];
        
        if let Some(ref m) = self.target.mutation {
            parts.push(format!("{m}[tiab]"));
        }
        
        parts.join(" AND ")
    }
    
    /// Get list of enabled sources
    pub fn enabled_sources(&self) -> Vec<String> {
        let mut sources = Vec::new();
        
        if self.data_sources.pubmed { sources.push("pubmed".to_string()); }
        if self.data_sources.europe_pmc { sources.push("europe_pmc".to_string()); }
        if self.data_sources.biorxiv { sources.push("biorxiv".to_string()); }
        if self.data_sources.clinical_trials { sources.push("clinical_trials".to_string()); }
        if self.data_sources.depmap { sources.push("depmap".to_string()); }
        if self.data_sources.cosmic { sources.push("cosmic".to_string()); }
        if self.data_sources.chembl { sources.push("chembl".to_string()); }
        
        for custom in &self.data_sources.custom {
            sources.push(custom.name.clone());
        }
        
        sources
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TargetConfig::default();
        assert_eq!(config.target.gene, "KRAS");
        assert_eq!(config.target.mutation, Some("G12D".to_string()));
        assert!(config.data_sources.pubmed);
    }

    #[test]
    fn test_scoring_weights_normalize() {
        let mut config = ScoringConfig::default();
        // Default weights sum to 1.0, so normalize should keep them the same
        let original_crispr = config.crispr_dependency;
        config.normalize();
        assert!((config.crispr_dependency - original_crispr).abs() < 0.01);
        
        // Test with non-normalized weights
        config.crispr_dependency = 0.5;
        config.mutation_frequency = 0.5;
        config.expression_dysregulation = 0.0;
        config.literature_evidence = 0.0;
        config.druggability = 0.0;
        config.pathway_position = 0.0;
        config.clinical_status = 0.0;
        config.normalize();
        assert!((config.crispr_dependency - 0.5).abs() < 0.01);
        assert!((config.mutation_frequency - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_search_query() {
        let config = TargetConfig::default();
        let query = config.build_search_query();
        assert!(query.contains("KRAS[tiab]"));
        assert!(query.contains("G12D[tiab]"));
        assert!(query.contains("Pancreatic Adenocarcinoma[tiab]"));
    }

    #[test]
    fn test_enabled_sources() {
        let config = TargetConfig::default();
        let sources = config.enabled_sources();
        assert!(sources.contains(&"pubmed".to_string()));
        assert!(sources.contains(&"depmap".to_string()));
    }

    #[test]
    fn test_yaml_roundtrip() {
        let config = TargetConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: TargetConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config.target.gene, parsed.target.gene);
    }
}
