//! Knowledge graph fact extraction from text.
//! Ported from Python scripts/build_kg.py

use regex::Regex;
use std::collections::HashSet;

/// Cancer type mappings from keywords to TCGA codes.
const CANCER_KEYWORDS: &[(&str, &str)] = &[
    ("pancreatic", "PAAD"),
    ("pancreas", "PAAD"),
    ("lung", "LUAD"),
    ("breast", "BRCA"),
    ("colorectal", "COAD"),
    ("colon", "COAD"),
    ("melanoma", "SKCM"),
    ("glioblastoma", "GBM"),
    ("brain", "GBM"),
    ("ovarian", "OV"),
    ("prostate", "PRAD"),
    ("liver", "LIHC"),
    ("hepatocellular", "LIHC"),
    ("gastric", "STAD"),
    ("stomach", "STAD"),
    ("kidney", "KIRC"),
    ("renal", "KIRC"),
    ("bladder", "BLCA"),
    ("leukemia", "LAML"),
    ("lymphoma", "DLBC"),
    ("thyroid", "THCA"),
    ("esophageal", "ESCA"),
    ("head and neck", "HNSC"),
    ("cervical", "CESC"),
    ("uterine", "UCEC"),
    ("sarcoma", "SARC"),
];

/// Extract cancer type from text.
pub fn extract_cancer_type(text: &str) -> Option<String> {
    let text_lower = text.to_lowercase();
    extract_cancer_type_from_lower(&text_lower)
}

/// Extract cancer type from already-lowercased text.
pub fn extract_cancer_type_from_lower(text_lower: &str) -> Option<String> {
    for (keyword, cancer_code) in CANCER_KEYWORDS {
        if text_lower.contains(keyword) {
            return Some(cancer_code.to_string());
        }
    }
    None
}

/// Extract mutation patterns (e.g., G12D, V600E, KRAS G12C).
pub fn extract_mutations(text: &str) -> Vec<MutationMention> {
    lazy_mutation_regex();
    let re = lazy_mutation_regex();

    let mut mutations = Vec::new();
    for cap in re.captures_iter(text) {
        let full_match = cap[0].to_string();
        let protein_change = cap.get(1).map(|m| m.as_str().to_string());

        mutations.push(MutationMention {
            text: full_match,
            protein_change,
        });
    }
    mutations
}

fn lazy_mutation_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Match patterns like: G12D, V600E, KRAS G12C, KRAS_G12D
        Regex::new(r"(?i)\b([A-Z]\d+[A-Z])\b").unwrap()
    })
}

/// A mention of a mutation in text.
#[derive(Debug, Clone)]
pub struct MutationMention {
    pub text: String,
    pub protein_change: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExtractedFact {
    pub fact_type: String,
    pub subject: String,
    pub object: String,
    pub evidence_count: i32,
}

/// Fast pattern-based relation extraction.
pub struct RelationExtractor {
    /// List of (pattern, predicate_name)
    /// Example: ("inhibits", "inhibits"), ("overexpressed in", "upregulated_in")
    patterns: Vec<(Regex, String)>,
}

impl RelationExtractor {
    pub fn new() -> Self {
        let rules = vec![
            (r"(?i)\binhibit[s]?\b", "inhibits"),
            (r"(?i)\bsuppress[es]?\b", "inhibits"),
            (r"(?i)\btarget[s]?\b", "targets"),
            (r"(?i)\bactivate[s]?\b", "activates"),
            (r"(?i)\binduce[s]?\b", "activates"),
            (r"(?i)\bdrive[s]?\b", "drives"),
            (r"(?i)\boverexpressed in\b", "upregulated_in"),
            (r"(?i)\bupregulated in\b", "upregulated_in"),
            (r"(?i)\bdownregulated in\b", "downregulated_in"),
            (r"(?i)\bmutated in\b", "mutated_in"),
            (r"(?i)\bmutation[s]? in\b", "mutated_in"),
            (r"(?i)\bvariant[s]? in\b", "mutated_in"),
            (r"(?i)\bassociated with\b", "associated_with"),
            (r"(?i)\blinked to\b", "associated_with"),
        ];

        let patterns = rules
            .into_iter()
            .map(|(re, pred)| (Regex::new(re).unwrap(), pred.to_string()))
            .collect();

        Self { patterns }
    }

    /// Return all predicates whose lexical patterns appear in text.
    pub fn matched_predicates(&self, text: &str) -> Vec<String> {
        let mut found = Vec::new();
        for (re, predicate) in &self.patterns {
            if re.is_match(text) {
                found.push(predicate.clone());
            }
        }
        found
    }

