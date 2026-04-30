# Multi-Channel Setup and Configuration

This directory contains WASM-based multi-channel implementations for Ferrumyx, enabling conversational oncology workflows across various communication platforms while maintaining data security and compliance.

## Purpose and Overview

Ferrumyx's multi-channel architecture allows users to interact with bioinformatics workflows through natural language interfaces across different platforms (WhatsApp, Slack, Discord, Telegram, Web). Each channel implementation includes:

- **Platform-specific formatting**: Optimized message formatting for each platform's capabilities
- **Data sensitivity filtering**: Automatic filtering of sensitive biomedical data based on channel trust levels
- **Oncology-specific workflows**: Specialized handling for biomedical queries and results
- **Security boundaries**: WASM sandboxing for secure execution
- **Audit logging**: Comprehensive tracking of all interactions

The channel implementations are built as WASM modules that integrate with the IronClaw runtime, providing consistent behavior across platforms while respecting each platform's constraints.

## Supported Channels

- **WhatsApp**: Basic text formatting, file sharing
- **Slack**: Markdown support, rich formatting
- **Discord**: Markdown and embed support
- **Telegram**: HTML/Markdown formatting
- **Web**: Full HTML/Markdown with interactive elements

## Installation/Setup Instructions

### Prerequisites

1. **IronClaw Runtime**: Core agent framework
   ```bash
   cargo build --release -p ferrumyx-runtime
   ```

2. **WASM Target**: Rust WASM compilation support
   ```bash
   rustup target add wasm32-wasi
   ```

3. **Platform APIs**: Configure API credentials for each platform

### Channel Compilation

```bash
# Build all channel implementations
cargo build --release --target wasm32-wasi

# Build specific channel
cargo build --release -p ferrumyx-channels-whatsapp --target wasm32-wasi
```

### Configuration

Create channel configuration file:

```toml
[channels]
enabled = ["web", "whatsapp", "slack"]

[channels.whatsapp]
api_token = "your_whatsapp_token"
webhook_url = "https://your-domain.com/webhook/whatsapp"

[channels.slack]
bot_token = "xoxb-your-slack-token"
signing_secret = "your-signing-secret"

[channels.security]
default_trust_level = "internal"
audit_log_path = "/var/log/ferrumyx/channels/"
```

## Usage Examples

### Basic Channel Setup

```rust
use ferrumyx_channels::*;

let mut channel_manager = ChannelManager::new();

// Register channels
channel_manager.register_channel(Box::new(WebChannel::new()));
channel_manager.register_channel(Box::new(WhatsAppChannel::new(config.whatsapp)));

// Start listening
channel_manager.start_all().await?;
```

### Oncology-Specific Formatting

```rust
// Automatic biomedical data formatting
let response = OncologyResponse {
    content: "Gene TP53 shows mutation in 45% of samples",
    data_sensitivity: SensitivityLevel::Internal,
};

let formatted = channel.format_biomedical_response(&response, "slack");
// Output: "**Gene TP53** shows mutation in *45%* of samples"
```

### Data Sensitivity Filtering

```rust
// Automatic filtering based on channel trust
let sensitive_response = Response::new(
    "Patient data: Stage III breast cancer with BRCA1 mutation",
    SensitivityLevel::Confidential
);

// Will be blocked on public channels (WhatsApp, Discord)
// Allowed on internal channels (Web, Slack)
let allowed_channels = router.route_message(&message);
```

## Configuration Options

### Channel-Specific Settings

```toml
[channels.whatsapp]
# WhatsApp Business API settings
api_version = "v17.0"
phone_number_id = "123456789"
access_token = "your_access_token"

# Message formatting
max_message_length = 4096
support_media = true

[channels.slack]
# Slack app settings
app_token = "xapp-your-app-token"
bot_token = "xoxb-your-bot-token"

# Rich formatting
support_blocks = true
support_buttons = true

[channels.discord]
# Discord bot settings
token = "your_bot_token"
application_id = "your_app_id"

# Channel permissions
required_permissions = ["send_messages", "embed_links"]
```

### Security Configuration

```toml
[channels.security]
# Trust levels per channel
trust_levels = { web = "internal", whatsapp = "public" }

# Data classification rules
sensitive_keywords = ["patient", "clinical", "phi", "hipaa"]

# Audit settings
audit_enabled = true
audit_retention_days = 365
encrypt_audit_logs = true
```

### Performance Tuning

```toml
[channels.performance]
# Connection pooling
max_connections = 100
connection_timeout_secs = 30

# Rate limiting
requests_per_minute = 60
burst_limit = 10

# Caching
response_cache_ttl_secs = 300
user_session_timeout_secs = 3600
```

## Troubleshooting Guide

### Common Issues

**Channel Connection Failed**
```
Error: Failed to connect to WhatsApp API
```
**Solution:** Verify API credentials and network connectivity
```bash
curl -H "Authorization: Bearer $WHATSAPP_TOKEN" \
     https://graph.facebook.com/v17.0/me
```

**Message Formatting Errors**
```
Error: Unsupported formatting for channel
```
**Solution:** Check channel capabilities and adjust formatting logic

**Data Sensitivity Blocks**
```
Warning: Response blocked due to sensitivity level
```
**Solution:** Review channel trust configuration or use appropriate channel

**WASM Compilation Issues**
```
Error: WASM target not found
```
**Solution:** Install WASM target
```bash
rustup target add wasm32-wasi
```

### Debugging

Enable channel debugging:

```bash
export FERRUMYX_CHANNEL_DEBUG=true
export FERRUMYX_LOG_LEVEL=debug
```

Monitor channel health:

```bash
curl http://localhost:3000/api/channels/health
```

Check audit logs:

```bash
tail -f /var/log/ferrumyx/channels/audit.log
```

### Platform-Specific Issues

**WhatsApp:**
- Ensure Business API approval
- Check webhook certificate validity
- Verify phone number registration

**Slack:**
- Confirm app permissions and scopes
- Check bot token validity
- Review rate limiting

**Discord:**
- Verify bot intents and permissions
- Check gateway connection
- Monitor for API changes

## Links to Related Documentation

- [Main README](../../README.md) - Project overview
- [IronClaw Channels](../../crates/ferrumyx-runtime/src/channels/) - Core channel framework
- [Oncology Channels](../../crates/ferrumyx-agent/src/channels.rs) - Implementation details
- [Security Guide](../../docs/SECURITY.md) - Security configuration
- [API Documentation](../../crates/ferrumyx-web/src/api/) - REST API reference