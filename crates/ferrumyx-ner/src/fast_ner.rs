//! Fast hybrid NER pipeline for high-throughput literature processing.
//!
//! This module provides a two-stage NER approach:
//! 1. **Rule-based pre-filtering** - Fast regex/dictionary matching to identify relevant texts
//! 2. **ML NER** - Run expensive transformer models only on promising candidates
//!
//! This approach is 10-100x faster than running ML models on every text,
//! making it suitable for processing hundreds of papers.
//!
//! # Example
//!
//! ```rust,no_run
//! use ferrumyx_ner::fast_ner::{FastNerPipeline, FilterConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let pipeline = FastNerPipeline::new(FilterConfig::default()).await?;
//!     
//!     let texts = vec![
//!         "The patient was diagnosed with lung carcinoma.",
//!         "Weather is nice today.",  // Will be filtered out
//!     ];
//!     
//!     // Only runs ML NER on texts matching filter criteria
//!     let results = pipeline.process_batch(&texts).await?;
//!     
//!     Ok(())
//! }
//! ```

use std::collections::HashSet;
use regex::Regex;
use tracing::{debug, info};

use crate::{NerModel, NerConfig, Result, NerEntity};

/// Configuration for rule-based filtering.
#[derive(Debug, Clone)]
pub struct FilterConfig {
    /// Gene symbols to look for (e.g., "KRAS", "TP53", "EGFR")
    pub gene_symbols: HashSet<String>,
    /// Cancer type keywords
    pub cancer_keywords: HashSet<String>,
    /// Drug/compound keywords
    pub drug_keywords: HashSet<String>,
    /// Minimum text length to consider
    pub min_text_length: usize,
    /// Maximum text length (truncate longer)
    pub max_text_length: usize,
    /// Require at least one gene symbol match
    pub require_gene: bool,
    /// Require at least one cancer keyword match
    pub require_cancer: bool,
}

impl Default for FilterConfig {
    fn default() -> Self {
        let mut gene_symbols = HashSet::new();
        // Common oncogenes/tumor suppressors
        for gene in &["KRAS", "TP53", "EGFR", "BRCA1", "BRCA2", "MYC", "PIK3CA", "PTEN", "ALK", "ROS1", "BRAF", "NRAS", "HRAS", "AKT1", "MTOR", "CDKN2A", "RB1", "ATM", "CHEK2", "MLH1", "MSH2", "MSH6", "PMS2"] {
            gene_symbols.insert(gene.to_string());
        }
        
        let mut cancer_keywords = HashSet::new();
        for cancer in &["cancer", "carcinoma", "tumor", "tumour", "neoplasm", "malignancy", "adenocarcinoma", "sarcoma", "lymphoma", "leukemia", "melanoma", "glioma", "blastoma"] {
            cancer_keywords.insert(cancer.to_string());
        }
        
        let mut drug_keywords = HashSet::new();
        for drug in &["inhibitor", "therapy", "treatment", "chemotherapy", "immunotherapy", "targeted therapy", "drug", "compound", "small molecule", "antibody"] {
            drug_keywords.insert(drug.to_string());
        }
        
        Self {
            gene_symbols,
            cancer_keywords,
            drug_keywords,
            min_text_length: 20,  // Lower for tests
            max_text_length: 2000,
            require_gene: false,  // Can match gene OR cancer
            require_cancer: false,
        }
    }
}

impl FilterConfig {
    /// Create a filter config for a specific cancer type.
    pub fn for_cancer_type(cancer_type: &str) -> Self {
        let mut config = Self::default();
        config.cancer_keywords.insert(cancer_type.to_lowercase());
        config.require_cancer = true;
        config
    }
    
    /// Create a filter config for specific genes.
    pub fn for_genes(genes: &[&str]) -> Self {
        let mut config = Self::default();
        config.gene_symbols.clear();
        for gene in genes {
            config.gene_symbols.insert(gene.to_uppercase());
        }
        config.require_gene = true;
        config
    }
}

/// Fast rule-based text filter for pre-screening.
pub struct TextFilter {
    config: FilterConfig,
    gene_regex: Regex,
    cancer_regex: Regex,
    drug_regex: Regex,
}

