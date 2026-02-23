//! Biomedical Named Entity Recognition using Candle.
//!
//! Provides Rust-native NER extraction without Python/Docker dependencies.
//! Uses pre-trained token classification models from Hugging Face.

mod ner_model;
mod entity_types;

pub use ner_model::{NerModel, NerConfig, NerEntity};
pub use entity_types::{EntityType, normalize_entity_label};

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
