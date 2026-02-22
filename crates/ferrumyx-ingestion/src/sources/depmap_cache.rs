//! DepMap bulk data cache manager.
//!
//! Loads and caches DepMap CRISPR gene effect data from bulk CSV downloads.
//! Provides fast in-memory queries for gene dependency scores.
//!
//! Data sources:
//! - CRISPR_gene_effect.csv: Gene effect scores (CERES) for gene x cell line
//! - Model.csv: Cell line metadata including cancer type (Oncotree code)
//!
//! See docs/depmap-integration.md for design rationale.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Gene dependency data from DepMap.
#[derive(Debug, Clone)]
pub struct GeneDependencyRecord {
    pub gene_symbol: String,
    pub cell_line: String,
    pub cancer_type: String,
    pub ceres_score: f64,
}

/// In-memory cache of DepMap CRISPR data.
///
/// Structure:
/// - `gene_effects`: gene_symbol -> (cell_line -> CERES score)
/// - `cell_line_cancers`: cell_line -> Oncotree cancer type code
/// - `cancer_cell_lines`: cancer_type -> set of cell lines (reverse index)
#[derive(Debug, Clone)]
pub struct DepMapCache {
    /// Gene → Cell Line → CERES score
    gene_effects: HashMap<String, HashMap<String, f64>>,
    /// Cell Line → Oncotree code (e.g., "PAAD", "LUAD")
    cell_line_cancers: HashMap<String, String>,
    /// Cancer type → Cell lines (reverse index for fast queries)
    cancer_cell_lines: HashMap<String, Vec<String>>,
    /// When the cache was loaded
    loaded_at: DateTime<Utc>,
    /// Source file paths
    source_files: Vec<PathBuf>,
}

impl DepMapCache {
    /// Load DepMap data from CSV files in the given directory.
    ///
    /// Expects:
    /// - `CRISPR_gene_effect.csv` (required)
    /// - `Model.csv` (required for cancer type mapping)
    ///
    /// If files don't exist, returns an error with download instructions.
    pub fn load_from_dir(data_dir: &Path) -> Result<Self> {
        let gene_effect_file = data_dir.join("CRISPR_gene_effect.csv");
        let model_file = data_dir.join("Model.csv");

        if !gene_effect_file.exists() {
            anyhow::bail!(
                "DepMap data not found at {:?}\n\
                 Download from: https://depmap.org/portal/download/all/\n\
                 Required file: CRISPR_gene_effect.csv",
                gene_effect_file
            );
        }

        if !model_file.exists() {
            anyhow::bail!(
                "DepMap cell line metadata not found at {:?}\n\
                 Download from: https://depmap.org/portal/download/all/\n\
                 Required file: Model.csv",
                model_file
            );
        }

        info!(
            gene_effect_file = %gene_effect_file.display(),
            model_file = %model_file.display(),
            "Loading DepMap data"
        );

        // Load cell line → cancer type mapping
        let cell_line_cancers = load_model_csv(&model_file)?;
        info!(n_cell_lines = cell_line_cancers.len(), "Loaded cell line metadata");

        // Build reverse index: cancer type → cell lines
        let mut cancer_cell_lines: HashMap<String, Vec<String>> = HashMap::new();
        for (cell_line, cancer_type) in &cell_line_cancers {
            cancer_cell_lines
                .entry(cancer_type.clone())
                .or_default()
                .push(cell_line.clone());
        }

        // Load gene effect scores
        let gene_effects = load_gene_effect_csv(&gene_effect_file)?;
        info!(
            n_genes = gene_effects.len(),
            "Loaded CRISPR gene effect data"
        );

        Ok(Self {
            gene_effects,
            cell_line_cancers,
            cancer_cell_lines,
            loaded_at: Utc::now(),
            source_files: vec![gene_effect_file, model_file],
        })
    }

    /// Load from default data directory.
    ///
    /// Default: `data/depmap/` relative to workspace root.
    pub fn load_default() -> Result<Self> {
        let data_dir = PathBuf::from("data/depmap");
        Self::load_from_dir(&data_dir)
    }

