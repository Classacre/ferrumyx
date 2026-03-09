//! Semantic Scholar literature source client.
//!
//! Currently a placeholder for Supplemental OA link retrieval.

use async_trait::async_trait;
use crate::models::{PaperMetadata, IngestionSource};
use crate::sources::LiteratureSource;

pub struct SemanticScholarClient;

impl SemanticScholarClient {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl LiteratureSource for SemanticScholarClient {
    async fn search(
        &self,
        _query: &str,
        _max_results: usize,
    ) -> anyhow::Result<Vec<PaperMetadata>> {
        // Mock search for now
        Ok(vec![])
    }

    async fn fetch_full_text(
        &self,
        _paper_id: &str,
    ) -> anyhow::Result<Option<String>> {
        Ok(None)
    }
}
