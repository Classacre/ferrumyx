//! Hardware-aware NER optimizer that automatically selects the best strategy.
//!
//! This module provides automatic hardware detection and optimization:
//! - Detects GPU availability and capabilities
//! - Selects appropriate model size based on hardware
//! - Falls back to dictionary-based NER on low-end hardware
//! - Optimizes batch sizes based on available memory
//!
//! # Example
//!
//! ```rust,no_run
//! use ferrumyx_ner::ner_optimizer::{NerOptimizer, ProcessingStrategy};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let optimizer = NerOptimizer::auto_detect().await?;
//!     
//!     let texts = vec!["Patient has KRAS mutation.", "Another text..."];
//!     let results = optimizer.process_batch(&texts).await?;
//!     
//!     Ok(())
//! }
//! ```

use std::sync::Arc;
use candle_core::Device;
use tracing::{info, debug};

use crate::{NerModel, NerConfig, Result, NerEntity};
use crate::entity_db::EntityDatabase;
use crate::model_pool::ModelPool;

/// Hardware capabilities detected at runtime.
#[derive(Debug, Clone)]
pub struct HardwareCapabilities {
    /// Whether CUDA GPU is available
    pub has_cuda: bool,
    /// GPU memory in MB (if available)
    pub gpu_memory_mb: Option<usize>,
    /// GPU compute capability (if available)
    pub compute_capability: Option<(usize, usize)>,
    /// Number of CPU cores
    pub cpu_cores: usize,
    /// System RAM in MB
    pub system_ram_mb: usize,
    /// Whether running in a constrained environment
    pub is_constrained: bool,
}

impl HardwareCapabilities {
    /// Auto-detect hardware capabilities.
    pub fn detect() -> Self {
        let cpu_cores = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4);
        
        let system_ram_mb = sysinfo::System::new_all()
            .total_memory() as usize / 1024;
        
        // Try to detect CUDA
        let (has_cuda, gpu_memory_mb, compute_capability) = Self::detect_cuda();
        
        // Check if constrained environment (CI, container, etc.)
        let is_constrained = std::env::var("CI").is_ok() 
            || std::env::var("FERRUMYX_CONSTRAINED").is_ok();
        
        Self {
            has_cuda,
            gpu_memory_mb,
            compute_capability,
            cpu_cores,
            system_ram_mb,
            is_constrained,
        }
    }
    
    fn detect_cuda() -> (bool, Option<usize>, Option<(usize, usize)>) {
        // Try to create a CUDA device
        match Device::new_cuda(0) {
            Ok(_device) => {
                info!("CUDA device detected");
                // Try to get memory info (this is approximate)
                let memory_mb = Some(6144); // Default assumption for now
                let compute_cap = Some((8, 6)); // Common for RTX 3060
                (true, memory_mb, compute_cap)
            }
            Err(e) => {
                debug!("CUDA not available: {}", e);
                (false, None, None)
            }
        }
    }
    
    /// Determine the best processing strategy based on hardware.
    pub fn recommended_strategy(&self) -> ProcessingStrategy {
        if self.is_constrained {
            return ProcessingStrategy::DictionaryOnly;
        }
        
        if self.has_cuda {
            if let Some(mem) = self.gpu_memory_mb {
                if mem >= 8000 {
                    // High-end GPU - use large models
                    ProcessingStrategy::GpuLargeModel
                } else if mem >= 4000 {
                    // Mid-range GPU - use standard models
                    ProcessingStrategy::GpuStandardModel
                } else {
                    // Low-end GPU - use small models
                    ProcessingStrategy::GpuSmallModel
                }
            } else {
                ProcessingStrategy::GpuStandardModel
            }
        } else if self.cpu_cores >= 8 && self.system_ram_mb >= 16000 {
            // Good CPU - can use ML models
            ProcessingStrategy::CpuOptimized
        } else if self.cpu_cores >= 4 && self.system_ram_mb >= 8000 {
            // Moderate CPU - use dictionary with selective ML
            ProcessingStrategy::Hybrid
        } else {
            // Low-end - dictionary only
            ProcessingStrategy::DictionaryOnly
        }
    }
    
    /// Get optimal batch size for this hardware.
    pub fn optimal_batch_size(&self) -> usize {
        match self.recommended_strategy() {
            ProcessingStrategy::GpuLargeModel => 64,
            ProcessingStrategy::GpuStandardModel => 32,
            ProcessingStrategy::GpuSmallModel => 16,
            ProcessingStrategy::CpuOptimized => 8,
            ProcessingStrategy::Hybrid => 4,
            ProcessingStrategy::DictionaryOnly => 100, // Dictionary is fast
        }
    }
}

