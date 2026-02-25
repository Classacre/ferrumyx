//! Ferrumyx NER - Fast biomedical entity recognition using dictionary-based matching.
//!
//! This crate provides fast, accurate entity recognition for genes, diseases,
//! chemicals, and mutations using Aho-Corasick trie matching against biomedical
//! databases (HGNC, MeSH, ChEMBL).
//!
//! # Features
//!
//! - **Fast Trie NER**: O(n) matching using Aho-Corasick automaton
//! - **Database Integration**: Auto-downloads HGNC, MeSH, ChEMBL databases
//! - **Entity Aggregation**: Builds knowledge graphs from extracted entities
//! - **Knowledge Graph Export**: RDF triples for downstream analysis
//!
//! # Example
//!
//! ```rust
//! use ferrumyx_ner::trie_ner::TrieNer;
//!
//! let ner = TrieNer::with_embedded_subset();
//! let entities = ner.extract("KRAS G12D mutations in pancreatic cancer");
//!
//! for entity in entities {
//!     println!("{:?}: {} (confidence: {:.2})", 
//!         entity.label, entity.text, entity.confidence);
//! }
//! ```

pub mod entity_types;
pub mod trie_ner;
pub mod entity_loader;
pub mod entity_db;
pub mod entity_aggregator;

// Re-export commonly used types
pub use entity_types::EntityType;
pub use trie_ner::{TrieNer, ExtractedEntity};
pub use entity_loader::{
    BiomedicalDatabase, GeneEntry, DiseaseEntry, ChemicalEntry, DiseaseCategory
};
pub use entity_db::EntityDatabase;
pub use entity_aggregator::{EntityAggregator, KgTriple, AggregationResult, BatchAggregationResult};

pub type Result<T> = anyhow::Result<T>;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::entity_types::EntityType;
    pub use crate::trie_ner::TrieNer;
    pub use crate::entity_loader::BiomedicalDatabase;
    pub use crate::entity_db::EntityDatabase;
}
