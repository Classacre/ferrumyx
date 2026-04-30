//! Multi-channel integration configuration
//!
//! Configuration for WhatsApp, Slack, Discord, and other communication channels.

use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Supported communication channels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelType {
    /// WhatsApp Business API
    WhatsApp,

    /// Slack API
    Slack,

    /// Discord API
    Discord,

    /// Telegram Bot API
    Telegram,

    /// Microsoft Teams
    Teams,

    /// Custom webhook-based channel
    Webhook,

    /// Email integration
    Email,

    /// SMS integration
    Sms,
}

/// Base channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseChannelConfig {
    /// Enable this channel
    #[serde(default)]
    pub enabled: bool,

    /// Channel type
    pub channel_type: ChannelType,

    /// Channel name/identifier
    pub name: String,

    /// Channel description
    pub description: String,

    /// Rate limiting
    #[serde(default)]
    pub rate_limit: ChannelRateLimit,

    /// Retry configuration
    #[serde(default)]
    pub retry: ChannelRetryConfig,

    /// Message templates
    #[serde(default)]
    pub templates: HashMap<String, String>,
}

/// WhatsApp-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    /// WhatsApp Business API token
    pub api_token: SecretString,

    /// WhatsApp Business Account ID
    pub account_id: String,

    /// Phone number ID
    pub phone_number_id: String,

    /// Webhook verification token
    pub verify_token: SecretString,

    /// API base URL
    #[serde(default = "default_whatsapp_base_url")]
    pub base_url: String,

    /// Media upload settings
    #[serde(default)]
    pub media: MediaConfig,
}

/// Slack-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    /// Slack bot token
    pub bot_token: SecretString,

    /// Slack signing secret
    pub signing_secret: SecretString,

    /// Bot user ID
    pub bot_user_id: String,

    /// Default channel for notifications
    pub default_channel: String,

    /// Allowed channels
    #[serde(default)]
    pub allowed_channels: Vec<String>,

    /// Event subscriptions
    #[serde(default)]
    pub events: SlackEventsConfig,
}

/// Slack events configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackEventsConfig {
    /// App mention events
    #[serde(default = "default_true")]
    pub app_mentions: bool,

    /// Direct messages
    #[serde(default = "default_true")]
    pub direct_messages: bool,

    /// Channel messages
    #[serde(default)]
    pub channel_messages: bool,

    /// Reaction events
    #[serde(default)]
    pub reactions: bool,

    /// File share events
    #[serde(default)]
    pub file_shares: bool,
}

/// Discord-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    /// Discord bot token
    pub bot_token: SecretString,

    /// Application ID
    pub application_id: String,

    /// Bot permissions integer
    #[serde(default = "default_discord_permissions")]
    pub permissions: u64,

    /// Guild ID (server ID)
    pub guild_id: Option<String>,

    /// Command prefix
    #[serde(default = "default_command_prefix")]
    pub command_prefix: String,

    /// Intents
    #[serde(default)]
    pub intents: DiscordIntents,
}

/// Discord intents configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordIntents {
    /// Guilds intent
    #[serde(default = "default_true")]
    pub guilds: bool,

    /// Guild members intent
    #[serde(default)]
    pub guild_members: bool,

    /// Guild messages intent
    #[serde(default = "default_true")]
    pub guild_messages: bool,

    /// Direct messages intent
    #[serde(default = "default_true")]
    pub direct_messages: bool,

    /// Message content intent
    #[serde(default)]
    pub message_content: bool,
}

/// Telegram-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    /// Telegram bot token
    pub bot_token: SecretString,

    /// Bot username
    pub bot_username: String,

    /// Webhook URL (for production)
    pub webhook_url: Option<String>,

    /// Polling timeout (for development)
    #[serde(default = "default_polling_timeout")]
    #[serde(with = "humantime_serde")]
    pub polling_timeout: Duration,

    /// Allowed chat types
    #[serde(default = "default_allowed_chat_types")]
    pub allowed_chat_types: Vec<String>,
}

/// Webhook-based channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Webhook URL
    pub url: String,

    /// HTTP method
    #[serde(default = "default_webhook_method")]
    pub method: String,

    /// Custom headers
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Authentication
    pub auth: Option<WebhookAuth>,

    /// Request timeout
    #[serde(default = "default_webhook_timeout")]
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,

    /// Retry on failure
    #[serde(default)]
    pub retry_on_failure: bool,
}

/// Webhook authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookAuth {
    /// Auth type
    pub auth_type: WebhookAuthType,

    /// Auth token/credentials
    pub credentials: SecretString,
}