    /// Extract facts between two entities in a given text chunk.
    pub fn extract_relations(&self, subject: &str, object: &str, text: &str) -> Vec<String> {
        let mut found = Vec::new();
        let text_lower = text.to_lowercase();

        // Ensure both entities are present in the text (basic proximity check could be added here)
        if !text_lower.contains(&subject.to_lowercase()) {
            return found;
        }
        let object_lower = object.to_lowercase();
        let object_in_text = text_lower.contains(&object_lower)
            || object_matches_cancer_code(&text_lower, &object_lower);
        if !object_in_text {
            return found;
        }

        for (re, predicate) in &self.patterns {
            if re.is_match(text) {
                found.push(predicate.clone());
            }
        }

        found
    }
}

fn object_matches_cancer_code(text_lower: &str, object_lower: &str) -> bool {
    CANCER_KEYWORDS.iter().any(|(keyword, cancer_code)| {
        cancer_code.eq_ignore_ascii_case(object_lower) && text_lower.contains(keyword)
    })
}

/// Build KG facts from gene mentions and text.
pub fn build_facts(gene_symbol: &str, text: &str) -> Vec<ExtractedFact> {
    let mut facts = Vec::new();
    let extractor = lazy_relation_extractor();

    // 1. Gene-Cancer relationships
    // First, check co-occurrence as a fallback, but prioritize patterns.
    if let Some(cancer) = extract_cancer_type(text) {
        let predicates = extractor.extract_relations(gene_symbol, &cancer, text);

        if predicates.is_empty() {
            // Fallback to generic association if co-occurring
            facts.push(ExtractedFact {
                fact_type: "associated_with".to_string(),
                subject: gene_symbol.to_uppercase(),
                object: cancer,
                evidence_count: 1,
            });
        } else {
            for pred in predicates {
                facts.push(ExtractedFact {
                    fact_type: pred,
                    subject: gene_symbol.to_uppercase(),
                    object: cancer.clone(),
                    evidence_count: 1,
                });
            }
        }
    }

    // 2. Gene-Mutation relationships
    for mutation in extract_mutations(text) {
        if let Some(protein_change) = mutation.protein_change {
            // Usually mutations IN genes.
            facts.push(ExtractedFact {
                fact_type: "has_mutation".to_string(),
                subject: gene_symbol.to_uppercase(),
                object: protein_change,
                evidence_count: 1,
            });
        }
    }

    facts
}

fn lazy_relation_extractor() -> &'static RelationExtractor {
    use std::sync::OnceLock;
    static EXTRACTOR: OnceLock<RelationExtractor> = OnceLock::new();
    EXTRACTOR.get_or_init(RelationExtractor::new)
}

/// Build facts for multiple genes in one pass over text.
/// This reduces repeated regex scans and mutation parsing.
pub fn build_facts_batch(gene_symbols: &[String], text: &str) -> Vec<ExtractedFact> {
    if gene_symbols.is_empty() || text.trim().is_empty() {
        return Vec::new();
    }
    let extractor = lazy_relation_extractor();
    let text_lower = text.to_lowercase();
    let cancer = extract_cancer_type_from_lower(&text_lower);
    let matched_predicates = extractor.matched_predicates(text);
    let mutations: Vec<String> = extract_mutations(text)
        .into_iter()
        .filter_map(|m| m.protein_change)
        .collect();

    let mut out = Vec::new();
    let mut seen_rel: HashSet<(String, String, String)> = HashSet::new();
    for gene in gene_symbols {
        let gene_trim = gene.trim();
        if gene_trim.is_empty() {
            continue;
        }
        let gene_up = gene_trim.to_uppercase();
        let gene_lower = gene_trim.to_lowercase();

        if let Some(cancer_code) = cancer.as_ref() {
            let cancer_lower = cancer_code.to_lowercase();
            let has_pair = text_lower.contains(&gene_lower)
                && (text_lower.contains(&cancer_lower)
                    || object_matches_cancer_code(&text_lower, &cancer_lower));
            if has_pair {
                if matched_predicates.is_empty() {
                    let key = (
                        "associated_with".to_string(),
                        gene_up.clone(),
                        cancer_code.clone(),
                    );
                    if seen_rel.insert(key.clone()) {
                        out.push(ExtractedFact {
                            fact_type: key.0,
                            subject: key.1,
                            object: key.2,
                            evidence_count: 1,
                        });
                    }
                } else {
                    for pred in &matched_predicates {
                        let key = (pred.clone(), gene_up.clone(), cancer_code.clone());
                        if seen_rel.insert(key.clone()) {
                            out.push(ExtractedFact {
                                fact_type: key.0,
                                subject: key.1,
                                object: key.2,
                                evidence_count: 1,
                            });
                        }
                    }
                }
            }
        }

        for mutation in &mutations {
            out.push(ExtractedFact {
                fact_type: "has_mutation".to_string(),
                subject: gene_up.clone(),
                object: mutation.clone(),
                evidence_count: 1,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_cancer_type() {
        assert_eq!(
            extract_cancer_type("KRAS mutations in pancreatic cancer"),
            Some("PAAD".to_string())
        );
        assert_eq!(
            extract_cancer_type("lung adenocarcinoma"),
            Some("LUAD".to_string())
        );
        assert_eq!(extract_cancer_type("no cancer mentioned"), None);
    }

    #[test]
    fn test_relation_extraction() {
        let extractor = RelationExtractor::new();
        let text = "KRAS drives pancreatic cancer progression.";
        let rels = extractor.extract_relations("KRAS", "pancreatic", text);
        assert!(rels.contains(&"drives".to_string()));
    }

    #[test]
    fn test_build_facts_with_patterns() {
        let text = "KRAS is overexpressed in lung adenocarcinoma";
        let facts = build_facts("KRAS", text);
        assert!(facts
            .iter()
            .any(|f| f.fact_type == "upregulated_in" && f.object == "LUAD"));
    }
}
