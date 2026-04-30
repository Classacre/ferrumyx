//! Security configuration management
//!
//! Comprehensive security settings including JWT, encryption,
//! audit logging, authentication, and authorization.

use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// JWT token configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// JWT secret key
    pub secret_key: SecretString,

    /// Token issuer
    #[serde(default = "default_issuer")]
    pub issuer: String,

    /// Token audience
    #[serde(default)]
    pub audience: Option<String>,

    /// Access token expiration
    #[serde(default = "default_access_token_expiry")]
    #[serde(with = "humantime_serde")]
    pub access_token_expiry: Duration,

    /// Refresh token expiration
    #[serde(default = "default_refresh_token_expiry")]
    #[serde(with = "humantime_serde")]
    pub refresh_token_expiry: Duration,

    /// Algorithm (HS256, RS256, etc.)
    #[serde(default = "default_algorithm")]
    pub algorithm: String,
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Master encryption key
    pub master_key: SecretString,

    /// Key rotation settings
    #[serde(default)]
    pub key_rotation: KeyRotationConfig,

    /// Algorithm (AES-256-GCM, ChaCha20-Poly1305, etc.)
    #[serde(default = "default_encryption_algorithm")]
    pub algorithm: String,

    /// Key derivation function
    #[serde(default = "default_kdf")]
    pub kdf: String,

    /// Encryption context for AEAD
    #[serde(default)]
    pub context: Option<String>,
}

/// Key rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotationConfig {
    /// Enable automatic key rotation
    #[serde(default)]
    pub enabled: bool,

    /// Rotation interval
    #[serde(default = "default_rotation_interval")]
    #[serde(with = "humantime_serde")]
    pub interval: Duration,

    /// Number of old keys to keep
    #[serde(default = "default_key_retention")]
    pub key_retention_count: usize,

    /// Grace period for old keys
    #[serde(default = "default_grace_period")]
    #[serde(with = "humantime_serde")]
    pub grace_period: Duration,
}

/// Audit logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Enable audit logging
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Audit log level
    #[serde(default = "default_audit_level")]
    pub level: String,

    /// Log destination (file, database, syslog, etc.)
    #[serde(default = "default_audit_destination")]
    pub destination: String,

    /// Audit log path (for file destination)
    #[serde(default = "default_audit_path")]
    pub log_path: String,

    /// Events to audit
    #[serde(default = "default_audit_events")]
    pub events: Vec<String>,

    /// Retention period for audit logs
    #[serde(default = "default_audit_retention")]
    #[serde(with = "humantime_serde")]
    pub retention_period: Duration,

    /// Compress old audit logs
    #[serde(default = "default_true")]
    pub compress_logs: bool,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Authentication methods
    #[serde(default)]
    pub methods: Vec<AuthMethod>,

    /// Session configuration
    #[serde(default)]
    pub session: SessionConfig,

    /// Multi-factor authentication
    #[serde(default)]
    pub mfa: MfaConfig,

    /// Password policy
    #[serde(default)]
    pub password_policy: PasswordPolicy,

    /// OAuth providers
    #[serde(default)]
    pub oauth_providers: HashMap<String, OauthProviderConfig>,
}

/// Authentication methods
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthMethod {
    /// Password-based authentication
    Password,

    /// JWT token authentication
    Jwt,

    /// OAuth 2.0 / OpenID Connect
    Oauth,

    /// API key authentication
    ApiKey,

    /// Certificate-based authentication
    Certificate,
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Session timeout
    #[serde(default = "default_session_timeout")]
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,

    /// Session cookie settings
    #[serde(default)]
    pub cookie: CookieConfig,

    /// Maximum concurrent sessions per user
    #[serde(default = "default_max_sessions")]
    pub max_concurrent_sessions: usize,

    /// Session store backend
    #[serde(default = "default_session_store")]
    pub store: String,
}

/// Cookie configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieConfig {
    /// Cookie name
    #[serde(default = "default_cookie_name")]
    pub name: String,

    /// Secure flag
    #[serde(default = "default_secure")]
    pub secure: bool,

    /// HttpOnly flag
    #[serde(default = "default_http_only")]
    pub http_only: bool,

    /// SameSite policy
    #[serde(default = "default_same_site")]
    pub same_site: String,

    /// Cookie domain
    pub domain: Option<String>,

    /// Cookie path
    #[serde(default = "default_cookie_path")]
    pub path: String,
}

