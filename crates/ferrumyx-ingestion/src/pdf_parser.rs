//! Ferrules-based PDF parsing with section detection.
//! Fast Rust-native alternative to Docling.
//! See ARCHITECTURE.md §2.5-2.7

use anyhow::Result;
use std::path::Path;
use uuid::Uuid;

use crate::chunker::{DocumentSection, ChunkerConfig, chunk_document};
use crate::models::SectionType;

/// Parse a PDF file and extract structured sections.
pub fn parse_pdf_sections(pdf_path: &Path) -> Result<ParsedPdf> {
    // Ferrules API: use lopdf-based extraction
    use lopdf::Document as PdfDoc;

    let pdf = PdfDoc::load(pdf_path)?;
    
    // Extract text from all pages
    let mut full_text = String::new();
    let mut pages: Vec<(u32, String)> = Vec::new();

    for (page_num, page) in pdf.get_pages() {
        let mut page_text = String::new();
        if let Ok(content) = pdf.get_page_content(page) {
            // Simple text extraction (basic - Ferrules provides better)
            for obj in content.objects.values() {
                if let lopdf::Object::String(bytes, _) = obj {
                    if let Ok(text) = String::from_utf8(bytes.clone()) {
                        page_text.push_str(&text);
                        page_text.push(' ');
                    }
                }
            }
        }
        pages.push((page_num, page_text.clone()));
        full_text.push_str(&page_text);
        full_text.push('\n');
    }

    // Section detection via heuristics
    let sections = detect_sections(&full_text, &pages);

    Ok(ParsedPdf {
        title: extract_title(&full_text),
        sections,
        full_text,
        page_count: pages.len(),
    })
}

/// Detect sections using keyword heuristics.
fn detect_sections(text: &str, pages: &[(u32, String)]) -> Vec<DocumentSection> {
    let section_markers = [
        ("Abstract", SectionType::Abstract),
        ("Introduction", SectionType::Introduction),
        ("Methods", SectionType::Methods),
        ("Materials and Methods", SectionType::Methods),
        ("Results", SectionType::Results),
        ("Discussion", SectionType::Discussion),
        ("Conclusion", SectionType::Conclusion),
        ("References", SectionType::References),
    ];

    let mut sections = Vec::new();
    let lower_text = text.to_lowercase();

    for (marker, stype) in section_markers {
        if let Some(pos) = lower_text.find(&marker.to_lowercase()) {
            // Extract text until next section or end
            let start = pos;
            let end = find_next_section(&lower_text, pos + marker.len());
            let section_text = text[start..end].to_string();

            sections.push(DocumentSection {
                section_type: stype,
                heading: Some(marker.to_string()),
                text: section_text,
                page_number: find_page_number(pages, pos),
            });
        }
    }

    sections
}

fn find_next_section(text: &str, after: usize) -> usize {
    let remaining = &text[after..];
    let markers = ["\nintroduction", "\nmethods", "\nresults", "\ndiscussion", "\nconclusion", "\nreferences"];
    
    let mut earliest = text.len();
    for marker in markers {
        if let Some(pos) = remaining.find(marker) {
            earliest = earliest.min(after + pos);
        }
    }
    earliest
}

fn find_page_number(pages: &[(u32, String)], char_pos: usize) -> Option<u32> {
    let mut count = 0;
    for (page_num, page_text) in pages {
        count += page_text.len();
        if count > char_pos {
            return Some(*page_num);
        }
    }
    None
}

fn extract_title(text: &str) -> Option<String> {
    // First non-empty line is usually title
    text.lines()
        .find(|l| l.trim().len() > 10)
        .map(|s| s.trim().to_string())
}

/// Parse PDF and create chunks directly.
pub fn parse_pdf_to_chunks(
    pdf_path: &Path,
    paper_id: Uuid,
    config: Option<ChunkerConfig>,
) -> Result<Vec<crate::models::DocumentChunk>> {
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
        .map(|(i, c)| crate::models::DocumentChunk {
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
    fn test_section_detection() {
        let text = "Introduction\nThis paper studies KRAS.\nMethods\nWe used CRISPR.\nResults\nWe found mutations.";
        let pages = vec![(1, text.to_string())];
        let sections = detect_sections(text, &pages);
        assert!(!sections.is_empty());
    }
}
