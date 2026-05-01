//! Configuration backup and restore functionality

use chrono::{DateTime, Utc};
use ferrumyx_config::FerrumyxConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Backup configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigBackup {
    /// Backup metadata
    pub metadata: BackupMetadata,
    /// Ferrumyx configuration
    pub config: FerrumyxConfig,
    /// Environment variables (optional, only if include_sensitive is true)
    pub environment: Option<HashMap<String, String>>,
    /// Additional files (relative paths and contents)
    pub additional_files: HashMap<String, String>,
}

/// Backup metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    /// Backup version
    pub version: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Environment type
    pub environment: String,
    /// Ferrumyx version
    pub ferrumyx_version: String,
    /// Include sensitive data flag
    pub includes_sensitive: bool,
    /// Backup description
    pub description: Option<String>,
}

/// Create configuration backup
pub async fn create_backup(name: &str, include_sensitive: bool) -> Result<(), Box<dyn std::error::Error>> {
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let backup_filename = format!("{}_{}.json", name, timestamp);
    let backup_path = PathBuf::from(&backup_filename);

    println!("Creating backup: {}", backup_filename);

    // Load current configuration
    let config = FerrumyxConfig::load().await?;

    // Collect environment variables
    let environment = if include_sensitive {
        Some(collect_environment_variables()?)
    } else {
        None
    };

    // Collect additional configuration files
    let additional_files = collect_additional_files().await?;

    // Create backup structure
    let backup = ConfigBackup {
        metadata: BackupMetadata {
            version: "1.0".to_string(),
            created_at: Utc::now(),
            environment: std::env::var("NODE_ENV").unwrap_or_else(|_| "development".to_string()),
            ferrumyx_version: env!("CARGO_PKG_VERSION").to_string(),
            includes_sensitive: include_sensitive,
            description: Some(format!("Ferrumyx configuration backup created on {}", Utc::now().format("%Y-%m-%d %H:%M:%S UTC"))),
        },
        config,
        environment,
        additional_files,
    };

    // Serialize and write backup
    let backup_json = serde_json::to_string_pretty(&backup)?;
    fs::write(&backup_path, backup_json)?;

    println!("✅ Backup created successfully: {}", backup_path.display());

    // Display backup summary
    display_backup_summary(&backup, &backup_path);

    Ok(())
}

/// Restore configuration from backup
pub async fn restore_backup(backup_path: &PathBuf, restore_sensitive: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Restoring from backup: {}", backup_path.display());

    // Read and parse backup file
    let backup_content = fs::read_to_string(backup_path)?;
    let backup: ConfigBackup = serde_json::from_str(&backup_content)?;

    // Validate backup compatibility
    validate_backup_compatibility(&backup)?;

    // Confirm restoration
    if !confirm_restore(&backup)? {
        println!("Restore cancelled by user");
        return Ok(());
    }

    // Restore Ferrumyx configuration
    restore_ferrumyx_config(&backup.config).await?;

    // Restore environment variables if requested and available
    if restore_sensitive {
        if let Some(env_vars) = &backup.environment {
            restore_environment_variables(env_vars)?;
        } else {
            println!("⚠️  No sensitive data found in backup");
        }
    }

    // Restore additional files
    restore_additional_files(&backup.additional_files).await?;

    println!("✅ Configuration restored successfully!");
    println!("You may need to restart services for changes to take effect.");

    Ok(())
}

/// Collect environment variables from .env file
fn collect_environment_variables() -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let mut env_vars = HashMap::new();

    // Try to read .env file
    if let Ok(content) = fs::read_to_string(".env") {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                env_vars.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
    }

    // Also collect from current environment (for any missing values)
    for (key, value) in std::env::vars() {
        if key.starts_with("FERRUMYX_") || key.starts_with("DATABASE_") || key.starts_with("REDIS_") {
            env_vars.entry(key).or_insert(value);
        }
    }

    Ok(env_vars)
}

/// Collect additional configuration files
async fn collect_additional_files() -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let mut files = HashMap::new();
    let config_files = vec![
        "ferrumyx.toml",
        "docker-compose.yml",
        "docker-compose.dev.yml",
        "docker-compose.prod.yml",
        ".env.example",
        "config/ferrumyx.example.toml",
    ];

    for file_path in config_files {
        let path = PathBuf::from(file_path);
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    files.insert(file_path.to_string(), content);
                }
                Err(e) => {
                    println!("Warning: Could not read {}: {}", file_path, e);
                }
            }
        }
    }

    Ok(files)
}

/// Validate backup compatibility
fn validate_backup_compatibility(backup: &ConfigBackup) -> Result<(), Box<dyn std::error::Error>> {
    // Check version compatibility
    if backup.metadata.version != "1.0" {
        println!("⚠️  Backup version {} may not be fully compatible with current version",
                 backup.metadata.version);
    }

    // Check Ferrumyx version compatibility
    let current_version = env!("CARGO_PKG_VERSION");
    if backup.metadata.ferrumyx_version != current_version {
        println!("⚠️  Backup was created with Ferrumyx v{}, current version is v{}",
                 backup.metadata.ferrumyx_version, current_version);
        println!("   Some configuration may need manual adjustment.");
    }

    Ok(())
}

