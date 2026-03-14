//! Sci-Hub client for downloading full-text PDFs.
//!
//! Optional fallback source for papers not resolved via OA channels.
//! This module is best-effort and disabled by default.

use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
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

static SCIHUB_DOMAIN_COOLDOWN: OnceLock<Mutex<HashMap<String, std::time::Instant>>> =
    OnceLock::new();

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

        let candidate_domains: Vec<String> = self
            .domains
            .iter()
            .filter(|domain| !domain_on_cooldown(domain))
            .cloned()
            .collect();

        if candidate_domains.is_empty() {
            debug!("Skipping Sci-Hub fetch: all mirrors are on cooldown");
            return Ok(None);
        }

        let domain_parallelism = resolve_domain_parallelism()
            .max(1)
            .min(candidate_domains.len());
        let mut set = tokio::task::JoinSet::new();
        let mut next_idx = 0usize;
        let identifier = identifier.to_string();

        while next_idx < candidate_domains.len() || !set.is_empty() {
            while next_idx < candidate_domains.len() && set.len() < domain_parallelism {
                let domain = candidate_domains[next_idx].clone();
                let client = self.client.clone();
                let identifier = identifier.clone();
                set.spawn(async move {
                    let out = try_download_from_domain(client, &domain, &identifier).await;
                    (domain, out)
                });
                next_idx += 1;
            }

            if let Some(joined) = set.join_next().await {
                match joined {
                    Ok((domain, Ok(Some(bytes)))) => {
                        clear_domain_failure(&domain);
                        set.abort_all();
                        return Ok(Some(bytes));
                    }
                    Ok((domain, Ok(None))) => {
                        clear_domain_failure(&domain);
                        debug!("Domain {} did not have the PDF", domain);
                    }
                    Ok((domain, Err(e))) => {
                        mark_domain_failure(&domain);
                        warn!("Error fetching from domain {}: {}", domain, e);
                    }
                    Err(e) => {
                        warn!("Sci-Hub mirror task failed: {}", e);
                    }
                }
            }
        }

        warn!(
            "All Sci-Hub domains failed or didn't have PDF for {}",
            identifier
        );
        Ok(None)
    }
}

fn resolve_domain_parallelism() -> usize {
    std::env::var("FERRUMYX_SCIHUB_DOMAIN_PARALLELISM")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(4)
        .clamp(1, 16)
}

fn resolve_domain_cooldown_secs() -> u64 {
    std::env::var("FERRUMYX_SCIHUB_DOMAIN_COOLDOWN_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(300)
        .clamp(15, 3600)
}

fn domain_on_cooldown(domain: &str) -> bool {
    let now = std::time::Instant::now();
    let cache = SCIHUB_DOMAIN_COOLDOWN.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut guard) = cache.lock() else {
        return false;
    };
    guard.retain(|_, until| *until > now);
    guard.get(domain).is_some_and(|until| *until > now)
}

fn mark_domain_failure(domain: &str) {
    let cache = SCIHUB_DOMAIN_COOLDOWN.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut guard) = cache.lock() else {
        return;
    };
    guard.insert(
        domain.to_string(),
        std::time::Instant::now() + Duration::from_secs(resolve_domain_cooldown_secs()),
    );
}

fn clear_domain_failure(domain: &str) {
    let cache = SCIHUB_DOMAIN_COOLDOWN.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut guard) = cache.lock() else {
        return;
    };
    guard.remove(domain);
}

async fn try_download_from_domain(
    client: Client,
    domain: &str,
    identifier: &str,
) -> anyhow::Result<Option<Vec<u8>>> {
    let search_url = format!("{}/{}", domain, identifier);
    let resp = client
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
        if resp.status().is_server_error() || resp.status().as_u16() == 429 {
            anyhow::bail!("Sci-Hub mirror unavailable: {}", resp.status());
        }
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

        if let Some(pdf) = try_download_candidate(&client, &resolved).await? {
            info!(
                "Downloaded Sci-Hub PDF from {} ({} bytes)",
                resolved,
                pdf.len()
            );
            return Ok(Some(pdf));
        }

        // Sci-Hub "not in DB" pages often contain OA links (e.g. doi.org).
        if let Some(pdf) = try_pdf_from_landing_page(&client, &resolved).await? {
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

async fn try_download_candidate(
    client: &Client,
    resolved_url: &str,
) -> anyhow::Result<Option<Vec<u8>>> {
    let resp = client
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

    if content_type.contains("application/pdf") || content_type.contains("application/octet-stream")
    {
        debug!(
            "Candidate looked like PDF by content-type but failed header check: {}",
            resolved_url
        );
    }

    Ok(None)
}

async fn try_pdf_from_landing_page(client: &Client, url: &str) -> anyhow::Result<Option<Vec<u8>>> {
    let resp = client
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
        if let Some(pdf) = try_download_candidate(client, &resolved).await? {
            return Ok(Some(pdf));
        }
    }
    Ok(None)
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
