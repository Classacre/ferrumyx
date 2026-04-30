//! Performance tuning configuration
//!
//! Batch sizes, timeouts, limits, caching, and other performance-related settings.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Batch processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Maximum batch size
    #[serde(default = "default_batch_size")]
    pub max_size: usize,

    /// Batch timeout
    #[serde(default = "default_batch_timeout")]
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,

    /// Enable adaptive batching
    #[serde(default)]
    pub adaptive: bool,

    /// Minimum batch size for processing
    #[serde(default = "default_min_batch_size")]
    pub min_size: usize,

    /// Maximum wait time for batch completion
    #[serde(default = "default_max_wait_time")]
    #[serde(with = "humantime_serde")]
    pub max_wait_time: Duration,
}

/// Timeout configuration for various operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Database query timeout
    #[serde(default = "default_db_timeout")]
    #[serde(with = "humantime_serde")]
    pub database: Duration,

    /// HTTP request timeout
    #[serde(default = "default_http_timeout")]
    #[serde(with = "humantime_serde")]
    pub http: Duration,

    /// File I/O timeout
    #[serde(default = "default_io_timeout")]
    #[serde(with = "humantime_serde")]
    pub io: Duration,

    /// External API timeout
    #[serde(default = "default_api_timeout")]
    #[serde(with = "humantime_serde")]
    pub api: Duration,

    /// Long-running task timeout
    #[serde(default = "default_task_timeout")]
    #[serde(with = "humantime_serde")]
    pub task: Duration,
}

/// Resource limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsConfig {
    /// Maximum memory usage (bytes)
    #[serde(default = "default_max_memory")]
    pub max_memory_bytes: u64,

    /// Maximum CPU usage (percentage)
    #[serde(default = "default_max_cpu")]
    pub max_cpu_percent: f32,

    /// Maximum concurrent connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// Maximum file size for uploads (bytes)
    #[serde(default = "default_max_file_size")]
    pub max_file_size_bytes: u64,

    /// Maximum request body size (bytes)
    #[serde(default = "default_max_request_size")]
    pub max_request_size_bytes: u64,

    /// Rate limit for requests per second
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_second: u32,
}

/// Caching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable caching
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Cache backend (memory, redis, disk)
    #[serde(default = "default_cache_backend")]
    pub backend: String,

    /// Default TTL for cache entries
    #[serde(default = "default_cache_ttl")]
    #[serde(with = "humantime_serde")]
    pub default_ttl: Duration,

    /// Maximum cache size
    #[serde(default = "default_max_cache_size")]
    pub max_size_bytes: u64,

    /// Cache compression
    #[serde(default)]
    pub compression: bool,

    /// Cache warming on startup
    #[serde(default)]
    pub warming: bool,
}

/// Worker pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Number of worker threads
    #[serde(default = "default_worker_threads")]
    pub threads: usize,

    /// Worker queue size
    #[serde(default = "default_queue_size")]
    pub queue_size: usize,

    /// Thread pool configuration
    #[serde(default)]
    pub pool: ThreadPoolConfig,

    /// Task scheduling
    #[serde(default)]
    pub scheduling: SchedulingConfig,
}

/// Thread pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadPoolConfig {
    /// Core pool size
    #[serde(default = "default_core_pool_size")]
    pub core_size: usize,

    /// Maximum pool size
    #[serde(default = "default_max_pool_size")]
    pub max_size: usize,

    /// Keep alive time for idle threads
    #[serde(default = "default_keep_alive")]
    #[serde(with = "humantime_serde")]
    pub keep_alive: Duration,

    /// Allow core thread timeout
    #[serde(default)]
    pub allow_core_timeout: bool,
}

/// Task scheduling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingConfig {
    /// Enable task scheduling
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Maximum concurrent tasks
    #[serde(default = "default_max_concurrent_tasks")]
    pub max_concurrent: usize,

    /// Task priority levels
    #[serde(default = "default_priority_levels")]
    pub priority_levels: usize,

    /// Fair scheduling
    #[serde(default)]
    pub fair_scheduling: bool,
}

/// Performance monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable performance monitoring
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Metrics collection interval
    #[serde(default = "default_metrics_interval")]
    #[serde(with = "humantime_serde")]
    pub metrics_interval: Duration,

    /// Enable profiling
    #[serde(default)]
    pub profiling: bool,

    /// Profiling sample rate
    #[serde(default = "default_sample_rate")]
    pub sample_rate: f32,

    /// Slow query threshold
    #[serde(default = "default_slow_query_threshold")]
    #[serde(with = "humantime_serde")]
    pub slow_query_threshold: Duration,

    /// Alert thresholds
    #[serde(default)]
    pub alerts: AlertThresholds,
}

/// Performance alert thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// CPU usage threshold (percentage)
    #[serde(default = "default_cpu_threshold")]
    pub cpu_usage_percent: f32,

    /// Memory usage threshold (percentage)
    #[serde(default = "default_memory_threshold")]
    pub memory_usage_percent: f32,

    /// Response time threshold (milliseconds)
    #[serde(default = "default_response_time_threshold")]
    pub response_time_ms: u64,

    /// Error rate threshold (percentage)
    #[serde(default = "default_error_rate_threshold")]
    pub error_rate_percent: f32,

    /// Queue depth threshold
    #[serde(default = "default_queue_depth_threshold")]
    pub queue_depth: usize,
}

/// Performance configuration unifying all performance settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Batch processing settings
    #[serde(default)]
    pub batch: BatchConfig,

    /// Timeout settings
    #[serde(default)]
    pub timeout: TimeoutConfig,

    /// Resource limits
    #[serde(default)]
    pub limits: LimitsConfig,

    /// Caching settings
    #[serde(default)]
    pub cache: CacheConfig,

    /// Worker pool settings
    #[serde(default)]
    pub workers: WorkerConfig,

    /// Performance monitoring
    #[serde(default)]
    pub monitoring: MonitoringConfig,

    /// Optimization flags
    #[serde(default)]
    pub optimization: OptimizationConfig,
}

