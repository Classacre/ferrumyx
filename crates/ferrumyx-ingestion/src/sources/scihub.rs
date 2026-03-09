//! Sci-Hub client for downloading full-text PDFs.
//!
//! This is an optional, fallback source that attempts to download PDFs
//! for papers that are not Open Access. It operates by scraping the
//! currently active Sci-Hub domain.
//!
//! Note: This is disabled by default due to the legal gray area of Sci-Hub.

use reqwest::Client;
use scraper::{Html, Selector};
use tracing::{debug, info, warn, instrument};
use url::Url;

const DEFAULT_SCIHUB_DOMAINS: &[&str] = &[
    "https://sci-hub.se",
    "https://sci-hub.st",
    "https://sci-hub.ru",
    "https://sci-hub.do",
    "https://sci-hub.box",
    "https://sci-hub.wf",
];

pub struct SciHubClient {
    client: Client,
    domains: Vec<String>,
}

impl Default for SciHubClient {
    fn default() -> Self {
        Self::new()
    }
}

impl SciHubClient {
    pub fn new() -> Self {
        // Use a browser-like User-Agent to avoid basic blocks
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/115.0")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            domains: DEFAULT_SCIHUB_DOMAINS.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Set custom retry domains if the defaults are down.
    pub fn with_retry_domains(mut self, domains: Vec<String>) -> Self {
        self.domains = domains.into_iter().map(|url| url.trim_end_matches('/').to_string()).collect();
        self
    }

    /// Attempt to download a PDF by DOI or PMID.
    /// Iterates through domains and returns the raw PDF bytes if successful.
    #[instrument(skip(self))]
    pub async fn download_pdf(&self, identifier: &str) -> anyhow::Result<Option<Vec<u8>>> {
        info!("Attempting to fetch {} from Sci-Hub", identifier);

        for domain in &self.domains {
            debug!("Trying Sci-Hub domain: {}", domain);
            match self.try_download_from_domain(domain, identifier).await {
                Ok(Some(bytes)) => return Ok(Some(bytes)),
                Ok(None) => debug!("Domain {} did not have the PDF", domain),
                Err(e) => warn!("Error fetching from domain {}: {}", domain, e),
            }
        }
        
        warn!("All Sci-Hub domains failed or didn't have PDF for {}", identifier);
        Ok(None)
    }

    async fn try_download_from_domain(&self, domain: &str, identifier: &str) -> anyhow::Result<Option<Vec<u8>>> {
        // 1. Fetch the Sci-Hub page for the identifier
        let search_url = format!("{}/{}", domain, identifier);
        let resp = self.client.get(&search_url).send().await?;

        if !resp.status().is_success() {
            return Ok(None);
        }

        let html_content = resp.text().await?;
        let mut pdf_url = None;
        {
            let document = Html::parse_document(&html_content);

            // 2. Find the PDF link in the HTML
            let embed_selector = Selector::parse("embed[type='application/pdf'], iframe#pdf, #pdf, div#article embed").unwrap();
            
            for element in document.select(&embed_selector) {
                if let Some(src) = element.value().attr("src") {
                    pdf_url = Some(src.to_string());
                    break;
                }
            }

            if pdf_url.is_none() {
                let a_selector = Selector::parse("a[href$='.pdf']").unwrap();
                for element in document.select(&a_selector) {
                    if let Some(href) = element.value().attr("href") {
                        pdf_url = Some(href.to_string());
                        break;
                    }
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
        }

        let Some(mut raw_url) = pdf_url else {
            return Ok(None);
        };

        // 3. Normalize the URL
        if raw_url.starts_with("//") {
            raw_url = format!("https:{}", raw_url);
        } else if raw_url.starts_with('/') {
            raw_url = format!("{}{}", domain, raw_url);
        }

        let parsed_url = Url::parse(&raw_url)?;

        // 4. Download the actual PDF
        let pdf_resp = self.client.get(parsed_url.as_str()).send().await?;
        
        if !pdf_resp.status().is_success() {
            return Ok(None);
        }

        let content_type = pdf_resp.headers().get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !content_type.contains("application/pdf") && !content_type.contains("application/octet-stream") && !raw_url.ends_with(".pdf") {
            warn!("Resolved URL did not return a PDF (Content-Type: {})", content_type);
            return Ok(None);
        }

        let pdf_bytes = pdf_resp.bytes().await?.to_vec();
        
        // Final sanity check (PDF magic number)
        if pdf_bytes.len() < 4 || &pdf_bytes[0..4] != b"%PDF" {
             return Ok(None);
        }
        
        info!("Successfully downloaded PDF from Sci-Hub ({} bytes)", pdf_bytes.len());
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
