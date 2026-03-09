//! Cancer type normalisation logic using MSKCC OncoTree.

use std::collections::HashMap;
use anyhow::{Context, Result};
use tracing::info;

/// Whether a cancer pattern is a short OncoTree code or a full name.
/// Used by the NER trie to assign different confidence levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancerPatternKind {
    /// Short OncoTree code, e.g. "LUAD", "COAD" — controlled vocab, unambiguous
    Code,
    /// Full or partial name, e.g. "lung adenocarcinoma" — longer but could overlap
    Name,
}

/// A canonical OncoTree tumor type record.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OncoTreeRecord {
    pub code: String,
    pub name: String,
    pub main_type: String,
    pub tissue: String,
    pub parent: Option<String>,
}

/// OncoTree stable TSV URL.
const ONCOTREE_JSON_URL: &str = "https://oncotree.info/api/tumorTypes?version=oncotree_latest_stable";

pub struct CancerNormaliser {
    /// Mapping from lowercase name/synonym to canonical OncoTree code.
    lookup: HashMap<String, String>,
    /// Mapping from lowercase pattern to its kind (code vs name).
    pattern_kinds: HashMap<String, CancerPatternKind>,
    /// Mapping from code to full record.
    records: HashMap<String, OncoTreeRecord>,
}

impl CancerNormaliser {
    /// Build from OncoTree download.
    pub async fn from_download() -> Result<Self> {
        info!("Downloading OncoTree dataset from {}", ONCOTREE_JSON_URL);
        let resp = reqwest::get(ONCOTREE_JSON_URL)
            .await
            .context("OncoTree download failed")?
            .json::<serde_json::Value>()
            .await
            .context("OncoTree JSON parse failed")?;
        
        Self::from_json(&resp)
    }

    /// Synchronous version for use in spawn_blocking.
    pub fn from_download_blocking() -> Result<Self> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(Self::from_download())
    }

    /// Parse OncoTree JSON (handles both flat list and nested tree).
    pub fn from_json(json: &serde_json::Value) -> Result<Self> {
        let mut lookup = HashMap::new();
        let mut pattern_kinds = HashMap::new();
        let mut records = HashMap::new();

        fn traverse(
            node: &serde_json::Value,
            lookup: &mut HashMap<String, String>,
            pattern_kinds: &mut HashMap<String, CancerPatternKind>,
            records: &mut HashMap<String, OncoTreeRecord>,
            parent: Option<String>,
        ) {
            if let Some(obj) = node.as_object() {
                let code = obj.get("code").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let main_type = obj.get("mainType").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let tissue = obj.get("tissue").and_then(|v| v.as_str()).unwrap_or("").to_string();

                if !code.is_empty() {
                    let record = OncoTreeRecord {
                        code: code.clone(),
                        name: name.clone(),
                        main_type,
                        tissue,
                        parent: parent.clone(),
                    };

                    records.insert(code.clone(), record);

                    // Register the OncoTree code as a Code-kind pattern
                    let code_lc = code.to_lowercase();
                    lookup.insert(code_lc.clone(), code.clone());
                    pattern_kinds.insert(code_lc, CancerPatternKind::Code);

                    // Register the full name as a Name-kind pattern
                    if !name.is_empty() {
                        let name_lc = name.to_lowercase();
                        lookup.insert(name_lc.clone(), code.clone());
                        pattern_kinds.insert(name_lc, CancerPatternKind::Name);
                    }
                }

                if let Some(children) = obj.get("children").and_then(|v| v.as_object()) {
                    for child in children.values() {
                        traverse(child, lookup, pattern_kinds, records, Some(code.clone()));
                    }
                }
            } else if let Some(arr) = node.as_array() {
                for item in arr {
                    traverse(item, lookup, pattern_kinds, records, parent.clone());
                }
            }
        }

        traverse(json, &mut lookup, &mut pattern_kinds, &mut records, None);

        info!("OncoTree normaliser built: {} codes, {} lookup entries", records.len(), lookup.len());
        Ok(Self { lookup, pattern_kinds, records })
    }

    /// Normalise a cancerous entity name to its canonical OncoTree code.
    pub fn normalise(&self, name: &str) -> Option<String> {
        let name_lower = name.to_lowercase();
        
        // Exact match
        if let Some(code) = self.lookup.get(&name_lower) {
            return Some(code.clone());
        }
        
        // Substring match (only for longer synonyms to avoid false positives)
        for (synonym, code) in &self.lookup {
            if name_lower.contains(synonym.as_str()) && synonym.len() > 5 {
                return Some(code.clone());
            }
        }

        None
    }

    pub fn get_record(&self, code: &str) -> Option<&OncoTreeRecord> {
        self.records.get(code)
    }

    /// All patterns for the trie, paired with their kind.
    pub fn all_patterns_with_kind(&self) -> Vec<(String, CancerPatternKind)> {
        self.pattern_kinds.iter().map(|(k, &kind)| (k.clone(), kind)).collect()
    }

    /// All patterns (backward compat).
    pub fn all_patterns(&self) -> Vec<String> {
        self.lookup.keys().cloned().collect()
    }

    /// Get the kind of a pattern (for confidence scoring).
    pub fn pattern_kind(&self, pattern: &str) -> Option<CancerPatternKind> {
        self.pattern_kinds.get(&pattern.to_lowercase()).copied()
    }
}
