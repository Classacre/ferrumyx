//! Health checks for system components

use deadpool_postgres::Pool;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Health checker for system components
pub struct HealthChecker {
    client: Client,
    checks: Arc<RwLock<HashMap<String, HealthCheck>>>,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            client,
            checks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a health check
    pub async fn register_check(&self, name: String, check: HealthCheck) {
        self.checks.write().await.insert(name, check);
    }

    /// Run all registered health checks
    pub async fn run_all_checks(&self) -> HashMap<String, HealthStatus> {
        let checks = self.checks.read().await;
        let mut results = HashMap::new();

        for (name, check) in checks.iter() {
            let status = match &check.check_type {
                HealthCheckType::Database { pool } => self.check_database(pool).await,
                HealthCheckType::Http { url, expected_status } => {
                    self.check_http(url, *expected_status).await
                }
                HealthCheckType::Custom { checker } => checker().await,
            };

            results.insert(name.clone(), status);
        }

        results
    }

    /// Check database connectivity
    async fn check_database(&self, pool: &Pool) -> HealthStatus {
        let start = Instant::now();

        match pool.get().await {
            Ok(client) => {
                match client.simple_query("SELECT 1").await {
                    Ok(_) => HealthStatus {
                        healthy: true,
                        response_time: start.elapsed(),
                        message: "Database connection successful".to_string(),
                        details: None,
                    },
                    Err(e) => HealthStatus {
                        healthy: false,
                        response_time: start.elapsed(),
                        message: format!("Database query failed: {}", e),
                        details: Some(serde_json::json!({
                            "error": e.to_string()
                        })),
                    },
                }
            }
            Err(e) => HealthStatus {
                healthy: false,
                response_time: start.elapsed(),
                message: format!("Database connection failed: {}", e),
                details: Some(serde_json::json!({
                    "error": e.to_string()
                })),
            },
        }
    }

    /// Check HTTP endpoint health
    async fn check_http(&self, url: &str, expected_status: u16) -> HealthStatus {
        let start = Instant::now();

        match self.client.get(url).send().await {
            Ok(response) => {
                let status = response.status().as_u16();
                let healthy = status == expected_status;

                HealthStatus {
                    healthy,
                    response_time: start.elapsed(),
                    message: if healthy {
                        format!("HTTP check successful, status: {}", status)
                    } else {
                        format!("HTTP check failed, expected: {}, got: {}", expected_status, status)
                    },
                    details: Some(serde_json::json!({
                        "status_code": status,
                        "expected_status": expected_status
                    })),
                }
            }
            Err(e) => HealthStatus {
                healthy: false,
                response_time: start.elapsed(),
                message: format!("HTTP request failed: {}", e),
                details: Some(serde_json::json!({
                    "error": e.to_string()
                })),
            },
        }
    }

    /// Get health status for a specific component
    pub async fn get_component_health(&self, name: &str) -> Option<HealthStatus> {
        let checks = self.checks.read().await;
        if let Some(check) = checks.get(name) {
            Some(match &check.check_type {
                HealthCheckType::Database { pool } => self.check_database(pool).await,
                HealthCheckType::Http { url, expected_status } => {
                    self.check_http(url, *expected_status).await
                }
                HealthCheckType::Custom { checker } => checker().await,
            })
        } else {
            None
        }
    }
}

/// Individual health check configuration
pub struct HealthCheck {
    pub check_type: HealthCheckType,
    pub interval: Duration,
    pub timeout: Duration,
}

/// Types of health checks
pub enum HealthCheckType {
    /// Database connectivity check
    Database { pool: Pool },
    /// HTTP endpoint check
    Http { url: String, expected_status: u16 },
    /// Custom health check function
    Custom { checker: Box<dyn Fn() -> futures::future::BoxFuture<'static, HealthStatus> + Send + Sync> },
}

/// Health status result
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub healthy: bool,
    pub response_time: Duration,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl HealthStatus {
    /// Create a healthy status
    pub fn healthy(message: impl Into<String>) -> Self {
        Self {
            healthy: true,
            response_time: Duration::default(),
            message: message.into(),
            details: None,
        }
    }

    /// Create an unhealthy status
    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            healthy: false,
            response_time: Duration::default(),
            message: message.into(),
            details: None,
        }
    }
}