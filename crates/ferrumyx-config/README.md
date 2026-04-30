# Ferrumyx Configuration System

A comprehensive, unified configuration management system for Ferrumyx with hierarchical loading, validation, and hot reloading capabilities.

## Features

- **Hierarchical Loading**: secrets → environment variables → config files → defaults
- **JSON Schema Validation**: Compile-time and runtime validation of configurations
- **Migration Framework**: Automated configuration upgrades between versions
- **Type Safety**: Strongly typed configuration structures with validation
- **Hot Reloading**: Runtime configuration updates without restart
- **Unified Management**: Single source of truth for all Ferrumyx configurations

## Configuration Areas

- **Database**: PostgreSQL, Redis, and other database backends
- **LLM Providers**: OpenAI, Anthropic, and custom providers
- **Security**: JWT, encryption, audit logging, and authentication
- **Performance**: Batch sizes, timeouts, caching, and resource limits
- **Channels**: WhatsApp, Slack, Discord, and webhook integrations
- **Monitoring**: Prometheus metrics, logging, health checks, and alerting

## Usage

### Basic Configuration Loading

```rust
use ferrumyx_config::FerrumyxConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration with defaults
    let config = FerrumyxConfig::load().await?;

    // Validate configuration
    config.validate()?;

    // Access configuration sections
    println!("Database URL: {}", config.database.url);
    println!("Default LLM provider: {}", config.llm.default_provider);

    Ok(())
}
```

### Configuration Validation

```rust
use ferrumyx_config::FerrumyxConfig;

let config = FerrumyxConfig::default();

// Validate returns a Result
match config.validate() {
    Ok(_) => println!("Configuration is valid"),
    Err(e) => eprintln!("Configuration error: {}", e),
}
```

### Custom Configuration Loading

```rust
use ferrumyx_config::{FerrumyxConfig, ConfigSource};

// Load from specific sources
let config = FerrumyxConfig::load_with_sources(vec![
    ConfigSource::File("./my-config.toml".to_string()),
    ConfigSource::Environment,
    ConfigSource::Defaults,
]).await?;
```

## Configuration File Formats

The system supports multiple configuration file formats:

### TOML Format

```toml
version = "1.0.0"

[database]
url = "postgresql://localhost:5432/ferrumyx"
pool_size = 10
backend = "postgresql"

[llm]
default_provider = "openai"

[llm.providers.openai]
provider_type = "openai"
api_key = "sk-..."
base_url = "https://api.openai.com/v1"
model = "gpt-4"

[security]
jwt_secret = "your-jwt-secret-key"
encryption_key = "your-encryption-key"
audit_enabled = true

[channels]
enabled = true

[channels.slack]
bot_token = "xoxb-your-slack-token"
signing_secret = "your-signing-secret"
enabled = true
```

### JSON Format

```json
{
  "version": "1.0.0",
  "database": {
    "url": "postgresql://localhost:5432/ferrumyx",
    "pool_size": 10,
    "backend": "postgresql"
  },
  "llm": {
    "default_provider": "openai",
    "providers": {
      "openai": {
        "provider_type": "openai",
        "api_key": "sk-...",
        "base_url": "https://api.openai.com/v1",
        "model": "gpt-4"
      }
    }
  }
}
```

## Environment Variables

Configuration can be overridden using environment variables:

```bash
# Database
export DATABASE_URL="postgresql://prod-db:5432/ferrumyx"
export DATABASE_POOL_SIZE="20"

# LLM
export LLM_DEFAULT_PROVIDER="anthropic"
export LLM_PROVIDERS_ANTHROPIC_API_KEY="sk-ant-..."

# Security
export SECURITY_JWT_SECRET="production-jwt-secret"
export SECURITY_ENCRYPTION_KEY="production-encryption-key"
```

## Configuration Sections

### Database Configuration

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,           // Connection URL
    pub pool_size: u32,        // Connection pool size
    pub backend: String,       // Database backend type
}
```

### LLM Configuration

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub default_provider: String,
    pub providers: HashMap<String, LlmProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    pub provider_type: String,
    pub api_key: Option<String>,
    pub base_url: String,
    pub model: String,
}
```

### Security Configuration

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub jwt_secret: String,
    pub encryption_key: String,
    pub audit_enabled: bool,
}
```

### Channel Configuration

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelsConfig {
    pub enabled: bool,
    pub whatsapp: Option<WhatsAppConfig>,
    pub slack: Option<SlackConfig>,
    pub discord: Option<DiscordConfig>,
}
```

## Error Handling

The configuration system provides detailed error messages:

```rust
use ferrumyx_config::ConfigError;

match config.validate() {
    Ok(_) => println!("Valid configuration"),
    Err(ConfigError::Validation(msg)) => {
        eprintln!("Validation failed: {}", msg);
    }
    Err(ConfigError::Io(err)) => {
        eprintln!("File I/O error: {}", err);
    }
    Err(ConfigError::Serialization(err)) => {
        eprintln!("Serialization error: {}", err);
    }
    _ => eprintln!("Other configuration error"),
}
```

## Development

### Running Tests

```bash
cargo test -p ferrumyx-config
```

### Building Documentation

```bash
cargo doc -p ferrumyx-config --open
```

## Future Enhancements

- JSON Schema validation with custom validators
- Configuration migration framework
- Hot reloading with file watching
- Secrets management integration
- Environment-specific configuration profiles
- Configuration encryption at rest