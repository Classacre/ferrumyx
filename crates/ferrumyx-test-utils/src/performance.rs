//! Performance testing utilities

use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Performance test configuration
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    pub concurrent_users: usize,
    pub test_duration: Duration,
    pub ramp_up_duration: Duration,
    pub endpoints: Vec<String>,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            concurrent_users: 10,
            test_duration: Duration::from_secs(60),
            ramp_up_duration: Duration::from_secs(10),
            endpoints: vec!["/health".to_string()],
        }
    }
}

/// Performance test results
#[derive(Debug, Clone)]
pub struct PerformanceResults {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time: Duration,
    pub min_response_time: Duration,
    pub max_response_time: Duration,
    pub requests_per_second: f64,
    pub error_rate: f64,
}

/// Simple load testing framework
pub struct LoadTester {
    config: PerformanceConfig,
}

impl LoadTester {
    pub fn new(config: PerformanceConfig) -> Self {
        Self { config }
    }

    pub async fn run_test<F, Fut>(&self, test_function: F) -> Result<PerformanceResults, anyhow::Error>
    where
        F: Fn() -> Fut + Clone + 'static,
        Fut: std::future::Future<Output = Result<Duration, anyhow::Error>> + 'static,
    {
        let start_time = Instant::now();
        let mut all_durations = Vec::new();
        let mut error_count = 0;

        // Simple sequential testing for now to avoid lifetime issues
        let end_time = start_time + self.config.test_duration;
        while Instant::now() < end_time {
            match test_function().await {
                Ok(duration) => all_durations.push(duration),
                Err(_) => error_count += 1,
            }
        }

        let total_requests = all_durations.len() as u64 + error_count;
        let successful_requests = all_durations.len() as u64;
        let failed_requests = error_count;

        let average_response_time = if !all_durations.is_empty() {
            all_durations.iter().sum::<Duration>() / all_durations.len() as u32
        } else {
            Duration::from_secs(0)
        };

        let min_response_time = all_durations.iter().min().cloned().unwrap_or(Duration::from_secs(0));
        let max_response_time = all_durations.iter().max().cloned().unwrap_or(Duration::from_secs(0));

        let test_duration_secs = start_time.elapsed().as_secs_f64();
        let requests_per_second = total_requests as f64 / test_duration_secs;
        let error_rate = if total_requests > 0 {
            failed_requests as f64 / total_requests as f64
        } else {
            0.0
        };

        Ok(PerformanceResults {
            total_requests,
            successful_requests,
            failed_requests,
            average_response_time,
            min_response_time,
            max_response_time,
            requests_per_second,
            error_rate,
        })
    }
}

impl Default for LoadTester {
    fn default() -> Self {
        Self::new(PerformanceConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_tester_basic() {
        let tester = LoadTester::default();

        // Mock test function that simulates a 100ms response
        let test_fn = || async {
            sleep(Duration::from_millis(100)).await;
            Ok(Duration::from_millis(100))
        };

        let results = tester.run_test(test_fn).await.unwrap();

        assert!(results.total_requests > 0);
        assert!(results.successful_requests > 0);
        assert_eq!(results.failed_requests, 0);
        assert!(results.average_response_time >= Duration::from_millis(90));
        assert!(results.requests_per_second > 0.0);
        assert_eq!(results.error_rate, 0.0);
    }
}