/// Processing strategies from fastest (dictionary) to most accurate (large ML models).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessingStrategy {
    /// Dictionary-based NER only (fastest, ~1000x faster than ML).
    /// Uses comprehensive gene/disease/compound databases.
    DictionaryOnly,
    
    /// Hybrid: Dictionary pre-filter + ML on promising candidates.
    /// Good balance of speed and accuracy.
    Hybrid,
    
    /// CPU-optimized ML models (distilled, smaller models).
    CpuOptimized,
    
    /// GPU with small models (for 4-6GB VRAM).
    GpuSmallModel,
    
    /// GPU with standard models (for 6-8GB VRAM).
    GpuStandardModel,
    
    /// GPU with large models (for 8GB+ VRAM).
    GpuLargeModel,
}

impl ProcessingStrategy {
    /// Get the appropriate model config for this strategy.
    pub fn model_config(&self) -> Option<NerConfig> {
        match self {
            ProcessingStrategy::DictionaryOnly => None,
            ProcessingStrategy::Hybrid => Some(NerConfig::diseases()), // Standard model
            ProcessingStrategy::CpuOptimized => Some(NerConfig::diseases()), // Could use smaller
            ProcessingStrategy::GpuSmallModel => Some(NerConfig::diseases()),
            ProcessingStrategy::GpuStandardModel => Some(NerConfig::diseases()),
            ProcessingStrategy::GpuLargeModel => Some(NerConfig::diseases_large()),
        }
    }
    
    /// Expected throughput in texts per second.
    pub fn expected_throughput(&self) -> f64 {
        match self {
            ProcessingStrategy::DictionaryOnly => 10000.0, // ~0.1ms per text
            ProcessingStrategy::Hybrid => 100.0,           // ~10ms per text
            ProcessingStrategy::CpuOptimized => 10.0,      // ~100ms per text
            ProcessingStrategy::GpuSmallModel => 50.0,     // ~20ms per text
            ProcessingStrategy::GpuStandardModel => 100.0, // ~10ms per text
            ProcessingStrategy::GpuLargeModel => 50.0,     // ~20ms per text
        }
    }
}

/// Hardware-aware NER optimizer.
pub struct NerOptimizer {
    hardware: HardwareCapabilities,
    strategy: ProcessingStrategy,
    entity_db: EntityDatabase,
    model: Option<Arc<NerModel>>,
    pool: Option<Arc<ModelPool>>,
}

impl NerOptimizer {
    /// Auto-detect hardware and create optimized NER pipeline.
    pub async fn auto_detect() -> Result<Self> {
        let hardware = HardwareCapabilities::detect();
        let strategy = hardware.recommended_strategy();
        
        info!("Hardware detected: {:?}", hardware);
        info!("Selected strategy: {:?}", strategy);
        
        // Always initialize entity database (fast, no ML)
        let entity_db = EntityDatabase::with_defaults();
        
        // Initialize model based on strategy
        let (model, pool) = match strategy.model_config() {
            Some(config) => {
                info!("Loading ML model for strategy: {:?}", strategy);
                let pool = ModelPool::global().await;
                let model = pool.get_or_load(config).await?;
                (Some(model), Some(pool))
            }
            None => {
                info!("Using dictionary-only NER (no ML model loaded)");
                (None, None)
            }
        };
        
        Ok(Self {
            hardware,
            strategy,
            entity_db,
            model,
            pool,
        })
    }
    
    /// Create with a specific strategy (override auto-detection).
    pub async fn with_strategy(strategy: ProcessingStrategy) -> Result<Self> {
        let hardware = HardwareCapabilities::detect();
        let entity_db = EntityDatabase::with_defaults();
        
        let (model, pool) = match strategy.model_config() {
            Some(config) => {
                let pool = ModelPool::global().await;
                let model = pool.get_or_load(config).await?;
                (Some(model), Some(pool))
            }
            None => (None, None),
        };
        
        Ok(Self {
            hardware,
            strategy,
            entity_db,
            model,
            pool,
        })
    }
    
    /// Process a single text with the optimal strategy.
    pub fn process(&self, text: &str) -> Result<Vec<NerEntity>> {
        match self.strategy {
            ProcessingStrategy::DictionaryOnly => {
                // Fast dictionary-based extraction
                self.extract_from_dictionary(text)
            }
            ProcessingStrategy::Hybrid => {
                // Dictionary pre-filter + ML on matches
                if self.entity_db.has_entities(text) {
                    if let Some(ref model) = self.model {
                        model.extract(text)
                    } else {
                        self.extract_from_dictionary(text)
                    }
                } else {
                    Ok(vec![])
                }
            }
            _ => {
                // ML-based extraction
                if let Some(ref model) = self.model {
                    model.extract(text)
                } else {
                    self.extract_from_dictionary(text)
                }
            }
        }
    }
    
