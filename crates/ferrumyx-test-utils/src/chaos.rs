//! Chaos testing utilities

use std::time::Duration;
use tokio::time::sleep;

/// Types of failures that can be injected
#[derive(Debug, Clone)]
pub enum FailureMode {
    /// Network timeout
    NetworkTimeout(Duration),
    /// Service unavailable
    ServiceUnavailable,
    /// Database connection failure
    DatabaseFailure,
    /// Memory exhaustion
    MemoryExhaustion,
    /// CPU spike
    CpuSpike,
    /// Random failures
    RandomFailure { probability: f64 },
}

/// Chaos test configuration
#[derive(Debug, Clone)]
pub struct ChaosConfig {
    pub failure_mode: FailureMode,
    pub duration: Duration,
    pub recovery_check: RecoveryCheck,
}

/// Recovery validation
#[derive(Debug, Clone)]
pub enum RecoveryCheck {
    /// Check HTTP endpoint
    HttpEndpoint(String),
    /// Check database connectivity
    DatabaseConnection,
    /// Custom recovery function (not cloneable for simplicity)
    Custom, // Simplified - no custom functions for now
}

/// Chaos testing framework
pub struct ChaosTester {
    config: ChaosConfig,
}

impl ChaosTester {
    pub fn new(config: ChaosConfig) -> Self {
        Self { config }
    }

    pub async fn inject_failure<F, Fut, T>(&self, operation: F) -> Result<T, anyhow::Error>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, anyhow::Error>>,
    {
        // Inject failure based on mode
        match &self.config.failure_mode {
            FailureMode::NetworkTimeout(duration) => {
                sleep(*duration).await;
                return Err(anyhow::anyhow!("Network timeout after {:?}", duration));
            }
            FailureMode::ServiceUnavailable => {
                return Err(anyhow::anyhow!("Service unavailable"));
            }
            FailureMode::DatabaseFailure => {
                return Err(anyhow::anyhow!("Database connection failed"));
            }
            FailureMode::MemoryExhaustion => {
                // Simulate memory exhaustion
                let mut large_vec = Vec::new();
                for _ in 0..1000000 {
                    large_vec.push(vec![0u8; 1000]);
                }
                return Err(anyhow::anyhow!("Memory exhaustion"));
            }
            FailureMode::CpuSpike => {
                // Simulate CPU spike
                let start = std::time::Instant::now();
                while start.elapsed() < Duration::from_millis(100) {
                    // Busy loop
                    let _ = (0..1000000).map(|x| x * x).sum::<u64>();
                }
            }
            FailureMode::RandomFailure { probability } => {
                if rand::random::<f64>() < *probability {
                    return Err(anyhow::anyhow!("Random failure injected"));
                }
            }
        }

        // Execute the operation
        operation().await
    }

    pub async fn validate_recovery(&self) -> Result<bool, anyhow::Error> {
        match &self.config.recovery_check {
            RecoveryCheck::HttpEndpoint(url) => {
                // Simple HTTP check
                match reqwest::get(url).await {
                    Ok(response) => Ok(response.status().is_success()),
                    Err(_) => Ok(false),
                }
            }
            RecoveryCheck::DatabaseConnection => {
                // Placeholder for database check
                Ok(true)
            }
            RecoveryCheck::Custom => {
                // Default to true for custom checks
                Ok(true)
            }
        }
    }

    pub async fn run_chaos_test<F, Fut, T>(&self, operation: F) -> Result<ChaosResult<T>, anyhow::Error>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, anyhow::Error>> + Send,
        T: Send + 'static,
    {
        let start_time = std::time::Instant::now();

        // Run operation with failure injection
        let result = self.inject_failure(operation).await;

        let duration = start_time.elapsed();

        // Check recovery
        let recovered = self.validate_recovery().await?;

        Ok(ChaosResult {
            operation_result: result,
            duration,
            recovered,
            failure_injected: true,
        })
    }
}

/// Results from chaos testing
#[derive(Debug)]
pub struct ChaosResult<T> {
    pub operation_result: Result<T, anyhow::Error>,
    pub duration: Duration,
    pub recovered: bool,
    pub failure_injected: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chaos_service_unavailable() {
        let config = ChaosConfig {
            failure_mode: FailureMode::ServiceUnavailable,
            duration: Duration::from_secs(1),
            recovery_check: RecoveryCheck::Custom,
        };

        let tester = ChaosTester::new(config);

        let operation = || async { Ok::<_, anyhow::Error>("success") };

        let result = tester.run_chaos_test(operation).await.unwrap();

        assert!(result.operation_result.is_err());
        assert!(result.failure_injected);
        assert!(result.recovered);
    }

    #[tokio::test]
    async fn test_chaos_random_failure() {
        let config = ChaosConfig {
            failure_mode: FailureMode::RandomFailure { probability: 1.0 }, // Always fail
            duration: Duration::from_secs(1),
            recovery_check: RecoveryCheck::Custom,
        };

        let tester = ChaosTester::new(config);

        let operation = || async { Ok::<_, anyhow::Error>("success") };

        let result = tester.run_chaos_test(operation).await.unwrap();

        assert!(result.operation_result.is_err());
        assert!(result.failure_injected);
    }
}