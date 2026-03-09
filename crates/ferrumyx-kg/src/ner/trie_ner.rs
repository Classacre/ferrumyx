//! Fast NER using Aho-Corasick trie for dictionary matching.

use std::collections::HashMap;
use aho_corasick::{AhoCorasick, MatchKind};
use super::entity_types::EntityType;
use super::hgnc::{HgncNormaliser, SymbolTier};
use super::cancer_normaliser::{CancerNormaliser, CancerPatternKind};
use super::hgvs::HgvsMutationNormaliser;
use tracing::{info, warn};

#[derive(Clone, Debug)]
struct PatternMeta {
    pub entity_type: EntityType,
    pub text: String,
    pub confidence: f32,
    pub requires_word_boundary: bool,
}

pub struct TrieNer {
    automaton: AhoCorasick,
    pattern_info: Vec<PatternMeta>,
    hgnc: HgncNormaliser,
    cancers: CancerNormaliser,
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
        info!("Building TrieNer with complete databases (HGNC + OncoTree)...");
        let mut patterns = Vec::new();
        let mut pattern_info = Vec::new();

        // 1. Genes from HGNC
        let hgnc = HgncNormaliser::from_download_blocking()?;
        for (sym, tier) in hgnc.all_patterns_with_tier() {
            let len = sym.len();
            let (confidence, req_word_bound) = match (tier, len) {
                (SymbolTier::Preferred, l) if l >= 4 => (0.97, false),
                (SymbolTier::Preferred, 2..=3) => (0.80, true),
                (SymbolTier::Preferred, 1) => (0.60, true),
                (_, l) if l >= 4 => (0.82, false),
                _ => (0.65, true),
            };

            patterns.push(sym.clone());
            pattern_info.push(PatternMeta {
                entity_type: EntityType::Gene,
                text: sym,
                confidence,
                requires_word_boundary: req_word_bound,
            });
        }

        // 2. Cancer Types from OncoTree
        let cancers = CancerNormaliser::from_download_blocking()?;
        for (name, kind) in cancers.all_patterns_with_kind() {
            let len = name.len();
            let confidence = match kind {
                CancerPatternKind::Code => 0.90,
                CancerPatternKind::Name if len >= 8 => 0.93,
                CancerPatternKind::Name => 0.75,
            };

            patterns.push(name.clone());
            pattern_info.push(PatternMeta {
                entity_type: EntityType::CancerType,
                text: name,
                confidence,
                requires_word_boundary: kind == CancerPatternKind::Code || len <= 4,
            });
        }

        // 3. Mutations
        let mutations = HgvsMutationNormaliser::new();
        for mut_p in mutations.all_patterns() {
            patterns.push(mut_p.clone());
            pattern_info.push(PatternMeta {
                entity_type: EntityType::Mutation,
                text: mut_p.clone(),
                confidence: 0.90,
                requires_word_boundary: true,
            });
        }

        if patterns.is_empty() {
            anyhow::bail!("No NER patterns loaded. Check database availability.");
        }

        let automaton = AhoCorasick::builder()
            .match_kind(MatchKind::LeftmostLongest)
            .ascii_case_insensitive(true)
            .build(&patterns)?;
            
        Ok(Self { automaton, pattern_info, hgnc, cancers })
    }

    pub fn hgnc(&self) -> &HgncNormaliser {
        &self.hgnc
    }

    pub fn cancers(&self) -> &CancerNormaliser {
        &self.cancers
    }

    pub fn stats(&self) -> NerStats {
        let mut stats = NerStats {
            gene_count: 0,
            disease_count: 0,
            chemical_count: 0,
            total_patterns: self.pattern_info.len(),
        };

        for meta in &self.pattern_info {
            match meta.entity_type {
                EntityType::Gene => stats.gene_count += 1,
                EntityType::Disease => stats.disease_count += 1,
                EntityType::Chemical => stats.chemical_count += 1,
                _ => {}
            }
        }
        stats
    }

    pub fn extract(&self, text: &str) -> Vec<ExtractedEntity> {
        let mut entities = Vec::new();
        for mat in self.automaton.find_iter(text) {
            let pattern_idx = mat.pattern().as_usize();
            let meta = &self.pattern_info[pattern_idx];
            
            // Apply minimum confidence threshold
            if meta.confidence < 0.75 {
                continue;
            }

            let start = mat.start();
            let end = mat.end();

            // Word-boundary check for short / ambiguous symbols
            if meta.requires_word_boundary {
                let prev_char = if start > 0 { text[..start].chars().next_back() } else { None };
                let next_char = text[end..].chars().next();
                
                if prev_char.map_or(false, |c| c.is_alphabetic()) || next_char.map_or(false, |c| c.is_alphabetic()) {
                    continue; // Skip if part of a larger word
                }
            }

            entities.push(ExtractedEntity {
                text: text[start..end].to_string(),
                label: meta.entity_type,
                start,
                end,
                confidence: meta.confidence,
            });
        }
        entities
    }
}

pub struct NerStats {
    pub gene_count: usize,
    pub disease_count: usize,
    pub chemical_count: usize,
    pub total_patterns: usize,
}
