//! Binding pocket detection using fpocket.

use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{info, debug};

/// Wrapper for fpocket execution.
pub struct FPocketRunner {
    executable_path: PathBuf,
}

impl FPocketRunner {
    /// Create a new FPocketRunner.
    pub fn new<P: AsRef<Path>>(executable_path: P) -> Self {
        Self {
            executable_path: executable_path.as_ref().to_path_buf(),
        }
    }

    /// Run fpocket on a given PDB file.
    pub async fn run(&self, pdb_path: &Path) -> Result<PathBuf> {
        info!("Running fpocket on {:?}", pdb_path);

        let output = Command::new(&self.executable_path)
            .arg("-f")
            .arg(pdb_path)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("fpocket failed: {}", stderr);
        }

        // fpocket creates a directory named <pdb_name>_out
        let pdb_name = pdb_path.file_stem().unwrap().to_string_lossy();
        let out_dir = pdb_path.with_file_name(format!("{}_out", pdb_name));

        if !out_dir.exists() {
            anyhow::bail!("fpocket output directory not found: {:?}", out_dir);
        }

        debug!("fpocket completed successfully. Output in {:?}", out_dir);
        Ok(out_dir)
    }
}