/// Webhook authentication types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebhookAuthType {
    /// Bearer token
    Bearer,

    /// Basic auth
    Basic,

    /// API key in header
    ApiKey,

    /// Custom header
    Custom,
}

/// Email configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    /// SMTP server
    pub smtp_server: String,

    /// SMTP port
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,

    /// SMTP username
    pub username: String,

    /// SMTP password
    pub password: SecretString,

    /// Use TLS
    #[serde(default = "default_true")]
    pub use_tls: bool,

    /// From address
    pub from_address: String,

    /// From name
    pub from_name: String,

    /// Template directory
    #[serde(default = "default_email_templates")]
    pub template_dir: String,
}

/// SMS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsConfig {
    /// SMS provider
    pub provider: SmsProvider,

    /// API key
    pub api_key: SecretString,

    /// API secret (if required)
    pub api_secret: Option<SecretString>,

    /// Sender ID/phone number
    pub sender_id: String,

    /// API base URL
    pub base_url: Option<String>,
}

/// SMS providers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SmsProvider {
    /// Twilio
    Twilio,

    /// AWS SNS
    AwsSns,

    /// MessageBird
    MessageBird,

    /// Custom provider
    Custom,
}

/// Rate limiting for channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelRateLimit {
    /// Messages per minute
    #[serde(default = "default_messages_per_minute")]
    pub messages_per_minute: u32,

    /// Messages per hour
    #[serde(default = "default_messages_per_hour")]
    pub messages_per_hour: u32,

    /// Burst limit
    #[serde(default = "default_burst_limit")]
    pub burst_limit: u32,

    /// Cooldown between messages
    #[serde(default = "default_cooldown")]
    #[serde(with = "humantime_serde")]
    pub cooldown: Duration,
}

/// Retry configuration for channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelRetryConfig {
    /// Maximum retry attempts
    #[serde(default = "default_max_retries")]
    pub max_attempts: u32,

    /// Initial backoff duration
    #[serde(default = "default_initial_backoff")]
    #[serde(with = "humantime_serde")]
    pub initial_backoff: Duration,

    /// Maximum backoff duration
    #[serde(default = "default_max_backoff")]
    #[serde(with = "humantime_serde")]
    pub max_backoff: Duration,

    /// Retryable error codes
    #[serde(default = "default_retryable_errors")]
    pub retryable_errors: Vec<String>,
}

/// Media upload configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaConfig {
    /// Maximum file size (bytes)
    #[serde(default = "default_max_file_size")]
    pub max_file_size_bytes: u64,

    /// Allowed MIME types
    #[serde(default = "default_allowed_mime_types")]
    pub allowed_mime_types: Vec<String>,

    /// Upload timeout
    #[serde(default = "default_upload_timeout")]
    #[serde(with = "humantime_serde")]
    pub upload_timeout: Duration,

    /// CDN base URL for uploaded media
    pub cdn_base_url: Option<String>,
}

/// Channels configuration unifying all channel integrations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelsConfig {
    /// WhatsApp configuration
    pub whatsapp: Option<WhatsAppConfig>,

    /// Slack configuration
    pub slack: Option<SlackConfig>,

    /// Discord configuration
    pub discord: Option<DiscordConfig>,

    /// Telegram configuration
    pub telegram: Option<TelegramConfig>,

    /// Webhook channels
    #[serde(default)]
    pub webhooks: Vec<WebhookConfig>,

    /// Email configuration
    pub email: Option<EmailConfig>,

    /// SMS configuration
    pub sms: Option<SmsConfig>,

    /// Global channel settings
    #[serde(default)]
    pub global: GlobalChannelConfig,
}

/// Global channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalChannelConfig {
    /// Enable multi-channel support
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Default channel for broadcasts
    pub default_channel: Option<String>,

    /// Channel routing rules
    #[serde(default)]
    pub routing_rules: Vec<RoutingRule>,

    /// Message deduplication
    #[serde(default)]
    pub deduplication: MessageDeduplication,

    /// Message queuing
    #[serde(default)]
    pub queuing: MessageQueuing,
}

/// Message routing rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    /// Rule name
    pub name: String,

    /// Conditions for routing
    pub conditions: HashMap<String, serde_json::Value>,

    /// Target channels
    pub channels: Vec<String>,

    /// Priority (higher = more important)
    #[serde(default)]
    pub priority: i32,
}

