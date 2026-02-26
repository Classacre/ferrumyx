//! Database connection and table management.
//!
//! Provides a unified interface for LanceDB operations.

use crate::error::Result;
use crate::schema;
use arrow_array::RecordBatchIterator;
use arrow_schema::{Field, Schema, Fields, DataType};
use lancedb::connection::Connection;
use std::path::Path;
use std::sync::Arc;

/// Main database handle.
#[derive(Clone)]
pub struct Database {
    conn: Connection,
    path: String,
}

impl Database {
    /// Open or create a database at the specified path.
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        
        // Create directory if it doesn't exist
        if !path.as_ref().exists() {
            std::fs::create_dir_all(path.as_ref())?;
        }
        
        let conn = lancedb::connect(&path_str)
            .execute()
            .await?;
        
        Ok(Self {
            conn,
            path: path_str,
        })
    }
    
    /// Get the underlying connection.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }
    
    /// Get the database path.
    pub fn path(&self) -> &str {
        &self.path
    }
    
    /// Initialize all tables with schemas.
    ///
    /// This creates the tables if they don't exist.
    /// LanceDB requires initial data to create a table with a schema.
    pub async fn initialize(&self) -> Result<()> {
        // Create papers table if it doesn't exist
        if !self.table_exists(schema::TABLE_PAPERS).await? {
            self.create_papers_table().await?;
        }
        
        // Create chunks table if it doesn't exist
        if !self.table_exists(schema::TABLE_CHUNKS).await? {
            self.create_chunks_table().await?;
        }
        
        // Create entities table if it doesn't exist
        if !self.table_exists(schema::TABLE_ENTITIES).await? {
            self.create_entities_table().await?;
        }
        
        // Create entity_mentions table if it doesn't exist
        if !self.table_exists(schema::TABLE_ENTITY_MENTIONS).await? {
            self.create_entity_mentions_table().await?;
        }
        
        // Create kg_facts table if it doesn't exist
        if !self.table_exists(schema::TABLE_KG_FACTS).await? {
            self.create_kg_facts_table().await?;
        }
        
        // Create kg_conflicts table if it doesn't exist
        if !self.table_exists(schema::TABLE_KG_CONFLICTS).await? {
            self.create_kg_conflicts_table().await?;
        }
        
        Ok(())
    }
    
    /// Check if a table exists.
    pub async fn table_exists(&self, name: &str) -> Result<bool> {
        let tables = self.conn.table_names().execute().await?;
        Ok(tables.contains(&name.to_string()))
    }
    
    /// Create the papers table with an empty schema.
    async fn create_papers_table(&self) -> Result<()> {
        let fields: Fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("doi", DataType::Utf8, true),
            Field::new("pmid", DataType::Utf8, true),
            Field::new("title", DataType::Utf8, false),
            Field::new("abstract_text", DataType::Utf8, true),
            Field::new("full_text", DataType::Utf8, true),
            Field::new("source", DataType::Utf8, false),
            Field::new("source_id", DataType::Utf8, true),
            Field::new("published_at", DataType::Utf8, true),
            Field::new("authors", DataType::Utf8, true),
            Field::new("journal", DataType::Utf8, true),
            Field::new("volume", DataType::Utf8, true),
            Field::new("issue", DataType::Utf8, true),
            Field::new("pages", DataType::Utf8, true),
            Field::new("parse_status", DataType::Utf8, false),
            Field::new("ingested_at", DataType::Utf8, false),
            Field::new("abstract_simhash", DataType::Int64, true),
        ].into();
        
        let schema = Arc::new(Schema::new(fields));
        
        // Create empty iterator with schema
        let empty_iter = RecordBatchIterator::new(vec![], schema.clone());
        
        self.conn
            .create_table(schema::TABLE_PAPERS, empty_iter)
            .execute()
            .await?;
        
        Ok(())
    }
    
    /// Create the chunks table with embedding column.
    async fn create_chunks_table(&self) -> Result<()> {
        let embedding_field = Field::new(
            "embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, false)),
                schema::EMBEDDING_DIM as i32,
            ),
            true,
        );
        
        let fields: Fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("paper_id", DataType::Utf8, false),
            Field::new("chunk_index", DataType::Int64, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("section", DataType::Utf8, true),
            Field::new("page", DataType::Int64, true),
            Field::new("created_at", DataType::Utf8, false),
            embedding_field,
        ].into();
        
        let schema = Arc::new(Schema::new(fields));
        let empty_iter = RecordBatchIterator::new(vec![], schema);
        
        self.conn
            .create_table(schema::TABLE_CHUNKS, empty_iter)
            .execute()
            .await?;
        
        Ok(())
    }
    
    /// Create the entities table.
    async fn create_entities_table(&self) -> Result<()> {
        let fields: Fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("external_id", DataType::Utf8, false),
            Field::new("name", DataType::Utf8, false),
            Field::new("canonical_name", DataType::Utf8, true),
            Field::new("entity_type", DataType::Utf8, false),
            Field::new("synonyms", DataType::Utf8, true),
            Field::new("description", DataType::Utf8, true),
            Field::new("source_db", DataType::Utf8, false),
            Field::new("metadata", DataType::Utf8, true),
            Field::new("created_at", DataType::Utf8, false),
            Field::new("updated_at", DataType::Utf8, false),
        ].into();
        
        let schema = Arc::new(Schema::new(fields));
        let empty_iter = RecordBatchIterator::new(vec![], schema);
        
        self.conn
            .create_table(schema::TABLE_ENTITIES, empty_iter)
            .execute()
            .await?;
        
        Ok(())
    }
    
    /// Create the entity_mentions table.
    async fn create_entity_mentions_table(&self) -> Result<()> {
        let fields: Fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("entity_id", DataType::Utf8, false),
            Field::new("chunk_id", DataType::Utf8, false),
            Field::new("paper_id", DataType::Utf8, false),
            Field::new("start_offset", DataType::Int64, false),
            Field::new("end_offset", DataType::Int64, false),
            Field::new("text", DataType::Utf8, false),
            Field::new("confidence", DataType::Float32, true),
            Field::new("context", DataType::Utf8, true),
            Field::new("created_at", DataType::Utf8, false),
        ].into();
        
        let schema = Arc::new(Schema::new(fields));
        let empty_iter = RecordBatchIterator::new(vec![], schema);
        
        self.conn
            .create_table(schema::TABLE_ENTITY_MENTIONS, empty_iter)
            .execute()
            .await?;
        
        Ok(())
    }
    
    /// Create the kg_facts table.
    async fn create_kg_facts_table(&self) -> Result<()> {
        let fields: Fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("paper_id", DataType::Utf8, false),
            Field::new("subject_id", DataType::Utf8, false),
            Field::new("subject_name", DataType::Utf8, false),
            Field::new("predicate", DataType::Utf8, false),
            Field::new("object_id", DataType::Utf8, false),
            Field::new("object_name", DataType::Utf8, false),
            Field::new("confidence", DataType::Float32, true),
            Field::new("evidence", DataType::Utf8, true),
            Field::new("created_at", DataType::Utf8, false),
        ].into();
        
        let schema = Arc::new(Schema::new(fields));
        let empty_iter = RecordBatchIterator::new(vec![], schema);
        
        self.conn
            .create_table(schema::TABLE_KG_FACTS, empty_iter)
            .execute()
            .await?;
        
        Ok(())
    }
    
    /// Create the kg_conflicts table.
    async fn create_kg_conflicts_table(&self) -> Result<()> {
        let fields: Fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("fact_a_id", DataType::Utf8, false),
            Field::new("fact_b_id", DataType::Utf8, false),
            Field::new("conflict_type", DataType::Utf8, false),
            Field::new("net_confidence", DataType::Float32, false),
            Field::new("resolution", DataType::Utf8, false),
            Field::new("detected_at", DataType::Utf8, false),
        ].into();
        
        let schema = Arc::new(Schema::new(fields));
        let empty_iter = RecordBatchIterator::new(vec![], schema);
        
        self.conn
            .create_table(schema::TABLE_KG_CONFLICTS, empty_iter)
            .execute()
            .await?;
        
        Ok(())
    }
    
    /// Create a vector index on the chunks table for embedding search.
    pub async fn create_vector_index(&self) -> Result<()> {
        let table = self.conn
            .open_table(schema::TABLE_CHUNKS)
            .execute()
            .await?;
        
        table
            .create_index(
                &["embedding"],
                lancedb::index::Index::Auto,
            )
            .execute()
            .await?;
        
        Ok(())
    }
    
    /// Optimize all tables.
    pub async fn optimize(&self) -> Result<()> {
        let tables = self.conn.table_names().execute().await?;
        
        for table_name in tables {
            let table = self.conn
                .open_table(&table_name)
                .execute()
                .await?;
            table.optimize(lancedb::table::OptimizeAction::default()).await?;
        }
        
        Ok(())
    }
    
    /// Get table statistics.
    pub async fn stats(&self) -> Result<DatabaseStats> {
        let papers_count = if self.table_exists(schema::TABLE_PAPERS).await? {
            let table = self.conn.open_table(schema::TABLE_PAPERS).execute().await?;
            table.count_rows(None).await? as u64
        } else {
            0
        };
        
        let chunks_count = if self.table_exists(schema::TABLE_CHUNKS).await? {
            let table = self.conn.open_table(schema::TABLE_CHUNKS).execute().await?;
            table.count_rows(None).await? as u64
        } else {
            0
        };
        
        let entities_count = if self.table_exists(schema::TABLE_ENTITIES).await? {
            let table = self.conn.open_table(schema::TABLE_ENTITIES).execute().await?;
            table.count_rows(None).await? as u64
        } else {
            0
        };
        
        let mentions_count = if self.table_exists(schema::TABLE_ENTITY_MENTIONS).await? {
            let table = self.conn.open_table(schema::TABLE_ENTITY_MENTIONS).execute().await?;
            table.count_rows(None).await? as u64
        } else {
            0
        };
        
        let facts_count = if self.table_exists(schema::TABLE_KG_FACTS).await? {
            let table = self.conn.open_table(schema::TABLE_KG_FACTS).execute().await?;
            table.count_rows(None).await? as u64
        } else {
            0
        };
        
        Ok(DatabaseStats {
            papers: papers_count,
            chunks: chunks_count,
            entities: entities_count,
            entity_mentions: mentions_count,
            kg_facts: facts_count,
        })
    }
}

/// Database statistics.
#[derive(Debug, Clone, Default)]
pub struct DatabaseStats {
    pub papers: u64,
    pub chunks: u64,
    pub entities: u64,
    pub entity_mentions: u64,
    pub kg_facts: u64,
}
