//! Caching layers for performance optimization

use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::time::Duration;

/// Cache manager for different types of cached data
pub struct CacheManager {
    /// Embedding cache for vector embeddings
    pub embedding_cache: Cache<String, Vec<f32>>,
    /// Query result cache for database queries
    pub query_cache: Cache<String, serde_json::Value>,
    /// LLM response cache
    pub llm_cache: Cache<String, String>,
    /// Paper content cache
    pub paper_cache: Cache<String, String>,
}

impl CacheManager {
    /// Create a new cache manager
    pub async fn new() -> anyhow::Result<Self> {
        // Configure caches with different TTL and size limits based on data type
        let embedding_cache = Cache::builder()
            .max_capacity(10_000) // 10k embeddings
            .time_to_live(Duration::from_secs(3600)) // 1 hour
            .build();

        let query_cache = Cache::builder()
            .max_capacity(5_000) // 5k query results
            .time_to_live(Duration::from_secs(1800)) // 30 minutes
            .build();

        let llm_cache = Cache::builder()
            .max_capacity(2_000) // 2k LLM responses
            .time_to_live(Duration::from_secs(7200)) // 2 hours
            .build();

        let paper_cache = Cache::builder()
            .max_capacity(1_000) // 1k paper contents
            .time_to_live(Duration::from_secs(86400)) // 24 hours
            .build();

        Ok(Self {
            embedding_cache,
            query_cache,
            llm_cache,
            paper_cache,
        })
    }

    /// Get cached embedding
    pub async fn get_embedding(&self, key: &str) -> Option<Vec<f32>> {
        self.embedding_cache.get(key).await
    }

    /// Cache embedding
    pub async fn put_embedding(&self, key: String, embedding: Vec<f32>) {
        self.embedding_cache.insert(key, embedding).await;
    }

    /// Get cached query result
    pub async fn get_query_result(&self, key: &str) -> Option<serde_json::Value> {
        self.query_cache.get(key).await
    }

    /// Cache query result
    pub async fn put_query_result(&self, key: String, result: serde_json::Value) {
        self.query_cache.insert(key, result).await;
    }

    /// Get cached LLM response
    pub async fn get_llm_response(&self, key: &str) -> Option<String> {
        self.llm_cache.get(key).await
    }

    /// Cache LLM response
    pub async fn put_llm_response(&self, key: String, response: String) {
        self.llm_cache.insert(key, response).await;
    }

    /// Get cached paper content
    pub async fn get_paper_content(&self, key: &str) -> Option<String> {
        self.paper_cache.get(key).await
    }

    /// Cache paper content
    pub async fn put_paper_content(&self, key: String, content: String) {
        self.paper_cache.insert(key, content).await;
    }

    /// Clear all caches
    pub async fn clear_all(&self) {
        self.embedding_cache.invalidate_all();
        self.query_cache.invalidate_all();
        self.llm_cache.invalidate_all();
        self.paper_cache.invalidate_all();
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        CacheStats {
            embedding_entries: self.embedding_cache.entry_count(),
            query_entries: self.query_cache.entry_count(),
            llm_entries: self.llm_cache.entry_count(),
            paper_entries: self.paper_cache.entry_count(),
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub embedding_entries: u64,
    pub query_entries: u64,
    pub llm_entries: u64,
    pub paper_entries: u64,
}

/// Generate cache key for embeddings
pub fn embedding_cache_key(text: &str, model: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(text);
    let hash = hasher.finalize();
    format!("emb:{}:{:x}", model, hash)
}

/// Generate cache key for database queries
pub fn query_cache_key(query: &str, params: &[&str]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(params.join(","));
    let params_hash = hasher.finalize();

    let mut hasher = Sha256::new();
    hasher.update(query);
    let query_hash = hasher.finalize();

    format!("query:{:x}:{:x}", query_hash, params_hash)
}

/// Generate cache key for LLM requests
pub fn llm_cache_key(prompt: &str, model: &str, temperature: f32) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(prompt);
    let hash = hasher.finalize();
    format!("llm:{}:{}:{:x}", model, temperature, hash)
}

/// Generate cache key for papers
pub fn paper_cache_key(doi: &str, version: &str) -> String {
    format!("paper:{}:{}", doi, version)
}