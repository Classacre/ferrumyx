//! Monitoring and alerting configuration
//!
//! Prometheus metrics, logging, health checks, and alerting rules.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Metrics backend
    #[serde(default = "default_metrics_backend")]
    pub backend: String,

    /// Metrics collection interval
    #[serde(default = "default_collection_interval")]
    #[serde(with = "humantime_serde")]
    pub collection_interval: Duration,

    /// Metrics retention period
    #[serde(default = "default_retention_period")]
    #[serde(with = "humantime_serde")]
    pub retention_period: Duration,

    /// Custom metric labels
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

/// Prometheus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusConfig {
    /// Enable Prometheus metrics endpoint
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// HTTP endpoint path
    #[serde(default = "default_prometheus_path")]
    pub path: String,

    /// HTTP port
    #[serde(default = "default_prometheus_port")]
    pub port: u16,

    /// Enable histogram buckets
    #[serde(default = "default_true")]
    pub histogram_buckets: bool,

    /// Custom histogram buckets
    #[serde(default)]
    pub custom_buckets: Vec<f64>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Log format
    #[serde(default = "default_log_format")]
    pub format: String,

    /// Log output destination
    #[serde(default = "default_log_output")]
    pub output: String,

    /// Log file path (for file output)
    #[serde(default = "default_log_file")]
    pub file_path: String,

    /// Maximum log file size (bytes)
    #[serde(default = "default_max_log_size")]
    pub max_file_size: u64,

    /// Maximum number of log files
    #[serde(default = "default_max_log_files")]
    pub max_files: usize,

    /// Enable log rotation
    #[serde(default = "default_true")]
    pub rotation: bool,

    /// Structured logging fields
    #[serde(default)]
    pub structured_fields: Vec<String>,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Enable health checks
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Health check endpoint path
    #[serde(default = "default_health_path")]
    pub path: String,

    /// Health check timeout
    #[serde(default = "default_health_timeout")]
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,

    /// Health check interval
    #[serde(default = "default_health_interval")]
    #[serde(with = "humantime_serde")]
    pub interval: Duration,

    /// Dependency health checks
    #[serde(default)]
    pub dependencies: Vec<DependencyCheck>,
}

/// Dependency health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyCheck {
    /// Dependency name
    pub name: String,

    /// Check type
    pub check_type: HealthCheckType,

    /// Check endpoint/URL
    pub endpoint: String,

    /// Check timeout
    #[serde(default = "default_dependency_timeout")]
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,

    /// Expected status code
    #[serde(default = "default_expected_status")]
    pub expected_status: u16,

    /// Custom headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

/// Health check types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthCheckType {
    /// HTTP endpoint
    Http,

    /// TCP connection
    Tcp,

    /// Database connection
    Database,

    /// Redis connection
    Redis,

    /// Custom check
    Custom,
}

/// Alerting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingConfig {
    /// Enable alerting
    #[serde(default)]
    pub enabled: bool,

    /// Alert manager endpoint
    pub alertmanager_url: Option<String>,

    /// Default alert labels
    #[serde(default)]
    pub default_labels: HashMap<String, String>,

    /// Alert rules
    #[serde(default)]
    pub rules: Vec<AlertRule>,

    /// Notification channels
    #[serde(default)]
    pub notifications: Vec<NotificationChannel>,
}

/// Alert rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Rule name
    pub name: String,

    /// Rule description
    pub description: String,

    /// PromQL query
    pub query: String,

    /// Alert duration (how long condition must be true)
    #[serde(default = "default_alert_duration")]
    #[serde(with = "humantime_serde")]
    pub duration: Duration,

    /// Alert labels
    #[serde(default)]
    pub labels: HashMap<String, String>,

    /// Alert annotations
    #[serde(default)]
    pub annotations: HashMap<String, String>,

    /// Severity level
    #[serde(default = "default_severity")]
    pub severity: String,
}

/// Notification channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    /// Channel name
    pub name: String,

    /// Channel type
    pub channel_type: NotificationType,

    /// Configuration
    pub config: HashMap<String, String>,
}

/// Notification types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationType {
    /// Email notifications
    Email,

    /// Slack notifications
    Slack,

    /// PagerDuty
    Pagerduty,

    /// Webhook
    Webhook,

    /// SMS
    Sms,
}

/// Tracing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Enable distributed tracing
    #[serde(default)]
    pub enabled: bool,

    /// Tracing backend
    #[serde(default = "default_tracing_backend")]
    pub backend: String,

    /// Jaeger endpoint
    pub jaeger_endpoint: Option<String>,

    /// Service name
    #[serde(default = "default_service_name")]
    pub service_name: String,

    /// Sampling rate (0.0-1.0)
    #[serde(default = "default_sampling_rate")]
    pub sampling_rate: f32,

    /// Trace propagation headers
    #[serde(default = "default_propagation_headers")]
    pub propagation_headers: Vec<String>,
}

