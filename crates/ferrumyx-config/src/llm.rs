//! LLM provider configuration management
//!
//! Supports multiple LLM providers with unified configuration,
//! API key management, rate limiting, and failover settings.

use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Supported LLM providers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    /// OpenAI GPT models
    OpenAi,

    /// Anthropic Claude models
    Anthropic,

    /// xAI Grok models
    Grok,

    /// Ollama local models
    Ollama,

    /// Custom provider
    Custom(String),
}

/// API key configuration with rotation support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    /// Primary API key
    pub key: SecretString,

    /// Backup/rotation keys
    #[serde(default)]
    pub backup_keys: Vec<SecretString>,

    /// Key rotation schedule (cron expression)
    pub rotation_schedule: Option<String>,

    /// Last rotation timestamp
    pub last_rotated: Option<chrono::DateTime<chrono::Utc>>,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Requests per minute
    #[serde(default = "default_rpm")]
    pub requests_per_minute: u32,

    /// Requests per hour
    #[serde(default = "default_rph")]
    pub requests_per_hour: u32,

    /// Requests per day
    #[serde(default = "default_rpd")]
    pub requests_per_day: u32,

    /// Token limit per minute
    #[serde(default = "default_tpm")]
    pub tokens_per_minute: u32,

    /// Burst limit
    #[serde(default = "default_burst")]
    pub burst_limit: u32,
}

/// Model-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model identifier
    pub model_id: String,

    /// Display name
    pub display_name: String,

    /// Context window size
    pub context_window: usize,

    /// Maximum output tokens
    pub max_output_tokens: usize,

    /// Supported modalities
    #[serde(default)]
    pub modalities: Vec<String>,

    /// Model capabilities
    #[serde(default)]
    pub capabilities: Vec<String>,

    /// Cost per 1K input tokens (USD)
    pub cost_per_1k_input: f64,

    /// Cost per 1K output tokens (USD)
    pub cost_per_1k_output: f64,
}

/// Provider-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider type
    pub provider: LlmProvider,

    /// Base URL for API calls
    pub base_url: String,

    /// API version
    #[serde(default = "default_api_version")]
    pub api_version: String,

    /// API key configuration
    pub api_key: Option<ApiKeyConfig>,

    /// Rate limiting
    #[serde(default)]
    pub rate_limit: RateLimitConfig,

    /// Timeout settings
    #[serde(default)]
    pub timeout: TimeoutConfig,

    /// Retry configuration
    #[serde(default)]
    pub retry: RetryConfig,

    /// Enabled models for this provider
    #[serde(default)]
    pub enabled_models: Vec<String>,

    /// Custom headers
    #[serde(default)]
    pub custom_headers: HashMap<String, String>,

    /// Provider-specific options
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
}

/// Timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Connection timeout
    #[serde(default = "default_connection_timeout")]
    #[serde(with = "humantime_serde")]
    pub connect: Duration,

    /// Request timeout
    #[serde(default = "default_request_timeout")]
    #[serde(with = "humantime_serde")]
    pub request: Duration,

    /// Stream timeout
    #[serde(default = "default_stream_timeout")]
    #[serde(with = "humantime_serde")]
    pub stream: Duration,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    #[serde(default = "default_max_retries")]
    pub max_attempts: u32,

    /// Initial backoff duration
    #[serde(default = "default_initial_backoff")]
    #[serde(with = "humantime_serde")]
    pub initial_backoff: Duration,

    /// Maximum backoff duration
    #[serde(default = "default_max_backoff")]
    #[serde(with = "humantime_serde")]
    pub max_backoff: Duration,

    /// Backoff multiplier
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,

    /// Retryable status codes
    #[serde(default = "default_retryable_codes")]
    pub retryable_codes: Vec<u16>,
}

/// LLM configuration unifying all provider settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Default provider for new requests
    #[serde(default = "default_provider")]
    pub default_provider: String,

    /// Available providers
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,

    /// Available models across all providers
    #[serde(default)]
    pub models: HashMap<String, ModelConfig>,

    /// Global rate limiting
    #[serde(default)]
    pub global_rate_limit: RateLimitConfig,

    /// Circuit breaker configuration
    #[serde(default)]
    pub circuit_breaker: CircuitBreakerConfig,

    /// Fallback provider chain
    #[serde(default)]
    pub fallback_chain: Vec<String>,

    /// Cost tracking and limits
    #[serde(default)]
    pub cost_limits: CostLimitConfig,

    /// Caching configuration
    #[serde(default)]
    pub caching: CacheConfig,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Failure threshold
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,

    /// Recovery timeout
    #[serde(default = "default_recovery_timeout")]
    #[serde(with = "humantime_serde")]
    pub recovery_timeout: Duration,

    /// Success threshold for recovery
    #[serde(default = "default_success_threshold")]
    pub success_threshold: u32,

    /// Monitoring window
    #[serde(default = "default_monitoring_window")]
    #[serde(with = "humantime_serde")]
    pub monitoring_window: Duration,
}

