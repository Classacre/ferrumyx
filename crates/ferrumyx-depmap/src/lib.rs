//! DepMap (Cancer Dependency Map) integration for target scoring.
//!
//! Provides access to CRISPR-Cas9 gene dependency scores (CERES) from the
//! Broad Institute's DepMap portal. These scores quantify how essential
//! each gene is for cancer cell survival across ~1,000 cell lines.
//!
//! # CERES Score Interpretation
//!
//! | CERES Score | Interpretation |
//! |-------------|----------------|
//! | < -1.0 | Strongly essential (knockout kills cell) |
//! | -1.0 to -0.5 | Moderately essential |
//! | -0.5 to 0 | Weak dependency |
//! | > 0 | Not essential / proliferation advantage |
//!
//! # Example
//!
//! ```rust,no_run
//! use ferrumyx_depmap::DepMapClient;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = DepMapClient::new().await?;
//!     
//!     // Get mean CERES score for KRAS in lung cancer
//!     if let Some(score) = client.get_mean_ceres("KRAS", "LUAD") {
//!         println!("KRAS dependency in LUAD: {}", score);
//!     }
//!     
//!     // Get top 10 essential genes for breast cancer
//!     let top_deps = client.get_top_dependencies("BRCA", 10);
//!     for (gene, score) in top_deps {
//!         println!("{}: {}", gene, score);
//!     }
//!     
//!     Ok(())
//! }
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{info, debug, warn};

/// Default DepMap data URL for bulk downloads
pub const DEPMAP_DOWNLOAD_URL: &str = "https://depmap.org/portal/download/all/";

/// CRISPR gene effect filename
pub const CRISPR_GENE_EFFECT_FILE: &str = "CRISPRGeneEffect.csv";

/// Model (cell line metadata) filename
pub const MODEL_FILE: &str = "Model.csv";

/// A client for accessing DepMap dependency data
#[derive(Debug, Clone)]
pub struct DepMapClient {
    /// Gene effect data: gene_symbol -> cell_line_id -> CERES_score
    gene_effects: HashMap<String, HashMap<String, f64>>,
    /// Cell line metadata: cell_line_id -> cancer_type (OncoTree code)
    cell_line_cancers: HashMap<String, String>,
    /// Data directory path
    data_dir: PathBuf,
}

/// Gene dependency information for a specific cancer type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneDependency {
    pub gene_symbol: String,
    pub cancer_type: String,
    pub mean_ceres: f64,
    pub median_ceres: f64,
    pub std_ceres: f64,
    pub num_cell_lines: usize,
}

impl DepMapClient {
    /// Create a new DepMap client, loading data from the specified directory.
    /// If data doesn't exist, it will be downloaded.
    pub async fn new() -> Result<Self> {
        Self::with_data_dir(Self::default_data_dir()).await
    }

    /// Create a new DepMap client with a specific data directory.
    pub async fn with_data_dir(data_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&data_dir)
            .with_context(|| format!("Failed to create data directory: {:?}", data_dir))?;

        let mut client = Self {
            gene_effects: HashMap::new(),
            cell_line_cancers: HashMap::new(),
            data_dir,
        };

        // Try to load existing data
        if client.data_files_exist() {
            info!("Loading DepMap data from {:?}", client.data_dir);
            client.load_data().await?;
        } else {
            info!("DepMap data not found, downloading...");
            client.download_data().await?;
            client.load_data().await?;
        }

        info!(
            "DepMap client ready: {} genes, {} cell lines",
            client.gene_effects.len(),
            client.cell_line_cancers.len()
        );

