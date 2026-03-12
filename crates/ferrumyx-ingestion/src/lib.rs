//! ferrumyx-ingestion — Literature ingestion pipeline.
//! Covers Phase 2 of ARCHITECTURE.md.

pub mod chunker;
pub mod dedup;
pub mod embed;
pub mod embedding;
pub mod models;
pub mod normalise;
pub mod pdf_parser;
pub mod pipeline;
pub mod repository;
pub mod sources;

pub use embed::embedder::BiomedBertEmbedder;

// Re-export for backwards compatibility
pub use repository::IngestionRepository;
