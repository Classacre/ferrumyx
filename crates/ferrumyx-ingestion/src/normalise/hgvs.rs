//! HGVS mutation notation normalisation.
//!
//! Maps short variant notations (e.g. "G12D", "p.G12D", "Gly12Asp")
//! into structured, canonical HGVS p. notation and, for common KRAS mutations,
//! known dbSNP rsIDs.
//!
//! # Example
//! ```ignore
//! let norm = HgvsMutationNormaliser::new();
//! let m = norm.normalise("G12D", Some("KRAS")).unwrap();
//! assert_eq!(m.hgvs_p, "p.Gly12Asp");
//! assert_eq!(m.rs_id.as_deref(), Some("rs121913529"));
//! ```

use std::collections::HashMap;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Single-letter → three-letter amino acid map.
fn aa1_to_aa3(aa: &str) -> Option<&'static str> {
    match aa.to_uppercase().as_str() {
        "A" => Some("Ala"), "C" => Some("Cys"), "D" => Some("Asp"),
        "E" => Some("Glu"), "F" => Some("Phe"), "G" => Some("Gly"),
        "H" => Some("His"), "I" => Some("Ile"), "K" => Some("Lys"),
        "L" => Some("Leu"), "M" => Some("Met"), "N" => Some("Asn"),
        "P" => Some("Pro"), "Q" => Some("Gln"), "R" => Some("Arg"),
        "S" => Some("Ser"), "T" => Some("Thr"), "V" => Some("Val"),
        "W" => Some("Trp"), "Y" => Some("Tyr"), "*" => Some("Ter"),
        _ => None,
    }
}

/// Normalised mutation result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalisedMutation {
    /// Input text as supplied.
    pub raw: String,
    /// Canonical HGVS protein notation, e.g. "p.Gly12Asp"
    pub hgvs_p: String,
    /// Position number extracted.
    pub position: u32,
    /// Reference amino acid (3-letter), e.g. "Gly"
    pub ref_aa: String,
    /// Alt amino acid (3-letter), e.g. "Asp"
    pub alt_aa: String,
    /// dbSNP rsID if known, e.g. "rs121913529"
    pub rs_id: Option<String>,
}

/// Static table of well-characterised KRAS variants.
/// Extend as more rsIDs are confirmed.
fn build_rsid_table() -> HashMap<(&'static str, &'static str), &'static str> {
    let mut m = HashMap::new();
    // KRAS
    m.insert(("KRAS", "p.Gly12Asp"), "rs121913529");
    m.insert(("KRAS", "p.Gly12Val"), "rs121913530");
    m.insert(("KRAS", "p.Gly12Cys"), "rs121913527");
    m.insert(("KRAS", "p.Gly12Arg"), "rs121913528");
    m.insert(("KRAS", "p.Gly12Ser"), "rs121913529"); // G12S shares same rsID family
    m.insert(("KRAS", "p.Gly13Asp"), "rs112445441");
    m.insert(("KRAS", "p.Gln61His"), "rs121913240");
    m.insert(("KRAS", "p.Gln61Leu"), "rs121913240");
    // NRAS
    m.insert(("NRAS", "p.Gln61Lys"), "rs121913254");
    m.insert(("NRAS", "p.Gly12Asp"), "rs121913239");
    // BRAF
    m.insert(("BRAF", "p.Val600Glu"), "rs113488022");
    m.insert(("BRAF", "p.Val600Lys"), "rs121913227");
    // TP53
    m.insert(("TP53", "p.Arg175His"), "rs28934578");
    m.insert(("TP53", "p.Arg248Trp"), "rs28934578");
    m
}

/// Three-letter amino acid name → three-letter (title-case) canonical form.
fn normalise_aa3(aa: &str) -> Option<&'static str> {
    match aa.to_lowercase().as_str() {
        "ala" => Some("Ala"), "cys" => Some("Cys"), "asp" => Some("Asp"),
        "glu" => Some("Glu"), "phe" => Some("Phe"), "gly" => Some("Gly"),
        "his" => Some("His"), "ile" => Some("Ile"), "lys" => Some("Lys"),
        "leu" => Some("Leu"), "met" => Some("Met"), "asn" => Some("Asn"),
        "pro" => Some("Pro"), "gln" => Some("Gln"), "arg" => Some("Arg"),
        "ser" => Some("Ser"), "thr" => Some("Thr"), "val" => Some("Val"),
        "trp" => Some("Trp"), "tyr" => Some("Tyr"), "ter" => Some("Ter"),
        "stop" | "*" => Some("Ter"),
        _ => None,
    }
}

pub struct HgvsMutationNormaliser {
    /// Gene-specific rsID table.
    rsid_table: HashMap<(&'static str, &'static str), &'static str>,
    /// Regex: single-letter notation, e.g. G12D
    re_single: Regex,
    /// Regex: p.AA123AA notation (single or triple letter), e.g. p.G12D, p.Gly12Asp
    re_hgvs: Regex,
}

