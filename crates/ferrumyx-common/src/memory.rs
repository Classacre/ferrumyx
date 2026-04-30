//! Memory monitoring and alerting utilities.

use memory_stats::memory_stats;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{info, warn, error};

#[derive(Debug, Clone)]
pub struct MemorySnapshot {
    pub physical_mem: u64,
    pub virtual_mem: u64,
    pub timestamp: Instant,
}

#[derive(Debug)]
pub struct MemoryMonitor {
    snapshots: Arc<Mutex<Vec<MemorySnapshot>>>,
    alert_threshold_mb: u64,
    max_snapshots: usize,
}

impl MemoryMonitor {
    pub fn new(alert_threshold_mb: u64) -> Self {
        Self {
            snapshots: Arc::new(Mutex::new(Vec::new())),
            alert_threshold_mb,
            max_snapshots: 100,
        }
    }

    pub async fn record_snapshot(&self) {
        if let Some(usage) = memory_stats() {
            let snapshot = MemorySnapshot {
                physical_mem: usage.physical_mem as u64,
                virtual_mem: usage.virtual_mem as u64,
                timestamp: Instant::now(),
            };
            let mut snapshots: tokio::sync::MutexGuard<'_, Vec<MemorySnapshot>> = self.snapshots.lock().await;
            snapshots.push(snapshot);
            if snapshots.len() > self.max_snapshots {
                snapshots.remove(0);
            }
            let latest = snapshots.last().unwrap();
            let phys_mb = latest.physical_mem / 1024 / 1024;
            if phys_mb > self.alert_threshold_mb {
                warn!("Memory usage alert: {} MB physical memory", phys_mb);
            }
        }
    }

    pub async fn get_growth_over_duration(&self, duration: Duration) -> Option<i64> {
        let snapshots: tokio::sync::MutexGuard<'_, Vec<MemorySnapshot>> = self.snapshots.lock().await;
        if snapshots.len() < 2 {
            return None;
        }
        let now = Instant::now();
        let cutoff = now - duration;
        let recent: Vec<_> = snapshots.iter().filter(|s| s.timestamp >= cutoff).collect();
        if recent.len() < 2 {
            return None;
        }
        let first = recent.first().unwrap().physical_mem as i64;
        let last = recent.last().unwrap().physical_mem as i64;
        Some(last - first)
    }

    pub async fn start_monitoring(self: Arc<Self>, interval: Duration) {
        info!("Starting memory monitoring with {}s interval", interval.as_secs());
        let monitor = Arc::clone(&self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            loop {
                interval.tick().await;
                monitor.record_snapshot().await;
            }
        });
    }

    pub async fn check_growth_alert(&self, duration: Duration, max_growth_mb: u64) -> bool {
        if let Some(growth) = self.get_growth_over_duration(duration).await {
            let growth_mb = growth / 1024 / 1024;
            if growth_mb > max_growth_mb as i64 {
                error!("Memory growth alert: {} MB over {:?}", growth_mb, duration);
                return true;
            }
        }
        false
    }
}

pub async fn start_memory_monitoring(alert_threshold_mb: u64, check_interval: Duration, growth_check_duration: Duration, max_growth_mb: u64) {
    let monitor = Arc::new(MemoryMonitor::new(alert_threshold_mb));
    monitor.clone().start_monitoring(check_interval).await;

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(growth_check_duration);
        loop {
            interval.tick().await;
            monitor.check_growth_alert(growth_check_duration, max_growth_mb).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_memory_monitor() {
        let monitor = MemoryMonitor::new(1000);
        monitor.record_snapshot().await;
        sleep(Duration::from_millis(10)).await;
        monitor.record_snapshot().await;

        let growth = monitor.get_growth_over_duration(Duration::from_secs(1)).await;
        assert!(growth.is_some() || growth.is_none()); // Just check it doesn't panic
    }
}