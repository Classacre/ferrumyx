//! Interactive setup wizard for Ferrumyx configuration

use crate::config::{generate_config_files, ConfigOptions};
use crate::security::{generate_secure_password, generate_secure_key, validate_api_key};
use crate::validate::validate_database_url;
use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Confirm, FuzzySelect, Input, Password, Select};
use ferrumyx_config::{
    ChannelsConfig, DatabaseConfig, DiscordConfig, FerrumyxConfig, LlmConfig, LlmProviderConfig,
    MonitoringConfig, PerformanceConfig, SecurityConfig, SlackConfig, WhatsAppConfig,
};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::path::PathBuf;

/// Environment types
#[derive(Debug, Clone)]
pub enum Environment {
    Development,
    Production,
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Environment::Development => write!(f, "Development"),
            Environment::Production => write!(f, "Production"),
        }
    }
}

/// Run interactive setup wizard
pub async fn run_interactive(env_type: &str, output_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let term = Term::stdout();
    let theme = ColorfulTheme::default();

    // Welcome message
    term.clear_screen()?;
    println!("{}", style("Welcome to Ferrumyx Setup Wizard").bold().cyan());
    println!("{}", style("==================================").cyan());
    println!();

    // Environment selection
    let environment = if env_type == "development" {
        Environment::Development
    } else if env_type == "production" {
        Environment::Production
    } else {
        let env_options = vec!["Development", "Production"];
        let selection = FuzzySelect::with_theme(&theme)
            .with_prompt("Select environment type")
            .items(&env_options)
            .default(0)
            .interact()?;

        match selection {
            0 => Environment::Development,
            1 => Environment::Production,
            _ => Environment::Development,
        }
    };

    println!("Selected environment: {}", style(environment.to_string()).green());
    println!();

    // Progress bar for setup steps
    let pb = ProgressBar::new(8);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    // Step 1: Database configuration
    pb.set_message("Configuring database...");
    let db_config = configure_database(&theme, &environment).await?;
    pb.inc(1);

    // Step 2: LLM provider setup
    pb.set_message("Setting up LLM providers...");
    let llm_config = configure_llm_providers(&theme, &environment).await?;
    pb.inc(1);

    // Step 3: Security settings
    pb.set_message("Configuring security...");
    let security_config = configure_security(&theme, &environment).await?;
    pb.inc(1);

    // Step 4: Multi-channel configuration
    pb.set_message("Setting up multi-channel integration...");
    let channels_config = configure_channels(&theme).await?;
    pb.inc(1);

    // Step 5: Performance tuning
    pb.set_message("Configuring performance settings...");
    let performance_config = configure_performance(&theme, &environment).await?;
    pb.inc(1);

    // Step 6: Monitoring setup
    pb.set_message("Setting up monitoring...");
    let monitoring_config = configure_monitoring(&theme, &environment).await?;
    pb.inc(1);

    // Step 7: Generate configuration files
    pb.set_message("Generating configuration files...");
    let config = FerrumyxConfig {
        version: "1.0.0".to_string(),
        database: db_config,
        llm: llm_config,
        security: security_config,
        performance: performance_config,
        channels: channels_config,
        monitoring: monitoring_config,
    };

    let config_options = ConfigOptions {
        environment: environment.clone(),
        output_dir: output_dir.clone(),
        include_env_file: true,
        include_toml_file: true,
        generate_secrets: true,
    };

    generate_config_files(&config, &config_options).await?;
    pb.inc(1);

    // Step 8: Validation
    pb.set_message("Validating configuration...");
    config.validate()?;
    pb.inc(1);

    pb.finish_with_message("Setup completed successfully!");

    // Summary
    display_summary(&config, output_dir);

    Ok(())
}

/// Run non-interactive setup with defaults
pub async fn run_non_interactive(env_type: &str, output_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let environment = if env_type == "development" {
        Environment::Development
    } else {
        Environment::Production
    };

    // Generate default configuration
    let config = generate_default_config(&environment).await?;

    let config_options = ConfigOptions {
        environment,
        output_dir: output_dir.clone(),
        include_env_file: true,
        include_toml_file: true,
        generate_secrets: true,
    };

    generate_config_files(&config, &config_options).await?;

    println!("{}", style("✅ Non-interactive setup completed!").green());
    Ok(())
}

