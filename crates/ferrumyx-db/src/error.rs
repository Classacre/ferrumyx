//! Database error types.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, DbError>;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("LanceDB error: {0}")]
    LanceDb(String),
    
    #[error("Arrow error: {0}")]
    Arrow(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Entity not found: {0}")]
    NotFound(String),
    
    #[error("Duplicate entry: {0}")]
    Duplicate(String),
    
    #[error("Invalid embedding dimension: expected {expected}, got {actual}")]
    InvalidEmbeddingDimension { expected: usize, actual: usize },
    
    #[error("Database not initialized")]
    NotInitialized,
    
    #[error("Table not found: {0}")]
    TableNotFound(String),
    
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
}

impl From<lancedb::Error> for DbError {
    fn from(err: lancedb::Error) -> Self {
        DbError::LanceDb(err.to_string())
    }
}

impl From<arrow_schema::ArrowError> for DbError {
    fn from(err: arrow_schema::ArrowError) -> Self {
        DbError::Arrow(err.to_string())
    }
}
