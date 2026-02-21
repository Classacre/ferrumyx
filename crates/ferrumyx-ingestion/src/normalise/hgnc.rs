//! HGNC gene symbol normalisation.
//!
//! Downloads the HGNC complete set TSV (~7 MB) and builds an in-memory
//! lookup table mapping any known symbol or alias → canonical HGNC record.
//!
//! Usage:
//! ```ignore
//! let normaliser = HgncNormaliser::from_download().await?;
//! let rec = normaliser.lookup("K-RAS");  // returns Some(HgncRecord { id: "HGNC:6407", symbol: "KRAS", ... })
//! ```

use std::collections::HashMap;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

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
/// Build once at startup; share as `Arc<HgncNormaliser>`.
pub struct HgncNormaliser {
    /// Map from any known symbol/alias/prev symbol → canonical record.
    lookup: HashMap<String, HgncRecord>,
    /// Total records loaded.
    n_records: usize,
}

impl HgncNormaliser {
    // ── Constructors ──────────────────────────────────────────────────────────

    /// Build from the HGNC complete set downloaded at runtime.
    pub async fn from_download() -> Result<Self> {
        tracing::info!("Downloading HGNC complete set from {}", HGNC_COMPLETE_SET_URL);
        let resp = reqwest::get(HGNC_COMPLETE_SET_URL)
            .await
            .context("HGNC download failed")?
            .text()
            .await
            .context("HGNC response read failed")?;
        Self::from_tsv(&resp)
    }

    /// Build from a pre-downloaded TSV string (for testing / offline use).
    pub fn from_tsv(tsv: &str) -> Result<Self> {
        let mut lookup: HashMap<String, HgncRecord> = HashMap::new();
        let mut n_records = 0usize;

        for (line_no, line) in tsv.lines().enumerate() {
            // Skip header row
            if line_no == 0 { continue; }

            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() < 2 { continue; }

            // Column indices in HGNC complete set TSV (as of 2024):
            // 0  hgnc_id
            // 1  symbol
            // 2  name
            // 3  locus_group
            // 4  locus_type
            // 5  status
            // 6  location
            // 7  location_sortable
            // 8  alias_symbol     (pipe-separated)
            // 9  alias_name
            // 10 prev_symbol      (pipe-separated)
            // 11 prev_name
            // 18 entrez_id
            // 19 ensembl_gene_id

            let get = |i: usize| fields.get(i).copied().unwrap_or("").trim();

            let hgnc_id    = get(0).to_string();
            let symbol     = get(1).to_string();
            let name       = get(2).to_string();
            let status     = get(5);
            let entrez_id  = non_empty(get(18));
            let ensembl_id = non_empty(get(19));

            // Skip withdrawn/non-approved entries
            if !status.contains("Approved") { continue; }
            if symbol.is_empty() { continue; }

            let record = HgncRecord {
                hgnc_id: hgnc_id.clone(),
                symbol:  symbol.clone(),
                name:    name.clone(),
                entrez_id,
                ensembl_id,
            };

            // Insert canonical symbol
            lookup.insert(symbol.to_uppercase(), record.clone());

            // Insert all alias symbols
            for alias in get(8).split('|').filter(|s| !s.is_empty()) {
                lookup.entry(alias.trim().to_uppercase())
                    .or_insert_with(|| record.clone());
            }

            // Insert previous symbols
            for prev in get(10).split('|').filter(|s| !s.is_empty()) {
                lookup.entry(prev.trim().to_uppercase())
                    .or_insert_with(|| record.clone());
            }

            n_records += 1;
        }

        tracing::info!("HGNC normaliser built: {} records, {} lookup entries", n_records, lookup.len());
        Ok(Self { lookup, n_records })
    }

    // ── Lookup ────────────────────────────────────────────────────────────────

    /// Normalise a gene symbol/alias to a canonical HGNC record.
    /// Case-insensitive. Returns `None` if not found.
    pub fn lookup(&self, symbol: &str) -> Option<&HgncRecord> {
        self.lookup.get(&symbol.trim().to_uppercase())
    }

    /// Normalise and return just the canonical symbol (e.g. "K-RAS" → "KRAS").
    pub fn normalise_symbol(&self, symbol: &str) -> Option<String> {
        self.lookup(symbol).map(|r| r.symbol.clone())
    }

    /// Normalise and return the HGNC ID string (e.g. "KRAS" → "HGNC:6407").
    pub fn to_hgnc_id(&self, symbol: &str) -> Option<String> {
        self.lookup(symbol).map(|r| r.hgnc_id.clone())
    }

    /// Number of canonical gene records loaded.
    pub fn n_records(&self) -> usize { self.n_records }

    /// Number of lookup entries (includes aliases and previous symbols).
    pub fn n_lookup_entries(&self) -> usize { self.lookup.len() }
}

fn non_empty(s: &str) -> Option<String> {
    if s.is_empty() { None } else { Some(s.to_string()) }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal synthetic HGNC TSV for unit tests.
    fn sample_tsv() -> String {
        let header = "hgnc_id\tsymbol\tname\tlocus_group\tlocus_type\tstatus\tlocation\tlocation_sortable\talias_symbol\talias_name\tprev_symbol\tprev_name\t\t\t\t\t\t\tentrez_id\tensembl_gene_id";
        let kras = "HGNC:6407\tKRAS\tKRAS proto-oncogene, GTPase\tprotein-coding gene\tgene with protein product\tApproved\t12p12.1\t12p12.1\tK-RAS|KRAS2\t\tKI-RAS|C-K-RAS\t\t\t\t\t\t\t3845\tENSG00000133703";
        let tp53 = "HGNC:11998\tTP53\ttumor protein p53\tprotein-coding gene\tgene with protein product\tApproved\t17p13.1\t17p13.1\tp53\t\t\t\t\t\t\t\t\t7157\tENSG00000141510";
        format!("{header}\n{kras}\n{tp53}\n")
    }

    #[test]
    fn test_lookup_canonical() {
        let n = HgncNormaliser::from_tsv(&sample_tsv()).unwrap();
        let r = n.lookup("KRAS").unwrap();
        assert_eq!(r.hgnc_id, "HGNC:6407");
        assert_eq!(r.symbol, "KRAS");
    }

    #[test]
    fn test_lookup_alias() {
        let n = HgncNormaliser::from_tsv(&sample_tsv()).unwrap();
        let r = n.lookup("K-RAS").unwrap();
        assert_eq!(r.symbol, "KRAS");
    }

    #[test]
    fn test_lookup_prev_symbol() {
        let n = HgncNormaliser::from_tsv(&sample_tsv()).unwrap();
        let r = n.lookup("KI-RAS").unwrap();
        assert_eq!(r.symbol, "KRAS");
    }

    #[test]
    fn test_lookup_case_insensitive() {
        let n = HgncNormaliser::from_tsv(&sample_tsv()).unwrap();
        assert!(n.lookup("kras").is_some());
        assert!(n.lookup("Kras").is_some());
    }

    #[test]
    fn test_lookup_unknown_returns_none() {
        let n = HgncNormaliser::from_tsv(&sample_tsv()).unwrap();
        assert!(n.lookup("NOTAREALGENE999").is_none());
    }

    #[test]
    fn test_hgnc_id_lookup() {
        let n = HgncNormaliser::from_tsv(&sample_tsv()).unwrap();
        assert_eq!(n.to_hgnc_id("KRAS").as_deref(), Some("HGNC:6407"));
        assert_eq!(n.to_hgnc_id("TP53").as_deref(), Some("HGNC:11998"));
    }
}
