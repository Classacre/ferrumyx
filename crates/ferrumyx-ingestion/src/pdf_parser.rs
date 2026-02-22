//! Ferrules-based PDF parsing with section detection.
//! Fast Rust-native alternative to Docling.
//! See ARCHITECTURE.md §2.5-2.7

use anyhow::Result;
use std::path::Path;
use uuid::Uuid;

use crate::chunker::{DocumentSection, ChunkerConfig, chunk_document};
use crate::models::{DocumentChunk, SectionType};

/// Parse a PDF file and extract structured sections.
pub fn parse_pdf_sections(pdf_path: &Path) -> Result<ParsedPdf> {
    use ferrules::pdf::parse_pdf;
    use ferrules::structure::extract_sections;

    // Parse PDF with Ferrules
    let parsed = parse_pdf(pdf_path)
        .map_err(|e| anyhow::anyhow!("Ferrules PDF parse error: {:?}", e))?;

    // Extract sections
    let sections = extract_sections(&parsed);

    // Convert to our format
    let doc_sections: Vec<DocumentSection> = sections
        .iter()
        .map(|s| DocumentSection {
            section_type: SectionType::from_heading(&s.title),
            heading: Some(s.title.clone()),
            text: s.content.clone(),
            page_number: s.page_number.map(|p| p as u32),
        })
        .collect();

    // Extract full text
    let full_text = parsed.text.clone();

    Ok(ParsedPdf {
        title: parsed.title.clone(),
        sections: doc_sections,
        full_text,
        page_count: parsed.page_count,
    })
}

/// Parse PDF and create chunks directly.
pub fn parse_pdf_to_chunks(
    pdf_path: &Path,
    paper_id: Uuid,
    config: Option<ChunkerConfig>,
) -> Result<Vec<DocumentChunk>> {
    let parsed = parse_pdf_sections(pdf_path)?;
    let config = config.unwrap_or_default();

    // Use existing chunker with section-aware splitting
    let chunks = chunk_document(
        &parsed.full_text,
        Some(&parsed.sections),
        config,
    );

    // Convert to DocumentChunk format
    Ok(chunks
        .into_iter()
        .enumerate()
        .map(|(i, c)| DocumentChunk {
            paper_id,
            chunk_index: i,
            section_type: c.section_type,
            section_heading: c.heading,
            content: c.text,
            page_number: c.page_number,
            token_count: c.token_count,
        })
        .collect())
}

/// Parsed PDF document.
#[derive(Debug, Clone)]
pub struct ParsedPdf {
    pub title: Option<String>,
    pub sections: Vec<DocumentSection>,
    pub full_text: String,
    pub page_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_type_inference() {
        assert_eq!(
            SectionType::from_heading("Introduction"),
            SectionType::Introduction
        );
        assert_eq!(
            SectionType::from_heading("Materials and Methods"),
            SectionType::Methods
        );
        assert_eq!(
            SectionType::from_heading("RESULTS"),
            SectionType::Results
        );
    }
}