/// Cost limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostLimitConfig {
    /// Daily cost limit (USD)
    #[serde(default = "default_daily_cost_limit")]
    pub daily_limit: f64,

    /// Monthly cost limit (USD)
    #[serde(default = "default_monthly_cost_limit")]
    pub monthly_limit: f64,

    /// Alert threshold (percentage of limit)
    #[serde(default = "default_alert_threshold")]
    pub alert_threshold: f64,

    /// Hard stop when limit exceeded
    #[serde(default)]
    pub hard_stop: bool,
}

/// Cache configuration for LLM responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable response caching
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Cache TTL
    #[serde(default = "default_cache_ttl")]
    #[serde(with = "humantime_serde")]
    pub ttl: Duration,

    /// Maximum cache size (entries)
    #[serde(default = "default_max_cache_size")]
    pub max_size: usize,

    /// Cache backend (memory, redis)
    #[serde(default = "default_cache_backend")]
    pub backend: String,
}

// Default value functions
fn default_provider() -> String {
    "openai".to_string()
}

fn default_api_version() -> String {
    "v1".to_string()
}

fn default_rpm() -> u32 {
    60
}

fn default_rph() -> u32 {
    1000
}

fn default_rpd() -> u32 {
    10000
}

fn default_tpm() -> u32 {
    100000
}

fn default_burst() -> u32 {
    10
}

fn default_connection_timeout() -> Duration {
    Duration::from_secs(10)
}

fn default_request_timeout() -> Duration {
    Duration::from_secs(60)
}

fn default_stream_timeout() -> Duration {
    Duration::from_secs(300)
}

fn default_max_retries() -> u32 {
    3
}

fn default_initial_backoff() -> Duration {
    Duration::from_millis(100)
}

fn default_max_backoff() -> Duration {
    Duration::from_secs(30)
}

fn default_backoff_multiplier() -> f64 {
    2.0
}

fn default_retryable_codes() -> Vec<u16> {
    vec![429, 500, 502, 503, 504]
}

fn default_failure_threshold() -> u32 {
    5
}

fn default_recovery_timeout() -> Duration {
    Duration::from_secs(60)
}

fn default_success_threshold() -> u32 {
    3
}

fn default_monitoring_window() -> Duration {
    Duration::from_secs(60)
}

fn default_daily_cost_limit() -> f64 {
    10.0
}

fn default_monthly_cost_limit() -> f64 {
    100.0
}

fn default_alert_threshold() -> f64 {
    0.8
}

fn default_true() -> bool {
    true
}

fn default_cache_ttl() -> Duration {
    Duration::from_secs(3600)
}

fn default_max_cache_size() -> usize {
    10000
}

fn default_cache_backend() -> String {
    "memory".to_string()
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            default_provider: default_provider(),
            providers: HashMap::new(),
            models: HashMap::new(),
            global_rate_limit: RateLimitConfig::default(),
            circuit_breaker: CircuitBreakerConfig::default(),
            fallback_chain: vec![],
            cost_limits: CostLimitConfig::default(),
            caching: CacheConfig::default(),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: default_rpm(),
            requests_per_hour: default_rph(),
            requests_per_day: default_rpd(),
            tokens_per_minute: default_tpm(),
            burst_limit: default_burst(),
        }
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect: default_connection_timeout(),
            request: default_request_timeout(),
            stream: default_stream_timeout(),
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: default_max_retries(),
            initial_backoff: default_initial_backoff(),
            max_backoff: default_max_backoff(),
            backoff_multiplier: default_backoff_multiplier(),
            retryable_codes: default_retryable_codes(),
        }
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: default_failure_threshold(),
            recovery_timeout: default_recovery_timeout(),
            success_threshold: default_success_threshold(),
            monitoring_window: default_monitoring_window(),
        }
    }
}

impl Default for CostLimitConfig {
    fn default() -> Self {
        Self {
            daily_limit: default_daily_cost_limit(),
            monthly_limit: default_monthly_cost_limit(),
            alert_threshold: default_alert_threshold(),
            hard_stop: false,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl: default_cache_ttl(),
            max_size: default_max_cache_size(),
            backend: default_cache_backend(),
        }
    }
}