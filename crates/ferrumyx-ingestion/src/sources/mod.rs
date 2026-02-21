//! Literature source clients.

pub mod pubmed;
pub mod europepmc;

use async_trait::async_trait;
use crate::models::PaperMetadata;

/// Common interface for all literature source clients.
#[async_trait]
pub trait LiteratureSource: Send + Sync {
    /// Search for papers matching a query, returns metadata list.
    async fn search(
        &self,
        query: &str,
        max_results: usize,
    ) -> anyhow::Result<Vec<PaperMetadata>>;

    /// Fetch full text (XML or PDF URL) for a paper by its source ID.
    async fn fetch_full_text(
        &self,
        paper_id: &str,
    ) -> anyhow::Result<Option<String>>;
}
