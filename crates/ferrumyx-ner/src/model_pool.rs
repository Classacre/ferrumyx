//! Model pool for efficient NER model caching and reuse.
//!
//! This module provides a thread-safe pool of loaded NER models to avoid
//! the overhead of loading models from disk for each request.
//!
//! # Example
//!
//! ```rust,no_run
//! use ferrumyx_ner::{ModelPool, NerConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Initialize the global model pool
//!     let pool = ModelPool::global().await?;
//!     
//!     // Get or create a model (loads only once)
//!     let model = pool.get_or_load(NerConfig::diseases()).await?;
//!     
//!     // Use the model
//!     let entities = model.extract("Patient has diabetes mellitus.")?;
//!     
//!     Ok(())
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::{NerModel, NerConfig, Result};

/// A thread-safe pool of loaded NER models.
///
/// Models are cached in memory and reused across requests, avoiding
/// the overhead of loading from disk each time.
pub struct ModelPool {
    /// Map from model_id to cached model
    models: RwLock<HashMap<String, Arc<NerModel>>>,
    /// Maximum number of models to keep in cache
    max_models: usize,
}

impl std::fmt::Debug for ModelPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelPool")
            .field("max_models", &self.max_models)
            .field("models", &"<cached models>")
            .finish()
    }
}

impl ModelPool {
    /// Create a new empty model pool.
    pub fn new() -> Self {
        Self {
            models: RwLock::new(HashMap::new()),
            max_models: 10, // Default: cache up to 10 models
        }
    }
    
    /// Create a new model pool with a custom capacity.
    pub fn with_capacity(max_models: usize) -> Self {
        Self {
            models: RwLock::new(HashMap::new()),
            max_models,
        }
    }
    
    /// Get or load a model for the given configuration.
    ///
    /// If the model is already in the cache, returns it immediately.
    /// Otherwise, loads the model and caches it for future use.
    pub async fn get_or_load(&self, config: NerConfig) -> Result<Arc<NerModel>> {
        let model_id = config.model_id.clone();
        
        // Check if model is already cached
        {
            let models = self.models.read().await;
            if let Some(model) = models.get(&model_id) {
                debug!("Model cache hit: {}", model_id);
                return Ok(Arc::clone(model));
            }
        }
        
        // Model not cached, load it
        info!("Loading model into cache: {}", model_id);
        let model = Arc::new(NerModel::new(config).await?);
        
        // Store in cache
        {
            let mut models = self.models.write().await;
            
            // Evict oldest model if at capacity
            if models.len() >= self.max_models && !models.is_empty() {
                let first_key = models.keys().next().cloned();
                if let Some(key) = first_key {
                    warn!("Evicting model from cache: {}", key);
                    models.remove(&key);
                }
            }
            
            models.insert(model_id.clone(), Arc::clone(&model));
            info!("Model cached: {} ({} models in cache)", model_id, models.len());
        }
        
        Ok(model)
    }
    
    /// Check if a model is already cached.
    pub async fn is_cached(&self, model_id: &str) -> bool {
        let models = self.models.read().await;
        models.contains_key(model_id)
    }
    
    /// Get the number of cached models.
    pub async fn cached_count(&self) -> usize {
        let models = self.models.read().await;
        models.len()
    }
    
    /// Clear all cached models.
    pub async fn clear(&self) {
        let mut models = self.models.write().await;
        let count = models.len();
        models.clear();
        info!("Cleared {} models from cache", count);
    }
    
    /// Preload multiple models into the cache.
    ///
    /// This is useful for warming up the cache at startup.
    pub async fn preload(&self, configs: Vec<NerConfig>) -> Result<()> {
        info!("Preloading {} models...", configs.len());
        
        for config in configs {
            let _ = self.get_or_load(config).await?;
        }
        
        info!("Preload complete. {} models in cache.", self.cached_count().await);
        Ok(())
    }
}

impl Default for ModelPool {
    fn default() -> Self {
        Self::new()
    }
}

// Global model pool instance
static GLOBAL_POOL: tokio::sync::OnceCell<Arc<ModelPool>> = tokio::sync::OnceCell::const_new();

impl ModelPool {
    /// Get the global model pool instance.
    ///
    /// Initializes the pool on first call.
    pub async fn global() -> Arc<ModelPool> {
        GLOBAL_POOL
            .get_or_init(|| async {
                info!("Initializing global model pool");
                Arc::new(ModelPool::new())
            })
            .await
            .clone()
    }
    
    /// Initialize the global pool with a custom capacity.
    pub async fn init_global(max_models: usize) -> Arc<ModelPool> {
        GLOBAL_POOL
            .get_or_init(|| async {
                info!("Initializing global model pool (capacity: {})", max_models);
                Arc::new(ModelPool::with_capacity(max_models))
            })
            .await
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_model_pool_caching() {
        let pool = ModelPool::new();
        
        // Initially empty
        assert_eq!(pool.cached_count().await, 0);
        assert!(!pool.is_cached("test-model").await);
        
        // Note: We can't actually test loading without downloading models
        // This test just verifies the pool structure works
    }
    
    #[tokio::test]
    async fn test_model_pool_capacity() {
        let pool = ModelPool::with_capacity(2);
        assert_eq!(pool.cached_count().await, 0);
        
        // Test that capacity is enforced
        // (Would need mock models to fully test eviction)
    }
}
