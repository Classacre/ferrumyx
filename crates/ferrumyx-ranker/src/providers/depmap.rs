//! DepMap (Cancer Dependency Map) provider implementation.
//!
//! Provides access to CRISPR-Cas9 gene dependency scores (CERES) from the
//! Broad Institute's DepMap portal.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{info, debug};

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

        if client.data_files_exist() {
            info!("Loading DepMap data from {:?}", client.data_dir);
            client.load_data().await?;
        } else {
            info!("DepMap data not found, downloading...");
            client.download_data().await?;
            client.load_data().await?;
        }

        Ok(client)
    }

    fn default_data_dir() -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("ferrumyx")
            .join("depmap")
    }

    fn data_files_exist(&self) -> bool {
        self.gene_effect_path().exists() && self.model_path().exists()
    }

    fn gene_effect_path(&self) -> PathBuf {
        self.data_dir.join(CRISPR_GENE_EFFECT_FILE)
    }

    fn model_path(&self) -> PathBuf {
        self.data_dir.join(MODEL_FILE)
    }

    async fn download_data(&self) -> Result<()> {
        let client = reqwest::Client::new();
        
        for file in &[CRISPR_GENE_EFFECT_FILE, MODEL_FILE] {
            let url = format!("{}/{}", DEPMAP_DOWNLOAD_URL, file);
            let path = self.data_dir.join(file);
            info!("Downloading {}...", file);
            
            let response = client.get(&url).send().await?;
            if !response.status().is_success() {
                anyhow::bail!("Failed to download {}: HTTP {}", file, response.status());
            }
            
            let content = response.bytes().await?;
            tokio::fs::write(path, content).await?;
        }
        
        Ok(())
    }

    async fn load_data(&mut self) -> Result<()> {
        self.load_model_data().await?;
        self.load_gene_effects().await?;
        Ok(())
    }

    async fn load_model_data(&mut self) -> Result<()> {
        let path = self.model_path();
        let content = tokio::fs::read_to_string(&path).await?;
        let mut reader = csv::Reader::from_reader(content.as_bytes());
        
        for result in reader.records() {
            let record = result?;
            let model_id = record.get(0).map(|s| s.to_string());
            let oncotree_code = record.iter().find(|&s| s.len() == 4 && s.chars().all(|c| c.is_ascii_uppercase()));
            
            if let (Some(id), Some(cancer_type)) = (model_id, oncotree_code) {
                self.cell_line_cancers.insert(id, cancer_type.to_string());
            }
        }
        Ok(())
    }

    async fn load_gene_effects(&mut self) -> Result<()> {
        let path = self.gene_effect_path();
        let content = tokio::fs::read_to_string(&path).await?;
        let mut reader = csv::Reader::from_reader(content.as_bytes());
        
        let headers = reader.headers()?.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let gene_names: Vec<String> = headers.iter().skip(1).cloned().collect();
        
        for result in reader.records() {
            let record = result?;
            let cell_line_id = record.get(0).map(|s| s.to_string());
            if let Some(cell_line_id) = cell_line_id {
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
        }
        Ok(())
    }

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

    pub fn get_mean_ceres(&self, gene: &str, cancer_type: &str) -> Option<f64> {
        let scores = self.get_gene_scores(gene, cancer_type);
        if scores.is_empty() { None } else { Some(scores.iter().sum::<f64>() / scores.len() as f64) }
    }

    pub fn get_median_ceres(&self, gene: &str, cancer_type: &str) -> Option<f64> {
        let mut scores = self.get_gene_scores(gene, cancer_type);
        if scores.is_empty() { None } else {
            scores.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mid = scores.len() / 2;
            if scores.len() % 2 == 0 { Some((scores[mid - 1] + scores[mid]) / 2.0) } else { Some(scores[mid]) }
        }
    }

    pub fn get_top_dependencies(&self, cancer_type: &str, n: usize) -> Vec<(String, f64)> {
        let cancer_upper = cancer_type.to_uppercase();
        let mut gene_means = Vec::new();
        
        for (gene, cell_lines) in &self.gene_effects {
            let scores: Vec<f64> = cell_lines.iter().filter_map(|(id, score)| {
                self.cell_line_cancers.get(id).filter(|ct| ct.to_uppercase() == cancer_upper).map(|_| *score)
            }).collect();
            
            if !scores.is_empty() {
                gene_means.push((gene.clone(), scores.iter().sum::<f64>() / scores.len() as f64));
            }
        }
        gene_means.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        gene_means.truncate(n);
        gene_means
    }

    pub fn has_gene(&self, gene: &str) -> bool {
        self.gene_effects.contains_key(&gene.to_uppercase())
    }

    pub fn cancer_types(&self) -> Vec<String> {
        let mut types: Vec<String> = self.cell_line_cancers.values().cloned().collect();
        types.sort();
        types.dedup();
        types
    }
}