/// Confirm restore operation
fn confirm_restore(backup: &ConfigBackup) -> Result<bool, Box<dyn std::error::Error>> {
    println!();
    println!("Backup Details:");
    println!("  Created: {}", backup.metadata.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
    println!("  Environment: {}", backup.metadata.environment);
    println!("  Ferrumyx Version: {}", backup.metadata.ferrumyx_version);
    println!("  Contains sensitive data: {}", backup.metadata.includes_sensitive);
    println!();

    println!("This will overwrite existing configuration files!");
    println!("Make sure to backup your current configuration if needed.");
    println!();

    // In a real implementation, you'd use a proper confirmation prompt
    // For now, we'll assume the user has confirmed
    Ok(true)
}

/// Restore Ferrumyx configuration to file
async fn restore_ferrumyx_config(config: &FerrumyxConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Write ferrumyx.toml
    let toml_content = toml::to_string_pretty(config)?;
    fs::write("ferrumyx.toml", toml_content)?;

    println!("✅ Restored ferrumyx.toml");

    Ok(())
}

/// Restore environment variables to .env file
fn restore_environment_variables(env_vars: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut env_content = String::from("# Restored from backup\n# Generated on ");

    env_content.push_str(&Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string());
    env_content.push_str("\n\n");

    // Sort keys for consistent output
    let mut sorted_keys: Vec<_> = env_vars.keys().collect();
    sorted_keys.sort();

    for key in sorted_keys {
        if let Some(value) = env_vars.get(key) {
            env_content.push_str(&format!("{}={}\n", key, value));
        }
    }

    fs::write(".env", env_content)?;

    println!("✅ Restored environment variables to .env");

    Ok(())
}

/// Restore additional configuration files
async fn restore_additional_files(files: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
    for (file_path, content) in files {
        // Create directory if it doesn't exist
        if let Some(parent) = PathBuf::from(file_path).parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(file_path, content)?;
        println!("✅ Restored {}", file_path);
    }

    Ok(())
}

/// Display backup summary
fn display_backup_summary(backup: &ConfigBackup, backup_path: &PathBuf) {
    println!();
    println!("Backup Summary:");
    println!("  File: {}", backup_path.display());
    println!("  Created: {}", backup.metadata.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
    println!("  Environment: {}", backup.metadata.environment);
    println!("  Configuration version: {}", backup.config.version);
    println!("  LLM Providers: {}", backup.config.llm.providers.len());
    println!("  Multi-channel enabled: {}", backup.config.channels.enabled);
    println!("  Sensitive data included: {}", backup.metadata.includes_sensitive);
    println!("  Additional files: {}", backup.additional_files.len());

    if let Some(env_vars) = &backup.environment {
        println!("  Environment variables: {}", env_vars.len());
    }

    println!();
    println!("To restore this backup, run:");
    println!("  ferrumyx-setup restore {}", backup_path.display());
}

/// List available backups
pub fn list_backups() -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut backups = Vec::new();

    for entry in fs::read_dir(".")? {
        let entry = entry?;
        let path = entry.path();

        if let Some(extension) = path.extension() {
            if extension == "json" && path.file_name()
                .and_then(|n| n.to_str())
                .map_or(false, |n| n.contains("backup") || n.contains("ferrumyx-config"))
            {
                backups.push(path);
            }
        }
    }

    // Sort by modification time (newest first)
    backups.sort_by(|a, b| {
        let a_time = a.metadata().and_then(|m| m.modified()).ok();
        let b_time = b.metadata().and_then(|m| m.modified()).ok();
        b_time.cmp(&a_time)
    });

    Ok(backups)
}

/// Clean old backups
pub fn clean_old_backups(keep_count: usize) -> Result<(), Box<dyn std::error::Error>> {
    let backups = list_backups()?;

    if backups.len() <= keep_count {
        println!("No old backups to clean (keeping {})", keep_count);
        return Ok(());
    }

    let to_delete = &backups[keep_count..];
    println!("Cleaning {} old backups...", to_delete.len());

    for backup in to_delete {
        fs::remove_file(backup)?;
        println!("  Deleted: {}", backup.display());
    }

    println!("✅ Cleaned {} old backups", to_delete.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn test_backup_creation() {
        // Create a temporary directory for testing
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a minimal config file
        let config_content = r#"
[database]
url = "sqlite://./test.db"
pool_size = 5
backend = "sqlite"

[llm]
default_provider = "ollama"

[llm.ollama]
base_url = "http://localhost:11434"
model = "llama3.1:8b"

[security]
jwt_secret = "test-jwt-secret-at-least-32-characters-long"
encryption_key = "test-encryption-key-at-least-32-characters"

[performance]
max_concurrent = 50
timeout_seconds = 60
caching_enabled = true

[channels]
enabled = false

[monitoring]
metrics_enabled = true
metrics_endpoint = "/metrics"
log_level = "info"
health_checks_enabled = true
"#;
        fs::write("ferrumyx.toml", config_content).unwrap();

        // Create backup
        create_backup("test", false).await.unwrap();

        // Check that backup file was created
        let backup_files: Vec<_> = fs::read_dir(".")
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_str().unwrap().starts_with("test_"))
            .collect();

        assert_eq!(backup_files.len(), 1);

        // Verify backup content
        let backup_path = backup_files[0].path();
        let backup_content = fs::read_to_string(&backup_path).unwrap();
        let backup: ConfigBackup = serde_json::from_str(&backup_content).unwrap();

        assert_eq!(backup.metadata.version, "1.0");
        assert!(!backup.metadata.includes_sensitive);
        assert!(backup.environment.is_none());
    }

    #[test]
    fn test_list_backups() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create some backup files
        fs::write("ferrumyx-config-backup-20240101.json", "{}").unwrap();
        fs::write("test-backup-20240102.json", "{}").unwrap();
        fs::write("not-a-backup.json", "{}").unwrap();
        fs::write("regular-file.txt", "test").unwrap();

        let backups = list_backups().unwrap();
        assert_eq!(backups.len(), 2);
        // Should be sorted by modification time (newest first)
        assert!(backups[0].file_name().unwrap() == "test-backup-20240102.json" ||
                backups[0].file_name().unwrap() == "ferrumyx-config-backup-20240101.json");
    }
}