/// Configure database settings
async fn configure_database(theme: &ColorfulTheme, environment: &Environment) -> Result<DatabaseConfig, Box<dyn std::error::Error>> {
    println!("{}", style("Database Configuration").bold().blue());
    println!("{}", style("====================").blue());

    let backend_options = vec!["PostgreSQL", "SQLite"];
    let backend_selection = Select::with_theme(theme)
        .with_prompt("Select database backend")
        .items(&backend_options)
        .default(0)
        .interact()?;

    let backend = match backend_selection {
        0 => "postgresql",
        1 => "sqlite",
        _ => "postgresql",
    };

    let url = if matches!(environment, Environment::Development) {
        match backend {
            "postgresql" => "postgresql://ferrumyx:password@localhost:5432/ferrumyx_dev".to_string(),
            "sqlite" => "./data/ferrumyx_dev.db".to_string(),
            _ => "./data/ferrumyx_dev.db".to_string(),
        }
    } else {
        let default_url = match backend {
            "postgresql" => "postgresql://ferrumyx:${POSTGRES_PASSWORD}@postgres:5432/ferrumyx".to_string(),
            "sqlite" => "./data/ferrumyx.db".to_string(),
            _ => "./data/ferrumyx.db".to_string(),
        };

        Input::<String>::with_theme(theme)
            .with_prompt("Database connection URL")
            .default(default_url)
            .validate_with(|input: &String| validate_database_url(input))
            .interact_text()?
    };

    let pool_size = match environment {
        Environment::Development => 5,
        Environment::Production => {
            Input::<u32>::with_theme(theme)
                .with_prompt("Database connection pool size")
                .default(20)
                .interact_text()?
        }
    };

    println!();
    Ok(DatabaseConfig {
        url,
        pool_size,
        backend: backend.to_string(),
    })
}

/// Configure LLM providers
async fn configure_llm_providers(theme: &ColorfulTheme, environment: &Environment) -> Result<LlmConfig, Box<dyn std::error::Error>> {
    println!("{}", style("LLM Provider Configuration").bold().blue());
    println!("{}", style("==========================").blue());

    let provider_options = vec![
        "OpenAI",
        "Anthropic (Claude)",
        "Google Gemini",
        "Ollama (Local)",
        "OpenAI Compatible",
        "Skip for now",
    ];

    let provider_selection = FuzzySelect::with_theme(theme)
        .with_prompt("Select default LLM provider")
        .items(&provider_options)
        .default(3) // Ollama default
        .interact()?;

    let (default_provider, providers) = match provider_selection {
        0 => configure_openai(theme, environment).await?,
        1 => configure_anthropic(theme, environment).await?,
        2 => configure_gemini(theme, environment).await?,
        3 => configure_ollama(theme).await?,
        4 => configure_openai_compatible(theme).await?,
        _ => ("ollama".to_string(), HashMap::new()),
    };

    println!();
    Ok(LlmConfig {
        default_provider,
        providers,
    })
}

/// Configure OpenAI provider
async fn configure_openai(theme: &ColorfulTheme, environment: &Environment) -> Result<(String, HashMap<String, LlmProviderConfig>), Box<dyn std::error::Error>> {
    let api_key = if matches!(environment, Environment::Development) {
        Input::<String>::with_theme(theme)
            .with_prompt("OpenAI API Key (leave empty to skip)")
            .allow_empty(true)
            .interact_text()?
    } else {
        Password::with_theme(theme)
            .with_prompt("OpenAI API Key")
            .validate_with(|input: &String| validate_api_key("openai", input))
            .interact()?
    };

    if api_key.is_empty() {
        return Ok(("ollama".to_string(), HashMap::new()));
    }

    let mut providers = HashMap::new();
    providers.insert("openai".to_string(), LlmProviderConfig {
        provider_type: "openai".to_string(),
        api_key: Some(api_key),
        base_url: "https://api.openai.com/v1".to_string(),
        model: "gpt-4o-mini".to_string(),
    });

    Ok(("openai".to_string(), providers))
}