    /// Get CERES scores for a gene across all cell lines of a cancer type.
    ///
    /// Returns empty vec if gene or cancer type not found.
    pub fn get_gene_scores(&self, gene: &str, cancer_type: &str) -> Vec<f64> {
        let Some(cell_lines) = self.gene_effects.get(gene) else {
            return vec![];
        };

        let Some(cancer_cell_lines) = self.cancer_cell_lines.get(cancer_type) else {
            return vec![];
        };

        cancer_cell_lines
            .iter()
            .filter_map(|cl| cell_lines.get(cl).copied())
            .collect()
    }

    /// Get mean CERES score for a gene in a cancer type.
    ///
    /// Aggregates across all cell lines of that cancer type.
    /// Returns None if no data available.
    pub fn get_mean_ceres(&self, gene: &str, cancer_type: &str) -> Option<f64> {
        let scores = self.get_gene_scores(gene, cancer_type);
        if scores.is_empty() {
            return None;
        }
        let sum: f64 = scores.iter().sum();
        Some(sum / scores.len() as f64)
    }

    /// Get median CERES score (more robust to outliers).
    pub fn get_median_ceres(&self, gene: &str, cancer_type: &str) -> Option<f64> {
        let mut scores = self.get_gene_scores(gene, cancer_type);
        if scores.is_empty() {
            return None;
        }
        scores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = scores.len() / 2;
        if scores.len() % 2 == 0 {
            Some((scores[mid - 1] + scores[mid]) / 2.0)
        } else {
            Some(scores[mid])
        }
    }

    /// Get top N gene dependencies for a cancer type.
    ///
    /// Returns genes ranked by mean CERES score (most negative = most essential).
    pub fn get_top_dependencies(&self, cancer_type: &str, n: usize) -> Vec<(String, f64)> {
        let Some(cancer_cell_lines) = self.cancer_cell_lines.get(cancer_type) else {
            return vec![];
        };

        // Compute mean CERES for each gene in this cancer type
        let mut gene_means: Vec<(String, f64)> = self
            .gene_effects
            .iter()
            .filter_map(|(gene, cell_lines)| {
                let scores: Vec<f64> = cancer_cell_lines
                    .iter()
                    .filter_map(|cl| cell_lines.get(cl).copied())
                    .collect();
                if scores.is_empty() {
                    return None;
                }
                let mean = scores.iter().sum::<f64>() / scores.len() as f64;
                Some((gene.clone(), mean))
            })
            .collect();

        // Sort by CERES (ascending = more negative = more essential)
        gene_means.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        gene_means.truncate(n);
        gene_means
    }

    /// Check if a gene has dependency data.
    pub fn has_gene(&self, gene: &str) -> bool {
        self.gene_effects.contains_key(gene)
    }

    /// Check if a cancer type has cell lines in DepMap.
    pub fn has_cancer_type(&self, cancer_type: &str) -> bool {
        self.cancer_cell_lines.contains_key(cancer_type)
    }

    /// Get all available cancer types.
    pub fn available_cancer_types(&self) -> Vec<&str> {
        self.cancer_cell_lines.keys().map(|s| s.as_str()).collect()
    }

    /// Get number of genes in cache.
    pub fn gene_count(&self) -> usize {
        self.gene_effects.len()
    }

    /// Get number of cell lines in cache.
    pub fn cell_line_count(&self) -> usize {
        self.cell_line_cancers.len()
    }

    /// Get cache metadata.
    pub fn loaded_at(&self) -> DateTime<Utc> {
        self.loaded_at
    }
}

// ── CSV Parsing ─────────────────────────────────────────────────────────────

