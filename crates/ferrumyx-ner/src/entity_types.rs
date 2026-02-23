//! Entity type classification and normalization.

use std::collections::HashMap;

/// Normalized entity type for biomedical NER.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EntityType {
    Gene,
    Protein,
    Disease,
    Chemical,
    Mutation,
    Species,
    CellLine,
    Pathway,
    Drug,
    Person,
    Organization,
    Location,
    Other,
}

impl EntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EntityType::Gene => "GENE",
            EntityType::Protein => "PROTEIN",
            EntityType::Disease => "DISEASE",
            EntityType::Chemical => "CHEMICAL",
            EntityType::Mutation => "MUTATION",
            EntityType::Species => "SPECIES",
            EntityType::CellLine => "CELL_LINE",
            EntityType::Pathway => "PATHWAY",
            EntityType::Drug => "DRUG",
            EntityType::Person => "PERSON",
            EntityType::Organization => "ORGANIZATION",
            EntityType::Location => "LOCATION",
            EntityType::Other => "OTHER",
        }
    }
}

// Map model-specific labels to normalized types
fn get_label_map() -> &'static HashMap<String, EntityType> {
    use std::sync::OnceLock;
    static LABEL_MAP: OnceLock<HashMap<String, EntityType>> = OnceLock::new();
    LABEL_MAP.get_or_init(|| {
        let mut m = HashMap::new();
        
        // BC5CDR labels
        m.insert("Chemical".to_string(), EntityType::Chemical);
        m.insert("Disease".to_string(), EntityType::Disease);
        
        // SciSpacy / CRAFT labels
        m.insert("GGP".to_string(), EntityType::Gene);  // Gene or Gene Product
        m.insert("SO".to_string(), EntityType::Gene);   // Sequence Ontology
        m.insert("TAXON".to_string(), EntityType::Species);
        m.insert("CELL".to_string(), EntityType::CellLine);
        m.insert("CELL_TYPE".to_string(), EntityType::CellLine);
        m.insert("TISSUE".to_string(), EntityType::Other);
        m.insert("ORGAN".to_string(), EntityType::Other);
        m.insert("ORGANISM".to_string(), EntityType::Species);
        m.insert("PATHWAY".to_string(), EntityType::Pathway);
        
        // JNLPBA labels
        m.insert("DNA".to_string(), EntityType::Gene);
        m.insert("RNA".to_string(), EntityType::Gene);
        m.insert("cell_type".to_string(), EntityType::CellLine);
        m.insert("cell_line".to_string(), EntityType::CellLine);
        
        // NCBI Disease labels
        m.insert("SpecificDisease".to_string(), EntityType::Disease);
        m.insert("DiseaseClass".to_string(), EntityType::Disease);
        m.insert("CompositeMention".to_string(), EntityType::Disease);
        
        // BioNLP / BioCreative labels
        m.insert("Gene_or_gene_product".to_string(), EntityType::Gene);
        m.insert("Protein".to_string(), EntityType::Protein);
        m.insert("Simple_chemical".to_string(), EntityType::Chemical);
        m.insert("Amino_acid".to_string(), EntityType::Chemical);
        m.insert("Drug".to_string(), EntityType::Drug);
        m.insert("Cancer".to_string(), EntityType::Disease);
        
        // B-NER labels (BIO tagging)
        m.insert("B-GENE".to_string(), EntityType::Gene);
        m.insert("I-GENE".to_string(), EntityType::Gene);
        m.insert("B-DISEASE".to_string(), EntityType::Disease);
        m.insert("I-DISEASE".to_string(), EntityType::Disease);
        m.insert("B-CHEMICAL".to_string(), EntityType::Chemical);
        m.insert("I-CHEMICAL".to_string(), EntityType::Chemical);
        m.insert("B-MUTATION".to_string(), EntityType::Mutation);
        m.insert("I-MUTATION".to_string(), EntityType::Mutation);
        m.insert("B-SPECIES".to_string(), EntityType::Species);
        m.insert("I-SPECIES".to_string(), EntityType::Species);
        m.insert("B-CELL_LINE".to_string(), EntityType::CellLine);
        m.insert("I-CELL_LINE".to_string(), EntityType::CellLine);
        
        // Standard NER labels (CoNLL / OntoNotes)
        m.insert("PER".to_string(), EntityType::Person);
        m.insert("PERSON".to_string(), EntityType::Person);
        m.insert("ORG".to_string(), EntityType::Organization);
        m.insert("ORGANIZATION".to_string(), EntityType::Organization);
        m.insert("LOC".to_string(), EntityType::Location);
        m.insert("LOCATION".to_string(), EntityType::Location);
        m.insert("MISC".to_string(), EntityType::Other);
        
        m
    })
}

/// Normalize a model-specific entity label to our standard EntityType.
pub fn normalize_entity_label(label: &str) -> EntityType {
    // Handle BIO tagging (B-, I- prefixes)
    let clean_label = label.trim_start_matches("B-").trim_start_matches("I-");
    
    get_label_map().get(clean_label)
        .or_else(|| get_label_map().get(label))
        .copied()
        .unwrap_or(EntityType::Other)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_gene_labels() {
        assert_eq!(normalize_entity_label("GGP"), EntityType::Gene);
        assert_eq!(normalize_entity_label("B-GENE"), EntityType::Gene);
        assert_eq!(normalize_entity_label("Gene_or_gene_product"), EntityType::Gene);
    }

    #[test]
    fn test_normalize_disease_labels() {
        assert_eq!(normalize_entity_label("Disease"), EntityType::Disease);
        assert_eq!(normalize_entity_label("B-DISEASE"), EntityType::Disease);
        assert_eq!(normalize_entity_label("SpecificDisease"), EntityType::Disease);
    }
}
