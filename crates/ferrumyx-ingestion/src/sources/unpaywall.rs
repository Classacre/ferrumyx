//! Unpaywall API client.
//!
//! Endpoint: https://api.unpaywall.org/v2/{doi}?email={contact}

use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, instrument};

const UNPAYWALL_BASE_URL: &str = "https://api.unpaywall.org/v2";

#[derive(Debug, Deserialize)]
struct UnpaywallResponse {
    #[serde(default)]
    best_oa_location: Option<OaLocation>,
}

#[derive(Debug, Deserialize)]
struct OaLocation {
    #[serde(default)]
    url_for_pdf: Option<String>,
}

pub struct UnpaywallClient {
    client: Client,
    email: String,
}

impl UnpaywallClient {
    pub fn new(email: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            email: email.into(),
        }
    }

    #[instrument(skip(self))]
    pub async fn resolve_pdf_url(&self, doi: &str) -> anyhow::Result<Option<String>> {
        let doi = doi.trim();
        if doi.is_empty() {
            return Ok(None);
        }
        if self.email.trim().is_empty() {
            return Ok(None);
        }

        let url = format!("{UNPAYWALL_BASE_URL}/{doi}");
        let resp = self
            .client
            .get(&url)
            .query(&[("email", self.email.as_str())])
            .send()
            .await?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let parsed = resp.json::<UnpaywallResponse>().await?;
        let pdf = parsed
            .best_oa_location
            .and_then(|loc| loc.url_for_pdf)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        debug!(has_pdf = pdf.is_some(), "Unpaywall DOI lookup completed");
        Ok(pdf)
    }
}

