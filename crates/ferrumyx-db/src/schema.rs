//! Schema definitions for LanceDB tables.
//!
//! LanceDB uses Apache Arrow for storage, so we define schemas
//! using Arrow types with vector support for embeddings.

/// Embedding dimension (BiomedBERT-base outputs 768-dim vectors)
pub const EMBEDDING_DIM: usize = 768;

// =============================================================================
// Paper Schema
// =============================================================================

/// Paper record stored in LanceDB
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Paper {
    pub id: uuid::Uuid,
    pub doi: Option<String>,
    pub pmid: Option<String>,
    pub title: String,
    pub abstract_text: Option<String>,
    pub full_text: Option<String>,
    pub source: String,
    pub source_id: Option<String>,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
    pub authors: Option<String>,
    pub journal: Option<String>,
    pub volume: Option<String>,
    pub issue: Option<String>,
    pub pages: Option<String>,
    pub parse_status: String,
    pub ingested_at: chrono::DateTime<chrono::Utc>,
    pub abstract_simhash: Option<i64>,
}

impl Paper {
    pub fn new(title: String, source: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            doi: None,
            pmid: None,
            title,
            abstract_text: None,
            full_text: None,
            source,
            source_id: None,
            published_at: None,
            authors: None,
            journal: None,
            volume: None,
            issue: None,
            pages: None,
            parse_status: "pending".to_string(),
            ingested_at: chrono::Utc::now(),
            abstract_simhash: None,
        }
    }
}

// =============================================================================
// Chunk Schema
// =============================================================================

/// Document chunk with optional embedding
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Chunk {
    pub id: uuid::Uuid,
    pub paper_id: uuid::Uuid,
    pub chunk_index: i64,
    pub content: String,
    pub section: Option<String>,
    pub page: Option<i64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub embedding: Option<Vec<f32>>,
}

impl Chunk {
    pub fn new(paper_id: uuid::Uuid, chunk_index: i64, content: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            paper_id,
            chunk_index,
            content,
            section: None,
            page: None,
            created_at: chrono::Utc::now(),
            embedding: None,
        }
    }
}

// =============================================================================
// Entity Schema
// =============================================================================

/// Entity types supported by the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EntityType {
    Gene,
    Disease,
    Chemical,
    Mutation,
    CancerType,
    Pathway,
    Protein,
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityType::Gene => write!(f, "gene"),
            EntityType::Disease => write!(f, "disease"),
            EntityType::Chemical => write!(f, "chemical"),
            EntityType::Mutation => write!(f, "mutation"),
            EntityType::CancerType => write!(f, "cancer_type"),
            EntityType::Pathway => write!(f, "pathway"),
            EntityType::Protein => write!(f, "protein"),
        }
    }
}

impl std::str::FromStr for EntityType {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "gene" => Ok(EntityType::Gene),
            "disease" => Ok(EntityType::Disease),
            "chemical" => Ok(EntityType::Chemical),
            "mutation" => Ok(EntityType::Mutation),
            "cancer_type" | "cancertype" => Ok(EntityType::CancerType),
            "pathway" => Ok(EntityType::Pathway),
            "protein" => Ok(EntityType::Protein),
            _ => Err(format!("Unknown entity type: {}", s)),
        }
    }
}

/// Entity record
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Entity {
    pub id: uuid::Uuid,
    pub external_id: String,
    pub name: String,
    pub canonical_name: Option<String>,
    pub entity_type: String,
    pub synonyms: Option<String>,
    pub description: Option<String>,
    pub source_db: String,
    pub metadata: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Entity {
    pub fn new(entity_type: EntityType, name: String, external_id: String, source_db: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4(),
            external_id,
            name,
            canonical_name: None,
            entity_type: entity_type.to_string(),
            synonyms: None,
            description: None,
            source_db,
            metadata: None,
            created_at: now,
            updated_at: now,
        }
    }
}

// =============================================================================
// Knowledge Graph Fact Schema
// =============================================================================

/// Knowledge graph triple (subject, predicate, object)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KgFact {
    pub id: uuid::Uuid,
    pub paper_id: uuid::Uuid,
    pub subject_id: uuid::Uuid,
    pub subject_name: String,
    pub predicate: String,
    pub object_id: uuid::Uuid,
    pub object_name: String,
    pub confidence: Option<f32>,
    pub evidence: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl KgFact {
    pub fn new(
        paper_id: uuid::Uuid,
        subject_id: uuid::Uuid,
        subject_name: String,
        predicate: String,
        object_id: uuid::Uuid,
        object_name: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            paper_id,
            subject_id,
            subject_name,
            predicate,
            object_id,
            object_name,
            confidence: None,
            evidence: None,
            created_at: chrono::Utc::now(),
        }
    }
}

// =============================================================================
// Entity Mention Schema (for NER results)
// =============================================================================

/// Entity mention in a chunk
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntityMention {
    pub id: uuid::Uuid,
    pub entity_id: uuid::Uuid,
    pub chunk_id: uuid::Uuid,
    pub paper_id: uuid::Uuid,
    pub start_offset: i64,
    pub end_offset: i64,
    pub text: String,
    pub confidence: Option<f32>,
    pub context: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl EntityMention {
    pub fn new(
        entity_id: uuid::Uuid,
        chunk_id: uuid::Uuid,
        paper_id: uuid::Uuid,
        text: String,
        start_offset: i64,
        end_offset: i64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            entity_id,
            chunk_id,
            paper_id,
            start_offset,
            end_offset,
            text,
            confidence: None,
            context: None,
            created_at: chrono::Utc::now(),
        }
    }
}

// =============================================================================
// Knowledge Graph Conflicts Schema
// =============================================================================

/// Conflict between two KG facts
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KgConflict {
    pub id: uuid::Uuid,
    pub fact_a_id: uuid::Uuid,
    pub fact_b_id: uuid::Uuid,
    pub conflict_type: String,
    pub net_confidence: f32,
    pub resolution: String,
    pub detected_at: chrono::DateTime<chrono::Utc>,
}

impl KgConflict {
    pub fn new(fact_a_id: uuid::Uuid, fact_b_id: uuid::Uuid, conflict_type: String, net_confidence: f32, resolution: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            fact_a_id,
            fact_b_id,
            conflict_type,
            net_confidence,
            resolution,
            detected_at: chrono::Utc::now(),
        }
    }
}

// =============================================================================
// Table Names
// =============================================================================

pub const TABLE_PAPERS: &str = "papers";
pub const TABLE_CHUNKS: &str = "chunks";
pub const TABLE_ENTITIES: &str = "entities";
pub const TABLE_KG_FACTS: &str = "kg_facts";
pub const TABLE_ENTITY_MENTIONS: &str = "entity_mentions";
pub const TABLE_KG_CONFLICTS: &str = "kg_conflicts";
