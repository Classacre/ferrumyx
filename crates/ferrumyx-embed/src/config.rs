//! Configuration for the embedding service.

use serde::{Deserialize, Serialize};

/// Configuration for BiomedBERT embedder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Hugging Face model ID
    pub model_id: String,

    /// Maximum sequence length (default: 512)
    pub max_length: usize,

    /// Batch size for inference (default: 32)
    pub batch_size: usize,

    /// L2-normalize embeddings (default: true)
    pub normalize: bool,

    /// Pooling strategy (default: mean)
    pub pooling: super::PoolingStrategy,

    /// Use GPU if available (default: true)
    pub use_gpu: bool,

    /// Cache directory for downloaded models
    pub cache_dir: Option<String>,

    /// Maximum cache size for embeddings (number of entries)
    pub cache_size: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model_id: "NeuML/pubmedbert-base-embeddings".to_string(),
            max_length: 512,
            batch_size: 32,
            normalize: true,
            pooling: super::PoolingStrategy::Mean,
            use_gpu: true,
            cache_dir: None,
            cache_size: 10_000,
        }
    }
}

impl EmbeddingConfig {
    /// Create config for CPU-only inference.
    pub fn cpu() -> Self {
        Self {
            use_gpu: false,
            ..Default::default()
        }
    }

    /// Create config for GPU inference.
    pub fn gpu() -> Self {
        Self {
            use_gpu: true,
            ..Default::default()
        }
    }

    /// Use a custom model.
    pub fn with_model(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = model_id.into();
        self
    }

    /// Set batch size.
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Set maximum sequence length.
    pub fn with_max_length(mut self, length: usize) -> Self {
        self.max_length = length;
        self
    }
}