        Ok(client)
    }

    /// Get the default data directory
    fn default_data_dir() -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("ferrumyx")
            .join("depmap")
    }

    /// Check if data files exist locally
    fn data_files_exist(&self) -> bool {
        self.gene_effect_path().exists() && self.model_path().exists()
    }

    /// Get path to gene effect CSV
    fn gene_effect_path(&self) -> PathBuf {
        self.data_dir.join(CRISPR_GENE_EFFECT_FILE)
    }

    /// Get path to model CSV
    fn model_path(&self) -> PathBuf {
        self.data_dir.join(MODEL_FILE)
    }

    /// Download DepMap data files
    async fn download_data(&self) -> Result<()> {
        info!("Downloading DepMap data from {}", DEPMAP_DOWNLOAD_URL);
        
        let client = reqwest::Client::new();
        
        // Download CRISPR gene effect file
        let gene_effect_url = format!("{}/{}", DEPMAP_DOWNLOAD_URL, CRISPR_GENE_EFFECT_FILE);
        info!("Downloading {}...", CRISPR_GENE_EFFECT_FILE);
        
        let response = client
            .get(&gene_effect_url)
            .send()
            .await
            .with_context(|| format!("Failed to download {}", CRISPR_GENE_EFFECT_FILE))?;
        
        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to download {}: HTTP {}",
                CRISPR_GENE_EFFECT_FILE,
                response.status()
            );
        }
        
        let content = response.bytes().await?;
        tokio::fs::write(self.gene_effect_path(), content).await?;
        info!("Downloaded {} successfully", CRISPR_GENE_EFFECT_FILE);
        
        // Download Model file
        let model_url = format!("{}/{}", DEPMAP_DOWNLOAD_URL, MODEL_FILE);
        info!("Downloading {}...", MODEL_FILE);
        
        let response = client
            .get(&model_url)
            .send()
            .await
            .with_context(|| format!("Failed to download {}", MODEL_FILE))?;
        
        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to download {}: HTTP {}",
                MODEL_FILE,
                response.status()
            );
        }
        
        let content = response.bytes().await?;
        tokio::fs::write(self.model_path(), content).await?;
        info!("Downloaded {} successfully", MODEL_FILE);
        
        Ok(())
    }

    /// Load data from CSV files
    async fn load_data(&mut self) -> Result<()> {
        self.load_model_data().await?;
        self.load_gene_effects().await?;
        Ok(())
    }

    /// Load cell line metadata from Model.csv
    async fn load_model_data(&mut self) -> Result<()> {
        let path = self.model_path();
        debug!("Loading model data from {:?}", path);
        
        let content = tokio::fs::read_to_string(&path).await?;
        let mut reader = csv::Reader::from_reader(content.as_bytes());
        
        for result in reader.records() {
            let record = result?;
            
            // Model.csv columns: ModelID, PatientID, CellLineName, StrippedCellLineName, ...
            // We need ModelID and OncotreeCode
            let model_id = record.get(0).map(|s| s.to_string());
            let oncotree_code = record.iter().find(|&s| s.len() == 4 && s.chars().all(|c| c.is_ascii_uppercase()));
            
            if let (Some(id), Some(cancer_type)) = (model_id, oncotree_code) {
                self.cell_line_cancers.insert(id, cancer_type.to_string());
            }
        }
        
        info!("Loaded {} cell line mappings", self.cell_line_cancers.len());
        Ok(())
    }

    /// Load gene effect data from CRISPRGeneEffect.csv
    async fn load_gene_effects(&mut self) -> Result<()> {
        let path = self.gene_effect_path();
        debug!("Loading gene effects from {:?}", path);
        
        let content = tokio::fs::read_to_string(&path).await?;
        let mut reader = csv::Reader::from_reader(content.as_bytes());
        
        // First row contains gene symbols as column headers
        let headers = reader.headers()?.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        
        // First column is cell line ID, rest are gene CERES scores
        let gene_names: Vec<String> = headers.iter().skip(1).cloned().collect();
        
        for result in reader.records() {
            let record = result?;
            
            let cell_line_id = record.get(0).map(|s| s.to_string());
            if cell_line_id.is_none() {
                continue;
            }
            let cell_line_id = cell_line_id.unwrap();
            
            for (i, value) in record.iter().skip(1).enumerate() {
                if let Ok(ceres_score) = value.parse::<f64>() {
                    if let Some(gene_name) = gene_names.get(i) {
                        self.gene_effects
                            .entry(gene_name.clone())
                            .or_default()
                            .insert(cell_line_id.clone(), ceres_score);
                    }
                }
            }
        }
        
        info!("Loaded gene effects for {} genes", self.gene_effects.len());
        Ok(())
    }

    /// Get CERES scores for a gene across all cell lines of a cancer type
    pub fn get_gene_scores(&self, gene: &str, cancer_type: &str) -> Vec<f64> {
        let gene_upper = gene.to_uppercase();
        let cancer_upper = cancer_type.to_uppercase();
        
        let mut scores = Vec::new();
        
        if let Some(cell_lines) = self.gene_effects.get(&gene_upper) {
            for (cell_line_id, score) in cell_lines {
                if let Some(cell_cancer) = self.cell_line_cancers.get(cell_line_id) {
                    if cell_cancer.to_uppercase() == cancer_upper {
                        scores.push(*score);
                    }
                }
            }
        }
        
        scores
    }

    /// Get mean CERES score for a gene in a cancer type
    pub fn get_mean_ceres(&self, gene: &str, cancer_type: &str) -> Option<f64> {
        let scores = self.get_gene_scores(gene, cancer_type);
        if scores.is_empty() {
            None
        } else {
            Some(scores.iter().sum::<f64>() / scores.len() as f64)
        }
    }

    /// Get median CERES score for a gene in a cancer type
    pub fn get_median_ceres(&self, gene: &str, cancer_type: &str) -> Option<f64> {
        let mut scores = self.get_gene_scores(gene, cancer_type);
        if scores.is_empty() {
            None
        } else {
            scores.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mid = scores.len() / 2;
            if scores.len() % 2 == 0 {
                Some((scores[mid - 1] + scores[mid]) / 2.0)
            } else {
                Some(scores[mid])
            }
        }
    }

    /// Get detailed dependency information for a gene in a cancer type
    pub fn get_gene_dependency(&self, gene: &str, cancer_type: &str) -> Option<GeneDependency> {
        let scores = self.get_gene_scores(gene, cancer_type);
        if scores.is_empty() {
            return None;
        }
        
        let mean = scores.iter().sum::<f64>() / scores.len() as f64;
        
        let mut sorted_scores = scores.clone();
        sorted_scores.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mid = sorted_scores.len() / 2;
        let median = if sorted_scores.len() % 2 == 0 {
            (sorted_scores[mid - 1] + sorted_scores[mid]) / 2.0
        } else {
            sorted_scores[mid]
        };
        
        let variance = scores.iter()
            .map(|s| (s - mean).powi(2))
            .sum::<f64>() / scores.len() as f64;
        let std = variance.sqrt();
        
        Some(GeneDependency {
            gene_symbol: gene.to_uppercase(),
            cancer_type: cancer_type.to_uppercase(),
            mean_ceres: mean,
            median_ceres: median,
            std_ceres: std,
            num_cell_lines: scores.len(),
        })
    }

    /// Get top N most essential genes for a cancer type
    /// Returns genes with lowest (most negative) CERES scores
    pub fn get_top_dependencies(&self, cancer_type: &str, n: usize) -> Vec<(String, f64)> {
        let cancer_upper = cancer_type.to_uppercase();
        let mut gene_means: Vec<(String, f64)> = Vec::new();
        
        for (gene, cell_lines) in &self.gene_effects {
            let mut scores: Vec<f64> = cell_lines
                .iter()
                .filter_map(|(cell_line_id, score)| {
                    self.cell_line_cancers
                        .get(cell_line_id)
                        .filter(|ct| ct.to_uppercase() == cancer_upper)
                        .map(|_| *score)
                })
                .collect();
            
            if !scores.is_empty() {
                let mean = scores.iter().sum::<f64>() / scores.len() as f64;
                gene_means.push((gene.clone(), mean));
            }
        }
        
        // Sort by CERES score (most negative = most essential first)
        gene_means.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        gene_means.truncate(n);
        
        gene_means
    }

    /// Normalize a CERES score to a 0-1 scale
    /// More negative (more essential) = higher score
    pub fn normalize_ceres(ceres_score: f64) -> f64 {
        let clamped = ceres_score.clamp(-2.0, 0.0);
        let normalized = (clamped + 2.0) / 2.0; // -2.0 -> 0.0, 0.0 -> 1.0
        1.0 - normalized // Invert: essential genes get high scores
    }

    /// Get the number of genes in the dataset
    pub fn gene_count(&self) -> usize {
        self.gene_effects.len()
    }

    /// Get the number of cell lines in the dataset
    pub fn cell_line_count(&self) -> usize {
        self.cell_line_cancers.len()
    }

    /// Check if a gene is available in the dataset
    pub fn has_gene(&self, gene: &str) -> bool {
        self.gene_effects.contains_key(&gene.to_uppercase())
    }

    /// Get all available cancer types
    pub fn cancer_types(&self) -> Vec<String> {
        let mut types: Vec<String> = self.cell_line_cancers
            .values()
            .cloned()
            .collect();
        types.sort();
        types.dedup();
        types
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_ceres() {
        // Most essential genes get highest scores
        assert!((DepMapClient::normalize_ceres(-2.0) - 1.0).abs() < 0.001);
        assert!((DepMapClient::normalize_ceres(-1.0) - 0.5).abs() < 0.001);
        assert!((DepMapClient::normalize_ceres(0.0) - 0.0).abs() < 0.001);
        
        // Values outside range are clamped
        assert!((DepMapClient::normalize_ceres(-3.0) - 1.0).abs() < 0.001);
        assert!((DepMapClient::normalize_ceres(1.0) - 0.0).abs() < 0.001);
    }
}
