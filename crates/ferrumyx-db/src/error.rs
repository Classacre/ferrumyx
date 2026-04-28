//! Database error types.

use thiserror::Error;
use tokio_postgres::error::Error as PgError;

pub type Result<T> = std::result::Result<T, DbError>;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("PostgreSQL error: {0}")]
    Postgres(String),

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

impl From<PgError> for DbError {
    fn from(err: PgError) -> Self {
        DbError::Postgres(err.to_string())
    }
}
