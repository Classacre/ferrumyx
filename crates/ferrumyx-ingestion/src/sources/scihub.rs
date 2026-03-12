//! Sci-Hub client for downloading full-text PDFs.
//!
//! Optional fallback source for papers not resolved via OA channels.
//! This module is best-effort and disabled by default.

use reqwest::Client;
use scraper::{Html, Selector};
use std::time::Duration;
use tracing::{debug, info, instrument, warn};
use url::Url;

const DEFAULT_SCIHUB_DOMAINS: &[&str] = &[
    "https://sci-hub.al",
    "https://sci-hub.mk",
    "https://sci-hub.ee",
    "https://sci-hub.vg",
    "https://sci-hub.st",
    "http://sci-hub.al",
    "http://sci-hub.mk",
    "http://sci-hub.ee",
    "http://sci-hub.vg",
    "http://sci-hub.st",
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
        let timeout_secs = std::env::var("FERRUMYX_SCIHUB_REQUEST_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(10)
            .clamp(4, 45);

        let client = Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:128.0) Gecko/20100101 Firefox/128.0",
            )
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .unwrap_or_else(|_| Client::new());

        let domains = std::env::var("FERRUMYX_SCIHUB_DOMAINS")
            .ok()
            .map(|raw| {
                raw.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.trim_end_matches('/').to_string())
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| {
                DEFAULT_SCIHUB_DOMAINS
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            });

        Self { client, domains }
    }

    pub fn with_retry_domains(mut self, domains: Vec<String>) -> Self {
        self.domains = domains
            .into_iter()
            .map(|url| url.trim_end_matches('/').to_string())
            .collect();
        self
    }

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

        warn!(
            "All Sci-Hub domains failed or didn't have PDF for {}",
            identifier
        );
        Ok(None)
    }

    async fn try_download_from_domain(
        &self,
        domain: &str,
        identifier: &str,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        let search_url = format!("{}/{}", domain, identifier);
        let resp = self
            .client
            .get(&search_url)
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .send()
            .await?;

        if !resp.status().is_success() {
            debug!(
                domain = domain,
                identifier = identifier,
                status = %resp.status(),
                "Sci-Hub domain returned non-success"
            );
            return Ok(None);
        }

        let html_content = resp.text().await?;
        let candidates = extract_candidate_urls(&html_content);
        if candidates.is_empty() {
            return Ok(None);
        }

        for raw in candidates {
            let Some(resolved) = normalize_candidate_url(domain, &raw) else {
                continue;
            };

            if let Some(pdf) = self.try_download_candidate(&resolved).await? {
                info!(
                    "Downloaded Sci-Hub PDF from {} ({} bytes)",
                    resolved,
                    pdf.len()
                );
                return Ok(Some(pdf));
            }

            // Sci-Hub "not in DB" pages often contain OA links (e.g. doi.org).
            if let Some(pdf) = self.try_pdf_from_landing_page(&resolved).await? {
                info!(
                    "Downloaded PDF from Sci-Hub landing fallback {} ({} bytes)",
                    resolved,
                    pdf.len()
                );
                return Ok(Some(pdf));
            }
        }

        Ok(None)
    }

    async fn try_download_candidate(&self, resolved_url: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let resp = self
            .client
            .get(resolved_url)
            .header(
                "Accept",
                "application/pdf,application/octet-stream,text/html;q=0.8,*/*;q=0.5",
            )
            .send()
            .await?;

        if !resp.status().is_success() {
            return Ok(None);
        }

        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_lowercase();
        let bytes = resp.bytes().await?.to_vec();

        if bytes.len() >= 4 && &bytes[0..4] == b"%PDF" {
            return Ok(Some(bytes));
        }

        if content_type.contains("application/pdf")
            || content_type.contains("application/octet-stream")
        {
            debug!(
                "Candidate looked like PDF by content-type but failed header check: {}",
                resolved_url
            );
        }

        Ok(None)
    }

    async fn try_pdf_from_landing_page(&self, url: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let resp = self
            .client
            .get(url)
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .send()
            .await?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let html = resp.text().await?;
        let candidates = extract_candidate_urls(&html);
        for raw in candidates {
            let Some(resolved) = normalize_candidate_url(url, &raw) else {
                continue;
            };
            if let Some(pdf) = self.try_download_candidate(&resolved).await? {
                return Ok(Some(pdf));
            }
        }
        Ok(None)
    }
}

fn extract_candidate_urls(html: &str) -> Vec<String> {
    let doc = Html::parse_document(html);
    let mut urls = Vec::new();

    let selectors = [
        ("embed[type='application/pdf']", "src"),
        ("iframe#pdf", "src"),
        ("iframe[src]", "src"),
        ("embed[src]", "src"),
        ("object[data]", "data"),
        ("a[href$='.pdf']", "href"),
        ("a[href*='/downloads/']", "href"),
        ("a[href*='download']", "href"),
        ("a[href*='doi.org']", "href"),
        ("meta[name='citation_pdf_url']", "content"),
        ("meta[property='citation_pdf_url']", "content"),
    ];

    for (sel, attr) in selectors {
        let Ok(selector) = Selector::parse(sel) else {
            continue;
        };
        for node in doc.select(&selector) {
            if let Some(v) = node.value().attr(attr) {
                urls.push(v.to_string());
            }
        }
    }

    if let Ok(selector) = Selector::parse("button[onclick], a[onclick]") {
        for node in doc.select(&selector) {
            if let Some(onclick) = node.value().attr("onclick") {
                if let Some(start) = onclick.find('\'') {
                    if let Some(end) = onclick.rfind('\'') {
                        if start < end {
                            urls.push(onclick[start + 1..end].to_string());
                        }
                    }
                }
            }
        }
    }

    urls.sort();
    urls.dedup();
    urls
}

fn normalize_candidate_url(base: &str, raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() || raw.starts_with("data:") || raw.starts_with("javascript:") {
        return None;
    }
    if raw.starts_with("//") {
        return Some(format!("https:{}", raw));
    }
    if let Ok(u) = Url::parse(raw) {
        return Some(u.to_string());
    }
    let base_url = Url::parse(base)
        .ok()
        .or_else(|| Url::parse(&(base.to_string() + "/")).ok())?;
    base_url.join(raw).ok().map(|u| u.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_openaccess_and_pdf_candidates() {
        let html = r#"
        <html><head><meta name="citation_pdf_url" content="/paper.pdf" /></head>
        <body>
          <a href="//doi.org/10.1016/test">doi</a>
          <iframe id="pdf" src="/downloads/abc.pdf"></iframe>
          <button onclick="location.href='//example.org/file.pdf'"></button>
        </body></html>
        "#;
        let cands = extract_candidate_urls(html);
        assert!(cands.iter().any(|c| c.contains("paper.pdf")));
        assert!(cands.iter().any(|c| c.contains("doi.org")));
        assert!(cands.iter().any(|c| c.contains("downloads/abc.pdf")));
    }
}
