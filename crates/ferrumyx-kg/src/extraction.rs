//! Knowledge graph fact extraction from text.
//! Ported from Python scripts/build_kg.py

use regex::Regex;

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

/// A knowledge graph fact extracted from text.
#[derive(Debug, Clone)]
pub struct KgFact {
    pub fact_type: String,
    pub subject: String,
    pub object: String,
    pub evidence_count: i32,
}

/// Build KG facts from gene mentions and text.
pub fn build_facts(
    gene_symbol: &str,
    text: &str,
) -> Vec<KgFact> {
    let mut facts = Vec::new();
    
    // Gene-Cancer relationship
    if let Some(cancer) = extract_cancer_type(text) {
        facts.push(KgFact {
            fact_type: "gene_cancer".to_string(),
            subject: gene_symbol.to_uppercase(),
            object: cancer,
            evidence_count: 1,
        });
    }
    
    // Gene-Mutation relationships
    for mutation in extract_mutations(text) {
        if let Some(protein_change) = mutation.protein_change {
            facts.push(KgFact {
                fact_type: "gene_mutation".to_string(),
                subject: gene_symbol.to_uppercase(),
                object: protein_change,
                evidence_count: 1,
            });
        }
    }
    
    facts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_cancer_type() {
        assert_eq!(extract_cancer_type("KRAS mutations in pancreatic cancer"), Some("PAAD".to_string()));
        assert_eq!(extract_cancer_type("lung adenocarcinoma"), Some("LUAD".to_string()));
        assert_eq!(extract_cancer_type("no cancer mentioned"), None);
    }

    #[test]
    fn test_extract_mutations() {
        let text = "KRAS G12D and V600E mutations";
        let mutations = extract_mutations(text);
        assert_eq!(mutations.len(), 2);
        assert_eq!(mutations[0].protein_change, Some("G12D".to_string()));
        assert_eq!(mutations[1].protein_change, Some("V600E".to_string()));
    }
}
