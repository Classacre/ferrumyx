//! HGNC gene symbol normalisation logic.
//! Ported from ferrumyx-ingestion to ferrumyx-kg.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Tier of a gene symbol match — affects confidence scoring.
/// Preferred = approved HGNC symbol (highest signal).
/// Alias     = known alternate symbol (may have false positives).
/// Previous  = old/deprecated symbol (more noise, kept for coverage).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolTier {
    Preferred,
    Alias,
    Previous,
}

/// A canonical HGNC gene record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HgncRecord {
    /// HGNC accession, e.g. "HGNC:6407"
    pub hgnc_id: String,
    /// Approved symbol, e.g. "KRAS"
    pub symbol: String,
    /// Full gene name
    pub name: String,
    /// NCBI Gene ID (Entrez)
    pub entrez_id: Option<String>,
    /// Ensembl gene ID
    pub ensembl_id: Option<String>,
}

/// HGNC bulk download URL (approved complete set, TSV).
const HGNC_COMPLETE_SET_URL: &str =
    "https://storage.googleapis.com/public-download-files/hgnc/tsv/tsv/hgnc_complete_set.txt";

/// In-memory HGNC normaliser.
pub struct HgncNormaliser {
    /// Map from any known symbol/alias/prev symbol → (canonical record, tier).
    lookup: HashMap<String, (HgncRecord, SymbolTier)>,
    /// Total records loaded.
    n_records: usize,
}

impl HgncNormaliser {
    fn cache_path() -> PathBuf {
        let root =
            std::env::var("FERRUMYX_CACHE_DIR").unwrap_or_else(|_| "./data/cache/ner".to_string());
        PathBuf::from(root).join("hgnc_complete_set.txt")
    }

    /// Build from the HGNC complete set downloaded at runtime.
    pub async fn from_download() -> Result<Self> {
        let cache_path = Self::cache_path();
        if let Ok(tsv) = fs::read_to_string(&cache_path) {
            if let Ok(self_) = Self::from_tsv(&tsv) {
                tracing::info!("Loaded HGNC dataset from cache: {}", cache_path.display());
                return Ok(self_);
            }
        }

        tracing::info!(
            "Downloading HGNC complete set from {}",
            HGNC_COMPLETE_SET_URL
        );
        match reqwest::get(HGNC_COMPLETE_SET_URL).await {
            Ok(resp) => match resp.text().await {
                Ok(resp) => {
                    if let Some(parent) = cache_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let _ = fs::write(&cache_path, &resp);
                    match Self::from_tsv(&resp) {
                        Ok(self_) => Ok(self_),
                        Err(e) => {
                            tracing::warn!("Failed to parse HGNC TSV: {}, using empty", e);
                            Ok(Self::empty())
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to read HGNC response: {}, using empty", e);
                    Ok(Self::empty())
                }
            },
            Err(e) => {
                tracing::warn!("Failed to download HGNC: {}, using empty", e);
                Ok(Self::empty())
            }
        }
    }

    fn empty() -> Self {
        Self {
            lookup: HashMap::new(),
            n_records: 0,
        }
    }

    /// Synchronous version for use in spawn_blocking.
    pub fn from_download_blocking() -> Result<Self> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(Self::from_download())
    }

    /// Build from a pre-downloaded TSV string (for testing / offline use).
    pub fn from_tsv(tsv: &str) -> Result<Self> {
        let mut lookup: HashMap<String, (HgncRecord, SymbolTier)> = HashMap::new();
        let mut n_records = 0usize;

        for (line_no, line) in tsv.lines().enumerate() {
            if line_no == 0 {
                continue;
            }

            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() < 2 {
                continue;
            }

            let get = |i: usize| fields.get(i).copied().unwrap_or("").trim();

            let hgnc_id = get(0).to_string();
            let symbol = get(1).to_string();
            let name = get(2).to_string();
            let status = get(5);
            let entrez_id = non_empty(get(18));
            let ensembl_id = non_empty(get(19));

            if !status.contains("Approved") {
                continue;
            }
            if symbol.is_empty() {
                continue;
            }

            let record = HgncRecord {
                hgnc_id: hgnc_id.clone(),
                symbol: symbol.clone(),
                name: name.clone(),
                entrez_id,
                ensembl_id,
            };

            // Preferred symbol — highest tier
            lookup.insert(
                symbol.to_uppercase(),
                (record.clone(), SymbolTier::Preferred),
            );

            // Alias symbols — second tier
            for alias in get(8).split('|').filter(|s| !s.is_empty()) {
                let key = alias.trim().to_uppercase();
                lookup
                    .entry(key)
                    .or_insert_with(|| (record.clone(), SymbolTier::Alias));
            }

            // Previous symbols — lowest tier
            for prev in get(10).split('|').filter(|s| !s.is_empty()) {
                let key = prev.trim().to_uppercase();
                lookup
                    .entry(key)
                    .or_insert_with(|| (record.clone(), SymbolTier::Previous));
            }

            n_records += 1;
        }

        tracing::info!(
            "HGNC normaliser built: {} records, {} lookup entries",
            n_records,
            lookup.len()
        );
        Ok(Self { lookup, n_records })
    }

    /// Look up a symbol, returning both the record and its tier.
    pub fn lookup_with_tier(&self, symbol: &str) -> Option<(&HgncRecord, SymbolTier)> {
        self.lookup
            .get(&symbol.trim().to_uppercase())
            .map(|(rec, tier)| (rec, *tier))
    }

    pub fn lookup(&self, symbol: &str) -> Option<&HgncRecord> {
        self.lookup
            .get(&symbol.trim().to_uppercase())
            .map(|(rec, _)| rec)
    }

    pub fn normalise_symbol(&self, symbol: &str) -> Option<String> {
        self.lookup(symbol).map(|r| r.symbol.clone())
    }

    pub fn to_hgnc_id(&self, symbol: &str) -> Option<String> {
        self.lookup(symbol).map(|r| r.hgnc_id.clone())
    }

    pub fn n_records(&self) -> usize {
        self.n_records
    }
    pub fn n_lookup_entries(&self) -> usize {
        self.lookup.len()
    }

    /// All patterns for the trie, paired with their tier.
    pub fn all_patterns_with_tier(&self) -> Vec<(String, SymbolTier)> {
        self.lookup
            .iter()
            .map(|(k, (_, tier))| (k.clone(), *tier))
            .collect()
    }

    /// All patterns (for backward compat). Includes preferred + aliases + previous.
    pub fn all_patterns(&self) -> Vec<String> {
        self.lookup.keys().cloned().collect()
    }
}

fn non_empty(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}
