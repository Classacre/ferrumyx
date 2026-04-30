//! Configuration hot reloading with file watching
//!
//! Provides runtime configuration updates without restart using file system watching.

use crate::{ConfigError, FerrumyxConfig};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration watcher for hot reloading
pub struct ConfigWatcher {
    config: Arc<RwLock<FerrumyxConfig>>,
    callback: Arc<dyn Fn(FerrumyxConfig) + Send + Sync>,
    watcher: Option<RecommendedWatcher>,
    watched_paths: Vec<PathBuf>,
}

impl ConfigWatcher {
    /// Create a new configuration watcher
    pub fn new<F>(config: FerrumyxConfig, callback: F) -> Result<Self, ConfigError>
    where
        F: Fn(FerrumyxConfig) + Send + Sync + 'static,
    {
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            callback: Arc::new(callback),
            watcher: None,
            watched_paths: vec![],
        })
    }

    /// Watch configuration files for changes
    pub async fn watch_files(mut self, paths: Vec<PathBuf>) -> Result<Self, ConfigError> {
        let config = Arc::clone(&self.config);
        let callback = Arc::clone(&self.callback);

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        if Self::should_reload(&event) {
                            tokio::spawn(async move {
                                if let Err(e) = Self::reload_config(&config, &callback).await {
                                    tracing::error!("Failed to reload configuration: {}", e);
                                }
                            });
                        }
                    }
                    Err(e) => {
                        tracing::error!("Watch error: {:?}", e);
                    }
                }
            },
            Config::default(),
        )?;

        // Watch each path
        for path in &paths {
            if path.exists() {
                watcher.watch(path, RecursiveMode::NonRecursive)?;
                tracing::info!("Watching configuration file: {}", path.display());
            } else {
                tracing::warn!("Configuration file does not exist: {}", path.display());
            }
        }

        self.watcher = Some(watcher);
        self.watched_paths = paths;

        Ok(self)
    }

    /// Watch environment variables for changes (limited support)
    pub fn watch_env(self) -> Self {
        // Environment variable watching is more complex and platform-dependent
        // For now, we'll just log that it's not supported
        tracing::warn!("Environment variable watching is not currently supported");
        self
    }

    /// Get the current configuration (read-only)
    pub async fn get_config(&self) -> FerrumyxConfig {
        self.config.read().await.clone()
    }

    /// Manually trigger a configuration reload
    pub async fn reload(&self) -> Result<(), ConfigError> {
        Self::reload_config(&self.config, &self.callback).await
    }

    /// Stop watching for changes
    pub fn stop_watching(&mut self) {
        if let Some(watcher) = self.watcher.take() {
            drop(watcher); // This will stop the watcher
        }
        tracing::info!("Stopped watching configuration files");
    }

    /// Check if an event should trigger a reload
    fn should_reload(event: &Event) -> bool {
        matches!(
            event.kind,
            notify::EventKind::Modify(
                notify::event::ModifyKind::Data(_) | notify::event::ModifyKind::Name(_)
            ) | notify::EventKind::Create(_) | notify::EventKind::Remove(_)
        )
    }

    /// Reload configuration from watched files
    async fn reload_config(
        config: &Arc<RwLock<FerrumyxConfig>>,
        callback: &Arc<dyn Fn(FerrumyxConfig) + Send + Sync>,
    ) -> Result<(), ConfigError> {
        // Load new configuration
        // In a real implementation, this would use the same loading logic as ConfigLoader
        let new_config = FerrumyxConfig::load().await?;

        // Validate the new configuration
        let validation_result = new_config.validate();
        if !validation_result.is_valid {
            return Err(ConfigError::Validation(format!(
                "New configuration is invalid: {}",
                validation_result.errors.iter()
                    .map(|e| format!("{}: {}", e.path, e.message))
                    .collect::<Vec<_>>()
                    .join("; ")
            )));
        }

        // Update the configuration
        {
            let mut config_guard = config.write().await;
            *config_guard = new_config.clone();
        }

        // Call the callback with the new configuration
        callback(new_config);

        tracing::info!("Configuration reloaded successfully");
        Ok(())
    }
}

impl Drop for ConfigWatcher {
    fn drop(&mut self) {
        self.stop_watching();
    }
}

/// Configuration watching options
#[derive(Debug, Clone)]
pub struct WatchOptions {
    /// Enable file watching
    pub watch_files: bool,

    /// Configuration file paths to watch
    pub file_paths: Vec<PathBuf>,

    /// Enable environment variable watching
    pub watch_env: bool,

    /// Reload debounce time (milliseconds)
    pub debounce_ms: u64,

    /// Maximum reload frequency (reloads per minute)
    pub max_reload_rate: u32,
}

impl Default for WatchOptions {
    fn default() -> Self {
        Self {
            watch_files: true,
            file_paths: vec![
                "./config/ferrumyx.toml".into(),
                "./config/ferrumyx.json".into(),
                "./config/ferrumyx.yaml".into(),
            ],
            watch_env: false,
            debounce_ms: 500,
            max_reload_rate: 10,
        }
    }
}

/// Create a configuration watcher with default options
pub async fn create_watcher<F>(
    initial_config: FerrumyxConfig,
    callback: F,
) -> Result<ConfigWatcher, ConfigError>
where
    F: Fn(FerrumyxConfig) + Send + Sync + 'static,
{
    create_watcher_with_options(initial_config, callback, WatchOptions::default()).await
}

/// Create a configuration watcher with custom options
pub async fn create_watcher_with_options<F>(
    initial_config: FerrumyxConfig,
    callback: F,
    options: WatchOptions,
) -> Result<ConfigWatcher, ConfigError>
where
    F: Fn(FerrumyxConfig) + Send + Sync + 'static,
{
    let mut watcher = ConfigWatcher::new(initial_config, callback)?;

    if options.watch_files {
        watcher = watcher.watch_files(options.file_paths).await?;
    }

    if options.watch_env {
        watcher = watcher.watch_env();
    }

    Ok(watcher)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_create_watcher() {
        let config = FerrumyxConfig::default();
        let callback_called = Arc::new(std::sync::Mutex::new(false));

        let callback = {
            let callback_called = Arc::clone(&callback_called);
            move |_config: FerrumyxConfig| {
                *callback_called.lock().unwrap() = true;
            }
        };

        let watcher = create_watcher(config, callback).await;
        assert!(watcher.is_ok());

        // Test manual reload (should work even without files)
        let reload_result = timeout(Duration::from_secs(5), watcher.unwrap().reload()).await;
        assert!(reload_result.is_ok());
    }
}