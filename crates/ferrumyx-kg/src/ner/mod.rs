pub mod entity_types;
pub mod trie_ner;
pub mod entity_loader;
pub mod entity_db;
pub mod entity_aggregator;

pub use entity_types::EntityType;
pub use trie_ner::{TrieNer, ExtractedEntity};
pub use entity_loader::{BiomedicalDatabase, GeneEntry, DiseaseEntry, ChemicalEntry, DiseaseCategory};
pub use entity_db::EntityDatabase;
pub use entity_aggregator::{EntityAggregator, KgTriple, AggregationResult, BatchAggregationResult};