/// Performance optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    /// Enable SIMD optimizations
    #[serde(default)]
    pub enable_simd: bool,

    /// Enable GPU acceleration
    #[serde(default)]
    pub enable_gpu: bool,

    /// Memory pooling
    #[serde(default = "default_true")]
    pub memory_pooling: bool,

    /// Connection pooling
    #[serde(default = "default_true")]
    pub connection_pooling: bool,

    /// Query result caching
    #[serde(default = "default_true")]
    pub query_caching: bool,

    /// Compression for network traffic
    #[serde(default = "default_true")]
    pub compression: bool,

    /// Prefetching enabled
    #[serde(default)]
    pub prefetching: bool,
}

// Default value functions
fn default_batch_size() -> usize {
    100
}

fn default_batch_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_min_batch_size() -> usize {
    10
}

fn default_max_wait_time() -> Duration {
    Duration::from_secs(60)
}

fn default_db_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_http_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_io_timeout() -> Duration {
    Duration::from_secs(60)
}

fn default_api_timeout() -> Duration {
    Duration::from_secs(60)
}

fn default_task_timeout() -> Duration {
    Duration::from_secs(3600) // 1 hour
}

fn default_max_memory() -> u64 {
    4 * 1024 * 1024 * 1024 // 4GB
}

fn default_max_cpu() -> f32 {
    80.0
}

fn default_max_connections() -> usize {
    1000
}

fn default_max_file_size() -> u64 {
    100 * 1024 * 1024 // 100MB
}

fn default_max_request_size() -> u64 {
    10 * 1024 * 1024 // 10MB
}

fn default_rate_limit() -> u32 {
    100
}

fn default_true() -> bool {
    true
}

fn default_cache_backend() -> String {
    "memory".to_string()
}

fn default_cache_ttl() -> Duration {
    Duration::from_secs(3600) // 1 hour
}

fn default_max_cache_size() -> u64 {
    512 * 1024 * 1024 // 512MB
}

fn default_worker_threads() -> usize {
    num_cpus::get()
}

fn default_queue_size() -> usize {
    10000
}

fn default_core_pool_size() -> usize {
    num_cpus::get()
}

fn default_max_pool_size() -> usize {
    num_cpus::get() * 2
}

fn default_keep_alive() -> Duration {
    Duration::from_secs(60)
}

fn default_max_concurrent_tasks() -> usize {
    100
}

fn default_priority_levels() -> usize {
    3
}

fn default_metrics_interval() -> Duration {
    Duration::from_secs(60)
}

fn default_sample_rate() -> f32 {
    0.01
}

fn default_slow_query_threshold() -> Duration {
    Duration::from_millis(1000)
}

fn default_cpu_threshold() -> f32 {
    90.0
}

fn default_memory_threshold() -> f32 {
    85.0
}

fn default_response_time_threshold() -> u64 {
    5000 // 5 seconds
}

fn default_error_rate_threshold() -> f32 {
    5.0
}

fn default_queue_depth_threshold() -> usize {
    1000
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            batch: BatchConfig::default(),
            timeout: TimeoutConfig::default(),
            limits: LimitsConfig::default(),
            cache: CacheConfig::default(),
            workers: WorkerConfig::default(),
            monitoring: MonitoringConfig::default(),
            optimization: OptimizationConfig::default(),
        }
    }
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_size: default_batch_size(),
            timeout: default_batch_timeout(),
            adaptive: false,
            min_size: default_min_batch_size(),
            max_wait_time: default_max_wait_time(),
        }
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            database: default_db_timeout(),
            http: default_http_timeout(),
            io: default_io_timeout(),
            api: default_api_timeout(),
            task: default_task_timeout(),
        }
    }
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: default_max_memory(),
            max_cpu_percent: default_max_cpu(),
            max_connections: default_max_connections(),
            max_file_size_bytes: default_max_file_size(),
            max_request_size_bytes: default_max_request_size(),
            rate_limit_per_second: default_rate_limit(),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: default_cache_backend(),
            default_ttl: default_cache_ttl(),
            max_size_bytes: default_max_cache_size(),
            compression: false,
            warming: false,
        }
    }
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            threads: default_worker_threads(),
            queue_size: default_queue_size(),
            pool: ThreadPoolConfig::default(),
            scheduling: SchedulingConfig::default(),
        }
    }
}

impl Default for ThreadPoolConfig {
    fn default() -> Self {
        Self {
            core_size: default_core_pool_size(),
            max_size: default_max_pool_size(),
            keep_alive: default_keep_alive(),
            allow_core_timeout: false,
        }
    }
}

impl Default for SchedulingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_concurrent: default_max_concurrent_tasks(),
            priority_levels: default_priority_levels(),
            fair_scheduling: false,
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            metrics_interval: default_metrics_interval(),
            profiling: false,
            sample_rate: default_sample_rate(),
            slow_query_threshold: default_slow_query_threshold(),
            alerts: AlertThresholds::default(),
        }
    }
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            cpu_usage_percent: default_cpu_threshold(),
            memory_usage_percent: default_memory_threshold(),
            response_time_ms: default_response_time_threshold(),
            error_rate_percent: default_error_rate_threshold(),
            queue_depth: default_queue_depth_threshold(),
        }
    }
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            enable_simd: false,
            enable_gpu: false,
            memory_pooling: true,
            connection_pooling: true,
            query_caching: true,
            compression: true,
            prefetching: false,
        }
    }
}