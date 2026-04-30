-- init-db.sql - Ferrumyx PostgreSQL Database Initialization
-- This script creates the core database schema for the Ferrumyx oncology research platform

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS vector;

-- Papers table - Stores research paper metadata
CREATE TABLE IF NOT EXISTS papers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    doi TEXT UNIQUE,
    pmid TEXT,
    pmcid TEXT,
    title TEXT NOT NULL,
    abstract_text TEXT,
    full_text TEXT,
    raw_json JSONB,
    source TEXT NOT NULL,
    source_id TEXT,
    published_at TIMESTAMPTZ,
    authors JSONB,
    journal TEXT,
    volume TEXT,
    issue TEXT,
    pages TEXT,
    parse_status TEXT DEFAULT 'pending',
    open_access BOOLEAN DEFAULT FALSE,
    retrieval_tier INTEGER,
    ingested_at TIMESTAMPTZ DEFAULT NOW(),
    abstract_simhash BIGINT,
    published_version_doi TEXT
);

-- Chunks table - Document chunks with embeddings
CREATE TABLE IF NOT EXISTS chunks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    paper_id UUID REFERENCES papers(id) ON DELETE CASCADE,
    chunk_index BIGINT NOT NULL,
    token_count INTEGER,
    content TEXT NOT NULL,
    section TEXT,
    page BIGINT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    embedding vector(768),
    embedding_large vector(768)
);

-- Entities table - Named entities extracted from papers
CREATE TABLE IF NOT EXISTS entities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    paper_id UUID REFERENCES papers(id) ON DELETE CASCADE,
    entity_type TEXT NOT NULL,
    entity_text TEXT NOT NULL,
    normalized_id TEXT,
    score FLOAT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Entity mentions table - Locations of entities in text
CREATE TABLE IF NOT EXISTS entity_mentions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID REFERENCES entities(id) ON DELETE CASCADE,
    chunk_id UUID REFERENCES chunks(id) ON DELETE CASCADE,
    paper_id UUID REFERENCES papers(id) ON DELETE CASCADE,
    start_offset BIGINT NOT NULL,
    end_offset BIGINT NOT NULL,
    text TEXT NOT NULL,
    confidence FLOAT,
    context TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Knowledge graph facts table
CREATE TABLE IF NOT EXISTS kg_facts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    paper_id UUID REFERENCES papers(id) ON DELETE CASCADE,
    subject_id UUID NOT NULL,
    subject_name TEXT NOT NULL,
    predicate TEXT NOT NULL,
    object_id UUID NOT NULL,
    object_name TEXT NOT NULL,
    confidence FLOAT DEFAULT 1.0,
    evidence TEXT,
    evidence_type TEXT NOT NULL DEFAULT 'unknown',
    study_type TEXT,
    sample_size INTEGER,
    valid_from TIMESTAMPTZ DEFAULT NOW(),
    valid_until TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Knowledge graph conflicts table
CREATE TABLE IF NOT EXISTS kg_conflicts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fact_a_id UUID NOT NULL,
    fact_b_id UUID NOT NULL,
    conflict_type TEXT NOT NULL,
    net_confidence FLOAT NOT NULL,
    resolution TEXT NOT NULL,
    detected_at TIMESTAMPTZ DEFAULT NOW()
);

-- Target scores table - Drug target prioritization
CREATE TABLE IF NOT EXISTS target_scores (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene_id UUID NOT NULL,
    cancer_id UUID NOT NULL,
    score_version BIGINT DEFAULT 1,
    is_current BOOLEAN DEFAULT TRUE,
    composite_score DOUBLE PRECISION NOT NULL,
    confidence_adjusted_score DOUBLE PRECISION NOT NULL,
    penalty_score DOUBLE PRECISION NOT NULL,
    shortlist_tier TEXT NOT NULL,
    components_raw TEXT NOT NULL DEFAULT '{}',
    components_normed TEXT NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(gene_id, cancer_id)
);