impl TextFilter {
    /// Create a new text filter with the given configuration.
    pub fn new(config: FilterConfig) -> Self {
        let gene_pattern = config.gene_symbols.iter()
            .map(|g| regex::escape(g))
            .collect::<Vec<_>>()
            .join("|");
        let gene_regex = Regex::new(&format!(r"\b({})\b", gene_pattern)).unwrap();
        
        let cancer_pattern = config.cancer_keywords.iter()
            .map(|c| regex::escape(c))
            .collect::<Vec<_>>()
            .join("|");
        let cancer_regex = Regex::new(&format!(r"(?i)\b({})\b", cancer_pattern)).unwrap();
        
        let drug_pattern = config.drug_keywords.iter()
            .map(|d| regex::escape(d))
            .collect::<Vec<_>>()
            .join("|");
        let drug_regex = Regex::new(&format!(r"(?i)\b({})\b", drug_pattern)).unwrap();
        
        Self {
            config,
            gene_regex,
            cancer_regex,
            drug_regex,
        }
    }
    
    /// Check if a text passes the filter criteria.
    ///
    /// Returns (passes, match_info) where match_info describes what matched.
    pub fn filter(&self, text: &str) -> (bool, FilterMatch) {
        let text = if text.len() > self.config.max_text_length {
            &text[..self.config.max_text_length]
        } else {
            text
        };
        
        if text.len() < self.config.min_text_length {
            return (false, FilterMatch::TooShort);
        }
        
        let has_gene = self.gene_regex.is_match(text);
        let has_cancer = self.cancer_regex.is_match(text);
        let has_drug = self.drug_regex.is_match(text);
        
        let mut matched_genes = Vec::new();
        let mut matched_cancers = Vec::new();
        
        if has_gene {
            for cap in self.gene_regex.captures_iter(text) {
                if let Some(m) = cap.get(1) {
                    matched_genes.push(m.as_str().to_string());
                }
            }
        }
        
        if has_cancer {
            for cap in self.cancer_regex.captures_iter(text) {
                if let Some(m) = cap.get(1) {
                    matched_cancers.push(m.as_str().to_string());
                }
            }
        }
        
        // Check if passes filter criteria
        let passes = if self.config.require_gene && self.config.require_cancer {
            has_gene && has_cancer
        } else if self.config.require_gene {
            has_gene
        } else if self.config.require_cancer {
            has_cancer
        } else {
            has_gene || has_cancer || has_drug
        };
        
        let match_info = if passes {
            FilterMatch::Passed {
                genes: matched_genes,
                cancers: matched_cancers,
                has_drug,
            }
        } else {
            FilterMatch::NoMatch
        };
        
        (passes, match_info)
    }
    
