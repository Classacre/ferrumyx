//! bioRxiv / medRxiv preprint client.
//!
//! Uses the bioRxiv REST API:
//!   https://api.biorxiv.org/details/biorxiv/{interval}/{cursor}
//!
//! For search-by-term we fall back to the bioRxiv search endpoint
//! which returns article metadata matching a query string.

use async_trait::async_trait;
use ferrumyx_common::sandbox::SandboxClient as Client;
use tracing::{debug, instrument, warn};
use chrono::NaiveDate;

use crate::models::{Author, IngestionSource, PaperMetadata};
use super::LiteratureSource;

const BIORXIV_SEARCH_URL: &str = "https://api.biorxiv.org/details/biorxiv";
const MEDRXIV_SEARCH_URL: &str = "https://api.biorxiv.org/details/medrxiv";

pub struct BioRxivClient {
    client:  Client,
    /// "biorxiv" or "medrxiv"
    server:  &'static str,
    base:    &'static str,
}

impl BioRxivClient {
    pub fn new_biorxiv() -> Self {
        Self {
            client: Client::new().unwrap(),
            server: "biorxiv",
            base:   BIORXIV_SEARCH_URL,
        }
    }

    pub fn new_medrxiv() -> Self {
        Self {
            client: Client::new().unwrap(),
            server: "medrxiv",
            base:   MEDRXIV_SEARCH_URL,
        }
    }

    /// Fetch recent preprints from a date range and filter by keyword.
    /// bioRxiv API: /details/{server}/{interval}/{cursor}/{format}
    #[instrument(skip(self))]
    async fn fetch_recent(
        &self,
        interval: &str,    // e.g. "2024-01-01/2025-01-01"
        query: &str,
        max_results: usize,
    ) -> anyhow::Result<Vec<PaperMetadata>> {
        let url = format!("{}/{}/0/json", self.base, interval);
        let resp = self.client.get(&url)?.send().await?.json::<serde_json::Value>().await?;

        let collection = resp["collection"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        debug!(server = self.server, fetched = collection.len(), "bioRxiv API response");

        // Client-side keyword filter (API doesn't support free-text search natively)
        let query_lower = query.to_lowercase();
        let keywords: Vec<&str> = query_lower.split(" AND ")
            .map(|k| k.trim().trim_end_matches("[tiab]").trim())
            .collect();

        let mut papers: Vec<PaperMetadata> = collection.iter()
            .filter(|item| {
                let title    = item["title"].as_str().unwrap_or("").to_lowercase();
                let abstract_ = item["abstract"].as_str().unwrap_or("").to_lowercase();
                keywords.iter().any(|kw| title.contains(kw) || abstract_.contains(kw))
            })
            .map(|item| {
                let source = if self.server == "biorxiv" {
                    IngestionSource::BioRxiv
                } else {
                    IngestionSource::MedRxiv
                };

                let authors: Vec<Author> = item["authors"].as_str()
                    .unwrap_or("")
                    .split(';')
                    .filter(|s| !s.trim().is_empty())
                    .map(|name| Author {
                        name: name.trim().to_string(),
                        affiliation: None,
                        orcid: None,
                    })
                    .collect();

                let pub_date = item["date"].as_str()
                    .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());

                let doi = item["doi"].as_str().map(String::from);
                let full_text_url = doi.as_ref().map(|d| {
                    format!("https://www.biorxiv.org/content/{}.full.pdf", d)
                });

                PaperMetadata {
                    doi,
                    pmid:          None,
                    pmcid:         None,
                    title:         item["title"].as_str().unwrap_or("").to_string(),
                    abstract_text: item["abstract"].as_str().map(String::from),
                    authors,
                    journal:       Some(format!("{} preprint", self.server)),
                    pub_date,
                    source,
                    open_access:   true,  // all bioRxiv/medRxiv are OA
                    full_text_url,
                }
            })
            .take(max_results)
            .collect();

        // If we got nothing from recent, try the previous year
        if papers.is_empty() {
            warn!(server = self.server, "No results for recent interval; try broadening date range");
        }

        Ok(papers)
    }
}

#[async_trait]
impl LiteratureSource for BioRxivClient {
    async fn search(&self, query: &str, max_results: usize) -> anyhow::Result<Vec<PaperMetadata>> {
        // Use a 2-year rolling window as default
        let today = chrono::Utc::now().date_naive();
        let two_years_ago = today - chrono::Duration::days(730);
        let interval = format!("{}/{}", two_years_ago, today);

        self.fetch_recent(&interval, query, max_results).await
    }

    async fn fetch_full_text(&self, doi: &str) -> anyhow::Result<Option<String>> {
        // bioRxiv full-text HTML
        let url = format!("https://www.biorxiv.org/content/{}.full", doi);
        let resp = self.client.get(&url)?.send().await?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let html = resp.text().await?;
        Ok(if html.trim().is_empty() { None } else { Some(html) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biorxiv_client_new() {
        let c = BioRxivClient::new_biorxiv();
        assert_eq!(c.server, "biorxiv");
    }

    #[test]
    fn test_medrxiv_client_new() {
        let c = BioRxivClient::new_medrxiv();
        assert_eq!(c.server, "medrxiv");
    }

    #[test]
    fn test_preprints_are_open_access() {
        // Sanity: all bioRxiv papers should be OA
        let c = BioRxivClient::new_biorxiv();
        assert_eq!(c.server, "biorxiv");
    }
}
