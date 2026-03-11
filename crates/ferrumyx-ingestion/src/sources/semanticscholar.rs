//! Semantic Scholar Graph API client.
//!
//! Docs: https://api.semanticscholar.org/api-docs/graph

use async_trait::async_trait;
use chrono::NaiveDate;
use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, instrument};

use crate::models::{Author, IngestionSource, PaperMetadata};
use crate::sources::LiteratureSource;

const S2_SEARCH_URL: &str = "https://api.semanticscholar.org/graph/v1/paper/search";
const S2_PAPER_URL: &str = "https://api.semanticscholar.org/graph/v1/paper";

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    data: Vec<SearchPaper>,
}

#[derive(Debug, Deserialize)]
struct SearchPaper {
    #[serde(rename = "paperId")]
    paper_id: Option<String>,
    title: Option<String>,
    #[serde(rename = "abstract")]
    abstract_text: Option<String>,
    #[serde(default)]
    authors: Vec<SearchAuthor>,
    venue: Option<String>,
    year: Option<i32>,
    #[serde(rename = "externalIds")]
    external_ids: Option<ExternalIds>,
    #[serde(rename = "isOpenAccess")]
    is_open_access: Option<bool>,
    #[serde(rename = "openAccessPdf")]
    open_access_pdf: Option<OpenAccessPdf>,
}

#[derive(Debug, Deserialize)]
struct SearchAuthor {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExternalIds {
    DOI: Option<String>,
    PMID: Option<String>,
    PMCID: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAccessPdf {
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PaperDetails {
    #[serde(rename = "openAccessPdf")]
    open_access_pdf: Option<OpenAccessPdf>,
}

pub struct SemanticScholarClient {
    client: Client,
    api_key: Option<String>,
}

impl SemanticScholarClient {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key
                .map(|k| k.trim().to_string())
                .filter(|k| !k.is_empty()),
        }
    }

    fn apply_auth<'a>(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(ref key) = self.api_key {
            req.header("x-api-key", key)
        } else {
            req
        }
    }
}

impl Default for SemanticScholarClient {
    fn default() -> Self {
        Self::new(None)
    }
}

#[async_trait]
impl LiteratureSource for SemanticScholarClient {
    #[instrument(skip(self))]
    async fn search(
        &self,
        query: &str,
        max_results: usize,
    ) -> anyhow::Result<Vec<PaperMetadata>> {
        let limit = max_results.clamp(1, 100);
        let fields = "paperId,title,abstract,authors,venue,year,externalIds,isOpenAccess,openAccessPdf";
        let req = self
            .client
            .get(S2_SEARCH_URL)
            .query(&[
                ("query", query),
                ("limit", &limit.to_string()),
                ("fields", fields),
            ]);
        let resp = self.apply_auth(req).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Semantic Scholar search failed: HTTP {} - {}", status, body);
        }
        let parsed = resp.json::<SearchResponse>().await?;
        debug!(count = parsed.data.len(), "Semantic Scholar search returned results");

        let papers = parsed
            .data
            .into_iter()
            .map(|p| {
                let authors = p
                    .authors
                    .into_iter()
                    .filter_map(|a| {
                        let name = a.name.unwrap_or_default().trim().to_string();
                        if name.is_empty() {
                            None
                        } else {
                            Some(Author {
                                name,
                                affiliation: None,
                                orcid: None,
                            })
                        }
                    })
                    .collect::<Vec<_>>();

                let pub_date = p
                    .year
                    .and_then(|y| NaiveDate::from_ymd_opt(y, 1, 1));
                let ids = p.external_ids;
                let full_text_url = p
                    .open_access_pdf
                    .and_then(|pdf| pdf.url)
                    .filter(|u| !u.trim().is_empty());
                let title = p.title.unwrap_or_default();

                PaperMetadata {
                    doi: ids.as_ref().and_then(|x| x.DOI.clone()),
                    pmid: ids.as_ref().and_then(|x| x.PMID.clone()),
                    pmcid: ids.as_ref().and_then(|x| x.PMCID.clone()),
                    title,
                    abstract_text: p.abstract_text,
                    authors,
                    journal: p.venue,
                    pub_date,
                    source: IngestionSource::SemanticScholar,
                    open_access: p.is_open_access.unwrap_or(full_text_url.is_some()),
                    full_text_url,
                }
            })
            .filter(|p| !p.title.trim().is_empty())
            .collect::<Vec<_>>();

        Ok(papers)
    }

    #[instrument(skip(self))]
    async fn fetch_full_text(
        &self,
        paper_id: &str,
    ) -> anyhow::Result<Option<String>> {
        if paper_id.trim().is_empty() {
            return Ok(None);
        }
        let fields = "openAccessPdf";
        let url = format!("{}/{}", S2_PAPER_URL, paper_id);
        let req = self.client.get(url).query(&[("fields", fields)]);
        let resp = self.apply_auth(req).send().await?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let details = resp.json::<PaperDetails>().await?;
        Ok(details.open_access_pdf.and_then(|p| p.url))
    }
}
