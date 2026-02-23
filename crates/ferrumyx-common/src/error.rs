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

/// API error type for HTTP responses
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => ApiError::NotFound("Resource not found".to_string()),
            _ => ApiError::Internal(err.to_string()),
        }
    }
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            ApiError::NotFound(msg) => (axum::http::StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (axum::http::StatusCode::BAD_REQUEST, msg),
            ApiError::Internal(msg) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        let body = axum::Json(serde_json::json!({ "error": message }));
        (status, body).into_response()
    }
}
