//! Biomedical Named Entity Recognition using Candle.
//!
//! Provides Rust-native NER extraction without Python/Docker dependencies.
//! Uses pre-trained token classification models from Hugging Face.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use ferrumyx_ner::{NerModel, NerConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load a disease NER model
//!     let model = NerModel::new(NerConfig::diseases()).await?;
//!     
//!     // Extract entities
//!     let entities = model.extract("Patient diagnosed with diabetes mellitus.")?;
//!     
//!     for entity in entities {
//!         println!("{}: '{}' (score: {:.2})", entity.label, entity.text, entity.score);
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! # Available Models
//!
//! ## OpenMed DeBERTa-v2 Models (High Performance)
//!
//! - `NerConfig::diseases()` - Disease extraction (F1: 0.912)
//! - `NerConfig::pharmaceuticals()` - Drugs/chemicals (F1: 0.961)
//! - `NerConfig::genomic()` - Genes/proteins (F1: 0.998)
//! - `NerConfig::oncology()` - Cancer entities (F1: 0.899)
//! - `NerConfig::species()` - Organism mentions (F1: 0.965)
//!
//! ## General NER (BERT-based)
//!
//! - `NerConfig::general()` - General NER (CoNLL-2003: PER, ORG, LOC, MISC)
//!
//! See: https://huggingface.co/OpenMed for the full model catalog (380+ models).

mod ner_model;
mod entity_types;
pub mod pipeline;

#[cfg(test)]
mod pipeline_test;

pub use ner_model::{NerModel, NerConfig, NerEntity};
pub use entity_types::{EntityType, normalize_entity_label};
pub use pipeline::{NerPipeline, PipelineEntity};

pub type Result<T> = std::result::Result<T, NerError>;

#[derive(Debug, thiserror::Error)]
pub enum NerError {
    #[error("Model loading failed: {0}")]
    ModelLoad(String),
    
    #[error("Tokenization failed: {0}")]
    Tokenization(String),
    
    #[error("Inference failed: {0}")]
    Inference(String),
    
    #[error("Download failed: {0}")]
    Download(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl From<candle_core::Error> for NerError {
    fn from(e: candle_core::Error) -> Self {
        NerError::Inference(e.to_string())
    }
}

impl From<std::io::Error> for NerError {
    fn from(e: std::io::Error) -> Self {
        NerError::Download(e.to_string())
    }
}