impl HgvsMutationNormaliser {
    pub fn new() -> Self {
        Self {
            rsid_table: build_rsid_table(),
            // G12D, G12D, V600E, R175H etc.
            re_single: Regex::new(r"^([A-Z\*])(\d+)([A-Z\*])$").unwrap(),
            // p.Gly12Asp or p.G12D — optional "p." prefix
            re_hgvs: Regex::new(
                r"^(?:p\.)?([A-Z][a-z]{0,2}|[A-Z\*])(\d+)([A-Z][a-z]{0,2}|[A-Z\*])$"
            ).unwrap(),
        }
    }

    /// Normalise a mutation string to HGVS p. notation.
    ///
    /// `gene` is optional and used only for rsID lookup.
    /// Returns `None` if the input can't be parsed as a missense variant.
    pub fn normalise(&self, raw: &str, gene: Option<&str>) -> Option<NormalisedMutation> {
        let raw = raw.trim();

        // Try single-letter first: G12D
        if let Some(caps) = self.re_single.captures(raw) {
            let ref_1 = caps.get(1)?.as_str();
            let pos: u32 = caps.get(2)?.as_str().parse().ok()?;
            let alt_1 = caps.get(3)?.as_str();

            let ref_aa = aa1_to_aa3(ref_1)?;
            let alt_aa = aa1_to_aa3(alt_1)?;
            let hgvs_p = format!("p.{}{}{}", ref_aa, pos, alt_aa);

            let rs_id = gene.and_then(|g| {
                self.rsid_table.get(&(g, hgvs_p.as_str()))
                    .map(|s| s.to_string())
            });

            return Some(NormalisedMutation {
                raw: raw.to_string(), hgvs_p, position: pos,
                ref_aa: ref_aa.to_string(), alt_aa: alt_aa.to_string(), rs_id,
            });
        }

        // Try HGVS notation: p.Gly12Asp, p.G12D, Gly12Asp
        if let Some(caps) = self.re_hgvs.captures(raw) {
            let ref_raw = caps.get(1)?.as_str();
            let pos: u32 = caps.get(2)?.as_str().parse().ok()?;
            let alt_raw = caps.get(3)?.as_str();

            // Resolve to 3-letter form
            let ref_aa = if ref_raw.len() == 1 {
                aa1_to_aa3(ref_raw)?
            } else {
                normalise_aa3(ref_raw)?
            };
            let alt_aa = if alt_raw.len() == 1 {
                aa1_to_aa3(alt_raw)?
            } else {
                normalise_aa3(alt_raw)?
            };

            let hgvs_p = format!("p.{}{}{}", ref_aa, pos, alt_aa);
            let rs_id = gene.and_then(|g| {
                self.rsid_table.get(&(g, hgvs_p.as_str()))
                    .map(|s| s.to_string())
            });

            return Some(NormalisedMutation {
                raw: raw.to_string(), hgvs_p, position: pos,
                ref_aa: ref_aa.to_string(), alt_aa: alt_aa.to_string(), rs_id,
            });
        }

        None
    }
}

impl Default for HgvsMutationNormaliser {
    fn default() -> Self { Self::new() }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn norm() -> HgvsMutationNormaliser { HgvsMutationNormaliser::new() }

    #[test]
    fn test_single_letter_g12d() {
        let m = norm().normalise("G12D", Some("KRAS")).unwrap();
        assert_eq!(m.hgvs_p, "p.Gly12Asp");
        assert_eq!(m.position, 12);
        assert_eq!(m.rs_id.as_deref(), Some("rs121913529"));
    }

    #[test]
    fn test_single_letter_v600e() {
        let m = norm().normalise("V600E", Some("BRAF")).unwrap();
        assert_eq!(m.hgvs_p, "p.Val600Glu");
        assert_eq!(m.rs_id.as_deref(), Some("rs113488022"));
    }

    #[test]
    fn test_hgvs_triple_letter() {
        let m = norm().normalise("p.Gly12Asp", Some("KRAS")).unwrap();
        assert_eq!(m.hgvs_p, "p.Gly12Asp");
        assert_eq!(m.rs_id.as_deref(), Some("rs121913529"));
    }

    #[test]
    fn test_hgvs_without_p_prefix() {
        let m = norm().normalise("Gly12Asp", Some("KRAS")).unwrap();
        assert_eq!(m.hgvs_p, "p.Gly12Asp");
    }

    #[test]
    fn test_no_rsid_for_unknown_gene() {
        let m = norm().normalise("G12D", None).unwrap();
        assert_eq!(m.hgvs_p, "p.Gly12Asp");
        assert!(m.rs_id.is_none());
    }

    #[test]
    fn test_unparseable_returns_none() {
        assert!(norm().normalise("wild-type", None).is_none());
        assert!(norm().normalise("exon 2 deletion", None).is_none());
        assert!(norm().normalise("", None).is_none());
    }

    #[test]
    fn test_nonsense_mutation() {
        let m = norm().normalise("R213*", None).unwrap();
        assert_eq!(m.hgvs_p, "p.Arg213Ter");
        assert_eq!(m.alt_aa, "Ter");
    }
}
