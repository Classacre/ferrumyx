//! Configuration validation using JSON Schema
//!
//! Provides compile-time and runtime validation of configuration files
//! with detailed error reporting and suggestions.

use crate::{ConfigError, FerrumyxConfig};
use jsonschema::{Draft, JSONSchema};
use secrecy::ExposeSecret;
use serde_json::Value;
use std::collections::HashMap;

/// Validation result with detailed error information
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed
    pub is_valid: bool,

    /// Validation errors
    pub errors: Vec<ValidationError>,

    /// Validation warnings
    pub warnings: Vec<ValidationWarning>,
}

/// Validation error with context
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error message
    pub message: String,

    /// JSON path to the invalid field
    pub path: String,

    /// Error code
    pub code: String,

    /// Suggested fix
    pub suggestion: Option<String>,
}

/// Validation warning
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// Warning message
    pub message: String,

    /// JSON path to the field
    pub path: String,

    /// Warning code
    pub code: String,

    /// Suggested improvement
    pub suggestion: Option<String>,
}

/// Configuration validator
pub struct ConfigValidator {
    /// JSON schemas for each config section
    schemas: HashMap<String, JSONSchema>,

    /// Custom validation rules
    custom_rules: Vec<Box<dyn CustomValidationRule>>,
}

/// Custom validation rule trait
pub trait CustomValidationRule: Send + Sync {
    /// Validate a configuration section
    fn validate(&self, config: &FerrumyxConfig, section: &str) -> Vec<ValidationError>;
}

/// Built-in validation rules
pub struct BuiltInValidationRules;

impl BuiltInValidationRules {
    /// Validate database configuration
    pub fn validate_database(config: &FerrumyxConfig) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Check PostgreSQL configuration
        if config.database.backend == crate::database::DatabaseBackend::PostgreSql {
            if config.database.postgresql.enable_pgvector && config.database.postgresql.vector_dimension == 0 {
                errors.push(ValidationError {
                    message: "Vector dimension must be greater than 0 when pgvector is enabled".to_string(),
                    path: "database.postgresql.vector_dimension".to_string(),
                    code: "invalid_vector_dimension".to_string(),
                    suggestion: Some("Set vector_dimension to a positive integer (e.g., 768)".to_string()),
                });
            }
        }

        // Check connection pool configuration
        if config.database.primary.pool_size == 0 {
            errors.push(ValidationError {
                message: "Database pool size must be greater than 0".to_string(),
                path: "database.primary.pool_size".to_string(),
                code: "invalid_pool_size".to_string(),
                suggestion: Some("Set pool_size to a positive integer (e.g., 10)".to_string()),
            });
        }

