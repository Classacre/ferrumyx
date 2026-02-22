//! Trait for dependency data access.
//!
//! Provides an abstraction over DepMap data sources, allowing the ranker
//! to query gene dependency scores without being tightly coupled to the
//! ingestion module's implementation.

use std::sync::Arc;

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

// ── Adapter for DepMapCache ─────────────────────────────────────────────────

/// Adapter that wraps a DepMapCache to implement DepMapProvider.
///
/// This allows the ranker to use the ingestion module's cache
/// without direct dependency on its concrete type.
pub struct DepMapCacheAdapter {
    // We use a trait object here to avoid circular dependencies.
    // The actual implementation is provided at runtime.
    inner: Arc<dyn DepMapProvider>,
}

impl DepMapCacheAdapter {
    pub fn new(cache: Arc<dyn DepMapProvider>) -> Self {
        Self { inner: cache }
    }
}

impl DepMapProvider for DepMapCacheAdapter {
    fn get_mean_ceres(&self, gene: &str, cancer_type: &str) -> Option<f64> {
        self.inner.get_mean_ceres(gene, cancer_type)
    }

    fn get_median_ceres(&self, gene: &str, cancer_type: &str) -> Option<f64> {
        self.inner.get_median_ceres(gene, cancer_type)
    }

    fn get_top_dependencies(&self, cancer_type: &str, n: usize) -> Vec<(String, f64)> {
        self.inner.get_top_dependencies(cancer_type, n)
    }

    fn has_gene(&self, gene: &str) -> bool {
        self.inner.has_gene(gene)
    }

    fn has_cancer_type(&self, cancer_type: &str) -> bool {
        self.inner.has_cancer_type(cancer_type)
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

    #[test]
    fn test_adapter() {
        let mock = Arc::new(MockDepMapProvider::new().with("KRAS", "PAAD", -1.5));
        let adapter = DepMapCacheAdapter::new(mock);

        assert_eq!(adapter.get_mean_ceres("KRAS", "PAAD"), Some(-1.5));
    }
}
