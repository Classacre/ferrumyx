//! Ferrules-based PDF parsing with section detection.
//! Fast Rust-native alternative to Docling.
//! See ARCHITECTURE.md §2.5-2.7

use anyhow::Result;
use std::path::Path;
use uuid::Uuid;

use crate::chunker::{chunk_document, ChunkerConfig, DocumentSection};
use crate::models::SectionType;

/// Parse a PDF file and extract structured sections.
pub fn parse_pdf_sections(pdf_path: &Path) -> Result<ParsedPdf> {
    use lopdf::Document as PdfDoc;

    let pdf = PdfDoc::load(pdf_path)?;

    // Extract text from all pages
    let mut full_text = String::new();
    let mut pages: Vec<(u32, String)> = Vec::new();

    for (page_num, page_id) in pdf.get_pages() {
        let mut page_text = pdf
            .extract_text(&[page_num])
            .ok()
            .map(|txt| normalize_whitespace(&txt))
            .unwrap_or_default();

        if page_text.trim().is_empty() {
            if let Ok(content) = pdf.get_page_content(page_id) {
                page_text = extract_from_content_stream(&content);
            }
        }

        pages.push((page_num, page_text.clone()));
        if !page_text.trim().is_empty() {
            full_text.push_str(&page_text);
            full_text.push('\n');
        }
    }

    // Section detection via heuristics
    let mut sections = detect_sections(&full_text, &pages);
    if sections.is_empty() {
        sections = fallback_sections_from_pages(&pages);
    }
    if sections.is_empty() && !full_text.trim().is_empty() {
        sections.push(DocumentSection {
            section_type: SectionType::Introduction,
            heading: Some("Body".to_string()),
            text: full_text.clone(),
            page_number: Some(1),
        });
    }

    Ok(ParsedPdf {
        title: extract_title(&full_text),
        sections,
        full_text,
        page_count: pages.len(),
    })
}

fn extract_from_content_stream(content: &[u8]) -> String {
    let mut out = String::new();
    let mut in_literal = false;
    let mut escaped = false;

    for ch in String::from_utf8_lossy(content).chars() {
        match ch {
            '\\' if in_literal && !escaped => escaped = true,
            '(' if !in_literal && !escaped => in_literal = true,
            ')' if in_literal && !escaped => {
                in_literal = false;
                out.push(' ');
            }
            _ if in_literal => {
                if ch.is_control() {
                    out.push(' ');
                } else {
                    out.push(ch);
                }
                escaped = false;
            }
            _ => escaped = false,
        }
    }
    normalize_whitespace(&out)
}

fn fallback_sections_from_pages(pages: &[(u32, String)]) -> Vec<DocumentSection> {
    let mut sections = Vec::new();
    for (page_num, page_text) in pages {
        let clean = normalize_whitespace(page_text);
        if clean.len() < 80 {
            continue;
        }
        sections.push(DocumentSection {
            section_type: SectionType::Introduction,
            heading: Some(format!("Page {}", page_num)),
            text: clean,
            page_number: Some(*page_num),
        });
    }
    sections
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
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
    let markers = [
        "\nintroduction",
        "\nmethods",
        "\nresults",
        "\ndiscussion",
        "\nconclusion",
        "\nreferences",
    ];

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
    let chunks = chunk_document(paper_id, parsed.sections, &config);

    Ok(chunks)
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
