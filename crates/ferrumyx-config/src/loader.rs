//! Configuration loader with hierarchical loading support
//!
//! Loads configuration from multiple sources with proper precedence:
//! secrets → environment variables → config files → defaults

use crate::{ConfigError, FerrumyxConfig, validation, migration};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Configuration source types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigSource {
    /// Load from file path
    File(String),

    /// Load from environment variables
    Environment,

    /// Load from database
    Database { user_id: String },

    /// Load from secrets store
    Secrets { user_id: String },

    /// Load from defaults only
    Defaults,
}

/// Configuration loader
pub struct ConfigLoader {
    sources: Vec<ConfigSource>,
    validate: bool,
    migrate: bool,
    watch: bool,
}

impl ConfigLoader {
    /// Create a new configuration loader with default sources
    pub fn new() -> Self {
        Self {
            sources: vec![
                ConfigSource::Secrets { user_id: "default".to_string() },
                ConfigSource::Environment,
                ConfigSource::File("./config/ferrumyx.toml".to_string()),
                ConfigSource::File("./config/ferrumyx.json".to_string()),
                ConfigSource::Defaults,
            ],
            validate: true,
            migrate: true,
            watch: false,
        }
    }

    /// Create a loader with custom sources
    pub fn with_sources(sources: Vec<ConfigSource>) -> Self {
        Self {
            sources,
            validate: true,
            migrate: true,
            watch: false,
        }
    }

    /// Enable or disable validation
    pub fn with_validation(mut self, validate: bool) -> Self {
        self.validate = validate;
        self
    }

    /// Enable or disable automatic migration
    pub fn with_migration(mut self, migrate: bool) -> Self {
        self.migrate = migrate;
        self
    }

    /// Enable or disable file watching
    pub fn with_watching(mut self, watch: bool) -> Self {
        self.watch = watch;
        self
    }

    /// Load configuration from all sources
    pub async fn load(mut self) -> Result<FerrumyxConfig, ConfigError> {
        let mut config = FerrumyxConfig::default();

        // Load from each source in order of precedence
        for source in &self.sources {
            match source {
                ConfigSource::File(path) => {
                    if let Ok(loaded_config) = Self::load_from_file(path).await {
                        self.merge_config(&mut config, loaded_config);
                    }
                }
                ConfigSource::Environment => {
                    if let Ok(loaded_config) = Self::load_from_env() {
                        self.merge_config(&mut config, loaded_config);
                    }
                }
                ConfigSource::Database { user_id } => {
                    if let Ok(loaded_config) = Self::load_from_database(user_id).await {
                        self.merge_config(&mut config, loaded_config);
                    }
                }
                ConfigSource::Secrets { user_id } => {
                    if let Ok(secrets) = Self::load_from_secrets(user_id).await {
                        self.merge_secrets(&mut config, secrets);
                    }
                }
                ConfigSource::Defaults => {
                    // Defaults are already set
                }
            }
        }

        // Apply migrations if enabled
        if self.migrate && migration::needs_migration(&config) {
            migration::apply_migrations(&mut config)?;
        }

        // Validate configuration if enabled
        if self.validate {
            let validation_result = config.validate();
            if !validation_result.is_valid {
                return Err(ConfigError::Validation(format!(
                    "Configuration validation failed: {}",
                    validation_result.errors.iter()
                        .map(|e| format!("{}: {}", e.path, e.message))
                        .collect::<Vec<_>>()
                        .join("; ")
                )));
            }
        }

        Ok(config)
    }

