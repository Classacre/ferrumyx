//! Fast NER using Aho-Corasick trie for dictionary matching.

use std::collections::HashMap;
use aho_corasick::{AhoCorasick, MatchKind};
use super::entity_types::EntityType;
use tracing::{info, warn};

pub struct TrieNer {
    automaton: AhoCorasick,
    pattern_info: Vec<(EntityType, String, String)>,
}

#[derive(Debug, Clone)]
pub struct ExtractedEntity {
    pub text: String,
    pub label: EntityType,
    pub start: usize,
    pub end: usize,
    pub confidence: f32,
}

impl TrieNer {
    pub fn with_complete_databases() -> anyhow::Result<Self> {
        // Logic moved from ferrumyx-ner
        let mut patterns = Vec::new();
        let mut pattern_info = Vec::new();
        // ... loading logic ...
        let automaton = AhoCorasick::builder()
            .match_kind(MatchKind::LeftmostLongest)
            .build::<&[String], &String>(patterns.as_slice())?;
        Ok(Self { automaton, pattern_info })
    }

    pub fn with_embedded_subset() -> anyhow::Result<Self> {
        Self::with_complete_databases()
    }

    pub fn stats(&self) -> NerStats {
        NerStats { total_patterns: self.pattern_info.len() }
    }

    pub fn extract(&self, text: &str) -> Vec<ExtractedEntity> {
        let mut entities = Vec::new();
        for mat in self.automaton.find_iter(text) {
            let pattern_idx = mat.pattern().as_usize();
            let (entity_type, _, _) = &self.pattern_info[pattern_idx];
            entities.push(ExtractedEntity {
                text: text[mat.start()..mat.end()].to_string(),
                label: *entity_type,
                start: mat.start(),
                end: mat.end(),
                confidence: 0.95,
            });
        }
        entities
    }
}

pub struct NerStats {
    pub total_patterns: usize,
}
