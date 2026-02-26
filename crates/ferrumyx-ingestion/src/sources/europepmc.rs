//! Europe PMC REST API client.
//! See ARCHITECTURE.md §2.1 (Europe PMC REST API)
//!
//! Endpoint: https://www.ebi.ac.uk/europepmc/webservices/rest/search

use async_trait::async_trait;
use ferrumyx_common::sandbox::SandboxClient as Client;
use tracing::{debug, instrument};

use crate::models::{Author, IngestionSource, PaperMetadata};
use super::LiteratureSource;

const EPMC_SEARCH_URL: &str = "https://www.ebi.ac.uk/europepmc/webservices/rest/search";

pub struct EuropePmcClient {
    client: Client,
}

impl EuropePmcClient {
    pub fn new() -> Self {
        Self { client: Client::new().unwrap() }
    }
}

impl Default for EuropePmcClient {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl LiteratureSource for EuropePmcClient {
    #[instrument(skip(self))]
    async fn search(&self, query: &str, max_results: usize) -> anyhow::Result<Vec<PaperMetadata>> {
        let params = [
            ("query", query),
            ("resultType", "core"),
            ("pageSize", &max_results.to_string()),
            ("format", "json"),
        ];

        let resp = self.client
            .get(EPMC_SEARCH_URL)?
            .query(&params)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let results = resp["resultList"]["result"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        debug!(count = results.len(), "Europe PMC search returned results");

        let papers = results.iter().map(|r| {
            let authors: Vec<Author> = r["authorList"]["author"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|a| Author {
                    name: a["fullName"].as_str().unwrap_or("").to_string(),
                    affiliation: None,
                    orcid: a["authorId"]["value"].as_str().map(String::from),
                })
                .collect();

            PaperMetadata {
                doi: r["doi"].as_str().map(String::from),
                pmid: r["pmid"].as_str().map(String::from),
                pmcid: r["pmcid"].as_str().map(String::from),
                title: r["title"].as_str().unwrap_or("").to_string(),
                abstract_text: r["abstractText"].as_str().map(String::from),
                authors,
                journal: r["journalTitle"].as_str().map(String::from),
                pub_date: None, // TODO: parse r["firstPublicationDate"]
                source: IngestionSource::EuropePmc,
                open_access: r["isOpenAccess"].as_str() == Some("Y"),
                full_text_url: r["fullTextUrlList"]["fullTextUrl"]
                    .as_array()
                    .and_then(|urls: &Vec<serde_json::Value>| {
                        urls.iter()
                            .find(|u| u["documentStyle"].as_str() == Some("pdf"))
                            .and_then(|u| u["url"].as_str())
                            .map(String::from)
                    }),
            }
        }).collect();

        Ok(papers)
    }

    async fn fetch_full_text(&self, pmcid: &str) -> anyhow::Result<Option<String>> {
        let url = format!(
            "https://www.ebi.ac.uk/europepmc/webservices/rest/{}/fullTextXML",
            pmcid
        );
        let resp = self.client.get(&url)?.send().await?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let xml = resp.text().await?;
        Ok(if xml.trim().is_empty() { None } else { Some(xml) })
    }
}
