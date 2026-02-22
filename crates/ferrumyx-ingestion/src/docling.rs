//! Docling integration for PDF/XML parsing with section-aware chunking.
//! See ARCHITECTURE.md §2.5-2.7

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use anyhow::Result;
use uuid::Uuid;

use crate::chunker::{DocumentSection, ChunkerConfig, chunk_document};
use crate::models::{DocumentChunk, SectionType};

const DOCLING_DEFAULT_URL: &str = "http://localhost:8003";

/// Docling service client for document parsing.
pub struct DoclingClient {
    base_url: String,
    client: Client,
}

impl DoclingClient {
    pub fn new(base_url: Option<&str>) -> Self {
        Self {
            base_url: base_url.unwrap_or(DOCLING_DEFAULT_URL).to_string(),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Check if Docling service is healthy.
    pub async fn health_check(&self) -> Result<bool> {
        let resp = self.client
            .get(format!("{}/health", self.base_url))
            .send()
            .await?;
        
        Ok(resp.status().is_success())
    }

    /// Parse a PDF file and return structured document.
    pub async fn parse_pdf(&self, pdf_path: &Path) -> Result<ParsedDocument> {
        let file_bytes = fs::read(pdf_path).await?;
        let filename = pdf_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document.pdf");

        let part = reqwest::multipart::Part::bytes(file_bytes)
            .file_name(filename.to_string())
            .mime_str("application/pdf")?;

        let form = reqwest::multipart::Form::new().part("file", part);

        let resp = self.client
            .post(format!("{}/parse", self.base_url))
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let error = resp.text().await?;
            anyhow::bail!("Docling parse failed: {}", error);
        }

        let doc: ParsedDocument = resp.json().await?;
        Ok(doc)
    }

    /// Parse a document from URL.
    pub async fn parse_url(&self, url: &str) -> Result<ParsedDocument> {
        let resp = self.client
            .post(format!("{}/parse-url?url={}", self.base_url, url))
            .send()
            .await?;

        if !resp.status().is_success() {
            let error = resp.text().await?;
            anyhow::bail!("Docling parse URL failed: {}", error);
        }

        let doc: ParsedDocument = resp.json().await?;
        Ok(doc)
    }

    /// Chunk text using Docling's hybrid chunker.
    pub async fn chunk_text(&self, text: &str, max_tokens: usize) -> Result<Vec<ChunkResult>> {
        #[derive(Serialize)]
        struct ChunkRequest {
            text: String,
            max_tokens: usize,
            merge_peers: bool,
        }

        let resp = self.client
            .post(format!("{}/chunk", self.base_url))
            .json(&ChunkRequest {
                text: text.to_string(),
                max_tokens,
                merge_peers: true,
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            let error = resp.text().await?;
            anyhow::bail!("Docling chunk failed: {}", error);
        }

        let chunks: Vec<ChunkResult> = resp.json().await?;
        Ok(chunks)
    }
}

/// Parsed document from Docling.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ParsedDocument {
    pub filename: String,
    pub title: Option<String>,
    pub sections: Vec<DoclingSection>,
    pub chunks: Vec<DoclingChunk>,
    pub full_text: String,
    pub metadata: DocumentMetadata,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DoclingSection {
    pub title: String,
    pub level: i32,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DoclingChunk {
    pub chunk_id: i32,
    pub text: String,
    pub contextualized: String,
    pub section: Option<String>,
    pub page: Option<i32>,
    pub token_count: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChunkResult {
    pub chunk_id: i32,
    pub text: String,
    pub contextualized: String,
    pub section: Option<String>,
    pub page: Option<i32>,
    pub token_count: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DocumentMetadata {
    pub page_count: Option<i32>,
    pub has_tables: bool,
    pub has_figures: bool,
}

impl ParsedDocument {
    /// Convert Docling sections to internal DocumentSection format.
    pub fn to_sections(&self) -> Vec<DocumentSection> {
        self.sections.iter().map(|s| {
            DocumentSection {
                section_type: infer_section_type(&s.title),
                heading: Some(s.title.clone()),
                text: s.text.clone(),
                page_number: None,
            }
        }).collect()
    }

    /// Convert Docling chunks directly to DocumentChunk format.
    pub fn to_chunks(&self, paper_id: Uuid) -> Vec<DocumentChunk> {
        self.chunks.iter().map(|c| {
            DocumentChunk {
                paper_id,
                chunk_index: c.chunk_id as usize,
                section_type: c.section
                    .as_ref()
                    .map(|s| infer_section_type(s))
                    .unwrap_or(SectionType::Other),
                section_heading: c.section.clone(),
                content: c.contextualized.clone(),
                page_number: c.page.map(|p| p as u32),
                token_count: c.token_count as usize,
            }
        }).collect()
    }
}

/// Infer section type from heading text.
fn infer_section_type(heading: &str) -> SectionType {
    SectionType::from_heading(heading)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_section_type() {
        assert_eq!(infer_section_type("Abstract"), SectionType::Abstract);
        assert_eq!(infer_section_type("INTRODUCTION"), SectionType::Introduction);
        assert_eq!(infer_section_type("Materials and Methods"), SectionType::Methods);
        assert_eq!(infer_section_type("Results"), SectionType::Results);
        assert_eq!(infer_section_type("Discussion"), SectionType::Discussion);
        assert_eq!(infer_section_type("References"), SectionType::References);
        assert_eq!(infer_section_type("Figure 1"), SectionType::FigureCaption);
        assert_eq!(infer_section_type("Unknown Section"), SectionType::Body);
    }
}
