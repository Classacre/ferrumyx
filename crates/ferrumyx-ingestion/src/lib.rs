//! ferrumyx-ingestion — Literature ingestion pipeline.
//! Covers Phase 2 of ARCHITECTURE.md:
//! - Paper discovery (PubMed, Europe PMC, bioRxiv, etc.)
//! - DOI resolution
//! - Full-text retrieval
//! - PDF parsing (Ferrules - fast Rust-native)
//! - Section-aware chunking
//! - Embedding pipeline
//! - Deduplication

pub mod sources;
pub mod chunker;
pub mod dedup;
pub mod pdf_parser;
pub mod models;
pub mod normalise;
pub mod repository;
pub mod pipeline;
pub mod embedding;

// Re-export for backwards compatibility
pub use repository::IngestionRepository;
