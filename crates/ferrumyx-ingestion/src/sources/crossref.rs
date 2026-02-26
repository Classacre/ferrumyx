//! CrossRef DOI resolution client.
//!
//! Used for two purposes:
//! 1. Resolving bare DOIs to full metadata (title, authors, journal, date)
//! 2. Enriching papers from other sources that have DOI but no abstract
//!
//! API: https://api.crossref.org/works/{doi}
//! Polite pool: set User-Agent with mailto (see CrossRef etiquette)

use async_trait::async_trait;
use ferrumyx_common::sandbox::SandboxClient as Client;
use tracing::{debug, instrument, warn};
use chrono::NaiveDate;

use crate::models::{Author, IngestionSource, PaperMetadata};
use super::LiteratureSource;

const CR_API_BASE:   &str = "https://api.crossref.org/works";
const CR_SEARCH_URL: &str = "https://api.crossref.org/works";
const USER_AGENT:    &str = "Ferrumyx/0.1 (mailto:ferrumyx@example.com)";

pub struct CrossRefClient {
    client: Client,
}

impl CrossRefClient {
    pub fn new() -> Self {
        let mut client = Client::new().expect("CrossRef client build failed");
        // Allow adding specific headers or handling user agents if needed by SandboxClient,
        // but for now SandboxClient encapsulates reqwest configuration.
        Self { client }
    }

    /// Resolve a single DOI → PaperMetadata.
    #[instrument(skip(self))]
    pub async fn resolve_doi(&self, doi: &str) -> anyhow::Result<Option<PaperMetadata>> {
        let url = format!("{}/{}", CR_API_BASE, doi);
        let resp = self.client.get(&url)?.send().await?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let body: serde_json::Value = resp.json().await?;
        let work = &body["message"];
        Ok(Some(work_to_paper(work)))
    }

    /// Search CrossRef by free-text query.
    #[instrument(skip(self))]
    async fn search_works(
        &self,
        query: &str,
        max_results: usize,
    ) -> anyhow::Result<Vec<serde_json::Value>> {
        let clean = query.replace("[tiab]", "").replace(" AND ", " ");
        let resp = self.client
            .get(CR_SEARCH_URL)?
            .query(&[
                ("query", clean.trim()),
                ("rows",  &max_results.to_string()),
                ("select", "DOI,title,abstract,author,container-title,published,type"),
            ])
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        Ok(resp["message"]["items"]
            .as_array()
            .cloned()
            .unwrap_or_default())
    }
}

impl Default for CrossRefClient {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl LiteratureSource for CrossRefClient {
    async fn search(&self, query: &str, max_results: usize) -> anyhow::Result<Vec<PaperMetadata>> {
        let works = self.search_works(query, max_results).await?;
        debug!(n = works.len(), "CrossRef search results");
        Ok(works.iter().map(work_to_paper).collect())
    }

    async fn fetch_full_text(&self, doi: &str) -> anyhow::Result<Option<String>> {
        // CrossRef doesn't serve full-text; return the DOI resolver URL
        Ok(Some(format!("https://doi.org/{}", doi)))
    }
}

// ── Conversion ─────────────────────────────────────────────────────────────

fn work_to_paper(work: &serde_json::Value) -> PaperMetadata {
    let doi = work["DOI"].as_str().map(String::from);

    let title = work["title"]
        .as_array()
        .and_then(|t| t.first())
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();

    let abstract_text = work["abstract"].as_str().map(|a| {
        // CrossRef returns JATS XML snippets in abstract; strip basic tags
        a.replace("<jats:p>", "").replace("</jats:p>", "\n")
         .replace("<jats:italic>", "").replace("</jats:italic>", "")
         .replace("<jats:bold>", "").replace("</jats:bold>", "")
    });

    let authors: Vec<Author> = work["author"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|a| {
            let given  = a["given"].as_str().unwrap_or("").trim().to_string();
            let family = a["family"].as_str().unwrap_or("").trim().to_string();
            let name = if given.is_empty() { family.clone() } else { format!("{given} {family}") };
            Author {
                name,
                affiliation: a["affiliation"]
                    .as_array()
                    .and_then(|af| af.first())
                    .and_then(|af| af["name"].as_str())
                    .map(String::from),
                orcid: a["ORCID"].as_str().map(String::from),
            }
        })
        .collect();

    let journal = work["container-title"]
        .as_array()
        .and_then(|j| j.first())
        .and_then(|j| j.as_str())
        .map(String::from);

    let pub_date = work["published"]["date-parts"]
        .as_array()
        .and_then(|dp| dp.first())
        .and_then(|dp| dp.as_array())
        .and_then(|parts| {
            let year  = parts.first()?.as_u64()? as i32;
            let month = parts.get(1).and_then(|m| m.as_u64()).unwrap_or(1) as u32;
            let day   = parts.get(2).and_then(|d| d.as_u64()).unwrap_or(1) as u32;
            NaiveDate::from_ymd_opt(year, month, day)
        });

    PaperMetadata {
        doi,
        pmid:         None,
        pmcid:        None,
        title,
        abstract_text,
        authors,
        journal,
        pub_date,
        source:       IngestionSource::CrossRef,
        open_access:  work["license"].as_array().map(|l| !l.is_empty()).unwrap_or(false),
        full_text_url: work["link"]
            .as_array()
            .and_then(|links| {
                links.iter()
                    .find(|l| l["content-type"].as_str() == Some("application/pdf"))
                    .and_then(|l| l["URL"].as_str())
                    .map(String::from)
            }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_work_to_paper_minimal() {
        let work = serde_json::json!({
            "DOI": "10.1000/test",
            "title": ["Test Paper Title"],
            "abstract": "<jats:p>Test abstract.</jats:p>",
            "author": [{ "given": "Jane", "family": "Doe" }],
            "container-title": ["Nature"],
            "published": { "date-parts": [[2024, 6, 1]] }
        });
        let p = work_to_paper(&work);
        assert_eq!(p.doi.as_deref(), Some("10.1000/test"));
        assert_eq!(p.title, "Test Paper Title");
        assert!(p.abstract_text.as_deref().unwrap().contains("Test abstract."));
        assert_eq!(p.authors[0].name, "Jane Doe");
        assert_eq!(p.journal.as_deref(), Some("Nature"));
        assert_eq!(p.pub_date, NaiveDate::from_ymd_opt(2024, 6, 1));
    }

    #[test]
    fn test_jats_tag_stripping() {
        let raw = "<jats:p>Hello <jats:italic>world</jats:italic>.</jats:p>";
        let cleaned = raw.replace("<jats:p>", "").replace("</jats:p>", "\n")
            .replace("<jats:italic>", "").replace("</jats:italic>", "");
        assert_eq!(cleaned.trim(), "Hello world.");
    }
}
