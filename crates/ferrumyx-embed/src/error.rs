//! Error types for the embedding service.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, EmbedError>;

#[derive(Error, Debug)]
pub enum EmbedError {
    #[error("Model loading failed: {0}")]
    ModelLoad(String),

    #[error("Tokenizer error: {0}")]
    Tokenizer(String),

    #[error("Inference error: {0}")]
    Inference(String),

    #[error("Device error: {0}")]
    Device(String),

    #[error("Model download failed: {0}")]
    Download(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Hugging Face Hub error: {0}")]
    HfHub(String),
}

impl From<candle_core::Error> for EmbedError {
    fn from(e: candle_core::Error) -> Self {
        EmbedError::Inference(e.to_string())
    }
}

impl From<tokenizers::Error> for EmbedError {
    fn from(e: tokenizers::Error) -> Self {
        EmbedError::Tokenizer(e.to_string())
    }
}

impl From<hf_hub::api::sync::ApiError> for EmbedError {
    fn from(e: hf_hub::api::sync::ApiError) -> Self {
        EmbedError::Download(e.to_string())
    }
}
