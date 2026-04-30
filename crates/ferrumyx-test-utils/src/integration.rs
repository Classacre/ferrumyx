//! Integration test utilities for end-to-end testing.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Test database manager for integration tests.
pub struct TestDatabaseManager {
    databases: RwLock<HashMap<String, Arc<dyn TestDatabase>>>,
}

#[async_trait::async_trait]
pub trait TestDatabase: Send + Sync {
    async fn setup(&self) -> anyhow::Result<()>;
    async fn teardown(&self) -> anyhow::Result<()>;
    async fn reset(&self) -> anyhow::Result<()>;
    fn connection_string(&self) -> String;
}

/// HTTP test client for API testing.
pub struct TestHttpClient {
    base_url: String,
    client: reqwest::Client,
}

impl TestHttpClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn get(&self, path: &str) -> anyhow::Result<reqwest::Response> {
        let url = format!("{}{}", self.base_url, path);
        Ok(self.client.get(&url).send().await?)
    }

    pub async fn post_json<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> anyhow::Result<reqwest::Response> {
        let url = format!("{}{}", self.base_url, path);
        Ok(self.client
            .post(&url)
            .json(body)
            .send()
            .await?)
    }

    pub async fn health_check(&self) -> anyhow::Result<bool> {
        match self.get("/api/health").await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

/// Performance benchmark runner.
pub struct BenchmarkRunner {
    results: RwLock<Vec<BenchmarkResult>>,
}

#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub duration_ms: u128,
    pub iterations: usize,
    pub avg_time_per_iter_ms: f64,
    pub metadata: HashMap<String, String>,
}

impl BenchmarkRunner {
    pub fn new() -> Self {
        Self {
            results: RwLock::new(Vec::new()),
        }
    }

    pub async fn run_benchmark<F, Fut>(
        &self,
        name: &str,
        iterations: usize,
        benchmark_fn: F,
    ) -> anyhow::Result<BenchmarkResult>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<()>>,
    {
        let start = std::time::Instant::now();
        let mut errors = 0;

        for _ in 0..iterations {
            if let Err(_) = benchmark_fn().await {
                errors += 1;
            }
        }

        let total_duration = start.elapsed();
        let duration_ms = total_duration.as_millis();
        let avg_time_per_iter_ms = duration_ms as f64 / iterations as f64;

        let result = BenchmarkResult {
            name: name.to_string(),
            duration_ms,
            iterations,
            avg_time_per_iter_ms,
            metadata: HashMap::from([
                ("errors".to_string(), errors.to_string()),
                ("success_rate".to_string(), format!("{:.2}%", (iterations - errors) as f64 / iterations as f64 * 100.0)),
            ]),
        };

        self.results.write().await.push(result.clone());
        Ok(result)
    }

    pub async fn get_results(&self) -> Vec<BenchmarkResult> {
        self.results.read().await.clone()
    }

    pub async fn print_report(&self) {
        let results = self.get_results().await;
        println!("=== Benchmark Report ===");
        for result in results {
            println!("{}: {:.2}ms avg ({} iterations)", result.name, result.avg_time_per_iter_ms, result.iterations);
            for (key, value) in &result.metadata {
                println!("  {}: {}", key, value);
            }
        }
    }
}

/// Load testing utilities.
pub struct LoadTester {
    client: TestHttpClient,
    concurrency: usize,
}

impl LoadTester {
    pub fn new(base_url: &str, concurrency: usize) -> Self {
        Self {
            client: TestHttpClient::new(base_url),
            concurrency,
        }
    }

    pub async fn run_load_test<F>(
        &self,
        name: &str,
        duration_secs: u64,
        request_fn: F,
    ) -> anyhow::Result<LoadTestResult>
    where
        F: Fn() -> anyhow::Result<()> + Send + Sync + 'static,
        F: Clone,
    {
        let start = std::time::Instant::now();
        let mut handles = vec![];
        let mut success_count = 0u64;
        let mut error_count = 0u64;

        // Spawn worker tasks
        for _ in 0..self.concurrency {
            let request_fn = request_fn.clone();
            let handle = tokio::spawn(async move {
                let mut local_success = 0u64;
                let mut local_error = 0u64;

                while start.elapsed().as_secs() < duration_secs {
                    match request_fn() {
                        Ok(_) => local_success += 1,
                        Err(_) => local_error += 1,
                    }
                }

                (local_success, local_error)
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            let (success, error) = handle.await?;
            success_count += success;
            error_count += error;
        }

        let total_requests = success_count + error_count;
        let success_rate = if total_requests > 0 {
            success_count as f64 / total_requests as f64
        } else {
            0.0
        };

        Ok(LoadTestResult {
            name: name.to_string(),
            duration_secs,
            concurrency: self.concurrency,
            total_requests,
            success_count,
            error_count,
            success_rate,
            requests_per_second: total_requests as f64 / duration_secs as f64,
        })
    }
}

#[derive(Debug, Clone)]
pub struct LoadTestResult {
    pub name: String,
    pub duration_secs: u64,
    pub concurrency: usize,
    pub total_requests: u64,
    pub success_count: u64,
    pub error_count: u64,
    pub success_rate: f64,
    pub requests_per_second: f64,
}

impl std::fmt::Display for LoadTestResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {:.0} req/sec, {:.1}% success ({} concurrency, {}s)",
            self.name, self.requests_per_second, self.success_rate * 100.0, self.concurrency, self.duration_secs
        )
    }
}