/// Multi-factor authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaConfig {
    /// Enable MFA
    #[serde(default)]
    pub enabled: bool,

    /// Required MFA methods
    #[serde(default)]
    pub required_methods: Vec<MfaMethod>,

    /// Grace period for MFA setup
    #[serde(default = "default_mfa_grace_period")]
    #[serde(with = "humantime_serde")]
    pub grace_period: Duration,

    /// Backup codes count
    #[serde(default = "default_backup_codes")]
    pub backup_codes_count: usize,
}

/// MFA methods
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MfaMethod {
    /// TOTP (Time-based One-Time Password)
    Totp,

    /// SMS verification
    Sms,

    /// Email verification
    Email,

    /// Hardware security keys (FIDO2/WebAuthn)
    HardwareKey,

    /// Backup codes
    BackupCodes,
}

/// Password policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordPolicy {
    /// Minimum password length
    #[serde(default = "default_min_password_length")]
    pub min_length: usize,

    /// Require uppercase letters
    #[serde(default = "default_true")]
    pub require_uppercase: bool,

    /// Require lowercase letters
    #[serde(default = "default_true")]
    pub require_lowercase: bool,

    /// Require numbers
    #[serde(default = "default_true")]
    pub require_numbers: bool,

    /// Require special characters
    #[serde(default)]
    pub require_special_chars: bool,

    /// Maximum password age
    #[serde(default = "default_max_password_age")]
    #[serde(with = "humantime_serde")]
    pub max_age: Duration,

    /// Prevent password reuse
    #[serde(default = "default_prevent_reuse")]
    pub prevent_reuse_count: usize,

    /// Password history retention
    #[serde(default = "default_password_history")]
    pub history_retention_count: usize,
}

/// OAuth provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OauthProviderConfig {
    /// Client ID
    pub client_id: SecretString,

    /// Client secret
    pub client_secret: SecretString,

    /// Authorization URL
    pub auth_url: String,

    /// Token URL
    pub token_url: String,

    /// User info URL
    pub user_info_url: String,

    /// Scopes to request
    #[serde(default)]
    pub scopes: Vec<String>,

    /// Redirect URI
    pub redirect_uri: String,
}

/// Authorization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationConfig {
    /// Enable role-based access control
    #[serde(default = "default_true")]
    pub rbac_enabled: bool,

    /// Role definitions
    #[serde(default)]
    pub roles: HashMap<String, RoleConfig>,

    /// Permission definitions
    #[serde(default)]
    pub permissions: HashMap<String, PermissionConfig>,

    /// Default role for new users
    #[serde(default = "default_default_role")]
    pub default_role: String,
}

/// Role configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleConfig {
    /// Role name
    pub name: String,

    /// Role description
    pub description: String,

    /// Inherited roles
    #[serde(default)]
    pub inherits: Vec<String>,

    /// Assigned permissions
    #[serde(default)]
    pub permissions: Vec<String>,
}

/// Permission configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionConfig {
    /// Permission name
    pub name: String,

    /// Permission description
    pub description: String,

    /// Resource type
    pub resource: String,

    /// Action
    pub action: String,

    /// Conditions
    #[serde(default)]
    pub conditions: HashMap<String, serde_json::Value>,
}

/// Security configuration unifying all security settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// JWT configuration
    pub jwt: JwtConfig,

    /// Encryption settings
    pub encryption: EncryptionConfig,

    /// Audit logging
    #[serde(default)]
    pub audit: AuditConfig,

    /// Authentication settings
    #[serde(default)]
    pub auth: AuthConfig,

    /// Authorization settings
    #[serde(default)]
    pub authorization: AuthorizationConfig,

    /// Security headers
    #[serde(default)]
    pub headers: SecurityHeadersConfig,

    /// Rate limiting
    #[serde(default)]
    pub rate_limiting: RateLimitingConfig,

    /// CORS configuration
    #[serde(default)]
    pub cors: CorsConfig,
}

/// Security headers configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityHeadersConfig {
    /// Content Security Policy
    pub csp: Option<String>,

    /// HTTP Strict Transport Security
    #[serde(default)]
    pub hsts: HstsConfig,

    /// X-Frame-Options
    #[serde(default = "default_x_frame_options")]
    pub x_frame_options: String,

    /// X-Content-Type-Options
    #[serde(default = "default_x_content_type_options")]
    pub x_content_type_options: String,

    /// Referrer Policy
    #[serde(default = "default_referrer_policy")]
    pub referrer_policy: String,
}