    /// Filter a batch of texts and return indices of passing texts.
    pub fn filter_batch(&self, texts: &[&str]) -> Vec<(usize, FilterMatch)> {
        texts.iter()
            .enumerate()
            .filter_map(|(idx, text)| {
                let (passes, match_info) = self.filter(text);
                if passes {
                    Some((idx, match_info))
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Information about what matched in the filter.
#[derive(Debug, Clone)]
pub enum FilterMatch {
    /// Text passed the filter.
    Passed {
        genes: Vec<String>,
        cancers: Vec<String>,
        has_drug: bool,
    },
    /// No relevant keywords found.
    NoMatch,
    /// Text too short.
    TooShort,
}

/// Fast hybrid NER pipeline.
pub struct FastNerPipeline {
    filter: TextFilter,
    model: Option<NerModel>,
    stats: PipelineStats,
}

/// Pipeline statistics.
#[derive(Debug, Default)]
pub struct PipelineStats {
    pub texts_processed: usize,
    pub texts_filtered: usize,
    pub texts_with_entities: usize,
    pub total_entities: usize,
}

impl FastNerPipeline {
    /// Create a new pipeline with rule-based filtering only (no ML NER).
    pub fn with_filter_only(config: FilterConfig) -> Self {
        Self {
            filter: TextFilter::new(config),
            model: None,
            stats: PipelineStats::default(),
        }
    }
    
    /// Create a new pipeline with both filtering and ML NER.
    pub async fn new(config: FilterConfig) -> Result<Self> {
        let model = NerModel::new(NerConfig::diseases()).await?;
        
        Ok(Self {
            filter: TextFilter::new(config),
            model: Some(model),
            stats: PipelineStats::default(),
        })
    }
    
    /// Create a new pipeline with a specific NER model.
    pub async fn with_model(config: FilterConfig, ner_config: NerConfig) -> Result<Self> {
        let model = NerModel::new(ner_config).await?;
        
        Ok(Self {
            filter: TextFilter::new(config),
            model: Some(model),
            stats: PipelineStats::default(),
        })
    }
    
    /// Process a single text through the pipeline.
    pub fn process(&mut self, text: &str) -> Result<Option<Vec<NerEntity>>> {
        self.stats.texts_processed += 1;
        
        let (passes, match_info) = self.filter.filter(text);
        
        if !passes {
            debug!("Text filtered out: {:?}", match_info);
            return Ok(None);
        }
        
        self.stats.texts_filtered += 1;
        
        // Run ML NER if model is available
        if let Some(ref model) = self.model {
            let entities = model.extract(text)?;
            
            if !entities.is_empty() {
                self.stats.texts_with_entities += 1;
                self.stats.total_entities += entities.len();
            }
            
            Ok(Some(entities))
        } else {
            // No ML model, return empty result
            Ok(Some(Vec::new()))
        }
    }
    
    /// Process a batch of texts through the pipeline.
    pub async fn process_batch(&mut self, texts: &[&str]) -> Result<Vec<Option<Vec<NerEntity>>>> {
        let start = std::time::Instant::now();
        
        // Step 1: Filter texts
        let passed = self.filter.filter_batch(texts);
        info!(
            "Filter passed {}/{} texts ({:.1}%)",
            passed.len(),
            texts.len(),
            100.0 * passed.len() as f64 / texts.len() as f64
        );
        
        // Step 2: Run ML NER on passed texts only
        let mut results: Vec<Option<Vec<NerEntity>>> = vec![None; texts.len()];
        
        if let Some(ref model) = self.model {
            // Extract texts that passed filter
            let texts_to_process: Vec<&str> = passed.iter()
                .map(|(idx, _)| texts[*idx])
                .collect();
            
            // Process in batches for efficiency
            let batch_size = 32;
            for chunk in texts_to_process.chunks(batch_size) {
                let batch_results = model.extract_batch(chunk)?;
                
                for (chunk_idx, entities) in batch_results.into_iter().enumerate() {
                    let original_idx = passed[chunk_idx].0;
                    
                    if !entities.is_empty() {
                        self.stats.texts_with_entities += 1;
                        self.stats.total_entities += entities.len();
                    }
                    
                    results[original_idx] = Some(entities);
                }
            }
        } else {
            // No ML model, just mark as passed
            for (idx, _) in &passed {
                results[*idx] = Some(Vec::new());
            }
        }
        
        self.stats.texts_processed += texts.len();
        self.stats.texts_filtered += passed.len();
        
        info!(
            "Batch processed in {:.2}s ({:.1} texts/sec)",
            start.elapsed().as_secs_f64(),
            texts.len() as f64 / start.elapsed().as_secs_f64()
        );
        
        Ok(results)
    }
    
    /// Get pipeline statistics.
    pub fn stats(&self) -> &PipelineStats {
        &self.stats
    }
    
    /// Reset statistics.
    pub fn reset_stats(&mut self) {
        self.stats = PipelineStats::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_text_filter_passes() {
        let filter = TextFilter::new(FilterConfig::default());
        
        let text = "The patient has KRAS mutation in lung cancer.";
        let (passes, match_info) = filter.filter(text);
        
        assert!(passes);
        match match_info {
            FilterMatch::Passed { genes, cancers, .. } => {
                assert!(genes.contains(&"KRAS".to_string()));
                assert!(cancers.contains(&"cancer".to_string()));
            }
            _ => panic!("Expected Passed match"),
        }
    }
    
    #[test]
    fn test_text_filter_fails() {
        let filter = TextFilter::new(FilterConfig::default());
        
        let text = "The weather is nice today.";
        let (passes, match_info) = filter.filter(text);
        
        assert!(!passes);
        match match_info {
            FilterMatch::NoMatch => {},
            _ => panic!("Expected NoMatch"),
        }
    }
    
    #[test]
    fn test_filter_batch() {
        let filter = TextFilter::new(FilterConfig::default());
        
        let texts = vec![
            "KRAS mutation in cancer.",
            "Weather is nice.",
            "TP53 and EGFR in tumor.",
        ];
        
        let passed = filter.filter_batch(&texts);
        
        assert_eq!(passed.len(), 2);  // First and third should pass
        assert_eq!(passed[0].0, 0);
        assert_eq!(passed[1].0, 2);
    }
}
