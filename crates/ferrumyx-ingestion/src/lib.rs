//! ferrumyx-ingestion — Literature ingestion pipeline.
//! Covers Phase 2 of ARCHITECTURE.md:
//! - Paper discovery (PubMed, Europe PMC, bioRxiv, etc.)
//! - DOI resolution
//! - Full-text retrieval
//! - Docling PDF parsing integration
//! - Section-aware chunking
//! - Embedding pipeline
//! - Deduplication

pub mod sources;
pub mod chunker;
pub mod dedup;
pub mod models;
pub mod normalise;
