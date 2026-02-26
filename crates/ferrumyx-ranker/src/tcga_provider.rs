//! Trait for TCGA survival correlation data access.

/// Trait for accessing TCGA survival correlations.
pub trait TcgaProvider: Send + Sync {
    /// Get survival correlation score for gene in a cancer type.
    fn get_survival_correlation(&self, gene_symbol: &str, cancer_type: &str) -> Option<f64>;
}

// ── Mock Implementation for Testing ────────────────────────────────────────

pub struct MockTcgaProvider {
    data: std::collections::HashMap<(String, String), f64>,
}

impl MockTcgaProvider {
    pub fn new() -> Self {
        Self {
            data: std::collections::HashMap::new(),
        }
    }

    pub fn with(mut self, gene: &str, cancer_type: &str, correlation: f64) -> Self {
        self.data.insert((gene.to_string(), cancer_type.to_string()), correlation);
        self
    }
}

impl Default for MockTcgaProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl TcgaProvider for MockTcgaProvider {
    fn get_survival_correlation(&self, gene_symbol: &str, cancer_type: &str) -> Option<f64> {
        self.data.get(&(gene_symbol.to_string(), cancer_type.to_string())).copied()
    }
}

// ── Adapter for TcgaClient ─────────────────────────────────────────────────

pub struct TcgaClientAdapter {
    client: ferrumyx_ingestion::sources::TcgaClient,
}

impl TcgaClientAdapter {
    pub fn new(client: ferrumyx_ingestion::sources::TcgaClient) -> Self {
        Self { client }
    }
}

impl TcgaProvider for TcgaClientAdapter {
    fn get_survival_correlation(&self, gene_symbol: &str, cancer_type: &str) -> Option<f64> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let res: anyhow::Result<Option<f64>> = self.client.get_survival_correlation(gene_symbol, cancer_type).await;
                res.unwrap_or(None)
            })
        })
    }
}
