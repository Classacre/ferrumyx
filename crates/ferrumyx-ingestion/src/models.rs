//! Data models for ingestion pipeline.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a discovered paper before full ingestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperMetadata {
    pub doi: Option<String>,
    pub pmid: Option<String>,
    pub pmcid: Option<String>,
    pub title: String,
    pub abstract_text: Option<String>,
    pub authors: Vec<Author>,
    pub journal: Option<String>,
    pub pub_date: Option<NaiveDate>,
    pub source: IngestionSource,
    pub open_access: bool,
    pub full_text_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    pub affiliation: Option<String>,
    pub orcid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IngestionSource {
    PubMed,
    EuropePmc,
    BioRxiv,
    MedRxiv,
    Arxiv,
    ClinicalTrials,
    CrossRef,
    SemanticScholar,
}

impl IngestionSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            IngestionSource::PubMed          => "pubmed",
            IngestionSource::EuropePmc       => "europepmc",
            IngestionSource::BioRxiv         => "biorxiv",
            IngestionSource::MedRxiv         => "medrxiv",
            IngestionSource::Arxiv           => "arxiv",
            IngestionSource::ClinicalTrials  => "clinicaltrials",
            IngestionSource::CrossRef        => "crossref",
            IngestionSource::SemanticScholar => "semanticscholar",
        }
    }
}

/// Section types mapped from PMC XML sec-type or inferred from headings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SectionType {
    Abstract,
    Introduction,
    Methods,
    Results,
    Discussion,
    Conclusion,
    References,
    Table,
    FigureCaption,
    SupplementaryMethods,
    Other,
}

impl SectionType {
    /// Infer section type from a heading string.
    /// See ARCHITECTURE.md §2.6 for the inference rules.
    pub fn from_heading(heading: &str) -> Self {
        let h = heading.to_lowercase();
        if h.contains("abstract")                              { SectionType::Abstract }
        else if h.contains("introduction") || h.starts_with("background") { SectionType::Introduction }
        else if h.contains("method") || h.contains("material") { SectionType::Methods }
        else if h.contains("result")                           { SectionType::Results }
        else if h.contains("discussion")                       { SectionType::Discussion }
        else if h.contains("conclusion")                       { SectionType::Conclusion }
        else if h.contains("supplement")                       { SectionType::SupplementaryMethods }
        else                                                   { SectionType::Other }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SectionType::Abstract              => "abstract",
            SectionType::Introduction          => "introduction",
            SectionType::Methods               => "methods",
            SectionType::Results               => "results",
            SectionType::Discussion            => "discussion",
            SectionType::Conclusion            => "conclusion",
            SectionType::References            => "references",
            SectionType::Table                 => "table",
            SectionType::FigureCaption         => "figure_caption",
            SectionType::SupplementaryMethods  => "supplementary_methods",
            SectionType::Other                 => "other",
        }
    }
}

/// A parsed document chunk ready for embedding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    pub paper_id: Uuid,
    pub chunk_id: Uuid,  // Unique ID for this chunk
    pub chunk_index: usize,
    pub section_type: SectionType,
    pub section_heading: Option<String>,
    pub content: String,
    pub page_number: Option<u32>,
    pub token_count: usize,
}
