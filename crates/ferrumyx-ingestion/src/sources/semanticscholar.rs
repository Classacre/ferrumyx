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
const S2_DEFAULT_CITATION_EXPANSION: usize = 16;
const S2_MAX_EXPANSION_SEEDS: usize = 4;

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
    #[serde(rename = "DOI")]
    doi: Option<String>,
    #[serde(rename = "PMID")]
    pmid: Option<String>,
    #[serde(rename = "PMCID")]
    pmcid: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAccessPdf {
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PaperDetails {
    #[serde(rename = "openAccessPdf")]
    open_access_pdf: Option<OpenAccessPdf>,
    embedding: Option<S2Embedding>,
}

#[derive(Debug, Deserialize)]
struct S2Embedding {
    #[serde(rename = "specter_v2")]
    specter_v2: Option<Vec<f32>>,
}

#[derive(Debug, Deserialize)]
struct EdgePaper {
    #[serde(rename = "citedPaper")]
    cited_paper: Option<SearchPaper>,
    #[serde(rename = "citingPaper")]
    citing_paper: Option<SearchPaper>,
}

#[derive(Debug, Deserialize)]
struct EdgeResponse {
    #[serde(default)]
    data: Vec<EdgePaper>,
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

    fn map_search_paper(p: SearchPaper) -> Option<PaperMetadata> {
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

        let pub_date = p.year.and_then(|y| NaiveDate::from_ymd_opt(y, 1, 1));
        let ids = p.external_ids;
        let full_text_url = p
            .open_access_pdf
            .and_then(|pdf| pdf.url)
            .filter(|u| !u.trim().is_empty());
        let title = p.title.unwrap_or_default();
        if title.trim().is_empty() {
            return None;
        }

        Some(PaperMetadata {
            doi: ids.as_ref().and_then(|x| x.doi.clone()),
            pmid: ids.as_ref().and_then(|x| x.pmid.clone()),
            pmcid: ids.as_ref().and_then(|x| x.pmcid.clone()),
            title,
            abstract_text: p.abstract_text,
            authors,
            journal: p.venue,
            pub_date,
            source: IngestionSource::SemanticScholar,
            open_access: p.is_open_access.unwrap_or(full_text_url.is_some()),
            full_text_url,
        })
    }

    fn citation_expansion_limit(max_results: usize) -> usize {
        let env_limit = std::env::var("FERRUMYX_S2_CITATION_EXPANSION")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(S2_DEFAULT_CITATION_EXPANSION);
        max_results.saturating_sub(1).min(env_limit).min(64)
    }

    async fn fetch_edges(
        &self,
        paper_id: &str,
        edge: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<PaperMetadata>> {
        if paper_id.trim().is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let fields = "title,abstract,authors,venue,year,externalIds,isOpenAccess,openAccessPdf";
        let url = format!("{}/{}/{}", S2_PAPER_URL, paper_id, edge);
        let req = self.client.get(url).query(&[
            ("fields", fields),
            ("limit", &limit.to_string()),
        ]);
        let resp = self.apply_auth(req).send().await?;
        if !resp.status().is_success() {
            return Ok(Vec::new());
        }
        let parsed = resp.json::<EdgeResponse>().await?;

        let mut out = Vec::new();
        for row in parsed.data {
            let mapped = match edge {
                "references" => row.cited_paper.and_then(Self::map_search_paper),
                "citations" => row.citing_paper.and_then(Self::map_search_paper),
                _ => None,
            };
            if let Some(paper) = mapped {
                out.push(paper);
            }
        }
        Ok(out)
    }

    async fn expand_citation_graph(
        &self,
        seed_ids: &[String],
        max_to_add: usize,
    ) -> anyhow::Result<Vec<PaperMetadata>> {
        if max_to_add == 0 || seed_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut expanded = Vec::new();
        let per_seed = (max_to_add / seed_ids.len().max(1)).clamp(2, 8);
        for paper_id in seed_ids.iter().take(S2_MAX_EXPANSION_SEEDS) {
            let refs = self.fetch_edges(paper_id, "references", per_seed).await?;
            expanded.extend(refs);
            if expanded.len() >= max_to_add {
                break;
            }
            let cites = self.fetch_edges(paper_id, "citations", per_seed).await?;
            expanded.extend(cites);
            if expanded.len() >= max_to_add {
                break;
            }
        }
        Ok(expanded)
    }

    pub async fn fetch_specter2_embedding(
        &self,
        paper_id: &str,
    ) -> anyhow::Result<Option<Vec<f32>>> {
        if paper_id.trim().is_empty() {
            return Ok(None);
        }
        let fields = "embedding.specter_v2";
        let url = format!("{}/{}", S2_PAPER_URL, paper_id);
        let req = self.client.get(url).query(&[("fields", fields)]);
        let resp = self.apply_auth(req).send().await?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let details = resp.json::<PaperDetails>().await?;
        Ok(details.embedding.and_then(|e| e.specter_v2))
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

        let mut seed_ids = Vec::new();
        let mut papers = Vec::new();
        for paper in parsed.data {
            if let Some(pid) = paper.paper_id.clone().filter(|v| !v.trim().is_empty()) {
                seed_ids.push(pid);
            }
            if let Some(mapped) = Self::map_search_paper(paper) {
                papers.push(mapped);
            }
        }

        let max_expand = Self::citation_expansion_limit(max_results).min(max_results.saturating_sub(papers.len()));
        if max_expand > 0 && !seed_ids.is_empty() {
            if let Ok(extra) = self.expand_citation_graph(&seed_ids, max_expand).await {
                papers.extend(extra);
            }
        }

        // final de-dup by DOI+PMID+title
        let mut seen = std::collections::HashSet::new();
        papers.retain(|p| {
            let key = format!(
                "{}|{}|{}",
                p.doi.clone().unwrap_or_default().to_lowercase(),
                p.pmid.clone().unwrap_or_default().to_lowercase(),
                p.title.to_lowercase()
            );
            seen.insert(key)
        });
        papers.truncate(max_results);
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
        let fields = "openAccessPdf,embedding.specter_v2";
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
