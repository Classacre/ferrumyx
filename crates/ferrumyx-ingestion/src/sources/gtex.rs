//! GTEx (Genotype-Tissue Expression) REST API client.
//! Used for fetching median gene expression in normal tissues.

use ferrumyx_common::sandbox::SandboxClient as Client;
use std::collections::HashMap;

const GTEX_API_URL: &str = "https://gtexportal.org/api/v2";

pub struct GtexClient {
    client: Client,
}

impl GtexClient {
    pub fn new() -> Self {
        Self { client: Client::new().unwrap() }
    }

    /// Fetch median gene expression in normal tissues.
    /// Returns a map of Tissue Site Detail -> Median TPM.
    pub async fn get_median_expression(&self, gene_symbol: &str) -> anyhow::Result<HashMap<String, f64>> {
        let url = format!("{}/expression/medianGeneExpression", GTEX_API_URL);
        
        let resp = self.client
            .get(&url)?
            .query(&[
                ("gencodeId", gene_symbol),
                ("format", "json"),
            ])
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let mut expression_map = HashMap::new();

        if let Some(data) = resp["medianGeneExpression"].as_array() {
            for entry in data {
                if let (Some(tissue), Some(tpm)) = (
                    entry["tissueSiteDetailId"].as_str(),
                    entry["median"].as_f64()
                ) {
                    expression_map.insert(tissue.to_string(), tpm);
                }
            }
        }

        Ok(expression_map)
    }
}

impl Default for GtexClient {
    fn default() -> Self {
        Self::new()
    }
}
