//! Performance metrics collection and export

use metrics::{counter, gauge, histogram, Counter, Gauge, Histogram};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::sync::Arc;
use std::time::Instant;

/// Metrics registry for collecting performance data
pub struct MetricsRegistry {
    /// Prometheus metrics handle
    prometheus_handle: PrometheusHandle,
}

impl MetricsRegistry {
    /// Create a new metrics registry
    pub fn new() -> anyhow::Result<Self> {
        let builder = PrometheusBuilder::new();
        let handle = builder.install_recorder()?;

        Ok(Self {
            prometheus_handle: handle,
        })
    }

    /// Get Prometheus metrics as string
    pub fn prometheus_metrics(&self) -> String {
        self.prometheus_handle.render()
    }

    /// Record request latency
    pub fn record_request_latency(&self, method: &str, endpoint: &str, duration: f64, status: u16) {
        // For now, record without labels to avoid lifetime issues
        // TODO: Implement proper labeled metrics
        histogram!("ferrumyx_request_duration_seconds").record(duration);
    }

    /// Record LLM call latency
    pub fn record_llm_latency(&self, provider: &str, model: &str, duration: f64, tokens: u32) {
        histogram!("ferrumyx_llm_call_duration_seconds").record(duration);
        histogram!("ferrumyx_llm_tokens_used").record(tokens as f64);
    }

    /// Record database query latency
    pub fn record_db_query_latency(&self, query_type: &str, table: &str, duration: f64) {
        histogram!("ferrumyx_db_query_duration_seconds").record(duration);
    }

    /// Record ingestion performance
    pub fn record_ingestion_metrics(&self, source: &str, papers_processed: u64, duration: f64) {
        counter!("ferrumyx_ingestion_papers_total").increment(papers_processed);
        histogram!("ferrumyx_ingestion_duration_seconds").record(duration);
    }

    /// Record error count
    pub fn record_error(&self, component: &str, error_type: &str) {
        counter!("ferrumyx_errors_total").increment(1);
    }

    /// Record cache hit/miss
    pub fn record_cache_operation(&self, cache_name: &str, hit: bool) {
        counter!("ferrumyx_cache_operations_total").increment(1);
    }

    /// Record throughput
    pub fn record_throughput(&self, component: &str, operations: u64, duration: f64) {
        gauge!("ferrumyx_throughput_ops_per_second").set(operations as f64 / duration);
    }

    /// Record system resource usage
    pub fn record_system_resources(&self, cpu_percent: f64, memory_percent: f64, disk_percent: f64) {
        gauge!("ferrumyx_cpu_usage_percent").set(cpu_percent);
        gauge!("ferrumyx_memory_usage_percent").set(memory_percent);
        gauge!("ferrumyx_disk_usage_percent").set(disk_percent);
    }

    /// Record health check status
    pub fn record_health_check(&self, component: &str, healthy: bool) {
        gauge!("ferrumyx_health_status").set(if healthy { 1.0 } else { 0.0 });
    }
}

/// Request timing helper
pub struct RequestTimer {
    start: Instant,
    method: String,
    endpoint: String,
}

impl RequestTimer {
    pub fn new(method: impl Into<String>, endpoint: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            method: method.into(),
            endpoint: endpoint.into(),
        }
    }

    pub fn finish_with_status(self, status: u16) -> f64 {
        let duration = self.start.elapsed().as_secs_f64();
        // This would normally use the metrics registry, but for now we'll just return the duration
        // In practice, you'd inject the registry here
        duration
    }
}

/// Database query timing helper
pub struct DbQueryTimer {
    start: Instant,
    query_type: String,
    table: String,
}

impl DbQueryTimer {
    pub fn new(query_type: impl Into<String>, table: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            query_type: query_type.into(),
            table: table.into(),
        }
    }

    pub fn finish(self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }
}