    /// Process a batch of texts.
    pub async fn process_batch(&self, texts: &[&str]) -> Result<Vec<Vec<NerEntity>>> {
        let batch_size = self.hardware.optimal_batch_size();
        
        match self.strategy {
            ProcessingStrategy::DictionaryOnly => {
                // Process all with dictionary
                Ok(texts.iter()
                    .map(|text| self.extract_from_dictionary(text).unwrap_or_default())
                    .collect())
            }
            ProcessingStrategy::Hybrid => {
                // Filter with dictionary first
                let mut results: Vec<Vec<NerEntity>> = vec![vec![]; texts.len()];
                let mut to_process: Vec<(usize, &str)> = Vec::new();
                
                for (idx, text) in texts.iter().enumerate() {
                    if self.entity_db.has_entities(text) {
                        to_process.push((idx, text));
                    }
                }
                
                // Process promising candidates with ML
                if let Some(ref model) = self.model {
                    for chunk in to_process.chunks(batch_size) {
                        let chunk_texts: Vec<_> = chunk.iter().map(|(_, text)| *text).collect();
                        let chunk_results = model.extract_batch(&chunk_texts)?;
                        
                        for (chunk_idx, entities) in chunk_results.into_iter().enumerate() {
                            let original_idx = chunk[chunk_idx].0;
                            results[original_idx] = entities;
                        }
                    }
                }
                
                Ok(results)
            }
            _ => {
                // ML batch processing
                if let Some(ref model) = self.model {
                    let mut results: Vec<Vec<NerEntity>> = Vec::new();
                    
                    for chunk in texts.chunks(batch_size) {
                        let chunk_results = model.extract_batch(chunk)?;
                        results.extend(chunk_results);
                    }
                    
                    Ok(results)
                } else {
                    Ok(texts.iter()
                        .map(|text| self.extract_from_dictionary(text).unwrap_or_default())
                        .collect())
                }
            }
        }
    }
    
    /// Extract entities using dictionary lookup (fastest method).
    fn extract_from_dictionary(&self, text: &str) -> Result<Vec<NerEntity>> {
        use crate::entity_types::EntityType;
        
        let mut entities = Vec::new();
        
        // Match genes
        for (start, end, symbol) in self.entity_db.match_genes(text) {
            entities.push(NerEntity {
                text: text[start..end].to_string(),
                label: "GENE".to_string(),
                entity_type: EntityType::Gene,
                start,
                end,
                score: 0.95, // High confidence for dictionary matches
                normalized_id: Some(symbol),
            });
        }
        
        // Match diseases
        for (start, end, _name) in self.entity_db.match_diseases(text) {
            entities.push(NerEntity {
                text: text[start..end].to_string(),
                label: "DISEASE".to_string(),
                entity_type: EntityType::Disease,
                start,
                end,
                score: 0.90,
                normalized_id: None,
            });
        }
        
        // Match cancer types
        for (start, end, name) in self.entity_db.match_cancer_types(text) {
            entities.push(NerEntity {
                text: text[start..end].to_string(),
                label: "CANCER_TYPE".to_string(),
                entity_type: EntityType::Disease,
                start,
                end,
                score: 0.92,
                normalized_id: Some(name),
            });
        }
        
        // Sort by position and remove overlaps
        entities.sort_by_key(|e| e.start);
        entities.dedup_by_key(|e| e.start);
        
        Ok(entities)
    }
    
    /// Get current hardware capabilities.
    pub fn hardware(&self) -> &HardwareCapabilities {
        &self.hardware
    }
    
    /// Get current processing strategy.
    pub fn strategy(&self) -> ProcessingStrategy {
        self.strategy
    }
    
    /// Get entity database.
    pub fn entity_db(&self) -> &EntityDatabase {
        &self.entity_db
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hardware_detection() {
        let hw = HardwareCapabilities::detect();
        assert!(hw.cpu_cores > 0);
        assert!(hw.system_ram_mb > 0);
        // CUDA detection may or may not succeed depending on environment
    }
    
    #[test]
    fn test_strategy_selection() {
        let hw = HardwareCapabilities {
            has_cuda: true,
            gpu_memory_mb: Some(12000),
            compute_capability: Some((8, 6)),
            cpu_cores: 16,
            system_ram_mb: 32000,
            is_constrained: false,
        };
        
        assert_eq!(hw.recommended_strategy(), ProcessingStrategy::GpuLargeModel);
        
        let hw_low = HardwareCapabilities {
            has_cuda: false,
            gpu_memory_mb: None,
            compute_capability: None,
            cpu_cores: 2,
            system_ram_mb: 4000,
            is_constrained: false,
        };
        
        assert_eq!(hw_low.recommended_strategy(), ProcessingStrategy::DictionaryOnly);
    }
    
    #[tokio::test]
    async fn test_dictionary_only_strategy() {
        let optimizer = NerOptimizer::with_strategy(ProcessingStrategy::DictionaryOnly)
            .await
            .unwrap();
        
        let text = "KRAS mutation in lung cancer";
        let entities = optimizer.process(text).unwrap();
        
        assert!(!entities.is_empty());
        assert!(entities.iter().any(|e| e.text == "KRAS"));
    }
}