-- Workspace memory table - AI agent memory
CREATE TABLE IF NOT EXISTS workspace_memory (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scope TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Entity enrichment tables for Phase 3
CREATE TABLE IF NOT EXISTS ent_genes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    hgnc_id TEXT,
    symbol TEXT NOT NULL,
    name TEXT,
    uniprot_id TEXT,
    ensembl_id TEXT,
    entrez_id TEXT,
    gene_biotype TEXT,
    chromosome TEXT,
    strand SMALLINT,
    aliases JSONB,
    oncogene_flag BOOLEAN DEFAULT FALSE,
    tsg_flag BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ent_mutations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene_id UUID REFERENCES ent_genes(id) ON DELETE CASCADE,
    hgvs_p TEXT,
    hgvs_c TEXT,
    rs_id TEXT,
    aa_ref TEXT,
    aa_alt TEXT,
    aa_position INTEGER,
    oncogenicity TEXT,
    hotspot_flag BOOLEAN DEFAULT FALSE,
    vaf_context TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ent_cancer_types (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    oncotree_code TEXT,
    oncotree_name TEXT,
    icd_o3_code TEXT,
    tissue TEXT,
    parent_code TEXT,
    level INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ent_pathways (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    kegg_id TEXT,
    reactome_id TEXT,
    go_term TEXT,
    name TEXT NOT NULL,
    gene_members JSONB,
    source TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ent_clinical_evidence (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    nct_id TEXT,
    pmid TEXT,
    doi TEXT,
    phase TEXT,
    intervention TEXT,
    target_gene_id UUID REFERENCES ent_genes(id),
    cancer_id UUID REFERENCES ent_cancer_types(id),
    primary_endpoint TEXT,
    outcome TEXT,
    evidence_grade TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ent_compounds (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chembl_id TEXT,
    name TEXT,
    smiles TEXT,
    inchi_key TEXT,
    moa TEXT,
    patent_status TEXT,
    max_phase INTEGER,
    target_gene_ids JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ent_structures (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene_id UUID REFERENCES ent_genes(id),
    pdb_ids JSONB,
    best_resolution REAL,
    exp_method TEXT,
    af_accession TEXT,
    af_plddt_mean REAL,
    af_plddt_active REAL,
    has_pdb BOOLEAN DEFAULT FALSE,
    has_alphafold BOOLEAN DEFAULT FALSE,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ent_druggability (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    structure_id UUID REFERENCES ent_structures(id),
    fpocket_score REAL,
    fpocket_volume REAL,
    fpocket_pocket_count INTEGER,
    dogsitescorer REAL,
    overall_assessment TEXT,
    assessed_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ent_synthetic_lethality (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene1_id UUID REFERENCES ent_genes(id),
    gene2_id UUID REFERENCES ent_genes(id),
    cancer_id UUID REFERENCES ent_cancer_types(id),
    evidence_type TEXT,
    source_db TEXT,
    screen_id TEXT,
    effect_size REAL,
    confidence REAL,
    pmid TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- External data provider tables
CREATE TABLE IF NOT EXISTS ent_tcga_survival (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene_symbol TEXT NOT NULL,
    cancer_code TEXT NOT NULL,
    tcga_project_id TEXT NOT NULL,
    survival_score DOUBLE PRECISION NOT NULL,
    source TEXT NOT NULL,
    fetched_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ent_cbio_mutation_frequency (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene_symbol TEXT NOT NULL,
    cancer_code TEXT NOT NULL,
    study_id TEXT NOT NULL,
    molecular_profile_id TEXT NOT NULL,
    sample_list_id TEXT NOT NULL,
    mutated_sample_count BIGINT NOT NULL,
    profiled_sample_count BIGINT NOT NULL,
    mutation_frequency DOUBLE PRECISION NOT NULL,
    source TEXT NOT NULL,
    fetched_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ent_cosmic_mutation_frequency (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene_symbol TEXT NOT NULL,
    cancer_code TEXT NOT NULL,
    mutated_sample_count BIGINT NOT NULL,
    profiled_sample_count BIGINT NOT NULL,
    mutation_frequency DOUBLE PRECISION NOT NULL,
    source TEXT NOT NULL,
    fetched_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ent_gtex_expression (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene_symbol TEXT NOT NULL,
    expression_score DOUBLE PRECISION NOT NULL,
    source TEXT NOT NULL,
    fetched_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ent_chembl_targets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene_symbol TEXT NOT NULL,
    inhibitor_count BIGINT NOT NULL,
    source TEXT NOT NULL,
    fetched_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ent_reactome_genes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene_symbol TEXT NOT NULL,
    pathway_count BIGINT NOT NULL,
    source TEXT NOT NULL,
    fetched_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ent_provider_refresh_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL,
    finished_at TIMESTAMPTZ NOT NULL,
    genes_requested BIGINT NOT NULL,
    genes_processed BIGINT NOT NULL,
    attempted BIGINT NOT NULL,
    success BIGINT NOT NULL,
    failed BIGINT NOT NULL,
    skipped BIGINT NOT NULL,
    duration_ms BIGINT NOT NULL,
    error_rate DOUBLE PRECISION NOT NULL,
    cadence_interval_secs BIGINT NOT NULL,
    trigger_reason TEXT NOT NULL
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_papers_doi ON papers(doi);
CREATE INDEX IF NOT EXISTS idx_papers_pmid ON papers(pmid);
CREATE INDEX IF NOT EXISTS idx_papers_ingested_at ON papers(ingested_at);
CREATE INDEX IF NOT EXISTS idx_chunks_paper_id ON chunks(paper_id);
CREATE INDEX IF NOT EXISTS idx_chunks_embedding ON chunks USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);
CREATE INDEX IF NOT EXISTS idx_chunks_embedding_large ON chunks USING ivfflat (embedding_large vector_cosine_ops) WITH (lists = 100);
CREATE INDEX IF NOT EXISTS idx_entities_paper_id ON entities(paper_id);
CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);
CREATE INDEX IF NOT EXISTS idx_entity_mentions_paper_id ON entity_mentions(paper_id);
CREATE INDEX IF NOT EXISTS idx_entity_mentions_chunk_id ON entity_mentions(chunk_id);
CREATE INDEX IF NOT EXISTS idx_kg_facts_paper_id ON kg_facts(paper_id);
CREATE INDEX IF NOT EXISTS idx_kg_facts_subject_id ON kg_facts(subject_id);
CREATE INDEX IF NOT EXISTS idx_kg_facts_object_id ON kg_facts(object_id);
CREATE INDEX IF NOT EXISTS idx_kg_facts_predicate ON kg_facts(predicate);
CREATE INDEX IF NOT EXISTS idx_target_scores_gene_id ON target_scores(gene_id);
CREATE INDEX IF NOT EXISTS idx_target_scores_cancer_id ON target_scores(cancer_id);
CREATE INDEX IF NOT EXISTS idx_target_scores_current ON target_scores(is_current) WHERE is_current = true;

-- Create user for application
DO $$
BEGIN
   IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'ferrumyx') THEN
      CREATE USER ferrumyx;
   END IF;
END
$$;

-- Grant permissions
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO ferrumyx;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO ferrumyx;

-- Create database role for read-only access (for monitoring/analytics)
DO $$
BEGIN
   IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'ferrumyx_readonly') THEN
      CREATE USER ferrumyx_readonly;
   END IF;
END
$$;

GRANT SELECT ON ALL TABLES IN SCHEMA public TO ferrumyx_readonly;