/// Configure Anthropic provider
async fn configure_anthropic(theme: &ColorfulTheme, environment: &Environment) -> Result<(String, HashMap<String, LlmProviderConfig>), Box<dyn std::error::Error>> {
    let api_key = if matches!(environment, Environment::Development) {
        Input::<String>::with_theme(theme)
            .with_prompt("Anthropic API Key (leave empty to skip)")
            .allow_empty(true)
            .interact_text()?
    } else {
        Password::with_theme(theme)
            .with_prompt("Anthropic API Key")
            .validate_with(|input: &String| validate_api_key("anthropic", input))
            .interact()?
    };

    if api_key.is_empty() {
        return Ok(("ollama".to_string(), HashMap::new()));
    }

    let mut providers = HashMap::new();
    providers.insert("anthropic".to_string(), LlmProviderConfig {
        provider_type: "anthropic".to_string(),
        api_key: Some(api_key),
        base_url: "https://api.anthropic.com".to_string(),
        model: "claude-3-5-haiku-20241022".to_string(),
    });

    Ok(("anthropic".to_string(), providers))
}

/// Configure Gemini provider
async fn configure_gemini(theme: &ColorfulTheme, environment: &Environment) -> Result<(String, HashMap<String, LlmProviderConfig>), Box<dyn std::error::Error>> {
    let api_key = if matches!(environment, Environment::Development) {
        Input::<String>::with_theme(theme)
            .with_prompt("Google AI API Key (leave empty to skip)")
            .allow_empty(true)
            .interact_text()?
    } else {
        Password::with_theme(theme)
            .with_prompt("Google AI API Key")
            .validate_with(|input: &String| validate_api_key("gemini", input))
            .interact()?
    };

    if api_key.is_empty() {
        return Ok(("ollama".to_string(), HashMap::new()));
    }

    let mut providers = HashMap::new();
    providers.insert("gemini".to_string(), LlmProviderConfig {
        provider_type: "gemini".to_string(),
        api_key: Some(api_key),
        base_url: "https://generativelanguage.googleapis.com".to_string(),
        model: "gemini-1.5-flash".to_string(),
    });

    Ok(("gemini".to_string(), providers))
}

/// Configure Ollama provider
async fn configure_ollama(theme: &ColorfulTheme) -> Result<(String, HashMap<String, LlmProviderConfig>), Box<dyn std::error::Error>> {
    let base_url = Input::<String>::with_theme(theme)
        .with_prompt("Ollama base URL")
        .default("http://localhost:11434".to_string())
        .interact_text()?;

    let model = Input::<String>::with_theme(theme)
        .with_prompt("Ollama model name")
        .default("llama3.1:8b".to_string())
        .interact_text()?;

    let mut providers = HashMap::new();
    providers.insert("ollama".to_string(), LlmProviderConfig {
        provider_type: "ollama".to_string(),
        api_key: None,
        base_url,
        model,
    });

    Ok(("ollama".to_string(), providers))
}

/// Configure OpenAI Compatible provider
async fn configure_openai_compatible(theme: &ColorfulTheme) -> Result<(String, HashMap<String, LlmProviderConfig>), Box<dyn std::error::Error>> {
    let base_url = Input::<String>::with_theme(theme)
        .with_prompt("API base URL")
        .default("https://api.groq.com/openai".to_string())
        .interact_text()?;

    let api_key = Password::with_theme(theme)
        .with_prompt("API Key")
        .interact()?;

    let model = Input::<String>::with_theme(theme)
        .with_prompt("Model name")
        .default("llama-3.1-70b-versatile".to_string())
        .interact_text()?;

    let mut providers = HashMap::new();
    providers.insert("openai_compatible".to_string(), LlmProviderConfig {
        provider_type: "openai_compatible".to_string(),
        api_key: Some(api_key),
        base_url,
        model,
    });

    Ok(("openai_compatible".to_string(), providers))
}

