//! Knowledge graph fact extraction from text.
//! Ported from Python scripts/build_kg.py and extended for typed pair extraction.

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

/// Built-in short lexicon for common targeted therapeutics.
const CHEMICAL_HINTS: &[&str] = &[
    "erlotinib",
    "gefitinib",
    "osimertinib",
    "afatinib",
    "sotorasib",
    "adagrasib",
    "trametinib",
    "selumetinib",
    "imatinib",
    "dasatinib",
    "nilotinib",
    "palbociclib",
    "ribociclib",
    "abemaciclib",
    "olaparib",
    "niraparib",
    "talazoparib",
    "cisplatin",
    "carboplatin",
    "paclitaxel",
    "docetaxel",
    "bevacizumab",
];

/// Built-in short lexicon for common pathways.
const PATHWAY_HINTS: &[&str] = &[
    "mapk pathway",
    "pi3k pathway",
    "jak stat pathway",
    "m tor pathway",
    "wnt pathway",
    "nf kb pathway",
    "tgf beta pathway",
    "hedgehog pathway",
    "ras pathway",
    "erk signaling pathway",
    "akt signaling pathway",
    "egfr signaling pathway",
    "vegf pathway",
];

/// Built-in short lexicon for common cell lines.
const CELL_LINE_HINTS: &[&str] = &[
    "hela", "hek293", "a549", "h1975", "pc9", "panc1", "mia paca 2", "bxpc3", "ht29", "hct116",
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

fn lazy_sentence_split_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[.!?;\n]+").unwrap())
}

fn lazy_drug_suffix_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)\b([A-Z][A-Za-z0-9\-]{3,40}(?:mab|nib|fenib|ciclib|parib|platin|taxel|mycin|azole|vir))\b",
        )
        .unwrap()
    })
}

fn lazy_pathway_phrase_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)\b([A-Za-z0-9\-_/ ]{2,80}?(?:pathway|signaling pathway|signalling pathway|axis))\b",
        )
        .unwrap()
    })
}

fn lazy_cell_line_regex() -> &'static Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b([A-Z]{1,5}-?\d{2,4}[A-Z0-9]*)\b").unwrap())
}

fn split_into_sentences(text: &str) -> Vec<String> {
    lazy_sentence_split_regex()
        .split(text)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
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
            (r"(?i)\bsuppress(?:es|ed)?\b", "inhibits"),
            (r"(?i)\btarget[s]?\b", "targets"),
            (r"(?i)\bactivate[s]?\b", "activates"),
            (r"(?i)\binduce[s]?\b", "activates"),
            (r"(?i)\bdrive[s]?\b", "drives"),
            (
                r"(?i)\bpromote[s]?\b.{0,30}\bproliferation\b",
                "promotes_proliferation",
            ),
            (r"(?i)\bproliferation\b", "promotes_proliferation"),
            (r"(?i)\btumou?rigenesis\b", "promotes_tumorigenesis"),
            (r"(?i)\bmetasta(sis|tic)\b", "drives_metastasis"),
            (r"(?i)\binva(sion|sive)\b", "drives_invasion"),
            (r"(?i)\boverexpress(?:ed|ion)?\b", "upregulated_in"),
            (r"(?i)\bupregulat(?:ed|ion)?\b", "upregulated_in"),
            (r"(?i)\bdownregulat(?:ed|ion)?\b", "downregulated_in"),
            (r"(?i)\bmutated in\b", "mutated_in"),
            (r"(?i)\bmutation[s]? in\b", "mutated_in"),
            (r"(?i)\bvariant[s]? in\b", "mutated_in"),
            (r"(?i)\bconfers? resistance\b", "confers_resistance"),
            (r"(?i)\bresistan(t|ce) to\b", "confers_resistance"),
            (r"(?i)\bsensiti[sz]es? to\b", "sensitizes_to"),
            (r"(?i)\bsensitive to\b", "sensitizes_to"),
            (r"(?i)\bbiomarker\b", "biomarker_of"),
            (r"(?i)\bpredictive marker\b", "biomarker_of"),
            (r"(?i)\bpoor prognosis\b", "prognostic_for_poor_outcome"),
            (r"(?i)\bworse survival\b", "prognostic_for_poor_outcome"),
            (r"(?i)\breduced survival\b", "prognostic_for_poor_outcome"),
            (
                r"(?i)\bfavorable prognosis\b",
                "prognostic_for_better_outcome",
            ),
            (
                r"(?i)\bimproved survival\b",
                "prognostic_for_better_outcome",
            ),
            (r"(?i)\bsynthetic lethal\b", "synthetic_lethal_with"),
            (r"(?i)\bessential for\b", "required_for_viability"),
            (r"(?i)\bdependency\b", "required_for_viability"),
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
        normalize_predicates(found)
    }

    /// Extract facts between two entities in a given text chunk.
    pub fn extract_relations(&self, subject: &str, object: &str, text: &str) -> Vec<String> {
        let mut found = Vec::new();
        let text_lower = text.to_lowercase();

        if !contains_symbol_ci(&text_lower, &subject.to_lowercase()) {
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

        normalize_predicates(found)
    }
}

