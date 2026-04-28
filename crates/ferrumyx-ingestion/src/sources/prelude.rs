//! Common imports for literature source clients.
//! This prelude re-exports items used across all (or most) source implementations.

pub use super::LiteratureSource;
pub use crate::models::{Author, IngestionSource, PaperMetadata};
pub use async_trait::async_trait;
pub use reqwest::Client;
pub use tracing::{debug, info, instrument, warn};
