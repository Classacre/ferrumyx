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

pub mod chunks;
pub mod database;
pub mod ent_stage;
pub mod entities;
pub mod entity_mentions;
pub mod error;
pub mod kg_conflicts;
pub mod kg_facts;
pub mod papers;
pub mod phase4_signals;
pub mod schema;
pub mod schema_arrow;
pub mod target_scores;

pub use chunks::ChunkRepository;
pub use database::{Database, DatabaseStats};
pub use ent_stage::{EntEnrichment, EntStageRepository};
pub use entities::EntityRepository;
pub use entity_mentions::EntityMentionRepository;
pub use error::{DbError, Result};
pub use kg_conflicts::KgConflictRepository;
pub use kg_facts::KgFactRepository;
pub use papers::PaperRepository;
pub use phase4_signals::Phase4SignalRepository;
pub use schema::{
    Chunk, Entity, EntityMention, EntityType, KgConflict, KgFact, Paper, TargetScore,
    EMBEDDING_DIM, TABLE_CHUNKS, TABLE_ENTITIES, TABLE_ENTITY_MENTIONS, TABLE_KG_CONFLICTS,
    TABLE_KG_FACTS, TABLE_PAPERS, TABLE_TARGET_SCORES,
};
pub use schema::{EntChemblTarget, EntGtexExpression, EntReactomeGene, EntTcgaSurvival};
pub use target_scores::TargetScoreRepository;