/// Profiling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilingConfig {
    /// Enable profiling
    #[serde(default)]
    pub enabled: bool,

    /// Profiling backend
    #[serde(default = "default_profiling_backend")]
    pub backend: String,

    /// Profiling sample rate
    #[serde(default = "default_profile_sample_rate")]
    pub sample_rate: f32,

    /// Profile output directory
    #[serde(default = "default_profile_output_dir")]
    pub output_dir: String,

    /// Maximum profile duration
    #[serde(default = "default_max_profile_duration")]
    #[serde(with = "humantime_serde")]
    pub max_duration: Duration,
}

/// Monitoring configuration unifying all monitoring settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Metrics collection
    #[serde(default)]
    pub metrics: MetricsConfig,

    /// Prometheus configuration
    #[serde(default)]
    pub prometheus: PrometheusConfig,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Health checks
    #[serde(default)]
    pub health: HealthCheckConfig,

    /// Alerting configuration
    #[serde(default)]
    pub alerting: AlertingConfig,

    /// Distributed tracing
    #[serde(default)]
    pub tracing: TracingConfig,

    /// Performance profiling
    #[serde(default)]
    pub profiling: ProfilingConfig,

    /// Custom monitoring extensions
    #[serde(default)]
    pub extensions: Vec<MonitoringExtension>,
}

/// Monitoring extension configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringExtension {
    /// Extension name
    pub name: String,

    /// Extension type
    pub extension_type: String,

    /// Configuration
    pub config: HashMap<String, serde_json::Value>,
}

// Default value functions
fn default_true() -> bool {
    true
}

fn default_metrics_backend() -> String {
    "prometheus".to_string()
}

fn default_collection_interval() -> Duration {
    Duration::from_secs(15)
}

fn default_retention_period() -> Duration {
    Duration::from_secs(30 * 24 * 60 * 60) // 30 days
}

fn default_prometheus_path() -> String {
    "/metrics".to_string()
}

fn default_prometheus_port() -> u16 {
    9090
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "json".to_string()
}

fn default_log_output() -> String {
    "stdout".to_string()
}

fn default_log_file() -> String {
    "./logs/ferrumyx.log".to_string()
}

fn default_max_log_size() -> u64 {
    100 * 1024 * 1024 // 100MB
}

fn default_max_log_files() -> usize {
    5
}

fn default_health_path() -> String {
    "/health".to_string()
}

fn default_health_timeout() -> Duration {
    Duration::from_secs(5)
}

fn default_health_interval() -> Duration {
    Duration::from_secs(30)
}

fn default_dependency_timeout() -> Duration {
    Duration::from_secs(10)
}

fn default_expected_status() -> u16 {
    200
}

fn default_alert_duration() -> Duration {
    Duration::from_secs(60)
}

fn default_severity() -> String {
    "warning".to_string()
}

fn default_tracing_backend() -> String {
    "jaeger".to_string()
}

fn default_service_name() -> String {
    "ferrumyx".to_string()
}

fn default_sampling_rate() -> f32 {
    0.1
}

fn default_propagation_headers() -> Vec<String> {
    vec![
        "x-request-id".to_string(),
        "x-trace-id".to_string(),
        "x-span-id".to_string(),
    ]
}

fn default_profiling_backend() -> String {
    "pprof".to_string()
}

fn default_profile_sample_rate() -> f32 {
    0.01
}

fn default_profile_output_dir() -> String {
    "./profiles".to_string()
}

fn default_max_profile_duration() -> Duration {
    Duration::from_secs(30)
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            metrics: MetricsConfig::default(),
            prometheus: PrometheusConfig::default(),
            logging: LoggingConfig::default(),
            health: HealthCheckConfig::default(),
            alerting: AlertingConfig::default(),
            tracing: TracingConfig::default(),
            profiling: ProfilingConfig::default(),
            extensions: vec![],
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: default_metrics_backend(),
            collection_interval: default_collection_interval(),
            retention_period: default_retention_period(),
            labels: HashMap::new(),
        }
    }
}

impl Default for PrometheusConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: default_prometheus_path(),
            port: default_prometheus_port(),
            histogram_buckets: true,
            custom_buckets: vec![],
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            output: default_log_output(),
            file_path: default_log_file(),
            max_file_size: default_max_log_size(),
            max_files: default_max_log_files(),
            rotation: true,
            structured_fields: vec![],
        }
    }
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: default_health_path(),
            timeout: default_health_timeout(),
            interval: default_health_interval(),
            dependencies: vec![],
        }
    }
}

impl Default for AlertingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            alertmanager_url: None,
            default_labels: HashMap::new(),
            rules: vec![],
            notifications: vec![],
        }
    }
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backend: default_tracing_backend(),
            jaeger_endpoint: None,
            service_name: default_service_name(),
            sampling_rate: default_sampling_rate(),
            propagation_headers: default_propagation_headers(),
        }
    }
}

impl Default for ProfilingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backend: default_profiling_backend(),
            sample_rate: default_profile_sample_rate(),
            output_dir: default_profile_output_dir(),
            max_duration: default_max_profile_duration(),
        }
    }
}