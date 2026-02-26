//! Molecular docking using AutoDock Vina.

use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{info, debug};

/// Configuration for a docking run.
#[derive(Debug, Clone)]
pub struct DockingConfig {
    pub receptor: PathBuf,
    pub ligand: PathBuf,
    pub center_x: f64,
    pub center_y: f64,
    pub center_z: f64,
    pub size_x: f64,
    pub size_y: f64,
    pub size_z: f64,
    pub exhaustiveness: u32,
    pub out: PathBuf,
}

/// Wrapper for AutoDock Vina execution.
pub struct VinaRunner {
    executable_path: PathBuf,
}

impl VinaRunner {
    /// Create a new VinaRunner.
    pub fn new<P: AsRef<Path>>(executable_path: P) -> Self {
        Self {
            executable_path: executable_path.as_ref().to_path_buf(),
        }
    }

    /// Run AutoDock Vina with the given configuration.
    pub async fn run(&self, config: &DockingConfig) -> Result<PathBuf> {
        info!("Running AutoDock Vina on {:?}", config.ligand);

        let output = Command::new(&self.executable_path)
            .arg("--receptor")
            .arg(&config.receptor)
            .arg("--ligand")
            .arg(&config.ligand)
            .arg("--center_x")
            .arg(config.center_x.to_string())
            .arg("--center_y")
            .arg(config.center_y.to_string())
            .arg("--center_z")
            .arg(config.center_z.to_string())
            .arg("--size_x")
            .arg(config.size_x.to_string())
            .arg("--size_y")
            .arg(config.size_y.to_string())
            .arg("--size_z")
            .arg(config.size_z.to_string())
            .arg("--exhaustiveness")
            .arg(config.exhaustiveness.to_string())
            .arg("--out")
            .arg(&config.out)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("AutoDock Vina failed: {}", stderr);
        }

        debug!("AutoDock Vina completed successfully. Output in {:?}", config.out);
        Ok(config.out.clone())
    }
}
