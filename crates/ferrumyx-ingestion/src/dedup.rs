//! Deduplication logic for ingested papers.
//! See ARCHITECTURE.md §2.10

use crate::models::PaperMetadata;

/// Result of a deduplication check.
#[derive(Debug)]
pub enum DedupResult {
    /// Paper is new — proceed with ingestion.
    New,
    /// Paper is a duplicate of an existing record (by DOI).
    DuplicateDoi(String),
    /// Paper is a probable duplicate based on abstract similarity.
    ProbableDuplicate { method: String, similarity: f64 },
}

/// Check if a paper is a duplicate using the staged algorithm.
/// Stage 1: DOI exact match (primary)
/// Stage 2: Abstract SimHash (secondary) — placeholder for now
/// Stage 3: Fuzzy title match (tertiary) — placeholder for now
///
/// In production, stages 2 and 3 require database lookups.
/// This module provides the pure computation logic.
pub fn check_duplicate(
    incoming: &PaperMetadata,
    existing_dois: &[String],
) -> DedupResult {
    // Stage 1: DOI exact match
    if let Some(doi) = &incoming.doi {
        if existing_dois.contains(doi) {
            return DedupResult::DuplicateDoi(doi.clone());
        }
    }

    // Stage 2 & 3: Require DB access — handled at repository layer.
    // Return New here; DB layer applies SimHash and fuzzy title checks.
    DedupResult::New
}

/// Compute a simple 64-bit SimHash of text for approximate deduplication.
/// Production implementation should use a proper SimHash library.
/// This is a simplified version for bootstrapping.
pub fn simhash(text: &str) -> u64 {
    let normalised = text.to_lowercase();
    let words: Vec<&str> = normalised.split_whitespace().collect();

    let mut v: [i64; 64] = [0; 64];

    for word in &words {
        // Skip common stop words
        if STOP_WORDS.contains(word) { continue; }

        let hash = fnv64(word.as_bytes());
        for i in 0..64u64 {
            if (hash >> i) & 1 == 1 {
                v[i as usize] += 1;
            } else {
                v[i as usize] -= 1;
            }
        }
    }

    let mut fingerprint: u64 = 0;
    for i in 0..64usize {
        if v[i] > 0 {
            fingerprint |= 1u64 << i;
        }
    }
    fingerprint
}

/// FNV-1a 64-bit hash.
fn fnv64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 14695981039346656037;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

/// Hamming distance between two 64-bit integers.
pub fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// Stop words to exclude from SimHash computation.
const STOP_WORDS: &[&str] = &[
    "the", "a", "an", "and", "or", "in", "of", "to", "is", "was",
    "for", "on", "with", "this", "that", "are", "were", "be", "been",
    "by", "from", "we", "our", "their", "which", "also",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_texts_same_simhash() {
        let text = "KRAS G12D mutation drives pancreatic ductal adenocarcinoma";
        assert_eq!(simhash(text), simhash(text));
    }

    #[test]
    fn test_similar_texts_small_hamming() {
        let t1 = "KRAS G12D mutation drives pancreatic ductal adenocarcinoma";
        let t2 = "KRAS G12D mutation drives pancreatic cancer adenocarcinoma";
        let dist = hamming_distance(simhash(t1), simhash(t2));
        // Similar texts should have small hamming distance (≤ 12)
        assert!(dist <= 12, "Hamming distance was {dist}");
    }

    #[test]
    fn test_different_texts_large_hamming() {
        let t1 = "KRAS G12D mutation drives pancreatic cancer";
        let t2 = "Deep learning for protein structure prediction with AlphaFold";
        let dist = hamming_distance(simhash(t1), simhash(t2));
        assert!(dist > 10, "Expected large hamming distance, got {dist}");
    }
}
