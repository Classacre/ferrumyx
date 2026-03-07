//! ferrumyx-ingestion — Literature ingestion pipeline.
//! Covers Phase 2 of ARCHITECTURE.md.

pub mod sources;
pub mod chunker;
pub mod dedup;
pub mod pdf_parser;
pub mod models;
pub mod normalise;
pub mod repository;
pub mod pipeline;
pub mod embed;
pub mod embedding;

pub use embed::embedder::BiomedBertEmbedder;

// Re-export for backwards compatibility
pub use repository::IngestionRepository;
