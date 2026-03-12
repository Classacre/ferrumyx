//! Entity type classification and normalization.

use std::collections::HashMap;

/// Normalized entity type for biomedical NER.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EntityType {
    Gene,
    Mutation,
    Disease,
    CancerType,
    Chemical,
    CellLine,
    Pathway,
    Other,
}

impl EntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EntityType::Gene => "GENE",
            EntityType::Disease => "DISEASE",
            EntityType::CancerType => "CANCER_TYPE",
            EntityType::Chemical => "CHEMICAL",
            EntityType::Mutation => "MUTATION",
            EntityType::CellLine => "CELL_LINE",
            EntityType::Pathway => "PATHWAY",
            EntityType::Other => "OTHER",
        }
    }
}

pub fn normalize_entity_label(label: &str) -> EntityType {
    let clean_label = label.trim_start_matches("B-").trim_start_matches("I-");
    get_label_map()
        .get(clean_label)
        .or_else(|| get_label_map().get(label))
        .copied()
        .unwrap_or(EntityType::Other)
}

fn get_label_map() -> &'static HashMap<String, EntityType> {
    use std::sync::OnceLock;
    static LABEL_MAP: OnceLock<HashMap<String, EntityType>> = OnceLock::new();
    LABEL_MAP.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert("Gene".to_string(), EntityType::Gene);
        m.insert("Disease".to_string(), EntityType::Disease);
        m.insert("Chemical".to_string(), EntityType::Chemical);
        m.insert("Mutation".to_string(), EntityType::Mutation);
        m.insert("DNA".to_string(), EntityType::Gene);
        m.insert("RNA".to_string(), EntityType::Gene);
        m.insert("Cancer".to_string(), EntityType::CancerType);
        m.insert("CancerType".to_string(), EntityType::CancerType);
        m.insert("GGP".to_string(), EntityType::Gene);
        m
    })
}
