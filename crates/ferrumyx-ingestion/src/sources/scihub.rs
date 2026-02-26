//! Sci-Hub client for downloading full-text PDFs.
//!
//! This is an optional, fallback source that attempts to download PDFs
//! for papers that are not Open Access. It operates by scraping the
//! currently active Sci-Hub domain.
//!
//! Note: This is disabled by default due to the legal gray area of Sci-Hub.

use ferrumyx_common::sandbox::SandboxClient as Client;
use scraper::{Html, Selector};
use tracing::{debug, info, warn, instrument};
use url::Url;

const DEFAULT_SCIHUB_URL: &str = "https://sci-hub.se";

pub struct SciHubClient {
    client: Client,
    base_url: String,
}

impl Default for SciHubClient {
    fn default() -> Self {
        Self::new()
    }
}

impl SciHubClient {
    pub fn new() -> Self {
        // Use a browser-like User-Agent to avoid basic blocks
        let mut client = Client::new().expect("Failed to build SciHub client");
        client.allow_domain("sci-hub.se");

        Self {
            client,
            base_url: DEFAULT_SCIHUB_URL.to_string(),
        }
    }

    /// Set a custom base URL if the default one is down.
    pub fn with_base_url(mut self, url: &str) -> Self {
        self.base_url = url.trim_end_matches('/').to_string();
        self
    }

    /// Attempt to download a PDF by DOI or PMID.
    /// Returns the raw PDF bytes if successful.
    #[instrument(skip(self))]
    pub async fn download_pdf(&self, identifier: &str) -> anyhow::Result<Option<Vec<u8>>> {
        info!("Attempting to fetch {} from Sci-Hub", identifier);

        // 1. Fetch the Sci-Hub page for the identifier
        let search_url = format!("{}/{}", self.base_url, identifier);
        let resp = self.client.get(&search_url)?.send().await?;

        if !resp.status().is_success() {
            warn!("Sci-Hub returned status {} for {}", resp.status(), identifier);
            return Ok(None);
        }

        let html_content = resp.text().await?;
        let document = Html::parse_document(&html_content);

        // 2. Find the PDF link in the HTML
        // Sci-Hub usually puts the PDF in an <embed> or <iframe> tag, or a direct link with id="pdf"
        let embed_selector = Selector::parse("embed[type='application/pdf'], iframe#pdf").unwrap();
        
        let mut pdf_url = None;
        for element in document.select(&embed_selector) {
            if let Some(src) = element.value().attr("src") {
                pdf_url = Some(src.to_string());
                break;
            }
        }

        // Fallback: sometimes it's just a button
        if pdf_url.is_none() {
            let button_selector = Selector::parse("button[onclick^='location.href']").unwrap();
            for element in document.select(&button_selector) {
                if let Some(onclick) = element.value().attr("onclick") {
                    // Extract URL from location.href='//domain.com/file.pdf'
                    if let Some(start) = onclick.find('\'') {
                        if let Some(end) = onclick.rfind('\'') {
                            if start < end {
                                pdf_url = Some(onclick[start + 1..end].to_string());
                                break;
                            }
                        }
                    }
                }
            }
        }

        let Some(mut raw_url) = pdf_url else {
            debug!("Could not find PDF link on Sci-Hub page for {}", identifier);
            return Ok(None);
        };

        // 3. Normalize the URL
        // Sci-Hub URLs often start with "//" (protocol-relative)
        if raw_url.starts_with("//") {
            raw_url = format!("https:{}", raw_url);
        } else if raw_url.starts_with('/') {
            raw_url = format!("{}{}", self.base_url, raw_url);
        }

        // Ensure it's a valid URL
        let parsed_url = Url::parse(&raw_url)?;
        debug!("Found PDF URL: {}", parsed_url);

        // 4. Download the actual PDF
        let pdf_resp = self.client.get(parsed_url.as_str())?.send().await?;
        
        if !pdf_resp.status().is_success() {
            warn!("Failed to download PDF from resolved URL: {}", pdf_resp.status());
            return Ok(None);
        }

        let content_type = pdf_resp.headers().get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !content_type.contains("application/pdf") {
            warn!("Resolved URL did not return a PDF (Content-Type: {})", content_type);
            return Ok(None);
        }

        let pdf_bytes = pdf_resp.bytes().await?.to_vec();
        info!("Successfully downloaded PDF ({} bytes)", pdf_bytes.len());

        Ok(Some(pdf_bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Hits external Sci-Hub API"]
    async fn test_scihub_download() {
        let client = SciHubClient::new();
        // A well-known older paper DOI
        let doi = "10.1038/nature14539"; 
        
        let result = client.download_pdf(doi).await.unwrap();
        assert!(result.is_some());
        let bytes = result.unwrap();
        assert!(bytes.len() > 100_000); // Should be a decent sized PDF
        
        // Check PDF magic number
        assert_eq!(&bytes[0..4], b"%PDF");
    }
}
