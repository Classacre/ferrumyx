//! Section-aware document chunker.
//! See ARCHITECTURE.md §2.7

use crate::models::{DocumentChunk, SectionType};
use uuid::Uuid;

/// Configuration for the chunker.
#[derive(Debug, Clone)]
pub struct ChunkerConfig {
    /// Maximum tokens per chunk (BiomedBERT limit: 512 including special tokens).
    pub max_tokens: usize,
    /// Token overlap between consecutive chunks.
    pub overlap_tokens: usize,
}

impl Default for ChunkerConfig {
    fn default() -> Self {
        Self {
            max_tokens: 510,    // 512 - 2 for [CLS] and [SEP]
            overlap_tokens: 64,
        }
    }
}

/// A section of a parsed document.
#[derive(Debug, Clone)]
pub struct DocumentSection {
    pub section_type: SectionType,
    pub heading: Option<String>,
    pub text: String,
    pub page_number: Option<u32>,
}

/// Chunk a document into retrieval-optimised units.
/// See ARCHITECTURE.md §2.7 for chunking rules.
pub fn chunk_document(
    paper_id: Uuid,
    sections: Vec<DocumentSection>,
    config: &ChunkerConfig,
) -> Vec<DocumentChunk> {
    let mut chunks = Vec::new();
    let mut chunk_index = 0;

    for section in sections {
        let section_chunks = chunk_section(
            paper_id,
            &section,
            &mut chunk_index,
            config,
        );
        chunks.extend(section_chunks);
    }

    chunks
}

fn chunk_section(
    paper_id: Uuid,
    section: &DocumentSection,
    chunk_index: &mut usize,
    config: &ChunkerConfig,
) -> Vec<DocumentChunk> {
    let mut chunks = Vec::new();

    // Abstract and Figure Captions: always a single chunk
    if matches!(section.section_type, SectionType::Abstract | SectionType::FigureCaption) {
        chunks.push(DocumentChunk {
            paper_id,
            chunk_index: *chunk_index,
            section_type: section.section_type.clone(),
            section_heading: section.heading.clone(),
            content: section.text.clone(),
            page_number: section.page_number,
            token_count: estimate_tokens(&section.text),
        });
        *chunk_index += 1;
        return chunks;
    }

    // All other sections: sliding window with overlap
    let words: Vec<&str> = section.text.split_whitespace().collect();
    if words.is_empty() {
        return chunks;
    }

    // Approximate: 1 token ≈ 0.75 words (WordPiece tokenization)
    let words_per_chunk = (config.max_tokens as f32 * 0.75) as usize;
    let overlap_words   = (config.overlap_tokens as f32 * 0.75) as usize;

    let mut start = 0;
    while start < words.len() {
        let end = (start + words_per_chunk).min(words.len());
        let content = words[start..end].join(" ");
        let token_count = estimate_tokens(&content);

        chunks.push(DocumentChunk {
            paper_id,
            chunk_index: *chunk_index,
            section_type: section.section_type.clone(),
            section_heading: section.heading.clone(),
            content,
            page_number: section.page_number,
            token_count,
        });
        *chunk_index += 1;

        if end == words.len() {
            break;
        }
        // Advance by chunk size minus overlap
        start += words_per_chunk.saturating_sub(overlap_words);
    }

    chunks
}

/// Rough token estimation: words / 0.75 (WordPiece averages ~1.3 tokens/word).
pub fn estimate_tokens(text: &str) -> usize {
    let words = text.split_whitespace().count();
    ((words as f32) / 0.75).ceil() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abstract_is_single_chunk() {
        let paper_id = Uuid::new_v4();
        let sections = vec![DocumentSection {
            section_type: SectionType::Abstract,
            heading: Some("Abstract".to_string()),
            text: "This is a short abstract about KRAS G12D in pancreatic cancer.".to_string(),
            page_number: Some(1),
        }];
        let chunks = chunk_document(paper_id, sections, &ChunkerConfig::default());
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].section_type, SectionType::Abstract);
    }

    #[test]
    fn test_long_section_splits_into_multiple_chunks() {
        let paper_id = Uuid::new_v4();
        // Create a very long methods section
        let text = "word ".repeat(2000);
        let sections = vec![DocumentSection {
            section_type: SectionType::Methods,
            heading: Some("Methods".to_string()),
            text,
            page_number: Some(3),
        }];
        let config = ChunkerConfig { max_tokens: 100, overlap_tokens: 10 };
        let chunks = chunk_document(paper_id, sections, &config);
        assert!(chunks.len() > 1, "Long section should produce multiple chunks");
    }
}
