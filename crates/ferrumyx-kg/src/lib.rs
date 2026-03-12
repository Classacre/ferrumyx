//! ferrumyx-kg — Knowledge graph construction and querying.
//! Covers Phase 3 of ARCHITECTURE.md.

pub mod conflict;
pub mod extraction;
pub mod ner;
pub mod repository;
pub mod scoring;
pub mod update;

pub use extraction::{build_facts, extract_cancer_type, extract_mutations, ExtractedFact};
pub use repository::KgRepository;
pub use scoring::compute_target_scores;