fn normalize_predicates(mut predicates: Vec<String>) -> Vec<String> {
    if predicates.is_empty() {
        return predicates;
    }
    let mut seen = HashSet::new();
    predicates.retain(|p| seen.insert(p.clone()));
    let has_specific = predicates.iter().any(|p| p != "associated_with");
    if has_specific {
        predicates.retain(|p| p != "associated_with");
    }
    predicates
}

fn object_matches_cancer_code(text_lower: &str, object_lower: &str) -> bool {
    CANCER_KEYWORDS.iter().any(|(keyword, cancer_code)| {
        cancer_code.eq_ignore_ascii_case(object_lower) && text_lower.contains(keyword)
    })
}

fn contains_symbol_ci(text_lower: &str, symbol_lower: &str) -> bool {
    if symbol_lower.is_empty() {
        return false;
    }
    let mut start = 0usize;
    while let Some(idx_rel) = text_lower[start..].find(symbol_lower) {
        let idx = start + idx_rel;
        let end = idx + symbol_lower.len();
        let left_ok = idx == 0
            || !text_lower[..idx]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        let right_ok = end >= text_lower.len()
            || !text_lower[end..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        if left_ok && right_ok {
            return true;
        }
        start = idx.saturating_add(1);
        if start >= text_lower.len() {
            break;
        }
    }
    false
}

fn normalize_token_phrase(value: &str) -> String {
    value
        .trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase()
}

fn load_external_dictionary_terms(path: &str, max_terms: usize) -> Vec<String> {
    let content = match std::fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut terms = Vec::new();
    for line in content.lines() {
        if terms.len() >= max_terms {
            break;
        }
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        terms.push(t.to_lowercase());
    }
    terms
}

fn env_dictionary_terms(var: &str, max_terms: usize) -> Vec<String> {
    std::env::var(var)
        .ok()
        .map(|v| {
            v.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .take(max_terms)
                .map(|s| s.to_lowercase())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn lazy_chemical_terms() -> &'static Vec<String> {
    use std::sync::OnceLock;
    static TERMS: OnceLock<Vec<String>> = OnceLock::new();
    TERMS.get_or_init(|| {
        let mut terms: Vec<String> = CHEMICAL_HINTS.iter().map(|s| s.to_string()).collect();
        terms.extend(load_external_dictionary_terms(
            "data/dictionaries/chemicals.txt",
            8_000,
        ));
        terms.extend(env_dictionary_terms("FERRUMYX_KG_CHEMICAL_HINTS", 512));
        let mut seen = HashSet::new();
        terms
            .into_iter()
            .map(|s| s.to_lowercase())
            .filter(|s| !s.is_empty())
            .filter(|s| seen.insert(s.clone()))
            .collect()
    })
}

fn lazy_pathway_terms() -> &'static Vec<String> {
    use std::sync::OnceLock;
    static TERMS: OnceLock<Vec<String>> = OnceLock::new();
    TERMS.get_or_init(|| {
        let mut terms: Vec<String> = PATHWAY_HINTS.iter().map(|s| s.to_string()).collect();
        terms.extend(load_external_dictionary_terms(
            "data/dictionaries/pathways.txt",
            8_000,
        ));
        terms.extend(env_dictionary_terms("FERRUMYX_KG_PATHWAY_HINTS", 512));
        let mut seen = HashSet::new();
        terms
            .into_iter()
            .map(|s| s.to_lowercase())
            .filter(|s| !s.is_empty())
            .filter(|s| seen.insert(s.clone()))
            .collect()
    })
}

fn lazy_cell_line_terms() -> &'static Vec<String> {
    use std::sync::OnceLock;
    static TERMS: OnceLock<Vec<String>> = OnceLock::new();
    TERMS.get_or_init(|| {
        let mut terms: Vec<String> = CELL_LINE_HINTS.iter().map(|s| s.to_string()).collect();
        terms.extend(load_external_dictionary_terms(
            "data/dictionaries/cell_lines.txt",
            8_000,
        ));
        terms.extend(env_dictionary_terms("FERRUMYX_KG_CELL_LINE_HINTS", 512));
        let mut seen = HashSet::new();
        terms
            .into_iter()
            .map(|s| s.to_lowercase())
            .filter(|s| !s.is_empty())
            .filter(|s| seen.insert(s.clone()))
            .collect()
    })
}

fn detect_chemical_mentions(sentence: &str, sentence_lower: &str, max_out: usize) -> Vec<String> {
    let mut found = Vec::new();
    let mut seen = HashSet::new();

    for cap in lazy_drug_suffix_regex().captures_iter(sentence) {
        let token = normalize_token_phrase(cap.get(1).map(|m| m.as_str()).unwrap_or_default());
        if !token.is_empty() && seen.insert(token.clone()) {
            found.push(token);
            if found.len() >= max_out {
                return found;
            }
        }
    }

    for term in lazy_chemical_terms() {
        if found.len() >= max_out {
            break;
        }
        if term.len() < 4 {
            continue;
        }
        if sentence_lower.contains(term) {
            let token = normalize_token_phrase(term);
            if !token.is_empty() && seen.insert(token.clone()) {
                found.push(token);
            }
        }
    }

    found
}

fn detect_pathway_mentions(sentence: &str, sentence_lower: &str, max_out: usize) -> Vec<String> {
    let mut found = Vec::new();
    let mut seen = HashSet::new();

    for cap in lazy_pathway_phrase_regex().captures_iter(sentence) {
        let token = normalize_token_phrase(cap.get(1).map(|m| m.as_str()).unwrap_or_default());
        if token.len() >= 6 && seen.insert(token.clone()) {
            found.push(token);
            if found.len() >= max_out {
                return found;
            }
        }
    }

    for term in lazy_pathway_terms() {
        if found.len() >= max_out {
            break;
        }
        if term.len() < 6 {
            continue;
        }
        if sentence_lower.contains(term) {
            let token = normalize_token_phrase(term);
            if !token.is_empty() && seen.insert(token.clone()) {
                found.push(token);
            }
        }
    }

    found
}

fn detect_cell_line_mentions(sentence: &str, sentence_lower: &str, max_out: usize) -> Vec<String> {
    let mut found = Vec::new();
    let mut seen = HashSet::new();

    for cap in lazy_cell_line_regex().captures_iter(sentence) {
        let raw = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
        if raw.len() < 3 || raw.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let token = normalize_token_phrase(raw);
        if seen.insert(token.clone()) {
            found.push(token);
            if found.len() >= max_out {
                return found;
            }
        }
    }

    for term in lazy_cell_line_terms() {
        if found.len() >= max_out {
            break;
        }
        if term.len() < 3 {
            continue;
        }
        if sentence_lower.contains(term) {
            let token = normalize_token_phrase(term);
            if seen.insert(token.clone()) {
                found.push(token);
            }
        }
    }

    found
}

fn choose_gene_cancer_predicates(
    matched_predicates: &[String],
    sentence_lower: &str,
) -> Vec<String> {
    let mut out = Vec::new();
    let matched: HashSet<&str> = matched_predicates.iter().map(|s| s.as_str()).collect();

    if matched.contains("mutated_in") {
        out.push("mutated_in".to_string());
    }
    if matched.contains("prognostic_for_poor_outcome") {
        out.push("prognostic_for_poor_outcome".to_string());
    }
    if matched.contains("prognostic_for_better_outcome") {
        out.push("prognostic_for_better_outcome".to_string());
    }
    if matched.contains("biomarker_of") {
        out.push("biomarker_of".to_string());
    }
    if matched.contains("drives") || matched.contains("promotes_tumorigenesis") {
        out.push("drives".to_string());
    }
    if matched.contains("drives_metastasis") {
        out.push("drives_metastasis".to_string());
    }
    if matched.contains("drives_invasion") {
        out.push("drives_invasion".to_string());
    }
    if matched.contains("upregulated_in") {
        out.push("upregulated_in".to_string());
    }
    if matched.contains("downregulated_in") {
        out.push("downregulated_in".to_string());
    }

    if out.is_empty()
        && (sentence_lower.contains("driver")
            || sentence_lower.contains("oncogenic")
            || sentence_lower.contains("tumor suppressor"))
    {
        out.push("drives".to_string());
    }

    if out.is_empty() {
        out.push("associated_with".to_string());
    }
    normalize_predicates(out)
}

fn choose_gene_chemical_predicates(
    matched_predicates: &[String],
    sentence_lower: &str,
) -> Vec<String> {
    let matched: HashSet<&str> = matched_predicates.iter().map(|s| s.as_str()).collect();
    let mut out = Vec::new();

    if matched.contains("inhibits") || matched.contains("targets") {
        out.push("targeted_by".to_string());
    }
    if matched.contains("sensitizes_to") || sentence_lower.contains("sensit") {
        out.push("sensitized_by".to_string());
    }
    if matched.contains("confers_resistance") || sentence_lower.contains("resistan") {
        out.push("resistance_to".to_string());
    }
    if matched.contains("activates") {
        out.push("activated_by_compound".to_string());
    }

    if out.is_empty() {
        out.push("targeted_by".to_string());
    }
    normalize_predicates(out)
}

fn choose_gene_pathway_predicates(
    matched_predicates: &[String],
    sentence_lower: &str,
) -> Vec<String> {
    let matched: HashSet<&str> = matched_predicates.iter().map(|s| s.as_str()).collect();
    let mut out = Vec::new();

    if matched.contains("activates") || sentence_lower.contains("activation") {
        out.push("activates_pathway".to_string());
    }
    if matched.contains("inhibits")
        || sentence_lower.contains("suppression")
        || sentence_lower.contains("suppresses")
    {
        out.push("suppresses_pathway".to_string());
    }
    if out.is_empty() {
        out.push("in_pathway".to_string());
    }
    normalize_predicates(out)
}

fn choose_gene_cell_line_predicates(sentence_lower: &str) -> Vec<String> {
    if sentence_lower.contains("dependency")
        || sentence_lower.contains("essential")
        || sentence_lower.contains("viability")
    {
        vec!["dependency_in_cell_line".to_string()]
    } else {
        vec!["studied_in_cell_line".to_string()]
    }
}

/// Build KG facts from gene mentions and text.
pub fn build_facts(gene_symbol: &str, text: &str) -> Vec<ExtractedFact> {
    build_facts_batch(&[gene_symbol.to_string()], text)
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
    let global_cancer = extract_cancer_type_from_lower(&text_lower);
    let global_mutations: Vec<String> = extract_mutations(text)
        .into_iter()
        .filter_map(|m| m.protein_change)
        .collect();

    let mut out = Vec::new();
    let mut seen_rel: HashSet<(String, String, String)> = HashSet::new();
    let normalized_genes: Vec<(String, String)> = gene_symbols
        .iter()
        .map(|g| (g.trim().to_uppercase(), g.trim().to_lowercase()))
        .filter(|(up, lc)| !up.is_empty() && !lc.is_empty())
        .collect();

    for sentence in split_into_sentences(text) {
        let sentence_lower = sentence.to_lowercase();
        let matched_predicates = extractor.matched_predicates(&sentence);
        let sentence_cancer = extract_cancer_type_from_lower(&sentence_lower).or(global_cancer.clone());
        let sentence_mutations: Vec<String> = extract_mutations(&sentence)
            .into_iter()
            .filter_map(|m| m.protein_change)
            .collect();
        let chemicals = detect_chemical_mentions(&sentence, &sentence_lower, 6);
        let pathways = detect_pathway_mentions(&sentence, &sentence_lower, 6);
        let cell_lines = detect_cell_line_mentions(&sentence, &sentence_lower, 6);

        for (gene_up, gene_lower) in &normalized_genes {
            if !contains_symbol_ci(&sentence_lower, gene_lower) {
                continue;
            }

            let mut has_typed_relation = false;

            if let Some(cancer_code) = sentence_cancer.as_ref() {
                for pred in choose_gene_cancer_predicates(&matched_predicates, &sentence_lower) {
                    let key = (pred.clone(), gene_up.clone(), cancer_code.clone());
                    if seen_rel.insert(key.clone()) {
                        out.push(ExtractedFact {
                            fact_type: key.0,
                            subject: key.1,
                            object: key.2,
                            evidence_count: 1,
                        });
                    }
                    if pred != "associated_with" {
                        has_typed_relation = true;
                    }
                }
            }

            for mutation in &sentence_mutations {
                let key = ("has_mutation".to_string(), gene_up.clone(), mutation.to_uppercase());
                if seen_rel.insert(key.clone()) {
                    out.push(ExtractedFact {
                        fact_type: key.0,
                        subject: key.1,
                        object: key.2,
                        evidence_count: 1,
                    });
                }
                has_typed_relation = true;
                if sentence_lower.contains("resistan") {
                    let key = (
                        "mutation_confers_resistance".to_string(),
                        gene_up.clone(),
                        mutation.to_uppercase(),
                    );
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

            for chem in &chemicals {
                for pred in choose_gene_chemical_predicates(&matched_predicates, &sentence_lower) {
                    let key = (pred, gene_up.clone(), chem.clone());
                    if seen_rel.insert(key.clone()) {
                        out.push(ExtractedFact {
                            fact_type: key.0,
                            subject: key.1,
                            object: key.2,
                            evidence_count: 1,
                        });
                    }
                    has_typed_relation = true;
                }
            }

            for pathway in &pathways {
                for pred in choose_gene_pathway_predicates(&matched_predicates, &sentence_lower) {
                    let key = (pred, gene_up.clone(), pathway.clone());
                    if seen_rel.insert(key.clone()) {
                        out.push(ExtractedFact {
                            fact_type: key.0,
                            subject: key.1,
                            object: key.2,
                            evidence_count: 1,
                        });
                    }
                    has_typed_relation = true;
                }
            }

            for cell_line in &cell_lines {
                for pred in choose_gene_cell_line_predicates(&sentence_lower) {
                    let key = (pred, gene_up.clone(), cell_line.clone());
                    if seen_rel.insert(key.clone()) {
                        out.push(ExtractedFact {
                            fact_type: key.0,
                            subject: key.1,
                            object: key.2,
                            evidence_count: 1,
                        });
                    }
                    has_typed_relation = true;
                }
            }

            // Generic fallback only when no typed relation could be resolved.
            if !has_typed_relation {
                if let Some(cancer_code) = sentence_cancer.as_ref() {
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
                }
            }
        }
    }

    if out.is_empty() && !global_mutations.is_empty() {
        // Safety fallback for sparse text: at least preserve mutation observations.
        for (gene_up, _) in normalized_genes {
            for mutation in &global_mutations {
                let key = ("has_mutation".to_string(), gene_up.clone(), mutation.to_uppercase());
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
    fn test_build_facts_typed_edges() {
        let text = "KRAS activates MAPK pathway in pancreatic cancer. KRAS is sensitive to sotorasib.";
        let facts = build_facts_batch(&["KRAS".to_string()], text);
        assert!(facts.iter().any(|f| f.fact_type == "activates_pathway"));
        assert!(facts.iter().any(|f| f.fact_type == "targeted_by" || f.fact_type == "sensitized_by"));
        assert!(facts.iter().any(|f| f.object == "PAAD"));
    }
}
