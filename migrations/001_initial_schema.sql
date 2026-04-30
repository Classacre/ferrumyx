-- migrations/001_initial_schema.sql
-- Initial database schema for Ferrumyx v0.1.0

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS vector;

-- Papers table
CREATE TABLE papers (
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

-- Chunks table
CREATE TABLE chunks (
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

-- Entities table
CREATE TABLE entities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    paper_id UUID REFERENCES papers(id) ON DELETE CASCADE,
    entity_type TEXT NOT NULL,
    entity_text TEXT NOT NULL,
    normalized_id TEXT,
    score FLOAT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Entity mentions table
CREATE TABLE entity_mentions (
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
CREATE TABLE kg_facts (
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
CREATE TABLE kg_conflicts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fact_a_id UUID NOT NULL,
    fact_b_id UUID NOT NULL,
    conflict_type TEXT NOT NULL,
    net_confidence FLOAT NOT NULL,
    resolution TEXT NOT NULL,
    detected_at TIMESTAMPTZ DEFAULT NOW()
);

-- Target scores table
CREATE TABLE target_scores (
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

-- Workspace memory table
CREATE TABLE workspace_memory (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scope TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create indexes
CREATE INDEX idx_papers_doi ON papers(doi);
CREATE INDEX idx_papers_pmid ON papers(pmid);
CREATE INDEX idx_papers_ingested_at ON papers(ingested_at);
CREATE INDEX idx_chunks_paper_id ON chunks(paper_id);
CREATE INDEX idx_chunks_embedding ON chunks USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);
CREATE INDEX idx_entities_paper_id ON entities(paper_id);
CREATE INDEX idx_entities_type ON entities(entity_type);
CREATE INDEX idx_kg_facts_paper_id ON kg_facts(paper_id);
CREATE INDEX idx_target_scores_gene_id ON target_scores(gene_id);
CREATE INDEX idx_target_scores_cancer_id ON target_scores(cancer_id);
CREATE INDEX idx_target_scores_current ON target_scores(is_current) WHERE is_current = true;