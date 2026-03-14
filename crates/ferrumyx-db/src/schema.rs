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
    pub raw_json: Option<String>,
    pub source: String,
    pub source_id: Option<String>,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
    pub authors: Option<String>,
    pub journal: Option<String>,
    pub volume: Option<String>,
    pub issue: Option<String>,
    pub pages: Option<String>,
    pub parse_status: String,
    pub open_access: bool,
    pub retrieval_tier: Option<i32>,
    pub ingested_at: chrono::DateTime<chrono::Utc>,
    pub abstract_simhash: Option<i64>,
    pub published_version_doi: Option<String>,
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
            raw_json: None,
            source,
            source_id: None,
            published_at: None,
            authors: None,
            journal: None,
            volume: None,
            issue: None,
            pages: None,
            parse_status: "pending".to_string(),
            open_access: false,
            retrieval_tier: None,
            ingested_at: chrono::Utc::now(),
            abstract_simhash: None,
            published_version_doi: None,
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
    pub token_count: i32,
    pub content: String,
    pub section: Option<String>,
    pub page: Option<i64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub embedding: Option<Vec<f32>>,
    pub embedding_large: Option<Vec<f32>>,
}

impl Chunk {
    pub fn new(paper_id: uuid::Uuid, chunk_index: i64, content: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            paper_id,
            chunk_index,
            token_count: 0,
            content,
            section: None,
            page: None,
            created_at: chrono::Utc::now(),
            embedding: None,
            embedding_large: None,
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
    pub fn new(
        entity_type: EntityType,
        name: String,
        external_id: String,
        source_db: String,
    ) -> Self {
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
// Specific Entity Type Schemas (Phase 3)
// =============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntGene {
    pub id: uuid::Uuid,
    pub hgnc_id: Option<String>,
    pub symbol: String,
    pub name: Option<String>,
    pub uniprot_id: Option<String>,
    pub ensembl_id: Option<String>,
    pub entrez_id: Option<String>,
    pub gene_biotype: Option<String>,
    pub chromosome: Option<String>,
    pub strand: Option<i16>,
    pub aliases: Option<Vec<String>>,
    pub oncogene_flag: bool,
    pub tsg_flag: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntMutation {
    pub id: uuid::Uuid,
    pub gene_id: uuid::Uuid,
    pub hgvs_p: Option<String>,
    pub hgvs_c: Option<String>,
    pub rs_id: Option<String>,
    pub aa_ref: Option<String>,
    pub aa_alt: Option<String>,
    pub aa_position: Option<i32>,
    pub oncogenicity: Option<String>,
    pub hotspot_flag: bool,
    pub vaf_context: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntCancerType {
    pub id: uuid::Uuid,
    pub oncotree_code: Option<String>,
    pub oncotree_name: Option<String>,
    pub icd_o3_code: Option<String>,
    pub tissue: Option<String>,
    pub parent_code: Option<String>,
    pub level: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntPathway {
    pub id: uuid::Uuid,
    pub kegg_id: Option<String>,
    pub reactome_id: Option<String>,
    pub go_term: Option<String>,
    pub name: String,
    pub gene_members: Option<Vec<String>>,
    pub source: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntClinicalEvidence {
    pub id: uuid::Uuid,
    pub nct_id: Option<String>,
    pub pmid: Option<String>,
    pub doi: Option<String>,
    pub phase: Option<String>,
    pub intervention: Option<String>,
    pub target_gene_id: uuid::Uuid,
    pub cancer_id: uuid::Uuid,
    pub primary_endpoint: Option<String>,
    pub outcome: Option<String>,
    pub evidence_grade: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntCompound {
    pub id: uuid::Uuid,
    pub chembl_id: Option<String>,
    pub name: Option<String>,
    pub smiles: Option<String>,
    pub inchi_key: Option<String>,
    pub moa: Option<String>,
    pub patent_status: Option<String>,
    pub max_phase: Option<i32>,
    pub target_gene_ids: Option<Vec<uuid::Uuid>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntStructure {
    pub id: uuid::Uuid,
    pub gene_id: uuid::Uuid,
    pub pdb_ids: Option<Vec<String>>,
    pub best_resolution: Option<f32>,
    pub exp_method: Option<String>,
    pub af_accession: Option<String>,
    pub af_plddt_mean: Option<f32>,
    pub af_plddt_active: Option<f32>,
    pub has_pdb: bool,
    pub has_alphafold: bool,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntDruggability {
    pub id: uuid::Uuid,
    pub structure_id: uuid::Uuid,
    pub fpocket_score: Option<f32>,
    pub fpocket_volume: Option<f32>,
    pub fpocket_pocket_count: Option<i32>,
    pub dogsitescorer: Option<f32>,
    pub overall_assessment: Option<String>,
    pub assessed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntSyntheticLethality {
    pub id: uuid::Uuid,
    pub gene1_id: uuid::Uuid,
    pub gene2_id: uuid::Uuid,
    pub cancer_id: uuid::Uuid,
    pub evidence_type: Option<String>,
    pub source_db: Option<String>,
    pub screen_id: Option<String>,
    pub effect_size: Option<f32>,
    pub confidence: Option<f32>,
    pub pmid: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntTcgaSurvival {
    pub id: uuid::Uuid,
    pub gene_symbol: String,
    pub cancer_code: String,
    pub tcga_project_id: String,
    pub survival_score: f64,
    pub source: String,
    pub fetched_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntCbioMutationFrequency {
    pub id: uuid::Uuid,
    pub gene_symbol: String,
    pub cancer_code: String,
    pub study_id: String,
    pub molecular_profile_id: String,
    pub sample_list_id: String,
    pub mutated_sample_count: i64,
    pub profiled_sample_count: i64,
    pub mutation_frequency: f64,
    pub source: String,
    pub fetched_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntGtexExpression {
    pub id: uuid::Uuid,
    pub gene_symbol: String,
    pub expression_score: f64,
    pub source: String,
    pub fetched_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntChemblTarget {
    pub id: uuid::Uuid,
    pub gene_symbol: String,
    pub inhibitor_count: i64,
    pub source: String,
    pub fetched_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntReactomeGene {
    pub id: uuid::Uuid,
    pub gene_symbol: String,
    pub pathway_count: i64,
    pub source: String,
    pub fetched_at: chrono::DateTime<chrono::Utc>,
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
    pub confidence: f32,
    pub evidence: Option<String>,
    pub evidence_type: String,
    pub study_type: Option<String>,
    pub sample_size: Option<i32>,
    pub valid_from: chrono::DateTime<chrono::Utc>,
    pub valid_until: Option<chrono::DateTime<chrono::Utc>>,
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
            confidence: 1.0,
            evidence: None,
            evidence_type: "unknown".to_string(),
            study_type: None,
            sample_size: None,
            valid_from: chrono::Utc::now(),
            valid_until: None,
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
    pub fn new(
        fact_a_id: uuid::Uuid,
        fact_b_id: uuid::Uuid,
        conflict_type: String,
        net_confidence: f32,
        resolution: String,
    ) -> Self {
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
// Target Score Schema
// =============================================================================

/// Scored target result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TargetScore {
    pub id: uuid::Uuid,
    pub gene_id: uuid::Uuid,
    pub cancer_id: uuid::Uuid,
    pub score_version: i64,
    pub is_current: bool,
    pub composite_score: f64,
    pub confidence_adjusted_score: f64,
    pub penalty_score: f64,
    pub shortlist_tier: String,
    pub components_raw: String,
    pub components_normed: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl TargetScore {
    pub fn new(
        gene_id: uuid::Uuid,
        cancer_id: uuid::Uuid,
        composite_score: f64,
        confidence_adjusted_score: f64,
        penalty_score: f64,
        shortlist_tier: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            gene_id,
            cancer_id,
            score_version: 1,
            is_current: true,
            composite_score,
            confidence_adjusted_score,
            penalty_score,
            shortlist_tier,
            components_raw: "{}".to_string(),
            components_normed: "{}".to_string(),
            created_at: chrono::Utc::now(),
        }
    }
}

// =============================================================================
// Ingestion Audit Schema
// =============================================================================

/// Audit log for ingestion pipeline
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IngestionAudit {
    pub id: uuid::Uuid,
    pub job_id: Option<uuid::Uuid>,
    pub paper_id: Option<uuid::Uuid>,
    pub action: String,
    pub detail: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl IngestionAudit {
    pub fn new(action: String, detail: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            job_id: None,
            paper_id: None,
            action,
            detail,
            created_at: chrono::Utc::now(),
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
pub const TABLE_TARGET_SCORES: &str = "target_scores";
pub const TABLE_INGESTION_AUDIT: &str = "ingestion_audit";

// Entropy specific tables
pub const TABLE_ENT_GENES: &str = "ent_genes";
pub const TABLE_ENT_MUTATIONS: &str = "ent_mutations";
pub const TABLE_ENT_CANCER_TYPES: &str = "ent_cancer_types";
pub const TABLE_ENT_PATHWAYS: &str = "ent_pathways";
pub const TABLE_ENT_CLINICAL_EVIDENCE: &str = "ent_clinical_evidence";
pub const TABLE_ENT_COMPOUNDS: &str = "ent_compounds";
pub const TABLE_ENT_STRUCTURES: &str = "ent_structures";
pub const TABLE_ENT_DRUGGABILITY: &str = "ent_druggability";
pub const TABLE_ENT_SYNTHETIC_LETHALITY: &str = "ent_synthetic_lethality";
pub const TABLE_ENT_TCGA_SURVIVAL: &str = "ent_tcga_survival";
pub const TABLE_ENT_CBIO_MUTATION_FREQUENCY: &str = "ent_cbio_mutation_frequency";
pub const TABLE_ENT_GTEX_EXPRESSION: &str = "ent_gtex_expression";
pub const TABLE_ENT_CHEMBL_TARGETS: &str = "ent_chembl_targets";
pub const TABLE_ENT_REACTOME_GENES: &str = "ent_reactome_genes";
