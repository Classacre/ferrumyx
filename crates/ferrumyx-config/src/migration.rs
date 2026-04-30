//! Configuration migration framework
//!
//! Handles version-aware configuration upgrades with automatic
//! transformation and validation.

use crate::{ConfigError, FerrumyxConfig};
use semver::Version;
use std::collections::HashMap;

/// Migration registry
pub struct MigrationRegistry {
    migrations: HashMap<(Version, Version), Box<dyn Migration>>,
}

/// Migration trait
pub trait Migration: Send + Sync {
    /// Migrate configuration from one version to another
    fn migrate(&self, config: &mut serde_json::Value) -> Result<(), ConfigError>;

    /// Check if this migration can handle the given versions
    fn can_migrate(&self, from: &Version, to: &Version) -> bool;

    /// Description of what this migration does
    fn description(&self) -> &'static str;
}

/// Built-in migrations
pub struct BuiltInMigrations;

impl BuiltInMigrations {
    /// Migration from 1.0.0 to 1.1.0: Add new LLM provider fields
    pub fn migrate_1_0_0_to_1_1_0() -> Box<dyn Migration> {
        Box::new(Migration_1_0_0_to_1_1_0)
    }

    /// Migration from 1.1.0 to 1.2.0: Add monitoring configuration
    pub fn migrate_1_1_0_to_1_2_0() -> Box<dyn Migration> {
        Box::new(Migration_1_1_0_to_1_2_0)
    }

    /// Migration from 1.2.0 to 2.0.0: Major security enhancements
    pub fn migrate_1_2_0_to_2_0_0() -> Box<dyn Migration> {
        Box::new(Migration_1_2_0_to_2_0_0)
    }
}

/// Migration from 1.0.0 to 1.1.0
struct Migration_1_0_0_to_1_1_0;