/// Load Model.csv to get cell line → cancer type mapping.
///
/// Expected columns:
/// - ModelID: DepMap cell line ID (e.g., "ACH-000001")
/// - OncotreePrimaryDisease: Cancer type name
/// - OncotreeLineage: Broader lineage (optional)
/// - OncotreeCode: Oncotree code (e.g., "PAAD")
fn load_model_csv(path: &Path) -> Result<HashMap<String, String>> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open(path).context("Failed to open Model.csv")?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    // Parse header
    let header = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("Model.csv is empty"))??
        .split(',')
        .map(|s| s.trim().to_string())
        .collect::<Vec<_>>();

    // Find column indices
    let model_id_idx = header
        .iter()
        .position(|c| c == "ModelID")
        .ok_or_else(|| anyhow::anyhow!("Model.csv missing ModelID column"))?;
    let oncotree_idx = header
        .iter()
        .position(|c| c == "OncotreeCode" || c == "OncotreePrimaryDisease")
        .ok_or_else(|| anyhow::anyhow!("Model.csv missing OncotreeCode column"))?;

    let mut mapping = HashMap::new();
    for line in lines {
        let line = line?;
        let cols: Vec<&str> = line.split(',').collect();
        if cols.len() <= oncotree_idx.max(model_id_idx) {
            continue;
        }

        let model_id = cols[model_id_idx].trim();
        let oncotree = cols[oncotree_idx].trim();

        if !model_id.is_empty() && !oncotree.is_empty() {
            mapping.insert(model_id.to_string(), oncotree.to_string());
        }
    }

    Ok(mapping)
}

/// Load CRISPR_gene_effect.csv.
///
/// Format:
/// - First column: gene symbol (e.g., "KRAS (3845)")
/// - Subsequent columns: cell line IDs (header row)
/// - Values: CERES scores (float)
fn load_gene_effect_csv(path: &Path) -> Result<HashMap<String, HashMap<String, f64>>> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open(path).context("Failed to open CRISPR_gene_effect.csv")?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    // Parse header to get cell line IDs
    let header = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("CRISPR_gene_effect.csv is empty"))??
        .split(',')
        .map(|s| s.trim().to_string())
        .collect::<Vec<_>>();

    // First column is gene, rest are cell lines
    let cell_line_ids: Vec<&str> = header[1..].iter().map(|s| s.as_str()).collect();
    debug!(n_cell_lines = cell_line_ids.len(), "Parsed gene effect header");

    let mut gene_effects = HashMap::new();

    for line in lines {
        let line = line?;
        let cols: Vec<&str> = line.split(',').collect();
        if cols.is_empty() {
            continue;
        }

        // Parse gene symbol (format: "GENE (EntrezID)")
        let gene_col = cols[0].trim();
        let gene_symbol = gene_col
            .split(' ')
            .next()
            .unwrap_or(gene_col)
            .to_string();

        // Parse CERES scores for each cell line
        let mut cell_line_scores = HashMap::new();
        for (i, score_str) in cols[1..].iter().enumerate() {
            if i >= cell_line_ids.len() {
                break;
            }
            if let Ok(score) = score_str.trim().parse::<f64>() {
                // Skip NaN-like values
                if score.is_finite() {
                    cell_line_scores.insert(cell_line_ids[i].to_string(), score);
                }
            }
        }

        if !cell_line_scores.is_empty() {
            gene_effects.insert(gene_symbol, cell_line_scores);
        }
    }

    Ok(gene_effects)
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gene_symbol() {
        let gene_col = "KRAS (3845)";
        let symbol = gene_col.split(' ').next().unwrap();
        assert_eq!(symbol, "KRAS");
    }

    #[test]
    fn test_mean_calculation() {
        let scores = vec![-1.0, -0.5, -0.8];
        let mean: f64 = scores.iter().sum::<f64>() / scores.len() as f64;
        assert!((mean - (-0.766666)).abs() < 0.001);
    }

    #[test]
    fn test_median_calculation_odd() {
        let mut scores = vec![-1.0, -0.5, -0.8];
        scores.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mid = scores.len() / 2;
        assert_eq!(scores[mid], -0.8);
    }

    #[test]
    fn test_median_calculation_even() {
        let mut scores = vec![-1.0, -0.5, -0.8, -0.3];
        scores.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mid = scores.len() / 2;
        let median = (scores[mid - 1] + scores[mid]) / 2.0;
        assert_eq!(median, -0.65);
    }
}