        errors
    }

    /// Validate LLM configuration
    pub fn validate_llm(config: &FerrumyxConfig) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Check default provider exists
        if !config.llm.providers.contains_key(&config.llm.default_provider) {
            errors.push(ValidationError {
                message: format!("Default LLM provider '{}' not found in providers list", config.llm.default_provider),
                path: "llm.default_provider".to_string(),
                code: "missing_default_provider".to_string(),
                suggestion: Some(format!("Add '{}' to the llm.providers configuration", config.llm.default_provider)),
            });
        }

        // Validate provider configurations
        for (name, provider) in &config.llm.providers {
            if provider.api_key.is_none() && matches!(provider.provider, crate::llm::LlmProvider::OpenAi | crate::llm::LlmProvider::Anthropic) {
                errors.push(ValidationError {
                    message: format!("Provider '{}' requires an API key", name),
                    path: format!("llm.providers.{}.api_key", name),
                    code: "missing_api_key".to_string(),
                    suggestion: Some("Add an api_key configuration for this provider".to_string()),
                });
            }
        }

        // Validate rate limits
        if config.llm.global_rate_limit.requests_per_minute == 0 {
            errors.push(ValidationError {
                message: "Global rate limit requests per minute must be greater than 0".to_string(),
                path: "llm.global_rate_limit.requests_per_minute".to_string(),
                code: "invalid_rate_limit".to_string(),
                suggestion: Some("Set requests_per_minute to a positive integer".to_string()),
            });
        }

        errors
    }

    /// Validate security configuration
    pub fn validate_security(config: &FerrumyxConfig) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Check JWT secret key length
        if config.security.jwt.secret_key.expose_secret().len() < 32 {
            errors.push(ValidationError {
                message: "JWT secret key should be at least 32 characters long".to_string(),
                path: "security.jwt.secret_key".to_string(),
                code: "weak_jwt_secret".to_string(),
                suggestion: Some("Use a longer, randomly generated secret key".to_string()),
            });
        }

        // Check password policy
        if config.security.auth.password_policy.min_length < 8 {
            errors.push(ValidationError {
                message: "Minimum password length should be at least 8 characters".to_string(),
                path: "security.auth.password_policy.min_length".to_string(),
                code: "weak_password_policy".to_string(),
                suggestion: Some("Set min_length to 8 or higher".to_string()),
            });
        }

        // Check encryption key
        if config.security.encryption.master_key.expose_secret().len() < 32 {
            errors.push(ValidationError {
                message: "Encryption master key should be at least 32 characters long".to_string(),
                path: "security.encryption.master_key".to_string(),
                code: "weak_encryption_key".to_string(),
                suggestion: Some("Use a longer, randomly generated encryption key".to_string()),
            });
        }

        errors
    }

    /// Validate performance configuration
    pub fn validate_performance(config: &FerrumyxConfig) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Check batch configuration
        if config.performance.batch.max_size == 0 {
            errors.push(ValidationError {
                message: "Batch max size must be greater than 0".to_string(),
                path: "performance.batch.max_size".to_string(),
                code: "invalid_batch_size".to_string(),
                suggestion: Some("Set max_size to a positive integer".to_string()),
            });
        }

        // Check worker thread count
        if config.performance.workers.threads == 0 {
            errors.push(ValidationError {
                message: "Worker thread count must be greater than 0".to_string(),
                path: "performance.workers.threads".to_string(),
                code: "invalid_thread_count".to_string(),
                suggestion: Some("Set threads to a positive integer".to_string()),
            });
        }

        // Check memory limits
        if config.performance.limits.max_memory_bytes == 0 {
            errors.push(ValidationError {
                message: "Maximum memory limit must be greater than 0".to_string(),
                path: "performance.limits.max_memory_bytes".to_string(),
                code: "invalid_memory_limit".to_string(),
                suggestion: Some("Set max_memory_bytes to a positive value".to_string()),
            });
        }

        errors
    }

    /// Validate channels configuration
    pub fn validate_channels(config: &FerrumyxConfig) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Check WhatsApp configuration
        if let Some(whatsapp) = &config.channels.whatsapp {
            if whatsapp.api_token.expose_secret().is_empty() {
                errors.push(ValidationError {
                    message: "WhatsApp API token cannot be empty".to_string(),
                    path: "channels.whatsapp.api_token".to_string(),
                    code: "missing_whatsapp_token".to_string(),
                    suggestion: Some("Provide a valid WhatsApp Business API token".to_string()),
                });
            }
        }

        // Check Slack configuration
        if let Some(slack) = &config.channels.slack {
            if slack.bot_token.expose_secret().is_empty() {
                errors.push(ValidationError {
                    message: "Slack bot token cannot be empty".to_string(),
                    path: "channels.slack.bot_token".to_string(),
                    code: "missing_slack_token".to_string(),
                    suggestion: Some("Provide a valid Slack bot token".to_string()),
                });
            }
        }

        // Check rate limiting
        if config.channels.global.enabled {
            for (i, webhook) in config.channels.webhooks.iter().enumerate() {
                if webhook.url.is_empty() {
                    errors.push(ValidationError {
                        message: format!("Webhook {} URL cannot be empty", i),
                        path: format!("channels.webhooks[{}].url", i),
                        code: "missing_webhook_url".to_string(),
                        suggestion: Some("Provide a valid webhook URL".to_string()),
                    });
                }
            }
        }

        errors
    }
}