    /// Load configuration from a file
    async fn load_from_file(path: &str) -> Result<FerrumyxConfig, ConfigError> {
        let path = Path::new(path);

        if !path.exists() {
            return Err(ConfigError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Configuration file not found: {}", path.display())
            )));
        }

        let content = tokio::fs::read_to_string(path).await?;

        match path.extension().and_then(|s| s.to_str()) {
            Some("toml") => {
                toml::from_str(&content).map_err(ConfigError::Toml)
            }
            Some("json") => {
                serde_json::from_str(&content).map_err(ConfigError::Serialization)
            }
            Some("yaml") | Some("yml") => {
                serde_yaml::from_str(&content).map_err(ConfigError::Yaml)
            }
            _ => Err(ConfigError::Environment(format!(
                "Unsupported configuration file format: {}",
                path.display()
            ))),
        }
    }

    /// Load configuration from environment variables
    fn load_from_env() -> Result<FerrumyxConfig, ConfigError> {
        // In a real implementation, this would parse environment variables
        // and map them to configuration fields using a mapping table
        Ok(FerrumyxConfig::default())
    }

    /// Load configuration from database
    async fn load_from_database(_user_id: &str) -> Result<FerrumyxConfig, ConfigError> {
        // In a real implementation, this would query the database
        // for user-specific configuration settings
        Ok(FerrumyxConfig::default())
    }

    /// Load secrets from secrets store
    async fn load_from_secrets(_user_id: &str) -> Result<HashMap<String, SecretString>, ConfigError> {
        // In a real implementation, this would connect to a secrets store
        // like HashiCorp Vault, AWS Secrets Manager, etc.
        Ok(HashMap::new())
    }

    /// Merge two configurations with precedence
    fn merge_config(&self, base: &mut FerrumyxConfig, overlay: FerrumyxConfig) {
        // Deep merge logic would go here
        // For now, we'll do a simple field-by-field replacement where overlay has values

        if overlay.meta.version > base.meta.version {
            base.meta.version = overlay.meta.version;
        }
        base.meta.last_updated = chrono::Utc::now();

        // Merge database config
        if !overlay.database.primary.url.expose_secret().is_empty() {
            base.database = overlay.database;
        }

        // Merge LLM config
        if !overlay.llm.providers.is_empty() {
            base.llm = overlay.llm;
        }

        // Merge security config
        if !overlay.security.jwt.secret_key.expose_secret().is_empty() {
            base.security = overlay.security;
        }

        // Continue for other fields...
        base.performance = overlay.performance;
        base.channels = overlay.channels;
        base.monitoring = overlay.monitoring;
    }

    /// Merge secrets into configuration
    fn merge_secrets(&self, config: &mut FerrumyxConfig, secrets: HashMap<String, SecretString>) {
        // Inject secrets into appropriate configuration fields
        for (key, value) in secrets {
            match key.as_str() {
                "database_url" => {
                    config.database.primary.url = value;
                }
                "jwt_secret" => {
                    config.security.jwt.secret_key = value;
                }
                "encryption_key" => {
                    config.security.encryption.master_key = value;
                }
                "openai_api_key" => {
                    if let Some(provider) = config.llm.providers.get_mut("openai") {
                        provider.api_key = Some(crate::llm::ApiKeyConfig {
                            key: value,
                            backup_keys: vec![],
                            rotation_schedule: None,
                            last_rotated: None,
                        });
                    }
                }
                // Add more secret mappings as needed
                _ => {
                    tracing::warn!("Unknown secret key: {}", key);
                }
            }
        }
    }
}

/// Load configuration with default settings
pub async fn load_config() -> Result<FerrumyxConfig, ConfigError> {
    ConfigLoader::new().load().await
}

/// Load configuration from specific file
pub async fn load_config_from_file(path: &str) -> Result<FerrumyxConfig, ConfigError> {
    ConfigLoader::with_sources(vec![
        ConfigSource::File(path.to_string()),
        ConfigSource::Defaults,
    ]).load().await
}

/// Load configuration from environment only
pub async fn load_config_from_env() -> Result<FerrumyxConfig, ConfigError> {
    ConfigLoader::with_sources(vec![
        ConfigSource::Environment,
        ConfigSource::Defaults,
    ]).load().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_config_from_defaults() {
        let config = ConfigLoader::with_sources(vec![ConfigSource::Defaults])
            .with_validation(false)
            .with_migration(false)
            .load()
            .await;

        assert!(config.is_ok());
    }

    #[tokio::test]
    async fn test_load_config_from_file_not_found() {
        let config = load_config_from_file("/nonexistent/file.json").await;
        assert!(config.is_err());
    }
}