//! Trait for dependency data access.
//!
//! Provides an abstraction over DepMap data sources, allowing the ranker
//! to query gene dependency scores without being tightly coupled to the
//! ingestion module's implementation.



/// Trait for accessing CRISPR gene dependency data.
///
/// Implementations can use:
/// - DepMap bulk CSV cache (local)
/// - DepMap API (remote)
/// - Mock data (testing)
pub trait DepMapProvider: Send + Sync {
    /// Get mean CERES score for a gene in a cancer type.
    ///
    /// Returns None if:
    /// - Gene not in DepMap
    /// - Cancer type has no cell lines
    /// - No data available for this gene-cancer pair
    fn get_mean_ceres(&self, gene: &str, cancer_type: &str) -> Option<f64>;

    /// Get median CERES score (more robust to outliers).
    fn get_median_ceres(&self, gene: &str, cancer_type: &str) -> Option<f64>;

    /// Get top N dependencies for a cancer type.
    ///
    /// Returns genes ranked by mean CERES (most negative = most essential).
    fn get_top_dependencies(&self, cancer_type: &str, n: usize) -> Vec<(String, f64)>;

    /// Check if a gene has dependency data.
    fn has_gene(&self, gene: &str) -> bool;

    /// Check if a cancer type has cell lines.
    fn has_cancer_type(&self, cancer_type: &str) -> bool;
}

// ── Mock Implementation for Testing ────────────────────────────────────────

/// Mock provider with hardcoded data for unit tests.
pub struct MockDepMapProvider {
    data: std::collections::HashMap<(String, String), f64>,
}

impl MockDepMapProvider {
    pub fn new() -> Self {
        Self {
            data: std::collections::HashMap::new(),
        }
    }

    /// Add a gene-cancer dependency score.
    pub fn with(mut self, gene: &str, cancer_type: &str, ceres: f64) -> Self {
        self.data.insert((gene.to_string(), cancer_type.to_string()), ceres);
        self
    }
}

impl Default for MockDepMapProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DepMapProvider for MockDepMapProvider {
    fn get_mean_ceres(&self, gene: &str, cancer_type: &str) -> Option<f64> {
        self.data.get(&(gene.to_string(), cancer_type.to_string())).copied()
    }

    fn get_median_ceres(&self, gene: &str, cancer_type: &str) -> Option<f64> {
        self.get_mean_ceres(gene, cancer_type)
    }

    fn get_top_dependencies(&self, _cancer_type: &str, _n: usize) -> Vec<(String, f64)> {
        vec![]
    }

    fn has_gene(&self, gene: &str) -> bool {
        self.data.keys().any(|(g, _)| g == gene)
    }

    fn has_cancer_type(&self, cancer_type: &str) -> bool {
        self.data.keys().any(|(_, c)| c == cancer_type)
    }
}

// ── Adapter for DepMapClient ─────────────────────────────────────────────────

/// Adapter that wraps ferrumyx_depmap::DepMapClient to implement DepMapProvider.
///
/// This allows the ranker to use the ferrumyx-depmap crate's client
/// directly for querying gene dependency scores.
pub struct DepMapClientAdapter {
    client: ferrumyx_depmap::DepMapClient,
}

impl DepMapClientAdapter {
    /// Create a new adapter wrapping a DepMapClient.
    pub fn new(client: ferrumyx_depmap::DepMapClient) -> Self {
        Self { client }
    }
    
    /// Create a new adapter by initializing a DepMapClient.
    /// This will download data if not already cached.
    pub async fn init() -> anyhow::Result<Self> {
        let client = ferrumyx_depmap::DepMapClient::new().await?;
        Ok(Self { client })
    }
    
    /// Get the underlying client (for advanced usage).
    pub fn client(&self) -> &ferrumyx_depmap::DepMapClient {
        &self.client
    }
}

impl DepMapProvider for DepMapClientAdapter {
    fn get_mean_ceres(&self, gene: &str, cancer_type: &str) -> Option<f64> {
        self.client.get_mean_ceres(gene, cancer_type)
    }

    fn get_median_ceres(&self, gene: &str, cancer_type: &str) -> Option<f64> {
        self.client.get_median_ceres(gene, cancer_type)
    }

    fn get_top_dependencies(&self, cancer_type: &str, n: usize) -> Vec<(String, f64)> {
        self.client.get_top_dependencies(cancer_type, n)
    }

    fn has_gene(&self, gene: &str) -> bool {
        self.client.has_gene(gene)
    }

    fn has_cancer_type(&self, cancer_type: &str) -> bool {
        self.client.cancer_types().contains(&cancer_type.to_uppercase())
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_provider() {
        let provider = MockDepMapProvider::new()
            .with("KRAS", "PAAD", -1.2)
            .with("TP53", "PAAD", -0.8);

        assert_eq!(provider.get_mean_ceres("KRAS", "PAAD"), Some(-1.2));
        assert_eq!(provider.get_mean_ceres("TP53", "PAAD"), Some(-0.8));
        assert_eq!(provider.get_mean_ceres("MYC", "PAAD"), None);
        assert!(provider.has_gene("KRAS"));
        assert!(!provider.has_gene("MYC"));
    }

}
