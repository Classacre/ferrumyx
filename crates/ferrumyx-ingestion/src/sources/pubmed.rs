//! PubMed E-utilities client.
//! See ARCHITECTURE.md §2.1 (PubMed/NCBI E-utilities API)
//!
//! Endpoints used:
//!   esearch: https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi
//!   efetch:  https://eutils.ncbi.nlm.nih.gov/entrez/eutils/efetch.fcgi
//!   elink:   for PMC ID resolution

use async_trait::async_trait;
use quick_xml::events::Event;
use quick_xml::Reader;
use ferrumyx_common::sandbox::SandboxClient as Client;
use tracing::{debug, instrument, warn};

use crate::models::{Author, IngestionSource, PaperMetadata};
use super::LiteratureSource;

const ESEARCH_URL: &str = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi";
const EFETCH_URL:  &str = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/efetch.fcgi";

pub struct PubMedClient {
    client: Client,
    api_key: Option<String>,
}

impl PubMedClient {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: Client::new().unwrap(),
            api_key,
        }
    }

    fn base_params(&self) -> Vec<(&'static str, String)> {
        let mut params = vec![("retmode", "json".to_string())];
        if let Some(key) = &self.api_key {
            params.push(("api_key", key.clone()));
        }
        params
    }

    /// Search PubMed and return a list of PMIDs.
    #[instrument(skip(self))]
    async fn esearch(&self, query: &str, max: usize) -> anyhow::Result<Vec<String>> {
        let mut params = self.base_params();
        params.push(("db", "pubmed".to_string()));
        params.push(("term", query.to_string()));
        params.push(("retmax", max.to_string()));
        params.push(("usehistory", "n".to_string()));

        let resp: serde_json::Value = self.client
            .get(ESEARCH_URL)?
            .query(&params)
            .send()
            .await?
            .json()
            .await?;

        let ids = resp["esearchresult"]["idlist"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        debug!(?ids, "PubMed esearch returned PMIDs");
        Ok(ids)
    }

    /// Fetch PubMed XML for a list of PMIDs and parse into PaperMetadata.
    #[instrument(skip(self))]
    async fn efetch_abstracts(&self, pmids: &[String]) -> anyhow::Result<Vec<PaperMetadata>> {
        if pmids.is_empty() {
            return Ok(vec![]);
        }

        let mut params = vec![
            ("db", "pubmed".to_string()),
            ("id", pmids.join(",")),
            ("rettype", "abstract".to_string()),
            ("retmode", "xml".to_string()),
        ];
        if let Some(key) = &self.api_key {
            params.push(("api_key", key.clone()));
        }

        let xml = self.client
            .get(EFETCH_URL)?
            .query(&params)
            .send()
            .await?
            .text()
            .await?;

        parse_pubmed_xml(&xml)
    }
}

#[async_trait]
impl LiteratureSource for PubMedClient {
    async fn search(&self, query: &str, max_results: usize) -> anyhow::Result<Vec<PaperMetadata>> {
        let pmids = self.esearch(query, max_results).await?;
        self.efetch_abstracts(&pmids).await
    }

    async fn fetch_full_text(&self, pmcid: &str) -> anyhow::Result<Option<String>> {
        // Fetch PMC full-text XML
        let url = format!(
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/efetch.fcgi\
             ?db=pmc&id={}&rettype=xml&retmode=xml",
            pmcid
        );
        let xml = self.client.get(&url)?.send().await?.text().await?;
        if xml.trim().is_empty() || xml.contains("<error>") {
            return Ok(None);
        }
        Ok(Some(xml))
    }
}

/// Parse PubMed XML (efetch abstract mode) into PaperMetadata list.
/// Handles the <PubmedArticleSet><PubmedArticle> structure.
fn parse_pubmed_xml(xml: &str) -> anyhow::Result<Vec<PaperMetadata>> {
    let mut papers = Vec::new();
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    // State machine for XML parsing
    let mut current: Option<PaperMetadata> = None;
    let mut in_pmid       = false;
    let mut in_title      = false;
    let mut in_abstract   = false;
    let mut in_author     = false;
    let mut in_last_name  = false;
    let mut in_fore_name  = false;
    let mut in_journal    = false;
    let mut current_last  = String::new();
    let mut current_fore  = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name().as_ref() {
                    b"PubmedArticle" => {
                        current = Some(PaperMetadata {
                            doi: None, pmid: None, pmcid: None,
                            title: String::new(),
                            abstract_text: None,
                            authors: vec![],
                            journal: None,
                            pub_date: None,
                            source: IngestionSource::PubMed,
                            open_access: false,
                            full_text_url: None,
                        });
                    }
                    b"PMID"         => in_pmid = true,
                    b"ArticleTitle" => in_title = true,
                    b"AbstractText" => in_abstract = true,
                    b"Author"       => { in_author = true; current_last.clear(); current_fore.clear(); }
                    b"LastName"     => in_last_name = true,
                    b"ForeName"     => in_fore_name = true,
                    b"Title"        => in_journal = true,
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                if let Some(ref mut p) = current {
                    if in_pmid      { p.pmid = Some(text.clone()); }
                    if in_title     { p.title = text.clone(); }
                    if in_abstract  { p.abstract_text = Some(text.clone()); }
                    if in_last_name { current_last = text.clone(); }
                    if in_fore_name { current_fore = text.clone(); }
                    if in_journal   { p.journal = Some(text.clone()); }
                }
            }
            Ok(Event::End(ref e)) => {
                match e.name().as_ref() {
                    b"PMID"         => in_pmid = false,
                    b"ArticleTitle" => in_title = false,
                    b"AbstractText" => in_abstract = false,
                    b"LastName"     => in_last_name = false,
                    b"ForeName"     => in_fore_name = false,
                    b"Title"        => in_journal = false,
                    b"Author" => {
                        if in_author {
                            if let Some(ref mut p) = current {
                                let name = if current_fore.is_empty() {
                                    current_last.clone()
                                } else {
                                    format!("{} {}", current_fore, current_last)
                                };
                                p.authors.push(Author { name, affiliation: None, orcid: None });
                            }
                            in_author = false;
                        }
                    }
                    b"PubmedArticle" => {
                        if let Some(p) = current.take() {
                            if !p.title.is_empty() {
                                papers.push(p);
                            } else {
                                warn!("Skipping paper with empty title");
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                warn!("XML parse error: {}", e);
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(papers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_pubmed_xml() {
        let xml = r#"<?xml version="1.0"?>
<PubmedArticleSet>
  <PubmedArticle>
    <MedlineCitation>
      <PMID>12345678</PMID>
      <Article>
        <ArticleTitle>KRAS G12D in pancreatic cancer</ArticleTitle>
        <Abstract><AbstractText>Test abstract.</AbstractText></Abstract>
        <AuthorList>
          <Author><LastName>Smith</LastName><ForeName>John</ForeName></Author>
        </AuthorList>
        <Journal><Title>Nature</Title></Journal>
      </Article>
    </MedlineCitation>
  </PubmedArticle>
</PubmedArticleSet>"#;

        let papers = parse_pubmed_xml(xml).unwrap();
        assert_eq!(papers.len(), 1);
        assert_eq!(papers[0].pmid, Some("12345678".to_string()));
        assert_eq!(papers[0].title, "KRAS G12D in pancreatic cancer");
        assert_eq!(papers[0].authors[0].name, "John Smith");
    }
}
