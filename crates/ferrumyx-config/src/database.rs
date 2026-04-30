//! Database configuration management
//!
//! Supports PostgreSQL, Redis, and other database backends with connection pooling,
//! SSL/TLS configuration, and performance tuning.

use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Database backend types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseBackend {
    /// PostgreSQL with pgvector support
    PostgreSql,

    /// Redis for caching and sessions
    Redis,

    /// SQLite for development/testing
    Sqlite,

    /// Turso/libSQL for edge deployment
    LibSql,
}

/// SSL/TLS modes for database connections
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SslMode {
    /// No SSL/TLS
    Disable,

    /// Prefer SSL/TLS but allow fallback
    Prefer,

    /// Require SSL/TLS
    Require,

    /// Verify CA certificate
    VerifyCa,

    /// Verify full certificate chain
    VerifyFull,
}

/// Database connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConnection {
    /// Connection URL/DSN
    pub url: SecretString,

    /// Connection pool size
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,

    /// Connection timeout
    #[serde(default = "default_connection_timeout")]
    #[serde(with = "humantime_serde")]
    pub connection_timeout: Duration,

    /// Query timeout
    #[serde(default = "default_query_timeout")]
    #[serde(with = "humantime_serde")]
    pub query_timeout: Duration,

    /// SSL/TLS configuration
    #[serde(default)]
    pub ssl_mode: SslMode,

    /// SSL certificate path (optional)
    pub ssl_cert_path: Option<String>,

    /// SSL key path (optional)
    pub ssl_key_path: Option<String>,

    /// SSL CA certificate path (optional)
    pub ssl_ca_path: Option<String>,
}

/// PostgreSQL-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgreSqlConfig {
    /// Enable pgvector extension
    #[serde(default = "default_true")]
    pub enable_pgvector: bool,

    /// Vector dimension for embeddings
    #[serde(default = "default_vector_dimension")]
    pub vector_dimension: usize,

    /// Enable connection pooling
    #[serde(default = "default_true")]
    pub enable_pooling: bool,

    /// Maximum connections in pool
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    /// Minimum idle connections
    #[serde(default = "default_min_idle")]
    pub min_idle: u32,
}

/// Redis-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis cluster mode
    #[serde(default)]
    pub cluster_mode: bool,

    /// Key prefix for namespacing
    #[serde(default = "default_redis_prefix")]
    pub key_prefix: String,

    /// Default TTL for cached items
    #[serde(default = "default_redis_ttl")]
    #[serde(with = "humantime_serde")]
    pub default_ttl: Duration,

    /// Enable Redis persistence
    #[serde(default = "default_true")]
    pub enable_persistence: bool,
}

/// Database configuration unifying all database settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Primary database backend
    #[serde(default = "default_backend")]
    pub backend: DatabaseBackend,

    /// PostgreSQL configuration
    #[serde(default)]
    pub postgresql: PostgreSqlConfig,

    /// Redis configuration
    #[serde(default)]
    pub redis: RedisConfig,

    /// Primary database connection
    pub primary: DatabaseConnection,

    /// Read replica connections (optional)
    #[serde(default)]
    pub replicas: Vec<DatabaseConnection>,

    /// Migration settings
    #[serde(default)]
    pub migration: MigrationConfig,

    /// Backup settings
    #[serde(default)]
    pub backup: BackupConfig,
}

/// Database migration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationConfig {
    /// Enable automatic migrations
    #[serde(default = "default_true")]
    pub auto_migrate: bool,

    /// Migration directory path
    #[serde(default = "default_migration_dir")]
    pub migration_dir: String,

    /// Allow destructive migrations
    #[serde(default)]
    pub allow_destructive: bool,

    /// Create backups before migrations
    #[serde(default = "default_true")]
    pub backup_before_migrate: bool,
}

/// Database backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Enable automatic backups
    #[serde(default)]
    pub enabled: bool,

    /// Backup schedule (cron expression)
    #[serde(default = "default_backup_schedule")]
    pub schedule: String,

    /// Backup retention period
    #[serde(default = "default_backup_retention")]
    #[serde(with = "humantime_serde")]
    pub retention_period: Duration,

    /// Backup storage path
    #[serde(default = "default_backup_path")]
    pub storage_path: String,

    /// Compress backups
    #[serde(default = "default_true")]
    pub compress: bool,
}

// Default value functions
fn default_backend() -> DatabaseBackend {
    DatabaseBackend::PostgreSql
}

fn default_pool_size() -> u32 {
    10
}

fn default_connection_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_query_timeout() -> Duration {
    Duration::from_secs(300)
}

fn default_true() -> bool {
    true
}

fn default_vector_dimension() -> usize {
    768
}

fn default_max_connections() -> u32 {
    20
}

fn default_min_idle() -> u32 {
    2
}

fn default_redis_prefix() -> String {
    "ferrumyx:".to_string()
}

fn default_redis_ttl() -> Duration {
    Duration::from_secs(3600) // 1 hour
}

fn default_migration_dir() -> String {
    "./migrations".to_string()
}

fn default_backup_schedule() -> String {
    "0 2 * * *".to_string() // Daily at 2 AM
}

fn default_backup_retention() -> Duration {
    Duration::from_secs(30 * 24 * 60 * 60) // 30 days
}

fn default_backup_path() -> String {
    "./backups".to_string()
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            backend: default_backend(),
            postgresql: PostgreSqlConfig::default(),
            redis: RedisConfig::default(),
            primary: DatabaseConnection::default(),
            replicas: vec![],
            migration: MigrationConfig::default(),
            backup: BackupConfig::default(),
        }
    }
}

impl Default for PostgreSqlConfig {
    fn default() -> Self {
        Self {
            enable_pgvector: true,
            vector_dimension: default_vector_dimension(),
            enable_pooling: true,
            max_connections: default_max_connections(),
            min_idle: default_min_idle(),
        }
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            cluster_mode: false,
            key_prefix: default_redis_prefix(),
            default_ttl: default_redis_ttl(),
            enable_persistence: true,
        }
    }
}

impl Default for DatabaseConnection {
    fn default() -> Self {
        Self {
            url: SecretString::from("postgresql://localhost:5432/ferrumyx".to_string()),
            pool_size: default_pool_size(),
            connection_timeout: default_connection_timeout(),
            query_timeout: default_query_timeout(),
            ssl_mode: SslMode::Disable,
            ssl_cert_path: None,
            ssl_key_path: None,
            ssl_ca_path: None,
        }
    }
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            auto_migrate: true,
            migration_dir: default_migration_dir(),
            allow_destructive: false,
            backup_before_migrate: true,
        }
    }
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            schedule: default_backup_schedule(),
            retention_period: default_backup_retention(),
            storage_path: default_backup_path(),
            compress: true,
        }
    }
}