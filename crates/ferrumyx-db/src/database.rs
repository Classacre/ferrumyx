//! Database connection and table management.
//!
//! Provides a unified interface for LanceDB operations.

use crate::error::Result;
use crate::schema;
use arrow_array::RecordBatchIterator;
use arrow_schema::{DataType, Field, Fields, Schema};
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

        let conn = lancedb::connect(&path_str).execute().await?;

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

        // Create target_scores table if it doesn't exist
        if !self.table_exists(schema::TABLE_TARGET_SCORES).await? {
            self.create_target_scores_table().await?;
        }

        // Create ingestion_audit table if it doesn't exist
        if !self.table_exists(schema::TABLE_INGESTION_AUDIT).await? {
            self.create_ingestion_audit_table().await?;
        }

        // Entity Stage Tables (Phase 3)
        if !self.table_exists(schema::TABLE_ENT_GENES).await? {
            self.create_ent_genes_table().await?;
        }
        if !self.table_exists(schema::TABLE_ENT_MUTATIONS).await? {
            self.create_ent_mutations_table().await?;
        }
        if !self.table_exists(schema::TABLE_ENT_CANCER_TYPES).await? {
            self.create_ent_cancer_types_table().await?;
        }
        if !self.table_exists(schema::TABLE_ENT_PATHWAYS).await? {
            self.create_ent_pathways_table().await?;
        }
        if !self
            .table_exists(schema::TABLE_ENT_CLINICAL_EVIDENCE)
            .await?
        {
            self.create_ent_clinical_evidence_table().await?;
        }
        if !self.table_exists(schema::TABLE_ENT_COMPOUNDS).await? {
            self.create_ent_compounds_table().await?;
        }
        if !self.table_exists(schema::TABLE_ENT_STRUCTURES).await? {
            self.create_ent_structures_table().await?;
        }
        if !self.table_exists(schema::TABLE_ENT_DRUGGABILITY).await? {
            self.create_ent_druggability_table().await?;
        }
        if !self
            .table_exists(schema::TABLE_ENT_SYNTHETIC_LETHALITY)
            .await?
        {
            self.create_ent_synthetic_lethality_table().await?;
        }
        if !self.table_exists(schema::TABLE_ENT_TCGA_SURVIVAL).await? {
            self.create_ent_tcga_survival_table().await?;
        }
        if !self.table_exists(schema::TABLE_ENT_GTEX_EXPRESSION).await? {
            self.create_ent_gtex_expression_table().await?;
        }
        if !self.table_exists(schema::TABLE_ENT_CHEMBL_TARGETS).await? {
            self.create_ent_chembl_targets_table().await?;
        }
        if !self.table_exists(schema::TABLE_ENT_REACTOME_GENES).await? {
            self.create_ent_reactome_genes_table().await?;
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
            Field::new("raw_json", DataType::Utf8, true),
            Field::new("source", DataType::Utf8, false),
            Field::new("source_id", DataType::Utf8, true),
            Field::new("published_at", DataType::Utf8, true),
            Field::new("authors", DataType::Utf8, true),
            Field::new("journal", DataType::Utf8, true),
            Field::new("volume", DataType::Utf8, true),
            Field::new("issue", DataType::Utf8, true),
            Field::new("pages", DataType::Utf8, true),
            Field::new("parse_status", DataType::Utf8, false),
            Field::new("open_access", DataType::Boolean, false),
            Field::new("retrieval_tier", DataType::Int32, true),
            Field::new("ingested_at", DataType::Utf8, false),
            Field::new("abstract_simhash", DataType::Int64, true),
            Field::new("published_version_doi", DataType::Utf8, true),
        ]
        .into();

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
            Field::new("token_count", DataType::Int32, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("section", DataType::Utf8, true),
            Field::new("page", DataType::Int64, true),
            Field::new("created_at", DataType::Utf8, false),
            embedding_field,
            Field::new(
                "embedding_large",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, false)),
                    1024 as i32,
                ),
                true,
            ),
        ]
        .into();

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
        ]
        .into();

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
        ]
        .into();

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
            Field::new("confidence", DataType::Float32, false),
            Field::new("evidence", DataType::Utf8, true),
            Field::new("evidence_type", DataType::Utf8, false),
            Field::new("study_type", DataType::Utf8, true),
            Field::new("sample_size", DataType::Int32, true),
            Field::new("valid_from", DataType::Utf8, false),
            Field::new("valid_until", DataType::Utf8, true),
            Field::new("created_at", DataType::Utf8, false),
        ]
        .into();

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
        ]
        .into();

        let schema = Arc::new(Schema::new(fields));
        let empty_iter = RecordBatchIterator::new(vec![], schema);

        self.conn
            .create_table(schema::TABLE_KG_CONFLICTS, empty_iter)
            .execute()
            .await?;

        Ok(())
    }

    /// Create the target_scores table.
    async fn create_target_scores_table(&self) -> Result<()> {
        let fields: Fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("gene_id", DataType::Utf8, false),
            Field::new("cancer_id", DataType::Utf8, false),
            Field::new("score_version", DataType::Int64, false),
            Field::new("is_current", DataType::Boolean, false),
            Field::new("composite_score", DataType::Float64, false),
            Field::new("confidence_adjusted_score", DataType::Float64, false),
            Field::new("penalty_score", DataType::Float64, false),
            Field::new("shortlist_tier", DataType::Utf8, false),
            Field::new("components_raw", DataType::Utf8, false),
            Field::new("components_normed", DataType::Utf8, false),
            Field::new("created_at", DataType::Utf8, false),
        ]
        .into();

        let schema = Arc::new(Schema::new(fields));
        let empty_iter = RecordBatchIterator::new(vec![], schema);

        self.conn
            .create_table(schema::TABLE_TARGET_SCORES, empty_iter)
            .execute()
            .await?;

        Ok(())
    }

    /// Create the ingestion_audit table.
    async fn create_ingestion_audit_table(&self) -> Result<()> {
        let fields: Fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("job_id", DataType::Utf8, true),
            Field::new("paper_id", DataType::Utf8, true),
            Field::new("action", DataType::Utf8, false),
            Field::new("detail", DataType::Utf8, false),
            Field::new("created_at", DataType::Utf8, false),
        ]
        .into();

        let schema = Arc::new(Schema::new(fields));
        let empty_iter = RecordBatchIterator::new(vec![], schema);

        self.conn
            .create_table(schema::TABLE_INGESTION_AUDIT, empty_iter)
            .execute()
            .await?;

        Ok(())
    }

    /// Create a vector index on the chunks table for embedding search.
    pub async fn create_vector_index(&self) -> Result<()> {
        let table = self.conn.open_table(schema::TABLE_CHUNKS).execute().await?;

        table
            .create_index(&["embedding"], lancedb::index::Index::Auto)
            .execute()
            .await?;

        Ok(())
    }

    /// Optimize all tables.
    pub async fn optimize(&self) -> Result<()> {
        let tables = self.conn.table_names().execute().await?;

        for table_name in tables {
            let table = self.conn.open_table(&table_name).execute().await?;
            table
                .optimize(lancedb::table::OptimizeAction::default())
                .await?;
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
            let table = self
                .conn
                .open_table(schema::TABLE_ENTITIES)
                .execute()
                .await?;
            table.count_rows(None).await? as u64
        } else {
            0
        };

        let mentions_count = if self.table_exists(schema::TABLE_ENTITY_MENTIONS).await? {
            let table = self
                .conn
                .open_table(schema::TABLE_ENTITY_MENTIONS)
                .execute()
                .await?;
            table.count_rows(None).await? as u64
        } else {
            0
        };

        let facts_count = if self.table_exists(schema::TABLE_KG_FACTS).await? {
            let table = self
                .conn
                .open_table(schema::TABLE_KG_FACTS)
                .execute()
                .await?;
            table.count_rows(None).await? as u64
        } else {
            0
        };

        let target_scores_count = if self.table_exists(schema::TABLE_TARGET_SCORES).await? {
            let table = self
                .conn
                .open_table(schema::TABLE_TARGET_SCORES)
                .execute()
                .await?;
            table.count_rows(None).await? as u64
        } else {
            0
        };

        let ingestion_audit_count = if self.table_exists(schema::TABLE_INGESTION_AUDIT).await? {
            let table = self
                .conn
                .open_table(schema::TABLE_INGESTION_AUDIT)
                .execute()
                .await?;
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
            target_scores: target_scores_count,
            ingestion_audit: ingestion_audit_count,
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
    pub target_scores: u64,
    pub ingestion_audit: u64,
}

// =============================================================================
// Phase 3 Entity Table Creation
// =============================================================================
impl Database {
    pub async fn create_ent_genes_table(&self) -> Result<()> {
        let fields: Fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("hgnc_id", DataType::Utf8, true),
            Field::new("symbol", DataType::Utf8, false),
            Field::new("name", DataType::Utf8, true),
            Field::new("uniprot_id", DataType::Utf8, true),
            Field::new("ensembl_id", DataType::Utf8, true),
            Field::new("entrez_id", DataType::Utf8, true),
            Field::new("gene_biotype", DataType::Utf8, true),
            Field::new("chromosome", DataType::Utf8, true),
            Field::new("strand", DataType::Int16, true),
            Field::new("aliases", DataType::Utf8, true),
            Field::new("oncogene_flag", DataType::Boolean, false),
            Field::new("tsg_flag", DataType::Boolean, false),
            Field::new("created_at", DataType::Utf8, false),
        ]
        .into();
        let schema = Arc::new(Schema::new(fields));
        let empty_iter = RecordBatchIterator::new(vec![], schema);
        self.conn
            .create_table(schema::TABLE_ENT_GENES, empty_iter)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn create_ent_mutations_table(&self) -> Result<()> {
        let fields: Fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("gene_id", DataType::Utf8, false),
            Field::new("hgvs_p", DataType::Utf8, true),
            Field::new("hgvs_c", DataType::Utf8, true),
            Field::new("rs_id", DataType::Utf8, true),
            Field::new("aa_ref", DataType::Utf8, true),
            Field::new("aa_alt", DataType::Utf8, true),
            Field::new("aa_position", DataType::Int32, true),
            Field::new("oncogenicity", DataType::Utf8, true),
            Field::new("hotspot_flag", DataType::Boolean, false),
            Field::new("vaf_context", DataType::Utf8, true),
            Field::new("created_at", DataType::Utf8, false),
        ]
        .into();
        let schema = Arc::new(Schema::new(fields));
        let empty_iter = RecordBatchIterator::new(vec![], schema);
        self.conn
            .create_table(schema::TABLE_ENT_MUTATIONS, empty_iter)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn create_ent_cancer_types_table(&self) -> Result<()> {
        let fields: Fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("oncotree_code", DataType::Utf8, true),
            Field::new("oncotree_name", DataType::Utf8, true),
            Field::new("icd_o3_code", DataType::Utf8, true),
            Field::new("tissue", DataType::Utf8, true),
            Field::new("parent_code", DataType::Utf8, true),
            Field::new("level", DataType::Int32, true),
            Field::new("created_at", DataType::Utf8, false),
        ]
        .into();
        let schema = Arc::new(Schema::new(fields));
        let empty_iter = RecordBatchIterator::new(vec![], schema);
        self.conn
            .create_table(schema::TABLE_ENT_CANCER_TYPES, empty_iter)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn create_ent_pathways_table(&self) -> Result<()> {
        let fields: Fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("kegg_id", DataType::Utf8, true),
            Field::new("reactome_id", DataType::Utf8, true),
            Field::new("go_term", DataType::Utf8, true),
            Field::new("name", DataType::Utf8, false),
            Field::new("gene_members", DataType::Utf8, true),
            Field::new("source", DataType::Utf8, true),
            Field::new("created_at", DataType::Utf8, false),
        ]
        .into();
        let schema = Arc::new(Schema::new(fields));
        let empty_iter = RecordBatchIterator::new(vec![], schema);
        self.conn
            .create_table(schema::TABLE_ENT_PATHWAYS, empty_iter)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn create_ent_clinical_evidence_table(&self) -> Result<()> {
        let fields: Fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("nct_id", DataType::Utf8, true),
            Field::new("pmid", DataType::Utf8, true),
            Field::new("doi", DataType::Utf8, true),
            Field::new("phase", DataType::Utf8, true),
            Field::new("intervention", DataType::Utf8, true),
            Field::new("target_gene_id", DataType::Utf8, false),
            Field::new("cancer_id", DataType::Utf8, false),
            Field::new("primary_endpoint", DataType::Utf8, true),
            Field::new("outcome", DataType::Utf8, true),
            Field::new("evidence_grade", DataType::Utf8, true),
            Field::new("created_at", DataType::Utf8, false),
        ]
        .into();
        let schema = Arc::new(Schema::new(fields));
        let empty_iter = RecordBatchIterator::new(vec![], schema);
        self.conn
            .create_table(schema::TABLE_ENT_CLINICAL_EVIDENCE, empty_iter)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn create_ent_compounds_table(&self) -> Result<()> {
        let fields: Fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("chembl_id", DataType::Utf8, true),
            Field::new("name", DataType::Utf8, true),
            Field::new("smiles", DataType::Utf8, true),
            Field::new("inchi_key", DataType::Utf8, true),
            Field::new("moa", DataType::Utf8, true),
            Field::new("patent_status", DataType::Utf8, true),
            Field::new("max_phase", DataType::Int32, true),
            Field::new("target_gene_ids", DataType::Utf8, true),
            Field::new("created_at", DataType::Utf8, false),
        ]
        .into();
        let schema = Arc::new(Schema::new(fields));
        let empty_iter = RecordBatchIterator::new(vec![], schema);
        self.conn
            .create_table(schema::TABLE_ENT_COMPOUNDS, empty_iter)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn create_ent_structures_table(&self) -> Result<()> {
        let fields: Fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("gene_id", DataType::Utf8, false),
            Field::new("pdb_ids", DataType::Utf8, true),
            Field::new("best_resolution", DataType::Float32, true),
            Field::new("exp_method", DataType::Utf8, true),
            Field::new("af_accession", DataType::Utf8, true),
            Field::new("af_plddt_mean", DataType::Float32, true),
            Field::new("af_plddt_active", DataType::Float32, true),
            Field::new("has_pdb", DataType::Boolean, false),
            Field::new("has_alphafold", DataType::Boolean, false),
            Field::new("updated_at", DataType::Utf8, false),
        ]
        .into();
        let schema = Arc::new(Schema::new(fields));
        let empty_iter = RecordBatchIterator::new(vec![], schema);
        self.conn
            .create_table(schema::TABLE_ENT_STRUCTURES, empty_iter)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn create_ent_druggability_table(&self) -> Result<()> {
        let fields: Fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("structure_id", DataType::Utf8, false),
            Field::new("fpocket_score", DataType::Float32, true),
            Field::new("fpocket_volume", DataType::Float32, true),
            Field::new("fpocket_pocket_count", DataType::Int32, true),
            Field::new("dogsitescorer", DataType::Float32, true),
            Field::new("overall_assessment", DataType::Utf8, true),
            Field::new("assessed_at", DataType::Utf8, false),
        ]
        .into();
        let schema = Arc::new(Schema::new(fields));
        let empty_iter = RecordBatchIterator::new(vec![], schema);
        self.conn
            .create_table(schema::TABLE_ENT_DRUGGABILITY, empty_iter)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn create_ent_synthetic_lethality_table(&self) -> Result<()> {
        let fields: Fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("gene1_id", DataType::Utf8, false),
            Field::new("gene2_id", DataType::Utf8, false),
            Field::new("cancer_id", DataType::Utf8, false),
            Field::new("evidence_type", DataType::Utf8, true),
            Field::new("source_db", DataType::Utf8, true),
            Field::new("screen_id", DataType::Utf8, true),
            Field::new("effect_size", DataType::Float32, true),
            Field::new("confidence", DataType::Float32, true),
            Field::new("pmid", DataType::Utf8, true),
            Field::new("created_at", DataType::Utf8, false),
        ]
        .into();
        let schema = Arc::new(Schema::new(fields));
        let empty_iter = RecordBatchIterator::new(vec![], schema);
        self.conn
            .create_table(schema::TABLE_ENT_SYNTHETIC_LETHALITY, empty_iter)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn create_ent_tcga_survival_table(&self) -> Result<()> {
        let fields: Fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("gene_symbol", DataType::Utf8, false),
            Field::new("cancer_code", DataType::Utf8, false),
            Field::new("tcga_project_id", DataType::Utf8, false),
            Field::new("survival_score", DataType::Float64, false),
            Field::new("source", DataType::Utf8, false),
            Field::new("fetched_at", DataType::Utf8, false),
        ]
        .into();
        let schema = Arc::new(Schema::new(fields));
        let empty_iter = RecordBatchIterator::new(vec![], schema);
        self.conn
            .create_table(schema::TABLE_ENT_TCGA_SURVIVAL, empty_iter)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn create_ent_gtex_expression_table(&self) -> Result<()> {
        let fields: Fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("gene_symbol", DataType::Utf8, false),
            Field::new("expression_score", DataType::Float64, false),
            Field::new("source", DataType::Utf8, false),
            Field::new("fetched_at", DataType::Utf8, false),
        ]
        .into();
        let schema = Arc::new(Schema::new(fields));
        let empty_iter = RecordBatchIterator::new(vec![], schema);
        self.conn
            .create_table(schema::TABLE_ENT_GTEX_EXPRESSION, empty_iter)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn create_ent_chembl_targets_table(&self) -> Result<()> {
        let fields: Fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("gene_symbol", DataType::Utf8, false),
            Field::new("inhibitor_count", DataType::Int64, false),
            Field::new("source", DataType::Utf8, false),
            Field::new("fetched_at", DataType::Utf8, false),
        ]
        .into();
        let schema = Arc::new(Schema::new(fields));
        let empty_iter = RecordBatchIterator::new(vec![], schema);
        self.conn
            .create_table(schema::TABLE_ENT_CHEMBL_TARGETS, empty_iter)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn create_ent_reactome_genes_table(&self) -> Result<()> {
        let fields: Fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("gene_symbol", DataType::Utf8, false),
            Field::new("pathway_count", DataType::Int64, false),
            Field::new("source", DataType::Utf8, false),
            Field::new("fetched_at", DataType::Utf8, false),
        ]
        .into();
        let schema = Arc::new(Schema::new(fields));
        let empty_iter = RecordBatchIterator::new(vec![], schema);
        self.conn
            .create_table(schema::TABLE_ENT_REACTOME_GENES, empty_iter)
            .execute()
            .await?;
        Ok(())
    }
}
