//! Database connection and table management.
//!
//! Provides PostgreSQL with pgvector operations.

use crate::error::{DbError, Result};
use crate::schema;
use std::collections::HashSet;
use std::path::Path;

use tokio_postgres::{NoTls, Row};

/// Main database handle.
pub struct Database {
    client: tokio_postgres::Client,
    path: String,
}

impl Database {
    async fn table_names_set(&self) -> Result<HashSet<String>> {
        let rows = self.client.query(
            "SELECT tablename FROM pg_tables WHERE schemaname = 'public'",
            &[],
        ).await?;
        Ok(rows.iter().map(|r| r.get::<_, String>(0)).collect())
    }

    /// Open PostgreSQL database using connection string.
    /// postgresql://user:pass@host:port/dbname
    pub async fn open(path_or_url: impl AsRef<Path>) -> Result<Self> {
        let path_str = path_or_url.as_ref().to_string_lossy().to_string();

        // Check if it's a connection string (contains ://)
        if path_str.contains("://") {
            let (client, connection) = tokio_postgres::connect(&path_str, NoTls).await?;
            
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    tracing::error!("PostgreSQL connection error: {}", e);
                }
            });

            return Ok(Self {
                client,
                path: path_str,
            });
        }

        // Otherwise, treat as directory path for LanceDB (legacy)
        if !path_or_url.as_ref().exists() {
            std::fs::create_dir_all(path_or_url.as_ref())?;
        }

        // Legacy LanceDB - try to connect or error
        Err(DbError::InvalidQuery(format!(
            "LanceDB path '{}' no longer supported. Use PostgreSQL connection string: postgresql://user:pass@localhost:5432/dbname",
            path_str
        )))
    }

    /// Open with explicit connection string (preferred method).
    pub async fn open_with_url(connection_string: &str) -> Result<Self> {
        let (client, connection) = tokio_postgres::connect(connection_string, NoTls).await?;
        
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                tracing::error!("PostgreSQL connection error: {}", e);
            }
        });

        Ok(Self {
            client,
            path: connection_string.to_string(),
        })
    }

    /// Get the underlying client.
    pub fn client(&self) -> &tokio_postgres::Client {
        &self.client
    }

    /// Get the database path/URL.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Execute a query.
    pub async fn execute(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64> {
        Ok(self.client.execute(query, params).await?)
    }

    /// Query results.
    pub async fn query(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Vec<Row>> {
        Ok(self.client.query(query, params).await?)
    }

    /// Execute a single-row query returning one row.
    pub async fn query_opt(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Option<Row>> {
        Ok(self.client.query_opt(query, params).await?)
    }

    /// Initialize all tables.
    pub async fn initialize(&self) -> Result<()> {
        let mut existing_tables = self.table_names_set().await?;

        // Enable pgvector extension
        self.client.execute("CREATE EXTENSION IF NOT EXISTS vector", &[]).await?;

        macro_rules! create_if_missing {
            ($table:expr, $create_sql:expr) => {
                if existing_tables.insert($table.to_string()) {
                    self.client.execute($create_sql, &[]).await?;
                    tracing::info!("Created table: {}", $table);
                }
            };
        }

        // Papers table
        create_if_missing!(schema::TABLE_PAPERS, r#"
            CREATE TABLE IF NOT EXISTS papers (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                doi TEXT UNIQUE,
                pmid TEXT,
                pmcid TEXT,
                title TEXT NOT NULL,
                abstract TEXT,
                authors JSONB,
                journal TEXT,
                pub_date DATE,
                source TEXT,
                open_access BOOLEAN DEFAULT FALSE,
                full_text_url TEXT,
                ingested_at TIMESTAMPTZ DEFAULT NOW()
            )"#);

        // Chunks table
        create_if_missing!(schema::TABLE_CHUNKS, r#"
            CREATE TABLE IF NOT EXISTS chunks (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                paper_id UUID REFERENCES papers(id) ON DELETE CASCADE,
                section_type TEXT,
                chunk_index INTEGER,
                content TEXT NOT NULL,
                token_count INTEGER,
                embedding vector(768),
                created_at TIMESTAMPTZ DEFAULT NOW()
            )"#);

        // Entities table
        create_if_missing!(schema::TABLE_ENTITIES, r#"
            CREATE TABLE IF NOT EXISTS entities (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                paper_id UUID REFERENCES papers(id) ON DELETE CASCADE,
                entity_type TEXT NOT NULL,  -- 'GENE', 'DISEASE', 'CHEMICAL'
                entity_text TEXT NOT NULL,
                normalized_id TEXT,
                score FLOAT,
                created_at TIMESTAMPTZ DEFAULT NOW()
            )"#);

        // Entity mentions table
        create_if_missing!(schema::TABLE_ENTITY_MENTIONS, r#"
            CREATE TABLE IF NOT EXISTS entity_mentions (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                entity_id UUID REFERENCES entities(id) ON DELETE CASCADE,
                chunk_id UUID REFERENCES chunks(id) ON DELETE CASCADE,
                paper_id UUID REFERENCES papers(id) ON DELETE CASCADE,
                start_offset INTEGER,
                end_offset INTEGER,
                text TEXT NOT NULL,
                confidence FLOAT,
                context TEXT,
                created_at TIMESTAMPTZ DEFAULT NOW()
            )"#);

        // KG Facts table
        create_if_missing!(schema::TABLE_KG_FACTS, r#"
            CREATE TABLE IF NOT EXISTS kg_facts (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                paper_id UUID REFERENCES papers(id) ON DELETE CASCADE,
                subject_id UUID,
                subject_name TEXT NOT NULL,
                predicate TEXT NOT NULL,
                object_id UUID,
                object_name TEXT NOT NULL,
                confidence FLOAT,
                evidence TEXT,
                evidence_type TEXT NOT NULL,
                study_type TEXT,
                sample_size INTEGER,
                valid_from TIMESTAMPTZ DEFAULT NOW(),
                valid_until TIMESTAMPTZ,
                created_at TIMESTAMPTZ DEFAULT NOW()
            )"#);

        // Target scores table
        create_if_missing!(schema::TABLE_TARGET_SCORES, r#"
            CREATE TABLE IF NOT EXISTS target_scores (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                gene_id UUID,
                cancer_id UUID,
                score_version BIGINT DEFAULT 1,
                is_current BOOLEAN DEFAULT TRUE,
                composite_score DOUBLE PRECISION NOT NULL,
                confidence_adjusted_score DOUBLE PRECISION NOT NULL,
                penalty_score DOUBLE PRECISION NOT NULL,
                shortlist_tier TEXT NOT NULL,
                components_raw TEXT NOT NULL,
                components_normed TEXT NOT NULL,
                created_at TIMESTAMPTZ DEFAULT NOW(),
                UNIQUE(gene_id, cancer_id)
            )"#);

        // KG Conflicts table
        create_if_missing!(schema::TABLE_KG_CONFLICTS, r#"
            CREATE TABLE IF NOT EXISTS kg_conflicts (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                fact_a_id UUID NOT NULL,
                fact_b_id UUID NOT NULL,
                conflict_type TEXT NOT NULL,
                net_confidence FLOAT NOT NULL,
                resolution TEXT NOT NULL,
                detected_at TIMESTAMPTZ DEFAULT NOW()
            )"#);

        // Workspace memory table
        create_if_missing!(schema::TABLE_WORKSPACE_MEMORY, r#"
            CREATE TABLE IF NOT EXISTS workspace_memory (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                scope TEXT,  -- 'global', 'thread', 'user'
                content TEXT,
                created_at TIMESTAMPTZ DEFAULT NOW()
            )"#);

        // Create indexes for performance
        self.client.execute(
            "CREATE INDEX IF NOT EXISTS idx_papers_doi ON papers(doi)",
            &[]
        ).await?;
        self.client.execute(
            "CREATE INDEX IF NOT EXISTS idx_papers_pmid ON papers(pmid)",
            &[]
        ).await?;
        self.client.execute(
            "CREATE INDEX IF NOT EXISTS idx_chunks_paper_id ON chunks(paper_id)",
            &[]
        ).await?;
        self.client.execute(
            "CREATE INDEX IF NOT EXISTS idx_entities_paper_id ON entities(paper_id)",
            &[]
        ).await?;
        self.client.execute(
            "CREATE INDEX IF NOT EXISTS idx_kg_facts_paper_id ON kg_facts(paper_id)",
            &[]
        ).await?;
        self.client.execute(
            "CREATE INDEX IF NOT EXISTS idx_kg_facts_subject_id ON kg_facts(subject_id)",
            &[]
        ).await?;
        self.client.execute(
            "CREATE INDEX IF NOT EXISTS idx_kg_facts_object_id ON kg_facts(object_id)",
            &[]
        ).await?;
        self.client.execute(
            "CREATE INDEX IF NOT EXISTS idx_target_scores_gene_id ON target_scores(gene_id)",
            &[]
        ).await?;
        self.client.execute(
            "CREATE INDEX IF NOT EXISTS idx_target_scores_cancer_id ON target_scores(cancer_id)",
            &[]
        ).await?;
        self.client.execute(
            "CREATE INDEX IF NOT EXISTS idx_target_scores_current ON target_scores(is_current) WHERE is_current = true",
            &[]
        ).await?;

        // Create vector index
        self.client.execute(
            "CREATE INDEX IF NOT EXISTS idx_chunks_embedding ON chunks USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100)",
            &[]
        ).await?;

        tracing::info!("PostgreSQL tables initialized successfully");
        Ok(())
    }

    /// Check if a table exists.
    pub async fn table_exists(&self, name: &str) -> Result<bool> {
        Ok(self.table_names_set().await?.contains(name))
    }
}
