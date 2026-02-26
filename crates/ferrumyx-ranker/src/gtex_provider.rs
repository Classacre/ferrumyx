//! Trait for GTEx expression specificity data access.

use std::collections::HashMap;

/// Trait for accessing GTEx normal tissue expression.
pub trait GtexProvider: Send + Sync {
    /// Get median gene expression in normal tissues.
    fn get_median_expression(&self, gene_symbol: &str) -> Option<HashMap<String, f64>>;
}

// ── Mock Implementation for Testing ────────────────────────────────────────

pub struct MockGtexProvider {
    data: HashMap<String, HashMap<String, f64>>,
}

impl MockGtexProvider {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn with(mut self, gene: &str, tissue: &str, expression: f64) -> Self {
        let entry = self.data.entry(gene.to_string()).or_default();
        entry.insert(tissue.to_string(), expression);
        self
    }
}

impl Default for MockGtexProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl GtexProvider for MockGtexProvider {
    fn get_median_expression(&self, gene_symbol: &str) -> Option<HashMap<String, f64>> {
        self.data.get(gene_symbol).cloned()
    }
}

// ── Adapter for GtexClient ─────────────────────────────────────────────────

pub struct GtexClientAdapter {
    client: ferrumyx_ingestion::sources::GtexClient,
}

impl GtexClientAdapter {
    pub fn new(client: ferrumyx_ingestion::sources::GtexClient) -> Self {
        Self { client }
    }
}

impl GtexProvider for GtexClientAdapter {
    fn get_median_expression(&self, gene_symbol: &str) -> Option<HashMap<String, f64>> {
        // The API is async, but providers expect sync contexts.
        // We will execute a block_on internally for now.
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let res: anyhow::Result<HashMap<String, f64>> = self.client.get_median_expression(gene_symbol).await;
                res.ok()
            })
        })
    }
}