/// Message deduplication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDeduplication {
    /// Enable deduplication
    #[serde(default)]
    pub enabled: bool,

    /// Deduplication window
    #[serde(default = "default_deduplication_window")]
    #[serde(with = "humantime_serde")]
    pub window: Duration,

    /// Deduplication strategy
    #[serde(default = "default_deduplication_strategy")]
    pub strategy: String,
}

/// Message queuing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageQueuing {
    /// Enable queuing
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Queue size limit
    #[serde(default = "default_queue_size")]
    pub max_size: usize,

    /// Queue persistence
    #[serde(default)]
    pub persistence: bool,

    /// Dead letter queue
    #[serde(default)]
    pub dead_letter: bool,
}

// Default value functions
fn default_whatsapp_base_url() -> String {
    "https://graph.facebook.com/v18.0".to_string()
}

fn default_discord_permissions() -> u64 {
    2048 // Use slash commands
}

fn default_command_prefix() -> String {
    "!".to_string()
}

fn default_polling_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_allowed_chat_types() -> Vec<String> {
    vec!["private".to_string(), "group".to_string()]
}

fn default_webhook_method() -> String {
    "POST".to_string()
}

fn default_webhook_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_smtp_port() -> u16 {
    587
}

fn default_email_templates() -> String {
    "./templates/email".to_string()
}

fn default_messages_per_minute() -> u32 {
    60
}

fn default_messages_per_hour() -> u32 {
    1000
}

fn default_burst_limit() -> u32 {
    10
}

fn default_cooldown() -> Duration {
    Duration::from_millis(100)
}

fn default_max_retries() -> u32 {
    3
}

fn default_initial_backoff() -> Duration {
    Duration::from_millis(100)
}

fn default_max_backoff() -> Duration {
    Duration::from_secs(30)
}

fn default_retryable_errors() -> Vec<String> {
    vec![
        "timeout".to_string(),
        "connection_error".to_string(),
        "rate_limited".to_string(),
    ]
}

fn default_max_file_size() -> u64 {
    16 * 1024 * 1024 // 16MB
}

fn default_allowed_mime_types() -> Vec<String> {
    vec![
        "image/jpeg".to_string(),
        "image/png".to_string(),
        "image/gif".to_string(),
        "video/mp4".to_string(),
        "audio/mpeg".to_string(),
        "application/pdf".to_string(),
    ]
}

fn default_upload_timeout() -> Duration {
    Duration::from_secs(120)
}

fn default_true() -> bool {
    true
}

fn default_deduplication_window() -> Duration {
    Duration::from_secs(300) // 5 minutes
}

fn default_deduplication_strategy() -> String {
    "content_hash".to_string()
}

fn default_queue_size() -> usize {
    10000
}

impl Default for ChannelsConfig {
    fn default() -> Self {
        Self {
            whatsapp: None,
            slack: None,
            discord: None,
            telegram: None,
            webhooks: vec![],
            email: None,
            sms: None,
            global: GlobalChannelConfig::default(),
        }
    }
}

impl Default for GlobalChannelConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_channel: None,
            routing_rules: vec![],
            deduplication: MessageDeduplication::default(),
            queuing: MessageQueuing::default(),
        }
    }
}

impl Default for MessageDeduplication {
    fn default() -> Self {
        Self {
            enabled: false,
            window: default_deduplication_window(),
            strategy: default_deduplication_strategy(),
        }
    }
}

impl Default for MessageQueuing {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size: default_queue_size(),
            persistence: false,
            dead_letter: false,
        }
    }
}

impl Default for ChannelRateLimit {
    fn default() -> Self {
        Self {
            messages_per_minute: default_messages_per_minute(),
            messages_per_hour: default_messages_per_hour(),
            burst_limit: default_burst_limit(),
            cooldown: default_cooldown(),
        }
    }
}

impl Default for ChannelRetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: default_max_retries(),
            initial_backoff: default_initial_backoff(),
            max_backoff: default_max_backoff(),
            retryable_errors: default_retryable_errors(),
        }
    }
}

impl Default for DiscordIntents {
    fn default() -> Self {
        Self {
            guilds: true,
            guild_members: false,
            guild_messages: true,
            direct_messages: true,
            message_content: false,
        }
    }
}

impl Default for SlackEventsConfig {
    fn default() -> Self {
        Self {
            app_mentions: true,
            direct_messages: true,
            channel_messages: false,
            reactions: false,
            file_shares: false,
        }
    }
}

impl Default for MediaConfig {
    fn default() -> Self {
        Self {
            max_file_size_bytes: default_max_file_size(),
            allowed_mime_types: default_allowed_mime_types(),
            upload_timeout: default_upload_timeout(),
            cdn_base_url: None,
        }
    }
}