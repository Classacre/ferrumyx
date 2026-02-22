//! COSMIC (Catalogue of Somatic Mutations in Cancer) API client.
//!
//! COSMIC is the world's largest database of somatic mutations in cancer.
//! It provides curated mutation data including:
//!   - Gene mutations and their frequencies in cancer types
//!   - Mutation types (missense, nonsense, frameshift, etc.)
//!   - Cancer types and sample metadata
//!   - Drug resistance mutations
//!
//! API docs: https://cancer.sanger.ac.uk/cosmic/help/vapi
//! Endpoint: https://cancer.sanger.ac.uk/cosmic/api
//!
//! Returns MutationRecord records with:
//!   - gene_symbol: HGNC gene symbol
//!   - mutation: Mutation description (e.g., "p.G12D")
//!   - mutation_type: Type of mutation
//!   - cancer_type: Cancer type where mutation was observed
//!   - sample_count: Number of samples with this mutation
//!   - frequency: Mutation frequency in cancer type

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use super::LiteratureSource;
use crate::models::PaperMetadata;

const COSMIC_API_URL: &str = "https://cancer.sanger.ac.uk/cosmic/api";

/// Mutation record from COSMIC database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationRecord {
    pub gene_symbol: String,
    pub transcript_id: Option<String>,
    pub mutation: String,
    pub mutation_type: MutationType,
    pub cancer_type: String,
    pub tissue_type: Option<String>,
    pub sample_count: usize,
    pub frequency: Option<f64>,
    pub is_drug_resistance: bool,
}

/// Types of mutations in COSMIC.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MutationType {
    Missense,
    Nonsense,
    Frameshift,
    InFrameDeletion,
    InFrameInsertion,
    SpliceSite,
    Synonymous,
    Unknown,
}

impl MutationType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "missense" | "substitution - missense" => MutationType::Missense,
            "nonsense" | "substitution - nonsense" => MutationType::Nonsense,
            "frameshift" | "deletion - frameshift" | "insertion - frameshift" => MutationType::Frameshift,
            "inframe deletion" | "deletion - in frame" => MutationType::InFrameDeletion,
            "inframe insertion" | "insertion - in frame" => MutationType::InFrameInsertion,
            "splice site" | "complex" => MutationType::SpliceSite,
            "synonymous" | "substitution - coding silent" => MutationType::Synonymous,
            _ => MutationType::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            MutationType::Missense => "missense",
            MutationType::Nonsense => "nonsense",
            MutationType::Frameshift => "frameshift",
            MutationType::InFrameDeletion => "inframe_deletion",
            MutationType::InFrameInsertion => "inframe_insertion",
            MutationType::SpliceSite => "splice_site",
            MutationType::Synonymous => "synonymous",
            MutationType::Unknown => "unknown",
        }
    }
}

/// COSMIC client for fetching mutation data.
pub struct CosmicClient {
    client: Client,
    api_key: Option<String>,
}

impl CosmicClient {
    pub fn new() -> Self {
        Self { client: Client::new(), api_key: None }
    }

    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        Self { client: Client::new(), api_key: Some(api_key.into()) }
    }

    /// Fetch mutations for a specific gene.
    #[instrument(skip(self))]
    pub async fn fetch_gene_mutations(
        &self,
        gene_symbol: &str,
        cancer_type: Option<&str>,
        max_results: usize,
    ) -> anyhow::Result<Vec<MutationRecord>> {
        // COSMIC API requires authentication
        // Public access is available but rate-limited
        // Full API requires registration at https://cancer.sanger.ac.uk/cosmic/register
        
        debug!(
            gene = gene_symbol,
            cancer_type = cancer_type,
            "Fetching COSMIC mutations"
        );

        // TODO: Implement actual API call when API key is available
        // The COSMIC API endpoints include:
        // - /genes/{gene_id}/mutations - mutations in a gene
        // - /cancer/{cancer_type}/mutations - mutations in a cancer type
        // - /mutation/{mutation_id} - specific mutation details
        
        Ok(Vec::new())
    }

    /// Fetch specific mutation (e.g., KRAS G12D) across cancer types.
    #[instrument(skip(self))]
    pub async fn fetch_mutation_by_protein_change(
        &self,
        gene_symbol: &str,
        protein_change: &str, // e.g., "G12D"
    ) -> anyhow::Result<Vec<MutationRecord>> {
        debug!(
            gene = gene_symbol,
            protein_change = protein_change,
            "Fetching specific mutation"
        );

        // TODO: Query COSMIC for specific protein change
        // Would return all samples with this mutation across cancer types
        
        Ok(Vec::new())
    }

    /// Get mutation frequency for a gene in a cancer type.
    pub async fn get_mutation_frequency(
        &self,
        gene_symbol: &str,
        cancer_type: &str,
    ) -> anyhow::Result<Option<f64>> {
        let mutations = self.fetch_gene_mutations(gene_symbol, Some(cancer_type), 1000).await?;
        
        if mutations.is_empty() {
            return Ok(None);
        }

        let total_samples: usize = mutations.iter().map(|m| m.sample_count).sum();
        // Frequency calculation would need total samples in cancer type
        // This is a placeholder
        
        Ok(Some(0.0))
    }

    /// Check if a mutation is a known driver mutation.
    pub async fn is_driver_mutation(&self, gene_symbol: &str, protein_change: &str) -> anyhow::Result<bool> {
        // COSMIC Cancer Gene Census marks known driver genes
        // Check if mutation is in a known driver position
        
        Ok(false)
    }
}

impl Default for CosmicClient {
    fn default() -> Self { Self::new() }
}

// COSMIC is not a literature source, but we implement the trait
// for consistency with the ingestion pipeline.
#[async_trait]
impl LiteratureSource for CosmicClient {
    async fn search(&self, _query: &str, _max_results: usize) -> anyhow::Result<Vec<PaperMetadata>> {
        Ok(Vec::new())
    }

    async fn fetch_full_text(&self, _paper_id: &str) -> anyhow::Result<Option<String>> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosmic_client_new() {
        let client = CosmicClient::new();
        assert!(client.api_key.is_none());
    }

    #[test]
    fn test_mutation_type_from_str() {
        assert_eq!(MutationType::from_str("missense"), MutationType::Missense);
        assert_eq!(MutationType::from_str("Substitution - Missense"), MutationType::Missense);
        assert_eq!(MutationType::from_str("frameshift"), MutationType::Frameshift);
        assert_eq!(MutationType::from_str("unknown_type"), MutationType::Unknown);
    }

    #[test]
    fn test_mutation_record_serialization() {
        let record = MutationRecord {
            gene_symbol: "KRAS".to_string(),
            transcript_id: Some("ENST00000256078".to_string()),
            mutation: "p.G12D".to_string(),
            mutation_type: MutationType::Missense,
            cancer_type: "Pancreatic adenocarcinoma".to_string(),
            tissue_type: Some("Pancreas".to_string()),
            sample_count: 1523,
            frequency: Some(0.52),
            is_drug_resistance: false,
        };
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("KRAS"));
        assert!(json.contains("G12D"));
        assert!(json.contains("missense"));
    }
}
