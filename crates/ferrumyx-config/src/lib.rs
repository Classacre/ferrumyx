//! # Ferrumyx Configuration System
//!
//! A comprehensive, unified configuration management system for Ferrumyx.

use serde::{Deserialize, Serialize};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FerrumyxConfig {
    /// Configuration version
    pub version: String,

    /// Database configuration
    pub database: DatabaseConfig,

    /// LLM provider configurations
    pub llm: LlmConfig,

    /// Security settings
    pub security: SecurityConfig,

    /// Performance tuning
    pub performance: PerformanceConfig,

    /// Multi-channel integration
    pub channels: ChannelsConfig,

    /// Monitoring and alerting
    pub monitoring: MonitoringConfig,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Connection URL
    pub url: String,

    /// Pool size
    pub pool_size: u32,

    /// Backend type
    pub backend: String,
}

/// LLM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Default provider
    pub default_provider: String,

    /// Provider configurations
    pub providers: std::collections::HashMap<String, LlmProviderConfig>,
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    /// Provider type
    pub provider_type: String,

    /// API key
    pub api_key: Option<String>,

    /// Base URL
    pub base_url: String,

    /// Model name
    pub model: String,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// JWT secret
    pub jwt_secret: String,

    /// Encryption key
    pub encryption_key: String,

    /// Enable audit logging
    pub audit_enabled: bool,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Max concurrent requests
    pub max_concurrent: usize,

    /// Request timeout (seconds)
    pub timeout_seconds: u64,

    /// Enable caching
    pub caching_enabled: bool,
}

/// Channels configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelsConfig {
    /// Enable multi-channel support
    pub enabled: bool,

    /// WhatsApp configuration
    pub whatsapp: Option<WhatsAppConfig>,

    /// Slack configuration
    pub slack: Option<SlackConfig>,

    /// Discord configuration
    pub discord: Option<DiscordConfig>,
}

/// WhatsApp configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    /// API token
    pub api_token: String,

    /// Phone number ID
    pub phone_number_id: String,

    /// Enabled
    pub enabled: bool,
}

/// Slack configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    /// Bot token
    pub bot_token: String,

    /// Signing secret
    pub signing_secret: String,

    /// Enabled
    pub enabled: bool,
}

/// Discord configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    /// Bot token
    pub bot_token: String,

    /// Application ID
    pub application_id: String,

    /// Enabled
    pub enabled: bool,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable metrics
    pub metrics_enabled: bool,

    /// Metrics endpoint
    pub metrics_endpoint: String,

    /// Log level
    pub log_level: String,

    /// Enable health checks
    pub health_checks_enabled: bool,
}

impl Default for FerrumyxConfig {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            database: DatabaseConfig::default(),
            llm: LlmConfig::default(),
            security: SecurityConfig::default(),
            performance: PerformanceConfig::default(),
            channels: ChannelsConfig::default(),
            monitoring: MonitoringConfig::default(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://localhost:5432/ferrumyx".to_string(),
            pool_size: 10,
            backend: "postgresql".to_string(),
        }
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            default_provider: "openai".to_string(),
            providers: std::collections::HashMap::new(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "super-secret-jwt-key-that-is-long-enough-for-production-use".to_string(),
            encryption_key: "super-secret-encryption-key-that-is-long-enough-for-production-use".to_string(),
            audit_enabled: true,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 100,
            timeout_seconds: 30,
            caching_enabled: true,
        }
    }
}

impl Default for ChannelsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            whatsapp: None,
            slack: None,
            discord: None,
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            metrics_enabled: true,
            metrics_endpoint: "/metrics".to_string(),
            log_level: "info".to_string(),
            health_checks_enabled: true,
        }
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("TOML parsing error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Environment variable error: {0}")]
    Environment(String),

    #[error("Schema validation error: {0}")]
    Schema(String),
}

impl FerrumyxConfig {
    /// Load configuration with default settings
    pub async fn load() -> Result<Self, ConfigError> {
        Ok(Self::default())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Basic validation
        if self.database.pool_size == 0 {
            return Err(ConfigError::Validation("Database pool size must be > 0".to_string()));
        }
        if self.security.jwt_secret.len() < 32 {
            return Err(ConfigError::Validation("JWT secret too short".to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_default_config() {
        let config = FerrumyxConfig::load().await.unwrap();
        assert_eq!(config.version, "1.0.0");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_serialization() {
        let config = FerrumyxConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: FerrumyxConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.version, parsed.version);
    }
}