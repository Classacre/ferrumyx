//! Ferrumyx Embedding Service
//!
//! Pure Rust BiomedBERT embeddings using Candle (Hugging Face).
//! No Python dependency - direct model loading from Hugging Face Hub.
//!
//! # Features
//! - 768-dim embeddings from microsoft/BiomedNLP-BiomedBERT-base-uncased-abstract
//! - GPU support (CUDA, Metal) with automatic fallback to CPU
//! - Batched inference for throughput
//! - L2-normalized embeddings for cosine similarity
//!
//! # Example
//! ```rust
//! use ferrumyx_embed::{BiomedBertEmbedder, EmbeddingConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let embedder = BiomedBertEmbedder::new(EmbeddingConfig::default()).await?;
//!     
//!     let texts = vec![
//!         "KRAS G12D mutation in pancreatic cancer".to_string(),
//!         "TP53 tumor suppressor gene".to_string(),
//!     ];
//!     
//!     let embeddings = embedder.embed(&texts).await?;
//!     println!("Embedding dimension: {}", embeddings[0].len()); // 768
//!     
//!     Ok(())
//! }
//! ```

pub mod embedder;
pub mod config;
pub mod pooling;
pub mod batch;
pub mod error;

pub use embedder::BiomedBertEmbedder;
pub use config::EmbeddingConfig;
pub use error::{EmbedError, Result};
pub use pooling::PoolingStrategy;
