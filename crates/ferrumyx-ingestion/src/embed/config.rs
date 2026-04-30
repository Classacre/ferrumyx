//! Configuration for the embedding service.

use super::PoolingStrategy;
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
    pub pooling: PoolingStrategy,

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
            model_id: std::env::var("FERRUMYX_EMBED_MODEL_ID")
                .unwrap_or_else(|_| "microsoft/BiomedNLP-BiomedBERT-base-uncased-abstract-fulltext".to_string()),
            max_length: std::env::var("FERRUMYX_EMBED_MAX_LENGTH")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(512),
            batch_size: std::env::var("FERRUMYX_EMBED_BATCH_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(32),
            normalize: std::env::var("FERRUMYX_EMBED_NORMALIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(true),
            pooling: PoolingStrategy::Mean, // Could make this configurable too
            use_gpu: std::env::var("FERRUMYX_EMBED_USE_GPU")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(true),
            cache_dir: std::env::var("FERRUMYX_EMBED_CACHE_DIR").ok(),
            cache_size: std::env::var("FERRUMYX_EMBED_CACHE_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10_000),
        }
    }
}

impl EmbeddingConfig {
    pub fn cpu() -> Self {
        Self {
            use_gpu: false,
            ..Default::default()
        }
    }

    pub fn gpu() -> Self {
        Self {
            use_gpu: true,
            ..Default::default()
        }
    }

    pub fn with_model(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = model_id.into();
        self
    }
}

/// Runtime speed/quality trade-off for the Rust-native embedder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingSpeedMode {
    Fast,
    Balanced,
    Quality,
}

impl EmbeddingSpeedMode {
    pub const ENV_VAR: &'static str = "FERRUMYX_EMBED_SPEED_MODE";

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "fast" => Some(Self::Fast),
            "balanced" => Some(Self::Balanced),
            "quality" => Some(Self::Quality),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Balanced => "balanced",
            Self::Quality => "quality",
        }
    }

    pub fn max_length(self) -> usize {
        match self {
            Self::Fast => 256,
            Self::Balanced => 384,
            Self::Quality => 512,
        }
    }
}
