//! HGVS mutation notation normalisation.
//! Ported from ferrumyx-ingestion to ferrumyx-kg.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Single-letter → three-letter amino acid map.
fn aa1_to_aa3(aa: &str) -> Option<&'static str> {
    match aa.to_uppercase().as_str() {
        "A" => Some("Ala"),
        "C" => Some("Cys"),
        "D" => Some("Asp"),
        "E" => Some("Glu"),
        "F" => Some("Phe"),
        "G" => Some("Gly"),
        "H" => Some("His"),
        "I" => Some("Ile"),
        "K" => Some("Lys"),
        "L" => Some("Leu"),
        "M" => Some("Met"),
        "N" => Some("Asn"),
        "P" => Some("Pro"),
        "Q" => Some("Gln"),
        "R" => Some("Arg"),
        "S" => Some("Ser"),
        "T" => Some("Thr"),
        "V" => Some("Val"),
        "W" => Some("Trp"),
        "Y" => Some("Tyr"),
        "*" => Some("Ter"),
        _ => None,
    }
}

/// Normalised mutation result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalisedMutation {
    pub raw: String,
    pub hgvs_p: String,
    pub position: u32,
    pub ref_aa: String,
    pub alt_aa: String,
    pub rs_id: Option<String>,
}

/// Static table of well-characterised variants.
fn build_rsid_table() -> HashMap<(&'static str, &'static str), &'static str> {
    let mut m = HashMap::new();
    m.insert(("KRAS", "p.Gly12Asp"), "rs121913529");
    m.insert(("KRAS", "p.Gly12Val"), "rs121913530");
    m.insert(("KRAS", "p.Gly12Cys"), "rs121913527");
    m.insert(("KRAS", "p.Gly12Arg"), "rs121913528");
    m.insert(("KRAS", "p.Gly12Ser"), "rs121913529");
    m.insert(("KRAS", "p.Gly13Asp"), "rs112445441");
    m.insert(("KRAS", "p.Gln61His"), "rs121913240");
    m.insert(("KRAS", "p.Gln61Leu"), "rs121913240");
    m.insert(("NRAS", "p.Gln61Lys"), "rs121913254");
    m.insert(("NRAS", "p.Gly12Asp"), "rs121913239");
    m.insert(("BRAF", "p.Val600Glu"), "rs113488022");
    m.insert(("BRAF", "p.Val600Lys"), "rs121913227");
    m.insert(("TP53", "p.Arg175His"), "rs28934578");
    m.insert(("TP53", "p.Arg248Trp"), "rs28934578");
    m
}

fn normalise_aa3(aa: &str) -> Option<&'static str> {
    match aa.to_lowercase().as_str() {
        "ala" => Some("Ala"),
        "cys" => Some("Cys"),
        "asp" => Some("Asp"),
        "glu" => Some("Glu"),
        "phe" => Some("Phe"),
        "gly" => Some("Gly"),
        "his" => Some("His"),
        "ile" => Some("Ile"),
        "lys" => Some("Lys"),
        "leu" => Some("Leu"),
        "met" => Some("Met"),
        "asn" => Some("Asn"),
        "pro" => Some("Pro"),
        "gln" => Some("Gln"),
        "arg" => Some("Arg"),
        "ser" => Some("Ser"),
        "thr" => Some("Thr"),
        "val" => Some("Val"),
        "trp" => Some("Trp"),
        "tyr" => Some("Tyr"),
        "ter" => Some("Ter"),
        "stop" | "*" => Some("Ter"),
        _ => None,
    }
}

pub struct HgvsMutationNormaliser {
    rsid_table: HashMap<(&'static str, &'static str), &'static str>,
    re_single: Regex,
    re_hgvs: Regex,
}

impl HgvsMutationNormaliser {
    pub fn new() -> Self {
        Self {
            rsid_table: build_rsid_table(),
            re_single: Regex::new(r"^([A-Z\*])(\d+)([A-Z\*])$").unwrap(),
            re_hgvs: Regex::new(
                r"^(?:p\.)?([A-Z][a-z]{0,2}|[A-Z\*])(\d+)([A-Z][a-z]{0,2}|[A-Z\*])$",
            )
            .unwrap(),
        }
    }

    pub fn normalise(&self, raw: &str, gene: Option<&str>) -> Option<NormalisedMutation> {
        let raw = raw.trim();

        if let Some(caps) = self.re_single.captures(raw) {
            let ref_1 = caps.get(1)?.as_str();
            let pos: u32 = caps.get(2)?.as_str().parse().ok()?;
            let alt_1 = caps.get(3)?.as_str();

            let ref_aa = aa1_to_aa3(ref_1)?;
            let alt_aa = aa1_to_aa3(alt_1)?;
            let hgvs_p = format!("p.{}{}{}", ref_aa, pos, alt_aa);

            let rs_id = gene.and_then(|g| {
                self.rsid_table
                    .get(&(g, hgvs_p.as_str()))
                    .map(|s| s.to_string())
            });

            return Some(NormalisedMutation {
                raw: raw.to_string(),
                hgvs_p,
                position: pos,
                ref_aa: ref_aa.to_string(),
                alt_aa: alt_aa.to_string(),
                rs_id,
            });
        }

        if let Some(caps) = self.re_hgvs.captures(raw) {
            let ref_raw = caps.get(1)?.as_str();
            let pos: u32 = caps.get(2)?.as_str().parse().ok()?;
            let alt_raw = caps.get(3)?.as_str();

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
                self.rsid_table
                    .get(&(g, hgvs_p.as_str()))
                    .map(|s| s.to_string())
            });

            return Some(NormalisedMutation {
                raw: raw.to_string(),
                hgvs_p,
                position: pos,
                ref_aa: ref_aa.to_string(),
                alt_aa: alt_aa.to_string(),
                rs_id,
            });
        }

        None
    }

    pub fn all_patterns(&self) -> Vec<String> {
        self.rsid_table
            .keys()
            .map(|(_, p)| p.replace("p.", ""))
            .collect()
    }
}

impl Default for HgvsMutationNormaliser {
    fn default() -> Self {
        Self::new()
    }
}
