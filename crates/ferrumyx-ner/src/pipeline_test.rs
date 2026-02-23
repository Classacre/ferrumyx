//! Tests for the NER pipeline.

#[cfg(test)]
mod tests {
    use crate::{EntityType, normalize_entity_label};

    #[test]
    fn test_entity_type_as_str() {
        assert_eq!(EntityType::Disease.as_str(), "DISEASE");
        assert_eq!(EntityType::Gene.as_str(), "GENE");
        assert_eq!(EntityType::Chemical.as_str(), "CHEMICAL");
        assert_eq!(EntityType::Protein.as_str(), "PROTEIN");
        assert_eq!(EntityType::Mutation.as_str(), "MUTATION");
        assert_eq!(EntityType::Species.as_str(), "SPECIES");
        assert_eq!(EntityType::CellLine.as_str(), "CELL_LINE");
        assert_eq!(EntityType::Pathway.as_str(), "PATHWAY");
        assert_eq!(EntityType::Drug.as_str(), "DRUG");
        assert_eq!(EntityType::Person.as_str(), "PERSON");
        assert_eq!(EntityType::Organization.as_str(), "ORGANIZATION");
        assert_eq!(EntityType::Location.as_str(), "LOCATION");
        assert_eq!(EntityType::Other.as_str(), "OTHER");
    }

    #[test]
    fn test_normalize_entity_labels() {
        // BC5CDR labels
        assert_eq!(normalize_entity_label("Chemical"), EntityType::Chemical);
        assert_eq!(normalize_entity_label("Disease"), EntityType::Disease);
        
        // SciSpacy / CRAFT labels
        assert_eq!(normalize_entity_label("GGP"), EntityType::Gene);
        assert_eq!(normalize_entity_label("SO"), EntityType::Gene);
        assert_eq!(normalize_entity_label("TAXON"), EntityType::Species);
        assert_eq!(normalize_entity_label("CELL"), EntityType::CellLine);
        assert_eq!(normalize_entity_label("CELL_TYPE"), EntityType::CellLine);
        assert_eq!(normalize_entity_label("PATHWAY"), EntityType::Pathway);
        
        // JNLPBA labels
        assert_eq!(normalize_entity_label("DNA"), EntityType::Gene);
        assert_eq!(normalize_entity_label("RNA"), EntityType::Gene);
        assert_eq!(normalize_entity_label("cell_type"), EntityType::CellLine);
        assert_eq!(normalize_entity_label("cell_line"), EntityType::CellLine);
        
        // NCBI Disease labels
        assert_eq!(normalize_entity_label("SpecificDisease"), EntityType::Disease);
        assert_eq!(normalize_entity_label("DiseaseClass"), EntityType::Disease);
        assert_eq!(normalize_entity_label("CompositeMention"), EntityType::Disease);
        
        // BioNLP / BioCreative labels
        assert_eq!(normalize_entity_label("Gene_or_gene_product"), EntityType::Gene);
        assert_eq!(normalize_entity_label("Protein"), EntityType::Protein);
        assert_eq!(normalize_entity_label("Simple_chemical"), EntityType::Chemical);
        assert_eq!(normalize_entity_label("Amino_acid"), EntityType::Chemical);
        assert_eq!(normalize_entity_label("Drug"), EntityType::Drug);
        assert_eq!(normalize_entity_label("Cancer"), EntityType::Disease);
        
        // BIO tagging
        assert_eq!(normalize_entity_label("B-GENE"), EntityType::Gene);
        assert_eq!(normalize_entity_label("I-GENE"), EntityType::Gene);
        assert_eq!(normalize_entity_label("B-DISEASE"), EntityType::Disease);
        assert_eq!(normalize_entity_label("I-DISEASE"), EntityType::Disease);
        assert_eq!(normalize_entity_label("B-CHEMICAL"), EntityType::Chemical);
        assert_eq!(normalize_entity_label("I-CHEMICAL"), EntityType::Chemical);
        assert_eq!(normalize_entity_label("B-MUTATION"), EntityType::Mutation);
        assert_eq!(normalize_entity_label("I-MUTATION"), EntityType::Mutation);
        assert_eq!(normalize_entity_label("B-SPECIES"), EntityType::Species);
        assert_eq!(normalize_entity_label("I-SPECIES"), EntityType::Species);
        assert_eq!(normalize_entity_label("B-CELL_LINE"), EntityType::CellLine);
        assert_eq!(normalize_entity_label("I-CELL_LINE"), EntityType::CellLine);
        
        // Standard NER labels
        assert_eq!(normalize_entity_label("PER"), EntityType::Person);
        assert_eq!(normalize_entity_label("PERSON"), EntityType::Person);
        assert_eq!(normalize_entity_label("ORG"), EntityType::Organization);
        assert_eq!(normalize_entity_label("ORGANIZATION"), EntityType::Organization);
        assert_eq!(normalize_entity_label("LOC"), EntityType::Location);
        assert_eq!(normalize_entity_label("LOCATION"), EntityType::Location);
        assert_eq!(normalize_entity_label("MISC"), EntityType::Other);
        
        // Unknown labels
        assert_eq!(normalize_entity_label("UNKNOWN"), EntityType::Other);
        assert_eq!(normalize_entity_label("RandomLabel"), EntityType::Other);
    }

    #[test]
    fn test_entity_type_equality() {
        assert_eq!(EntityType::Disease, EntityType::Disease);
        assert_ne!(EntityType::Disease, EntityType::Gene);
    }

    #[test]
    fn test_entity_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(EntityType::Disease);
        set.insert(EntityType::Gene);
        set.insert(EntityType::Disease); // Duplicate
        assert_eq!(set.len(), 2);
    }
}
