//! # Ferrumyx Setup CLI
//!
//! Interactive configuration wizard for Ferrumyx deployment.
//! Provides secure credential handling, validation, and automated file generation.

mod wizard;
mod config;
mod security;
mod validate;
mod backup;

use clap::{Parser, Subcommand};
use console::style;
use ferrumyx_config::FerrumyxConfig;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ferrumyx-setup")]
#[command(version, about = "Interactive Ferrumyx configuration setup wizard")]
#[command(long_about = "
Ferrumyx Setup CLI - Interactive Configuration Wizard

This tool guides you through configuring Ferrumyx with secure credential handling,
input validation, and automated file generation for both development and production environments.

Features:
- Interactive step-by-step configuration wizard
- Secure password input with masking
- Real-time input validation with feedback
- Automated .env and configuration file generation
- Cross-platform support (Windows, macOS, Linux)
- Configuration backup and restore capabilities

Examples:
    ferrumyx-setup wizard          # Run interactive setup wizard
    ferrumyx-setup validate        # Validate existing configuration
    ferrumyx-setup backup          # Create configuration backup
    ferrumyx-setup restore backup.json  # Restore from backup
")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the interactive setup wizard
    Wizard {
        /// Environment type (development/production)
        #[arg(short, long, default_value = "development")]
        environment: String,

        /// Output directory for configuration files
        #[arg(short, long, default_value = ".")]
        output: PathBuf,

        /// Non-interactive mode (uses defaults)
        #[arg(short, long)]
        non_interactive: bool,
    },

    /// Validate existing configuration
    Validate {
        /// Configuration file to validate
        #[arg(short, long, default_value = "ferrumyx.toml")]
        config: PathBuf,

        /// Environment file to validate
        #[arg(short, long, default_value = ".env")]
        env_file: PathBuf,
    },

    /// Create configuration backup
    Backup {
        /// Backup file name
        #[arg(short, long, default_value = "ferrumyx-config-backup")]
        name: String,

        /// Include sensitive data in backup
        #[arg(short, long)]
        include_sensitive: bool,
    },

    /// Restore configuration from backup
    Restore {
        /// Backup file to restore from
        backup_file: PathBuf,

        /// Restore sensitive data
        #[arg(short, long)]
        restore_sensitive: bool,
    },

    /// Generate secure random keys and passwords
    Generate {
        /// Type of secret to generate (password/jwt-key/encryption-key)
        #[arg(short, long)]
        secret_type: String,

        /// Length of generated secret
        #[arg(short, long, default_value = "32")]
        length: usize,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Wizard {
            environment,
            output,
            non_interactive,
        } => {
            println!("{}", style("🚀 Ferrumyx Setup Wizard").bold().cyan());
            println!("{}", style("==========================").cyan());

            if non_interactive {
                println!("Running in non-interactive mode...");
                wizard::run_non_interactive(&environment, &output).await?;
            } else {
                wizard::run_interactive(&environment, &output).await?;
            }
        }

        Commands::Validate { config, env_file } => {
            println!("{}", style("🔍 Validating Configuration").bold().blue());
            println!("{}", style("==========================").blue());

            validate::validate_configuration(&config, &env_file).await?;
            println!("{}", style("✅ Configuration validation passed!").green());
        }

        Commands::Backup { name, include_sensitive } => {
            println!("{}", style("💾 Creating Configuration Backup").bold().yellow());
            println!("{}", style("===============================").yellow());

            backup::create_backup(&name, include_sensitive).await?;
            println!("{}", style("✅ Backup created successfully!").green());
        }

        Commands::Restore { backup_file, restore_sensitive } => {
            println!("{}", style("🔄 Restoring Configuration").bold().magenta());
            println!("{}", style("==========================").magenta());

            backup::restore_backup(&backup_file, restore_sensitive).await?;
            println!("{}", style("✅ Configuration restored successfully!").green());
        }

        Commands::Generate { secret_type, length } => {
            println!("{}", style("🔐 Generating Secure Secret").bold().green());
            println!("{}", style("==========================").green());

            let secret = security::generate_secure_key(length);
            println!("Generated {}: {}", secret_type, style(&secret).red());
        }
    }

    Ok(())
}