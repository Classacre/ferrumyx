//! arXiv Atom API client.
//!
//! Endpoint: http://export.arxiv.org/api/query

use async_trait::async_trait;
use chrono::NaiveDate;
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::Client;
use tracing::{debug, instrument, warn};

use super::LiteratureSource;
use crate::models::{Author, IngestionSource, PaperMetadata};

const ARXIV_API_URL: &str = "http://export.arxiv.org/api/query";

#[derive(Debug, Default)]
struct ArxivEntry {
    id: Option<String>,
    title: Option<String>,
    summary: Option<String>,
    published: Option<String>,
    doi: Option<String>,
    authors: Vec<String>,
}

pub struct ArxivClient {
    client: Client,
}

impl ArxivClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl Default for ArxivClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LiteratureSource for ArxivClient {
    #[instrument(skip(self))]
    async fn search(&self, query: &str, max_results: usize) -> anyhow::Result<Vec<PaperMetadata>> {
        let cap = max_results.clamp(1, 200);
        let search_query = format!("all:{}", query.replace(' ', "+"));

        let xml = self
            .client
            .get(ARXIV_API_URL)
            .query(&[
                ("search_query", search_query.as_str()),
                ("start", "0"),
                ("max_results", &cap.to_string()),
                ("sortBy", "submittedDate"),
                ("sortOrder", "descending"),
            ])
            .send()
            .await?
            .text()
            .await?;

        parse_arxiv_atom(&xml)
    }

    async fn fetch_full_text(&self, paper_id: &str) -> anyhow::Result<Option<String>> {
        if paper_id.trim().is_empty() {
            return Ok(None);
        }
        let id = paper_id.trim().trim_start_matches("http://arxiv.org/abs/");
        Ok(Some(format!("https://arxiv.org/pdf/{id}.pdf")))
    }
}

fn parse_arxiv_atom(xml: &str) -> anyhow::Result<Vec<PaperMetadata>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut papers = Vec::new();
    let mut current: Option<ArxivEntry> = None;
    let mut current_tag = String::new();
    let mut in_entry = false;
    let mut in_author = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                current_tag = tag.clone();
                match tag.as_str() {
                    "entry" => {
                        in_entry = true;
                        current = Some(ArxivEntry::default());
                    }
                    "author" => in_author = true,
                    _ => {}
                }
            }
            Ok(Event::Text(t)) => {
                if !in_entry {
                    buf.clear();
                    continue;
                }
                let text = t.unescape().map(|v| v.to_string()).unwrap_or_default();
                if text.trim().is_empty() {
                    buf.clear();
                    continue;
                }
                if let Some(ref mut entry) = current {
                    match current_tag.as_str() {
                        "id" => entry.id = Some(text),
                        "title" => entry.title = Some(text),
                        "summary" => entry.summary = Some(text),
                        "published" => entry.published = Some(text),
                        "doi" => entry.doi = Some(text),
                        "name" if in_author => entry.authors.push(text),
                        _ => {}
                    }
                }
            }
            Ok(Event::End(e)) => {
                let tag = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                match tag.as_str() {
                    "author" => in_author = false,
                    "entry" => {
                        in_entry = false;
                        if let Some(entry) = current.take() {
                            let title = entry.title.unwrap_or_default().trim().to_string();
                            if title.is_empty() {
                                warn!("Skipping arXiv entry with empty title");
                                buf.clear();
                                continue;
                            }
                            let pub_date = entry
                                .published
                                .as_deref()
                                .and_then(|d| d.get(0..10))
                                .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
                            let full_text_url = entry.id.as_ref().and_then(|id| {
                                let clean = id.trim().trim_start_matches("http://arxiv.org/abs/");
                                if clean.is_empty() {
                                    None
                                } else {
                                    Some(format!("https://arxiv.org/pdf/{clean}.pdf"))
                                }
                            });
                            let authors = entry
                                .authors
                                .into_iter()
                                .map(|name| Author {
                                    name,
                                    affiliation: None,
                                    orcid: None,
                                })
                                .collect::<Vec<_>>();

                            papers.push(PaperMetadata {
                                doi: entry.doi,
                                pmid: None,
                                pmcid: None,
                                title,
                                abstract_text: entry.summary.map(|s| s.trim().to_string()),
                                authors,
                                journal: Some("arXiv".to_string()),
                                pub_date,
                                source: IngestionSource::Arxiv,
                                open_access: true,
                                full_text_url,
                            });
                        }
                    }
                    _ => {}
                }
                current_tag.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                warn!("arXiv Atom parse warning: {}", e);
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    debug!(count = papers.len(), "arXiv search returned results");
    Ok(papers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_atom() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <entry>
    <id>http://arxiv.org/abs/2501.12345v1</id>
    <published>2025-01-21T18:00:00Z</published>
    <title>  KRAS Signaling In Tumors  </title>
    <summary>Paper summary text.</summary>
    <author><name>Jane Doe</name></author>
  </entry>
</feed>"#;
        let papers = parse_arxiv_atom(xml).unwrap();
        assert_eq!(papers.len(), 1);
        assert_eq!(papers[0].source.as_str(), "arxiv");
        assert!(papers[0]
            .full_text_url
            .as_deref()
            .unwrap_or_default()
            .contains("/pdf/2501.12345v1.pdf"));
    }
}
