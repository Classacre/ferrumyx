# Ferrumyx Setup CLI

A comprehensive interactive CLI setup tool for Ferrumyx that guides users through environment configuration with secure credential handling and validation.

## Features

- **Interactive Wizard**: Step-by-step configuration with user-friendly prompts
- **Secure Input**: Masked password input and secure credential handling
- **Input Validation**: Real-time validation with feedback
- **File Generation**: Automated creation of `.env` and configuration files
- **Cross-Platform**: Native support for Windows, macOS, Linux
- **Backup/Restore**: Configuration backup and restore capabilities
- **Non-Interactive Mode**: Automated setup for CI/CD pipelines

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/Classacre/ferrumyx.git
cd ferrumyx

# Build the setup tool
cargo build --release --package ferrumyx-setup

# The binary will be available at target/release/ferrumyx-setup
```

### Pre-built Binaries

Download the latest release from the [releases page](https://github.com/Classacre/ferrumyx/releases).

## Quick Start

### Interactive Setup

Run the interactive wizard to configure Ferrumyx:

```bash
ferrumyx-setup wizard
```

For development environment:

```bash
ferrumyx-setup wizard --environment development
```

For production environment:

```bash
ferrumyx-setup wizard --environment production
```

### Non-Interactive Setup

For automated deployments or CI/CD:

```bash
ferrumyx-setup wizard --environment production --non-interactive --output ./config
```

## Commands

### `wizard`

Run the interactive setup wizard.

```bash
ferrumyx-setup wizard [OPTIONS]
```

**Options:**
- `-e, --environment <ENV>`: Environment type (development/production) [default: development]
- `-o, --output <DIR>`: Output directory for configuration files [default: .]
- `--non-interactive`: Run in non-interactive mode with defaults

**What it configures:**
- Database settings (PostgreSQL/SQLite)
- LLM providers (OpenAI, Anthropic, Gemini, Ollama, etc.)
- Security keys (JWT, encryption)
- Multi-channel integration (WhatsApp, Slack, Discord)
- Performance tuning
- Monitoring settings

### `validate`

Validate existing configuration files.

```bash
ferrumyx-setup validate [OPTIONS]
```

**Options:**
- `-c, --config <FILE>`: Configuration file to validate [default: ferrumyx.toml]
- `-e, --env-file <FILE>`: Environment file to validate [default: .env]

### `backup`

Create a configuration backup.

```bash
ferrumyx-setup backup [OPTIONS]
```

**Options:**
- `-n, --name <NAME>`: Backup file name [default: ferrumyx-config-backup]
- `--include-sensitive`: Include sensitive data in backup

### `restore`

Restore configuration from backup.

```bash
ferrumyx-setup restore [OPTIONS] <BACKUP_FILE>
```

**Options:**
- `--restore-sensitive`: Restore sensitive data from backup

**Arguments:**
- `<BACKUP_FILE>`: Backup file to restore from

### `generate`

Generate secure random keys and passwords.

```bash
ferrumyx-setup generate [OPTIONS]
```

**Options:**
- `-s, --secret-type <TYPE>`: Type of secret (password/jwt-key/encryption-key)
- `-l, --length <LENGTH>`: Length of generated secret [default: 32]

## Configuration Structure

The setup tool generates the following files:

### `ferrumyx.toml`

Main configuration file containing:
- Version information
- Database settings
- LLM provider configurations
- Security settings
- Performance tuning
- Multi-channel integration
- Monitoring configuration

### `.env`

Environment variables file containing:
- Database connection strings
- API keys and secrets
- Service endpoints
- Runtime configuration

### `.env.example`

Template file with all available environment variables (safe to commit).

## Security Features

- **Masked Input**: Passwords and API keys are masked during input
- **Secure Generation**: Cryptographically secure random key generation
- **Validation**: Real-time input validation with helpful error messages
- **Backup Encryption**: Optional encryption for sensitive backup data
- **Audit Trail**: Configuration changes are logged

## Examples

### Development Setup

```bash
# Interactive development setup
ferrumyx-setup wizard --environment development

# This will configure:
# - SQLite database for local development
# - Ollama LLM provider
# - Debug logging
# - Development-friendly performance settings
```

### Production Setup

```bash
# Interactive production setup
ferrumyx-setup wizard --environment production

# This will configure:
# - PostgreSQL database
# - Multiple LLM providers
# - Production security settings
# - Optimized performance settings
# - Monitoring and alerting
```

### Automated Deployment

```bash
# Non-interactive setup for CI/CD
ferrumyx-setup wizard --environment production --non-interactive --output ./config

# Backup current configuration
ferrumyx-setup backup --name pre-deployment --include-sensitive

# Validate configuration
ferrumyx-setup validate --config ./config/ferrumyx.toml --env-file ./config/.env
```

### Configuration Management

```bash
# Create backup before changes
ferrumyx-setup backup --name before-upgrade

# Generate new encryption key
ferrumyx-setup generate --secret-type encryption-key --length 32

# Restore from backup if needed
ferrumyx-setup restore backup-file.json --restore-sensitive
```

## Integration with Existing Workflows

The setup tool integrates seamlessly with existing Ferrumyx scripts:

```bash
# Use setup tool to generate initial configuration
ferrumyx-setup wizard --environment development

# Then run existing setup scripts
bash scripts/setup.sh
```

## Troubleshooting

### Common Issues

1. **Permission Denied**: Ensure you have write permissions in the output directory
2. **Database Connection Failed**: Verify database credentials and network access
3. **API Key Invalid**: Check API key format and permissions
4. **Validation Errors**: Run `ferrumyx-setup validate` to check configuration syntax

### Debug Mode

Enable debug logging:

```bash
export RUST_LOG=ferrumyx_setup=debug
ferrumyx-setup wizard
```

### Reset Configuration

To start fresh:

```bash
# Backup current config
ferrumyx-setup backup --name backup-before-reset

# Remove configuration files
rm ferrumyx.toml .env

# Run setup again
ferrumyx-setup wizard
```

## Development

### Building

```bash
cargo build --package ferrumyx-setup
```

### Testing

```bash
cargo test --package ferrumyx-setup
```

### Code Structure

```
ferrumyx-setup/
├── src/
│   ├── main.rs          # CLI entry point and argument parsing
│   ├── wizard.rs        # Interactive setup wizard logic
│   ├── config.rs        # Configuration file generation
│   ├── security.rs      # Secure credential handling
│   ├── validate.rs      # Input validation and connectivity tests
│   └── backup.rs        # Backup and restore functionality
├── Cargo.toml           # Package configuration
└── README.md           # This file
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run `cargo test`
6. Submit a pull request

## License

Licensed under Apache License 2.0 OR MIT.

## Support

- **Issues**: [GitHub Issues](https://github.com/Classacre/ferrumyx/issues)
- **Discussions**: [GitHub Discussions](https://github.com/Classacre/ferrumyx/discussions)
- **Documentation**: [Ferrumyx Docs](https://ferrumyx.ai/docs)</content>
<parameter name="filePath">D:\AI\Ferrumyx\ferrumyx-setup\README.md