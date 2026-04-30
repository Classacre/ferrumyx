//! Ferrumyx Monitoring and Performance Optimization
//!
//! This crate provides comprehensive monitoring, metrics collection, health checks,
//! caching, and performance profiling for the Ferrumyx oncology research system.

pub mod cache;
pub mod health;
pub mod metrics;
pub mod profiling;
pub mod resources;

use std::sync::Arc;
use tokio::sync::RwLock;

/// Global monitoring state
#[derive(Clone)]
pub struct MonitoringState {
    /// Metrics registry
    pub metrics: Arc<metrics::MetricsRegistry>,
    /// Cache manager
    pub cache: Arc<cache::CacheManager>,
    /// Health checker
    pub health: Arc<health::HealthChecker>,
    /// Resource monitor
    pub resources: Arc<resources::ResourceMonitor>,
    /// Performance profiler
    pub profiler: Option<Arc<profiling::Profiler>>,
}

/// Initialize monitoring system
pub async fn init_monitoring() -> anyhow::Result<MonitoringState> {
    let metrics = Arc::new(metrics::MetricsRegistry::new()?);
    let cache = Arc::new(cache::CacheManager::new().await?);
    let health = Arc::new(health::HealthChecker::new());
    let resources = Arc::new(resources::ResourceMonitor::new());

    let profiler = if cfg!(feature = "profiling") {
        Some(Arc::new(profiling::Profiler::new()?))
    } else {
        None
    };

    Ok(MonitoringState {
        metrics,
        cache,
        health,
        resources,
        profiler,
    })
}