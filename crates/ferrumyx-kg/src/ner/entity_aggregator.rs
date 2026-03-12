//! Entity aggregation for knowledge graph construction.

pub struct EntityAggregator;

impl EntityAggregator {
    pub fn infer_predicate(type_a: &str, type_b: &str) -> &'static str {
        match (type_a, type_b) {
            ("GENE", "DISEASE") | ("DISEASE", "GENE") => "associated_with",
            ("CHEMICAL", "GENE") | ("GENE", "CHEMICAL") => "interacts_with",
            ("CHEMICAL", "DISEASE") | ("DISEASE", "CHEMICAL") => "treats",
            ("GENE", "GENE") => "interacts_with",
            _ => "related_to",
        }
    }
}

#[derive(Debug, Clone)]
pub struct KgTriple {
    pub subject_id: String,
    pub subject_type: String,
    pub predicate: String,
    pub object_id: String,
    pub object_type: String,
    pub evidence_count: i32,
    pub confidence: f32,
}

#[derive(Debug, Default)]
pub struct AggregationResult {
    pub entity_count: usize,
    pub cooccurrence_count: usize,
    pub triples: Vec<KgTriple>,
}

#[derive(Debug)]
pub struct BatchAggregationResult {
    pub papers_processed: usize,
    pub total_entities: usize,
    pub total_cooccurrences: usize,
    pub unique_triples: usize,
    pub triples: Vec<KgTriple>,
}