/// Configure security settings
async fn configure_security(theme: &ColorfulTheme, _environment: &Environment) -> Result<SecurityConfig, Box<dyn std::error::Error>> {
    println!("{}", style("Security Configuration").bold().blue());
    println!("{}", style("======================").blue());

    let generate_keys = Confirm::with_theme(theme)
        .with_prompt("Generate secure random keys?")
        .default(true)
        .interact()?;

    let (jwt_secret, encryption_key) = if generate_keys {
        println!("Generating secure keys...");
        let jwt = generate_secure_key(32);
        let enc = generate_secure_key(32);
        (jwt, enc)
    } else {
        let jwt = Password::with_theme(theme)
            .with_prompt("JWT Secret (32+ characters)")
            .validate_with(|input: &String| {
                if input.len() < 32 {
                    Err("JWT secret must be at least 32 characters")
                } else {
                    Ok(())
                }
            })
            .interact()?;

        let enc = Password::with_theme(theme)
            .with_prompt("Encryption Key (32+ characters)")
            .validate_with(|input: &String| {
                if input.len() < 32 {
                    Err("Encryption key must be at least 32 characters")
                } else {
                    Ok(())
                }
            })
            .interact()?;

        (jwt, enc)
    };

    let audit_enabled = Confirm::with_theme(theme)
        .with_prompt("Enable audit logging?")
        .default(true)
        .interact()?;

    println!();
    Ok(SecurityConfig {
        jwt_secret,
        encryption_key,
        audit_enabled,
    })
}

/// Configure multi-channel integration
async fn configure_channels(theme: &ColorfulTheme) -> Result<ChannelsConfig, Box<dyn std::error::Error>> {
    println!("{}", style("Multi-Channel Integration").bold().blue());
    println!("{}", style("==========================").blue());

    let enabled = Confirm::with_theme(theme)
        .with_prompt("Enable multi-channel integration?")
        .default(false)
        .interact()?;

    if !enabled {
        return Ok(ChannelsConfig {
            enabled: false,
            whatsapp: None,
            slack: None,
            discord: None,
        });
    }

    // WhatsApp configuration
    let whatsapp = if Confirm::with_theme(theme)
        .with_prompt("Configure WhatsApp integration?")
        .default(false)
        .interact()?
    {
        Some(WhatsAppConfig {
            api_token: Password::with_theme(theme)
                .with_prompt("WhatsApp API Token")
                .interact()?,
            phone_number_id: Input::<String>::with_theme(theme)
                .with_prompt("WhatsApp Phone Number ID")
                .interact_text()?,
            enabled: true,
        })
    } else {
        None
    };

    // Slack configuration
    let slack = if Confirm::with_theme(theme)
        .with_prompt("Configure Slack integration?")
        .default(false)
        .interact()?
    {
        Some(SlackConfig {
            bot_token: Password::with_theme(theme)
                .with_prompt("Slack Bot Token")
                .interact()?,
            signing_secret: Password::with_theme(theme)
                .with_prompt("Slack Signing Secret")
                .interact()?,
            enabled: true,
        })
    } else {
        None
    };

    // Discord configuration
    let discord = if Confirm::with_theme(theme)
        .with_prompt("Configure Discord integration?")
        .default(false)
        .interact()?
    {
        Some(DiscordConfig {
            bot_token: Password::with_theme(theme)
                .with_prompt("Discord Bot Token")
                .interact()?,
            application_id: Input::<String>::with_theme(theme)
                .with_prompt("Discord Application ID")
                .interact_text()?,
            enabled: true,
        })
    } else {
        None
    };

    println!();
    Ok(ChannelsConfig {
        enabled: true,
        whatsapp,
        slack,
        discord,
    })
}

/// Configure performance settings
async fn configure_performance(theme: &ColorfulTheme, environment: &Environment) -> Result<PerformanceConfig, Box<dyn std::error::Error>> {
    println!("{}", style("Performance Configuration").bold().blue());
    println!("{}", style("==========================").blue());

    let max_concurrent = match environment {
        Environment::Development => 50,
        Environment::Production => {
            Input::<usize>::with_theme(theme)
                .with_prompt("Max concurrent requests")
                .default(200)
                .interact_text()?
        }
    };

    let timeout_seconds = Input::<u64>::with_theme(theme)
        .with_prompt("Request timeout (seconds)")
        .default(60)
        .interact_text()?;

    let caching_enabled = Confirm::with_theme(theme)
        .with_prompt("Enable response caching?")
        .default(true)
        .interact()?;

    println!();
    Ok(PerformanceConfig {
        max_concurrent,
        timeout_seconds,
        caching_enabled,
    })
}