impl ConfigValidator {
    /// Create a new configuration validator
    pub fn new() -> Result<Self, ConfigError> {
        let mut schemas = HashMap::new();

        // Load JSON schemas for each configuration section
        schemas.insert("database".to_string(), Self::load_schema("database")?);
        schemas.insert("llm".to_string(), Self::load_schema("llm")?);
        schemas.insert("security".to_string(), Self::load_schema("security")?);
        schemas.insert("performance".to_string(), Self::load_schema("performance")?);
        schemas.insert("channels".to_string(), Self::load_schema("channels")?);
        schemas.insert("monitoring".to_string(), Self::load_schema("monitoring")?);

        Ok(Self {
            schemas,
            custom_rules: vec![
                Box::new(BuiltInValidationRules),
            ],
        })
    }

    /// Load JSON schema for a configuration section
    fn load_schema(section: &str) -> Result<JSONSchema, ConfigError> {
        // In a real implementation, these would be embedded JSON schema files
        // For now, we'll create basic schemas programmatically
        let schema_value = match section {
            "database" => serde_json::json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "backend": {"type": "string", "enum": ["postgresql", "redis", "sqlite", "libsql"]},
                    "primary": {
                        "type": "object",
                        "properties": {
                            "url": {"type": "string"},
                            "pool_size": {"type": "integer", "minimum": 1}
                        },
                        "required": ["url"]
                    }
                },
                "required": ["backend", "primary"]
            }),
            "llm" => serde_json::json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "default_provider": {"type": "string"},
                    "providers": {"type": "object"}
                },
                "required": ["default_provider"]
            }),
            _ => serde_json::json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object"
            }),
        };

        JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&schema_value)
            .map_err(|e| ConfigError::Schema(format!("Failed to compile schema for {}: {:?}", section, e)))
    }

    /// Add a custom validation rule
    pub fn add_rule(&mut self, rule: Box<dyn CustomValidationRule>) {
        self.custom_rules.push(rule);
    }

    /// Validate the entire configuration
    pub fn validate_config(&self, config: &FerrumyxConfig) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // JSON Schema validation
        let config_value = serde_json::to_value(config).unwrap_or(Value::Null);
        for (section, schema) in &self.schemas {
            if let Value::Object(ref obj) = config_value {
                if let Some(section_value) = obj.get(section) {
                    let validation = schema.validate(section_value);
                    for err in validation {
                        errors.push(ValidationError {
                            message: format!("{:?}", err),
                            path: format!("{}.{}", section, err.instance_path),
                            code: "schema_validation".to_string(),
                            suggestion: None,
                        });
                    }
                }
            }
        }

        // Custom validation rules
        for rule in &self.custom_rules {
            errors.extend(rule.validate(config, "database"));
            errors.extend(rule.validate(config, "llm"));
            errors.extend(rule.validate(config, "security"));
            errors.extend(rule.validate(config, "performance"));
            errors.extend(rule.validate(config, "channels"));
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }
}

impl CustomValidationRule for BuiltInValidationRules {
    fn validate(&self, config: &FerrumyxConfig, section: &str) -> Vec<ValidationError> {
        match section {
            "database" => Self::validate_database(config),
            "llm" => Self::validate_llm(config),
            "security" => Self::validate_security(config),
            "performance" => Self::validate_performance(config),
            "channels" => Self::validate_channels(config),
            _ => vec![],
        }
    }
}

/// Validate a configuration with default rules
pub fn validate_config(config: &FerrumyxConfig) -> ValidationResult {
    let validator = ConfigValidator::new().unwrap_or_else(|_| ConfigValidator {
        schemas: HashMap::new(),
        custom_rules: vec![Box::new(BuiltInValidationRules)],
    });

    validator.validate_config(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FerrumyxConfig;

    #[test]
    fn test_validation_with_invalid_config() {
        let config = FerrumyxConfig::default();
        let result = validate_config(&config);

        // Default config should pass basic validation
        assert!(result.errors.is_empty() || result.errors.iter().all(|e| e.code == "missing_api_key"));
    }
}