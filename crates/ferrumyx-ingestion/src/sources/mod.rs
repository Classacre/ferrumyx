//! Literature source clients.

pub mod pubmed;
pub mod europepmc;
pub mod biorxiv;
pub mod clinicaltrials;
pub mod crossref;
pub mod depmap;
pub mod depmap_cache;
pub mod cosmic;
pub mod chembl;
pub mod tcga;
pub mod gtex;
pub mod scihub;

use async_trait::async_trait;
use crate::models::PaperMetadata;

// Re-export types for convenience
pub use depmap::{DepMapClient, GeneDependency};
pub use depmap_cache::DepMapCache;
pub use cosmic::{CosmicClient, MutationRecord, MutationType};
pub use chembl::{ChemblClient, CompoundRecord, TargetRecord, ActivityRecord};
pub use tcga::TcgaClient;
pub use gtex::GtexClient;
pub use scihub::SciHubClient;

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