/// Security test utilities.
pub struct SecurityTester {
    client: TestHttpClient,
}

impl SecurityTester {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: TestHttpClient::new(base_url),
        }
    }

    /// Test for SQL injection vulnerabilities.
    pub async fn test_sql_injection(&self, endpoints: &[&str]) -> Vec<SecurityTestResult> {
        let payloads = vec![
            "' OR '1'='1",
            "'; DROP TABLE users; --",
            "' UNION SELECT * FROM users --",
            "admin' --",
        ];

        let mut results = vec![];

        for endpoint in endpoints {
            for payload in &payloads {
                let test_result = self.test_endpoint_with_payload(endpoint, payload).await;
                results.push(test_result);
            }
        }

        results
    }

    /// Test for XSS vulnerabilities.
    pub async fn test_xss(&self, endpoints: &[&str]) -> Vec<SecurityTestResult> {
        let payloads = vec![
            "<script>alert('xss')</script>",
            "<img src=x onerror=alert('xss')>",
            "javascript:alert('xss')",
        ];

        let mut results = vec![];

        for endpoint in endpoints {
            for payload in &payloads {
                let test_result = self.test_endpoint_with_payload(endpoint, payload).await;
                results.push(test_result);
            }
        }

        results
    }

    async fn test_endpoint_with_payload(&self, endpoint: &str, payload: &str) -> SecurityTestResult {
        // This is a simplified test - in real implementation, you'd test various input methods
        let test_data = serde_json::json!({
            "input": payload,
            "test": "security_scan"
        });

        let result = match self.client.post_json(endpoint, &test_data).await {
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();

                // Check if payload appears in response (potential vulnerability)
                let vulnerable = body.contains(payload);
                let severity = if vulnerable && status.is_success() {
                    SecuritySeverity::High
                } else if vulnerable {
                    SecuritySeverity::Medium
                } else {
                    SecuritySeverity::Low
                };

                SecurityTestResult {
                    endpoint: endpoint.to_string(),
                    payload: payload.to_string(),
                    status_code: status.as_u16(),
                    vulnerable,
                    severity,
                    notes: if vulnerable { "Payload reflected in response".to_string() } else { "".to_string() },
                }
            }
            Err(e) => SecurityTestResult {
                endpoint: endpoint.to_string(),
                payload: payload.to_string(),
                status_code: 0,
                vulnerable: false,
                severity: SecuritySeverity::Low,
                notes: format!("Request failed: {}", e),
            },
        };

        result
    }
}

#[derive(Debug, Clone)]
pub struct SecurityTestResult {
    pub endpoint: String,
    pub payload: String,
    pub status_code: u16,
    pub vulnerable: bool,
    pub severity: SecuritySeverity,
    pub notes: String,
}

#[derive(Debug, Clone)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_benchmark_runner() {
        let runner = BenchmarkRunner::new();

        let result = runner.run_benchmark("test_benchmark", 10, || async {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            Ok(())
        }).await.unwrap();

        assert_eq!(result.name, "test_benchmark");
        assert_eq!(result.iterations, 10);
        assert!(result.duration_ms > 0);
        assert!(result.avg_time_per_iter_ms > 0.0);

        let results = runner.get_results().await;
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_load_test_result_display() {
        let result = LoadTestResult {
            name: "test_load".to_string(),
            duration_secs: 60,
            concurrency: 10,
            total_requests: 1000,
            success_count: 950,
            error_count: 50,
            success_rate: 0.95,
            requests_per_second: 16.67,
        };

        let display = format!("{}", result);
        assert!(display.contains("test_load"));
        assert!(display.contains("16 req/sec"));
        assert!(display.contains("95.0% success"));
    }

    #[test]
    fn test_security_severity() {
        assert_eq!(std::mem::size_of::<SecuritySeverity>(), 1); // Should be small enum
    }
}