use thiserror::Error;

#[derive(Debug, Error)]
pub enum FerrumyxError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("XML parse error: {0}")]
    Xml(String),

    #[error("Entity not found: {0}")]
    EntityNotFound(String),

    #[error("Pipeline error: {0}")]
    Pipeline(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, FerrumyxError>;
