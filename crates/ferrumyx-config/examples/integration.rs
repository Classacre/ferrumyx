//! Example integration of the new ferrumyx-config system
//!
//! This example shows how to use the unified configuration system
//! in place of the existing scattered configuration approaches.

use ferrumyx_config::{FerrumyxConfig, ConfigError};

#[tokio::main]
async fn main() -> Result<(), ConfigError> {
    println!("🔧 Loading Ferrumyx Configuration...");

    // Load configuration with hierarchical loading
    let config = FerrumyxConfig::load().await?;
    println!("✅ Configuration loaded successfully");

    // Validate configuration
    config.validate()?;
    println!("✅ Configuration validation passed");

    // Display configuration summary
    println!("\n📊 Configuration Summary:");
    println!("Version: {}", config.version);
    println!("Database: {} (pool: {})", config.database.backend, config.database.pool_size);
    println!("LLM Provider: {}", config.llm.default_provider);
    println!("Security: JWT enabled, Audit: {}", config.security.audit_enabled);
    println!("Performance: Max concurrent: {}, Timeout: {}s",
             config.performance.max_concurrent, config.performance.timeout_seconds);
    println!("Channels: Enabled: {}", config.channels.enabled);
    println!("Monitoring: Metrics: {}, Health checks: {}",
             config.monitoring.metrics_enabled, config.monitoring.health_checks_enabled);

    // Example: Configure database connection
    println!("\n🗄️ Database Configuration:");
    println!("URL: {}", mask_sensitive(&config.database.url));
    println!("Backend: {}", config.database.backend);
    println!("Pool Size: {}", config.database.pool_size);

    // Example: Configure LLM providers
    println!("\n🤖 LLM Configuration:");
    println!("Default Provider: {}", config.llm.default_provider);
    println!("Available Providers: {}", config.llm.providers.len());

    for (name, provider) in &config.llm.providers {
        println!("  - {}: {} ({})",
                name,
                provider.provider_type,
                provider.model);
    }

    // Example: Configure channels
    println!("\n📱 Channel Configuration:");
    if config.channels.enabled {
        if let Some(ref slack) = config.channels.slack {
            println!("Slack: Enabled ({})", if slack.enabled { "active" } else { "inactive" });
        }
        if let Some(ref discord) = config.channels.discord {
            println!("Discord: Enabled ({})", if discord.enabled { "active" } else { "inactive" });
        }
        if let Some(ref whatsapp) = config.channels.whatsapp {
            println!("WhatsApp: Enabled ({})", if whatsapp.enabled { "active" } else { "inactive" });
        }
    } else {
        println!("Multi-channel support: Disabled");
    }

    // Example: Security overview
    println!("\n🔒 Security Configuration:");
    println!("JWT Secret: {}...", &config.security.jwt_secret[..16]);
    println!("Encryption Key: {}...", &config.security.encryption_key[..16]);
    println!("Audit Logging: {}", if config.security.audit_enabled { "Enabled" } else { "Disabled" });

    println!("\n🎉 Ferrumyx configuration system ready!");
    Ok(())
}

/// Mask sensitive information in URLs
fn mask_sensitive(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        if let Some(protocol_end) = url.find("://") {
            let protocol = &url[..protocol_end + 3];
            let rest = &url[at_pos..];
            format!("{}***:***{}", protocol, rest)
        } else {
            "***:***@***".to_string()
        }
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_config_integration() {
        let config = FerrumyxConfig::load().await.unwrap();
        assert!(config.validate().is_ok());
        assert_eq!(config.version, "1.0.0");
        assert!(!config.database.url.is_empty());
    }

    #[test]
    fn test_mask_sensitive() {
        assert_eq!(
            mask_sensitive("postgresql://user:pass@localhost:5432/db"),
            "postgresql://***:***@localhost:5432/db"
        );
        assert_eq!(
            mask_sensitive("redis://localhost:6379"),
            "redis://localhost:6379"
        );
    }
}