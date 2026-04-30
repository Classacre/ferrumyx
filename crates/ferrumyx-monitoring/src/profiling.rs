//! Performance profiling and bottleneck identification

#[cfg(feature = "profiling")]
use pprof::ProfilerGuard;
#[cfg(feature = "profiling")]
use std::fs::File;
#[cfg(feature = "profiling")]
use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Performance profiler for identifying bottlenecks
pub struct Profiler {
    #[cfg(feature = "profiling")]
    guard: ProfilerGuard<'static>,
    call_stack: Arc<RwLock<Vec<ProfileEntry>>>,
}

impl Profiler {
    /// Create a new profiler
    #[cfg(feature = "profiling")]
    pub fn new() -> anyhow::Result<Self> {
        let guard = pprof::ProfilerGuard::new(100)?;

        Ok(Self {
            guard,
            call_stack: Arc::new(RwLock::new(Vec::new())),
        })
    }

    #[cfg(not(feature = "profiling"))]
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            call_stack: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Start profiling a function call
    pub async fn start_call(&self, function_name: &str, module: &str) -> CallTimer {
        let entry = ProfileEntry {
            function_name: function_name.to_string(),
            module: module.to_string(),
            start_time: Instant::now(),
            end_time: None,
            children: Vec::new(),
        };

        self.call_stack.write().await.push(entry);

        CallTimer {
            call_stack: self.call_stack.clone(),
            index: self.call_stack.read().await.len() - 1,
        }
    }

    /// Generate flame graph
    #[cfg(feature = "profiling")]
    pub fn generate_flame_graph(&self, output_path: &str) -> anyhow::Result<()> {
        let report = self.guard.report().build()?;
        let file = File::create(output_path)?;
        report.flame_graph(file)?;
        Ok(())
    }

    #[cfg(not(feature = "profiling"))]
    pub fn generate_flame_graph(&self, _output_path: &str) -> anyhow::Result<()> {
        anyhow::bail!("Profiling feature not enabled")
    }

    /// Get profiling statistics
    pub async fn get_stats(&self) -> ProfileStats {
        let stack = self.call_stack.read().await;
        let mut total_time = Duration::default();
        let mut function_times = std::collections::HashMap::new();

        for entry in stack.iter() {
            if let Some(end_time) = entry.end_time {
                let duration = end_time.duration_since(entry.start_time);
                total_time += duration;

                *function_times.entry(entry.function_name.clone()).or_insert(Duration::default()) += duration;
            }
        }

        ProfileStats {
            total_time,
            function_times,
            call_count: stack.len(),
        }
    }

    /// Clear profiling data
    pub async fn clear(&self) {
        self.call_stack.write().await.clear();
    }
}

/// Timer for tracking function call duration
pub struct CallTimer {
    call_stack: Arc<RwLock<Vec<ProfileEntry>>>,
    index: usize,
}

impl Drop for CallTimer {
    fn drop(&mut self) {
        let mut stack = futures::executor::block_on(async { self.call_stack.write().await });
        if let Some(entry) = stack.get_mut(self.index) {
            entry.end_time = Some(Instant::now());
        }
    }
}

/// Profile entry for a function call
#[derive(Debug, Clone)]
pub struct ProfileEntry {
    pub function_name: String,
    pub module: String,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
    pub children: Vec<ProfileEntry>,
}

/// Profiling statistics
#[derive(Debug, Clone)]
pub struct ProfileStats {
    pub total_time: Duration,
    pub function_times: std::collections::HashMap<String, Duration>,
    pub call_count: usize,
}

impl ProfileStats {
    /// Get the slowest functions
    pub fn slowest_functions(&self, limit: usize) -> Vec<(String, Duration)> {
        let mut functions: Vec<_> = self.function_times.iter()
            .map(|(name, duration)| (name.clone(), *duration))
            .collect();

        functions.sort_by(|a, b| b.1.cmp(&a.1));
        functions.into_iter().take(limit).collect()
    }

    /// Calculate average call time
    pub fn average_call_time(&self) -> Duration {
        if self.call_count == 0 {
            Duration::default()
        } else {
            self.total_time / self.call_count as u32
        }
    }
}

/// Profiling macros for easy instrumentation
#[macro_export]
macro_rules! profile_call {
    ($profiler:expr, $function:expr) => {
        $profiler.start_call($function, module_path!()).await
    };
}

#[macro_export]
macro_rules! profile_block {
    ($profiler:expr, $name:expr, $code:block) => {
        {
            let _timer = $profiler.start_call($name, module_path!()).await;
            $code
        }
    };
}