/// HSTS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HstsConfig {
    /// Enable HSTS
    #[serde(default)]
    pub enabled: bool,

    /// Max age
    #[serde(default = "default_hsts_max_age")]
    #[serde(with = "humantime_serde")]
    pub max_age: Duration,

    /// Include subdomains
    #[serde(default)]
    pub include_subdomains: bool,

    /// Preload
    #[serde(default)]
    pub preload: bool,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingConfig {
    /// Enable rate limiting
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Requests per minute
    #[serde(default = "default_rpm")]
    pub requests_per_minute: u32,

    /// Requests per hour
    #[serde(default = "default_rph")]
    pub requests_per_hour: u32,

    /// Burst limit
    #[serde(default = "default_burst")]
    pub burst_limit: u32,

    /// Whitelisted IPs
    #[serde(default)]
    pub whitelist: Vec<String>,

    /// Blacklisted IPs
    #[serde(default)]
    pub blacklist: Vec<String>,
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Enable CORS
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Allowed origins
    #[serde(default = "default_cors_origins")]
    pub allowed_origins: Vec<String>,

    /// Allowed methods
    #[serde(default = "default_cors_methods")]
    pub allowed_methods: Vec<String>,

    /// Allowed headers
    #[serde(default = "default_cors_headers")]
    pub allowed_headers: Vec<String>,

    /// Allow credentials
    #[serde(default)]
    pub allow_credentials: bool,

    /// Max age for preflight requests
    #[serde(default = "default_cors_max_age")]
    #[serde(with = "humantime_serde")]
    pub max_age: Duration,
}

// Import required types
use std::collections::HashMap;

// Default value functions
fn default_issuer() -> String {
    "ferrumyx".to_string()
}

fn default_access_token_expiry() -> Duration {
    Duration::from_secs(15 * 60) // 15 minutes
}

fn default_refresh_token_expiry() -> Duration {
    Duration::from_secs(7 * 24 * 60 * 60) // 7 days
}

fn default_algorithm() -> String {
    "HS256".to_string()
}

fn default_encryption_algorithm() -> String {
    "AES-256-GCM".to_string()
}

fn default_kdf() -> String {
    "PBKDF2".to_string()
}

fn default_rotation_interval() -> Duration {
    Duration::from_secs(30 * 24 * 60 * 60) // 30 days
}

fn default_key_retention() -> usize {
    5
}

fn default_grace_period() -> Duration {
    Duration::from_secs(7 * 24 * 60 * 60) // 7 days
}

fn default_true() -> bool {
    true
}

fn default_audit_level() -> String {
    "info".to_string()
}

fn default_audit_destination() -> String {
    "file".to_string()
}

fn default_audit_path() -> String {
    "./logs/audit.log".to_string()
}

fn default_audit_events() -> Vec<String> {
    vec![
        "authentication".to_string(),
        "authorization".to_string(),
        "data_access".to_string(),
        "configuration_change".to_string(),
    ]
}

fn default_audit_retention() -> Duration {
    Duration::from_secs(365 * 24 * 60 * 60) // 1 year
}

fn default_session_timeout() -> Duration {
    Duration::from_secs(24 * 60 * 60) // 24 hours
}

fn default_max_sessions() -> usize {
    5
}

fn default_session_store() -> String {
    "redis".to_string()
}

fn default_cookie_name() -> String {
    "ferrumyx_session".to_string()
}

fn default_secure() -> bool {
    true
}

fn default_http_only() -> bool {
    true
}

fn default_same_site() -> String {
    "strict".to_string()
}

fn default_cookie_path() -> String {
    "/".to_string()
}

fn default_mfa_grace_period() -> Duration {
    Duration::from_secs(7 * 24 * 60 * 60) // 7 days
}

fn default_backup_codes() -> usize {
    10
}

fn default_min_password_length() -> usize {
    12
}

fn default_max_password_age() -> Duration {
    Duration::from_secs(90 * 24 * 60 * 60) // 90 days
}

fn default_prevent_reuse() -> usize {
    5
}

fn default_password_history() -> usize {
    10
}

fn default_default_role() -> String {
    "user".to_string()
}

fn default_x_frame_options() -> String {
    "DENY".to_string()
}