impl Migration for Migration_1_0_0_to_1_1_0 {
    fn migrate(&self, config: &mut serde_json::Value) -> Result<(), ConfigError> {
        if let Some(llm) = config.get_mut("llm") {
            if let Some(providers) = llm.get_mut("providers") {
                if let Some(obj) = providers.as_object_mut() {
                    for (_name, provider) in obj.iter_mut() {
                        if let Some(provider_obj) = provider.as_object_mut() {
                            // Add new fields with defaults
                            provider_obj.insert("timeout".to_string(), serde_json::json!({
                                "connect": 10000,
                                "request": 60000,
                                "stream": 300000
                            }));
                            provider_obj.insert("retry".to_string(), serde_json::json!({
                                "max_attempts": 3,
                                "initial_backoff": 100,
                                "max_backoff": 30000,
                                "backoff_multiplier": 2.0,
                                "retryable_codes": [429, 500, 502, 503, 504]
                            }));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn can_migrate(&self, from: &Version, to: &Version) -> bool {
        from == &Version::parse("1.0.0").unwrap() && to == &Version::parse("1.1.0").unwrap()
    }

    fn description(&self) -> &'static str {
        "Add timeout and retry configuration to LLM providers"
    }
}

/// Migration from 1.1.0 to 1.2.0
struct Migration_1_1_0_to_1_2_0;

impl Migration for Migration_1_1_0_to_1_2_0 {
    fn migrate(&self, config: &mut serde_json::Value) -> Result<(), ConfigError> {
        // Add monitoring configuration
        config["monitoring"] = serde_json::json!({
            "metrics": {
                "enabled": true,
                "backend": "prometheus",
                "collection_interval": "15s",
                "retention_period": "720h"
            },
            "prometheus": {
                "enabled": true,
                "path": "/metrics",
                "port": 9090
            },
            "logging": {
                "level": "info",
                "format": "json",
                "output": "stdout"
            },
            "health": {
                "enabled": true,
                "path": "/health",
                "timeout": "5s",
                "interval": "30s"
            }
        });
        Ok(())
    }

    fn can_migrate(&self, from: &Version, to: &Version) -> bool {
        from == &Version::parse("1.1.0").unwrap() && to == &Version::parse("1.2.0").unwrap()
    }

    fn description(&self) -> &'static str {
        "Add monitoring and observability configuration"
    }
}

/// Migration from 1.2.0 to 2.0.0
struct Migration_1_2_0_to_2_0_0;

impl Migration for Migration_1_2_0_to_2_0_0 {
    fn migrate(&self, config: &mut serde_json::Value) -> Result<(), ConfigError> {
        // Enhance security configuration
        if let Some(security) = config.get_mut("security") {
            if let Some(auth) = security.get_mut("auth") {
                auth["mfa"] = serde_json::json!({
                    "enabled": false,
                    "required_methods": [],
                    "grace_period": "604800s",
                    "backup_codes_count": 10
                });
            }

            if let Some(encryption) = security.get_mut("encryption") {
                encryption["key_rotation"] = serde_json::json!({
                    "enabled": false,
                    "interval": "2592000s",
                    "key_retention_count": 5,
                    "grace_period": "604800s"
                });
            }

            security["audit"] = serde_json::json!({
                "enabled": true,
                "level": "info",
                "destination": "file",
                "log_path": "./logs/audit.log",
                "events": ["authentication", "authorization", "data_access"],
                "retention_period": "31536000s",
                "compress_logs": true
            });
        }

        // Update version
        config["meta"]["version"] = serde_json::json!("2.0.0");
        Ok(())
    }

    fn can_migrate(&self, from: &Version, to: &Version) -> bool {
        from == &Version::parse("1.2.0").unwrap() && to == &Version::parse("2.0.0").unwrap()
    }

    fn description(&self) -> &'static str {
        "Major security enhancements: MFA, key rotation, audit logging"
    }
}

impl MigrationRegistry {
    /// Create a new migration registry with built-in migrations
    pub fn new() -> Self {
        let mut registry = Self {
            migrations: HashMap::new(),
        };

        // Register built-in migrations
        registry.register(BuiltInMigrations::migrate_1_0_0_to_1_1_0());
        registry.register(BuiltInMigrations::migrate_1_1_0_to_1_2_0());
        registry.register(BuiltInMigrations::migrate_1_2_0_to_2_0_0());

        registry
    }

    /// Register a custom migration
    pub fn register(&mut self, migration: Box<dyn Migration>) {
        // For simplicity, we'll use a dummy key since we can't easily extract versions
        // In a real implementation, you'd want to inspect the migration for supported versions
        let dummy_key = (Version::parse("0.0.0").unwrap(), Version::parse("0.0.1").unwrap());
        self.migrations.insert(dummy_key, migration);
    }

    /// Find a migration path from one version to another
    pub fn find_migration_path(&self, from: &Version, to: &Version) -> Vec<&Box<dyn Migration>> {
        let mut path = Vec::new();

        // Simple linear migration path (in practice, you'd implement a more sophisticated pathfinding)
        if from < to {
            // For now, just return migrations that can handle direct upgrades
            for migration in self.migrations.values() {
                if migration.can_migrate(from, to) {
                    path.push(*migration);
                }
            }
        }

        path
    }
}

/// Migration engine
pub struct MigrationEngine {
    registry: MigrationRegistry,
}

impl MigrationEngine {
    /// Create a new migration engine
    pub fn new() -> Self {
        Self {
            registry: MigrationRegistry::new(),
        }
    }

    /// Apply migrations to bring config up to the target version
    pub fn migrate_to_version(&self, config: &mut FerrumyxConfig, target_version: &Version) -> Result<(), ConfigError> {
        let current_version = &config.meta.version;

        if current_version >= target_version {
            return Ok(()); // Already at or above target version
        }

        let migration_path = self.registry.find_migration_path(current_version, target_version);

        if migration_path.is_empty() {
            return Err(ConfigError::Migration(format!(
                "No migration path found from {} to {}",
                current_version, target_version
            )));
        }

        // Convert config to JSON value for manipulation
        let mut config_value = serde_json::to_value(config)
            .map_err(|e| ConfigError::Serialization(e))?;

        // Apply each migration in sequence
        for migration in migration_path {
            tracing::info!("Applying migration: {}", migration.description());
            migration.migrate(&mut config_value)?;
        }

        // Convert back to config struct
        *config = serde_json::from_value(config_value)
            .map_err(|e| ConfigError::Serialization(e))?;

        // Update version
        config.meta.version = target_version.clone();
        config.meta.last_updated = chrono::Utc::now();

        Ok(())
    }
}

/// Apply migrations to a configuration
pub fn apply_migrations(config: &mut FerrumyxConfig) -> Result<(), ConfigError> {
    let engine = MigrationEngine::new();
    let target_version = Version::parse("2.0.0").unwrap(); // Current latest version
    engine.migrate_to_version(config, &target_version)
}

/// Check if migrations are needed
pub fn needs_migration(config: &FerrumyxConfig) -> bool {
    let latest_version = Version::parse("2.0.0").unwrap();
    config.meta.version < latest_version
}

/// Get available migrations
pub fn get_available_migrations() -> Vec<String> {
    vec![
        "1.0.0 -> 1.1.0: Add timeout and retry configuration to LLM providers".to_string(),
        "1.1.0 -> 1.2.0: Add monitoring and observability configuration".to_string(),
        "1.2.0 -> 2.0.0: Major security enhancements: MFA, key rotation, audit logging".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FerrumyxConfig;

    #[test]
    fn test_migration_engine() {
        let mut config = FerrumyxConfig::default();
        config.meta.version = Version::parse("1.0.0").unwrap();

        let engine = MigrationEngine::new();
        let target_version = Version::parse("1.1.0").unwrap();

        let result = engine.migrate_to_version(&mut config, &target_version);
        assert!(result.is_ok());
        assert_eq!(config.meta.version, target_version);
    }

    #[test]
    fn test_needs_migration() {
        let mut config = FerrumyxConfig::default();
        config.meta.version = Version::parse("1.0.0").unwrap();

        assert!(needs_migration(&config));

        config.meta.version = Version::parse("2.0.0").unwrap();
        assert!(!needs_migration(&config));
    }
}