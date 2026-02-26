//! PDB and AlphaFold structure fetching.

use anyhow::Result;
use ferrumyx_common::sandbox::SandboxClient as Client;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{info, debug};

/// Client for fetching protein structures from PDB and AlphaFold.
pub struct StructureFetcher {
    client: Client,
    cache_dir: PathBuf,
}

impl StructureFetcher {
    /// Create a new StructureFetcher with the given cache directory.
    pub fn new<P: AsRef<Path>>(cache_dir: P) -> Self {
        Self {
            client: Client::new().unwrap(),
            cache_dir: cache_dir.as_ref().to_path_buf(),
        }
    }

    /// Fetch a PDB file by its ID.
    pub async fn fetch_pdb(&self, pdb_id: &str) -> Result<PathBuf> {
        let file_name = format!("{}.pdb", pdb_id.to_lowercase());
        let file_path = self.cache_dir.join(&file_name);

        if file_path.exists() {
            debug!("PDB {} found in cache", pdb_id);
            return Ok(file_path);
        }

        info!("Fetching PDB {} from RCSB", pdb_id);
        let url = format!("https://files.rcsb.org/download/{}", file_name);
        let response = self.client.get(&url)?.send().await?.error_for_status()?;
        let content = response.bytes().await?;

        fs::create_dir_all(&self.cache_dir).await?;
        fs::write(&file_path, content).await?;

        Ok(file_path)
    }

    /// Fetch an AlphaFold structure by UniProt ID.
    pub async fn fetch_alphafold(&self, uniprot_id: &str) -> Result<PathBuf> {
        let file_name = format!("AF-{}-F1-model_v4.pdb", uniprot_id);
        let file_path = self.cache_dir.join(&file_name);

        if file_path.exists() {
            debug!("AlphaFold structure for {} found in cache", uniprot_id);
            return Ok(file_path);
        }

        info!("Fetching AlphaFold structure for {} from EBI", uniprot_id);
        let url = format!("https://alphafold.ebi.ac.uk/files/{}", file_name);
        let response = self.client.get(&url)?.send().await?.error_for_status()?;
        let content = response.bytes().await?;

        fs::create_dir_all(&self.cache_dir).await?;
        fs::write(&file_path, content).await?;

        Ok(file_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_fetch_pdb() {
        let dir = tempdir().unwrap();
        let fetcher = StructureFetcher::new(dir.path());
        
        // Fetch a small, known PDB (e.g., 1CRN - Crambin)
        let result = fetcher.fetch_pdb("1CRN").await;
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists());
    }
}