fn default_x_content_type_options() -> String {
    "nosniff".to_string()
}

fn default_referrer_policy() -> String {
    "strict-origin-when-cross-origin".to_string()
}

fn default_hsts_max_age() -> Duration {
    Duration::from_secs(365 * 24 * 60 * 60) // 1 year
}

fn default_rpm() -> u32 {
    60
}

fn default_rph() -> u32 {
    1000
}

fn default_burst() -> u32 {
    10
}

fn default_cors_origins() -> Vec<String> {
    vec!["http://localhost:3000".to_string(), "http://localhost:8080".to_string()]
}

fn default_cors_methods() -> Vec<String> {
    vec!["GET".to_string(), "POST".to_string(), "PUT".to_string(), "DELETE".to_string(), "OPTIONS".to_string()]
}

fn default_cors_headers() -> Vec<String> {
    vec![
        "Authorization".to_string(),
        "Content-Type".to_string(),
        "X-Requested-With".to_string(),
    ]
}

fn default_cors_max_age() -> Duration {
    Duration::from_secs(24 * 60 * 60) // 24 hours
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            jwt: JwtConfig {
                secret_key: SecretString::from("change-me-in-production".to_string()),
                issuer: default_issuer(),
                audience: None,
                access_token_expiry: default_access_token_expiry(),
                refresh_token_expiry: default_refresh_token_expiry(),
                algorithm: default_algorithm(),
            },
            encryption: EncryptionConfig {
                master_key: SecretString::from("change-me-in-production".to_string()),
                key_rotation: KeyRotationConfig::default(),
                algorithm: default_encryption_algorithm(),
                kdf: default_kdf(),
                context: None,
            },
            audit: AuditConfig::default(),
            auth: AuthConfig::default(),
            authorization: AuthorizationConfig::default(),
            headers: SecurityHeadersConfig::default(),
            rate_limiting: RateLimitingConfig::default(),
            cors: CorsConfig::default(),
        }
    }
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: default_audit_level(),
            destination: default_audit_destination(),
            log_path: default_audit_path(),
            events: default_audit_events(),
            retention_period: default_audit_retention(),
            compress_logs: true,
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            methods: vec![AuthMethod::Password, AuthMethod::Jwt],
            session: SessionConfig::default(),
            mfa: MfaConfig::default(),
            password_policy: PasswordPolicy::default(),
            oauth_providers: HashMap::new(),
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            timeout: default_session_timeout(),
            cookie: CookieConfig::default(),
            max_concurrent_sessions: default_max_sessions(),
            store: default_session_store(),
        }
    }
}

impl Default for CookieConfig {
    fn default() -> Self {
        Self {
            name: default_cookie_name(),
            secure: true,
            http_only: true,
            same_site: default_same_site(),
            domain: None,
            path: default_cookie_path(),
        }
    }
}

impl Default for MfaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            required_methods: vec![],
            grace_period: default_mfa_grace_period(),
            backup_codes_count: default_backup_codes(),
        }
    }
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: default_min_password_length(),
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_special_chars: false,
            max_age: default_max_password_age(),
            prevent_reuse_count: default_prevent_reuse(),
            history_retention_count: default_password_history(),
        }
    }
}

impl Default for AuthorizationConfig {
    fn default() -> Self {
        Self {
            rbac_enabled: true,
            roles: HashMap::new(),
            permissions: HashMap::new(),
            default_role: default_default_role(),
        }
    }
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            csp: None,
            hsts: HstsConfig::default(),
            x_frame_options: default_x_frame_options(),
            x_content_type_options: default_x_content_type_options(),
            referrer_policy: default_referrer_policy(),
        }
    }
}

impl Default for HstsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_age: default_hsts_max_age(),
            include_subdomains: false,
            preload: false,
        }
    }
}

impl Default for RateLimitingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_minute: default_rpm(),
            requests_per_hour: default_rph(),
            burst_limit: default_burst(),
            whitelist: vec![],
            blacklist: vec![],
        }
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_origins: default_cors_origins(),
            allowed_methods: default_cors_methods(),
            allowed_headers: default_cors_headers(),
            allow_credentials: false,
            max_age: default_cors_max_age(),
        }
    }
}

impl Default for KeyRotationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval: default_rotation_interval(),
            key_retention_count: default_key_retention(),
            grace_period: default_grace_period(),
        }
    }
}