//! Ferrumyx Database Layer
//!
//! This crate provides an embedded database layer using LanceDB for
//! zero-dependency storage of papers, chunks, entities, and knowledge graph facts.
//!
//! # Features
//!
//! - Embedded vector database (no external server required)
//! - Native HNSW indexing for vector similarity search
//! - Columnar storage optimized for analytics
//! - Pure Rust implementation
//!
//! # Example
//!
//! ```rust,no_run
//! use ferrumyx_db::{Database, PaperRepository, ChunkRepository};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Open database
//!     let db = Database::open("./data/ferrumyx.db").await?;
//!     db.initialize().await?;
//!     
//!     // Use repositories
//!     let papers = PaperRepository::new(std::sync::Arc::new(db));
//!     
//!     Ok(())
//! }
//! ```

pub mod database;
pub mod error;
pub mod schema;
pub mod schema_arrow;
pub mod papers;
pub mod chunks;
pub mod entities;
pub mod kg_facts;
pub mod entity_mentions;

pub use database::{Database, DatabaseStats};
pub use error::{DbError, Result};
pub use schema::{
    Paper, Chunk, Entity, KgFact, EntityMention,
    EntityType, EMBEDDING_DIM,
    TABLE_PAPERS, TABLE_CHUNKS, TABLE_ENTITIES, TABLE_KG_FACTS, TABLE_ENTITY_MENTIONS,
};
pub use papers::PaperRepository;
pub use chunks::ChunkRepository;
pub use entities::EntityRepository;
pub use kg_facts::KgFactRepository;
pub use entity_mentions::EntityMentionRepository;