/// Configure monitoring settings
async fn configure_monitoring(theme: &ColorfulTheme, environment: &Environment) -> Result<MonitoringConfig, Box<dyn std::error::Error>> {
    println!("{}", style("Monitoring Configuration").bold().blue());
    println!("{}", style("========================").blue());

    let metrics_enabled = Confirm::with_theme(theme)
        .with_prompt("Enable metrics collection?")
        .default(true)
        .interact()?;

    let metrics_endpoint = Input::<String>::with_theme(theme)
        .with_prompt("Metrics endpoint")
        .default("/metrics".to_string())
        .interact_text()?;

    let log_level = match environment {
        Environment::Development => "debug".to_string(),
        Environment::Production => {
            let levels = vec!["error", "warn", "info", "debug"];
            let selection = Select::with_theme(theme)
                .with_prompt("Log level")
                .items(&levels)
                .default(2) // info
                .interact()?;
            levels[selection].to_string()
        }
    };

    let health_checks_enabled = Confirm::with_theme(theme)
        .with_prompt("Enable health checks?")
        .default(true)
        .interact()?;

    println!();
    Ok(MonitoringConfig {
        metrics_enabled,
        metrics_endpoint,
        log_level,
        health_checks_enabled,
    })
}

/// Generate default configuration for non-interactive mode
async fn generate_default_config(environment: &Environment) -> Result<FerrumyxConfig, Box<dyn std::error::Error>> {
    let db_config = match environment {
        Environment::Development => DatabaseConfig {
            url: "postgresql://ferrumyx:password@localhost:5432/ferrumyx_dev".to_string(),
            pool_size: 5,
            backend: "postgresql".to_string(),
        },
        Environment::Production => DatabaseConfig {
            url: "postgresql://ferrumyx:${POSTGRES_PASSWORD}@postgres:5432/ferrumyx".to_string(),
            pool_size: 20,
            backend: "postgresql".to_string(),
        },
    };

    let mut providers = HashMap::new();
    providers.insert("ollama".to_string(), LlmProviderConfig {
        provider_type: "ollama".to_string(),
        api_key: None,
        base_url: "http://localhost:11434".to_string(),
        model: "llama3.1:8b".to_string(),
    });

    Ok(FerrumyxConfig {
        version: "1.0.0".to_string(),
        database: db_config,
        llm: LlmConfig {
            default_provider: "ollama".to_string(),
            providers,
        },
        security: SecurityConfig {
            jwt_secret: generate_secure_key(32),
            encryption_key: generate_secure_key(32),
            audit_enabled: true,
        },
        performance: PerformanceConfig {
            max_concurrent: if matches!(environment, Environment::Development) { 50 } else { 200 },
            timeout_seconds: 60,
            caching_enabled: true,
        },
        channels: ChannelsConfig {
            enabled: false,
            whatsapp: None,
            slack: None,
            discord: None,
        },
        monitoring: MonitoringConfig {
            metrics_enabled: true,
            metrics_endpoint: "/metrics".to_string(),
            log_level: if matches!(environment, Environment::Development) { "debug" } else { "info" }.to_string(),
            health_checks_enabled: true,
        },
    })
}

/// Display setup summary
fn display_summary(config: &FerrumyxConfig, output_dir: &PathBuf) {
    println!();
    println!("{}", style("Setup Complete!").bold().green());
    println!("{}", style("===============").green());
    println!();

    println!("Configuration Summary:");
    println!("• Database: {}", config.database.backend);
    println!("• LLM Provider: {}", config.llm.default_provider);
    println!("• Security: {}configured", if config.security.audit_enabled { "audit enabled, " } else { "" });
    println!("• Multi-channel: {}", if config.channels.enabled { "enabled" } else { "disabled" });
    println!("• Monitoring: {}", if config.monitoring.metrics_enabled { "enabled" } else { "disabled" });

    println!();
    println!("Generated files:");
    println!("• {}", output_dir.join("ferrumyx.toml").display());
    println!("• {}", output_dir.join(".env").display());

    println!();
    println!("Next steps:");
    println!("1. Review the generated configuration files");
    println!("2. Set any required API keys in .env");
    println!("3. Run: docker-compose up -d");
    println!("4. Access the web UI at http://localhost:3000");

    println!();
    println!("{}", style("Happy coding with Ferrumyx! 🚀").cyan());
}