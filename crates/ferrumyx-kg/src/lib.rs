//! ferrumyx-kg — Knowledge graph construction and querying.
//! Covers Phase 3 of ARCHITECTURE.md.

pub mod repository;
pub mod conflict;
pub mod update;
pub mod pg_repository;
pub mod extraction;
pub mod scoring;

pub use extraction::{KgFact, build_facts, extract_cancer_type, extract_mutations};
pub use scoring::compute_target_scores;
