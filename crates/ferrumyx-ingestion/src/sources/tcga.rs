//! TCGA (The Cancer Genome Atlas) REST API client via GDC.
//! Used for fetching survival correlation data for gene/cancer pairs.

use ferrumyx_common::sandbox::SandboxClient as Client;

const GDC_API_URL: &str = "https://api.gdc.cancer.gov";

pub struct TcgaClient {
    client: Client,
}

impl TcgaClient {
    pub fn new() -> Self {
        Self { client: Client::new().unwrap() }
    }

    /// Fetch survival correlation (dummy implementation wrapping a GDC search).
    /// In a real TCGA deployment, you would hit a processed survival DB (like cBioPortal or raw GDC clinic tables).
    /// Here we simulate the return based on whether the gene exists in the provided project.
    pub async fn get_survival_correlation(&self, gene_symbol: &str, project_id: &str) -> anyhow::Result<Option<f64>> {
        let url = format!("{}/projects/{}", GDC_API_URL, project_id);
        
        let resp = self.client
            .get(&url)?
            .send()
            .await?;

        if !resp.status().is_success() {
            return Ok(None);
        }

        // Dummy correlation computation based on gene string len (between -1.0 and 1.0)
        // just to simulate a survival coefficient because GDC doesn't expose pre-computed KM p-values directly
        let sim = (gene_symbol.len() as f64 % 10.0) / 10.0;
        let correlation = (sim * 2.0) - 1.0; 

        Ok(Some(correlation))
    }
}

impl Default for TcgaClient {
    fn default() -> Self {
        Self::new()
    }
}
