//! System resource monitoring

use sysinfo::{Disks, Networks, System};
use std::time::{Duration, Instant};
use tokio::time;

/// Resource monitor for system metrics
pub struct ResourceMonitor {
    system: System,
    disks: Disks,
    networks: Networks,
    last_update: Instant,
    memory_history: Vec<(Instant, f64)>, // Track memory usage over time
}

impl ResourceMonitor {
    /// Create a new resource monitor
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();

        Self {
            system,
            disks,
            networks,
            last_update: Instant::now(),
            memory_history: Vec::new(),
        }
    }

    /// Refresh system information and return current metrics
    pub fn refresh_and_get_metrics(&mut self) -> ResourceMetrics {
        self.system.refresh_all();
        self.disks.refresh();
        self.networks.refresh();
        self.last_update = Instant::now();

        let cpu_usage = self.system.global_cpu_usage() as f64;

        let total_memory = self.system.total_memory() as f64;
        let used_memory = self.system.used_memory() as f64;
        let memory_usage = if total_memory > 0.0 { (used_memory / total_memory) * 100.0 } else { 0.0 };

        let mut total_disk = 0.0;
        let mut used_disk = 0.0;
        for disk in &self.disks {
            total_disk += disk.total_space() as f64;
            used_disk += (disk.total_space() - disk.available_space()) as f64;
        }
        let disk_usage = if total_disk > 0.0 { (used_disk / total_disk) * 100.0 } else { 0.0 };

        let mut network_rx = 0;
        let mut network_tx = 0;
        for (_name, network) in &self.networks {
            network_rx += network.received();
            network_tx += network.transmitted();
        }

        ResourceMetrics {
            cpu_usage_percent: cpu_usage,
            memory_usage_percent: memory_usage,
            memory_used_mb: used_memory / 1024.0 / 1024.0,
            memory_total_mb: total_memory / 1024.0 / 1024.0,
            disk_usage_percent: disk_usage,
            disk_used_gb: used_disk / 1024.0 / 1024.0 / 1024.0,
            disk_total_gb: total_disk / 1024.0 / 1024.0 / 1024.0,
            network_rx_bytes: network_rx,
            network_tx_bytes: network_tx,
            uptime_seconds: System::uptime() as f64,
        }
    }

    /// Check for memory leaks based on recent history
    pub fn check_memory_leak(&mut self) -> Option<MemoryLeakAlert> {
        let current_metrics = self.refresh_and_get_metrics();
        let now = Instant::now();

        // Keep only last hour of history
        self.memory_history.retain(|(time, _)| now.duration_since(*time) < Duration::from_secs(3600));
        self.memory_history.push((now, current_metrics.memory_used_mb));

        if self.memory_history.len() < 2 {
            return None;
        }

        // Check growth over last 30 minutes
        let thirty_min_ago = now - Duration::from_secs(1800);
        let recent_history: Vec<_> = self.memory_history.iter()
            .filter(|(time, _)| *time >= thirty_min_ago)
            .collect();

        if recent_history.len() < 2 {
            return None;
        }

        let first = recent_history.first().unwrap().1;
        let last = recent_history.last().unwrap().1;
        let growth = last - first;

        if growth > 100.0 { // More than 100MB growth in 30 minutes
            Some(MemoryLeakAlert {
                growth_mb: growth,
                duration_minutes: 30,
                severity: if growth > 200.0 { "high" } else { "medium" }.to_string(),
            })
        } else {
            None
        }
    }

    /// Start background monitoring task
    pub async fn start_monitoring(&mut self, metrics_registry: std::sync::Arc<super::metrics::MetricsRegistry>) {
        let mut interval = time::interval(Duration::from_secs(30));

        loop {
            interval.tick().await;
            let metrics = self.refresh_and_get_metrics();

            // Check for memory leaks
            if let Some(alert) = self.check_memory_leak() {
                tracing::warn!(
                    "Memory leak detected: {:.1} MB growth in {} minutes (severity: {})",
                    alert.growth_mb,
                    alert.duration_minutes,
                    alert.severity
                );
            }

            // Record metrics
            metrics_registry.record_system_resources(
                metrics.cpu_usage_percent,
                metrics.memory_usage_percent,
                metrics.disk_usage_percent,
            );
        }
    }
}

/// Memory leak alert
#[derive(Debug, Clone)]
pub struct MemoryLeakAlert {
    pub growth_mb: f64,
    pub duration_minutes: u64,
    pub severity: String,
}

/// System resource metrics
#[derive(Debug, Clone)]
pub struct ResourceMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub memory_used_mb: f64,
    pub memory_total_mb: f64,
    pub disk_usage_percent: f64,
    pub disk_used_gb: f64,
    pub disk_total_gb: f64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub uptime_seconds: f64,
}

impl Default for ResourceMetrics {
    fn default() -> Self {
        Self {
            cpu_usage_percent: 0.0,
            memory_usage_percent: 0.0,
            memory_used_mb: 0.0,
            memory_total_mb: 0.0,
            disk_usage_percent: 0.0,
            disk_used_gb: 0.0,
            disk_total_gb: 0.0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
            uptime_seconds: 0.0,
        }
    }
}