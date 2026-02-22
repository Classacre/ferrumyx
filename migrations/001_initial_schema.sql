-- Ferrumyx Initial Schema Migration
-- See ARCHITECTURE.md §1.4 for full schema documentation
-- Run with: sqlx migrate run

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "vector";
CREATE EXTENSION IF NOT EXISTS "pg_trgm"; -- for fuzzy title matching

-- ---------------------------------------------------------------------------
-- Papers and source tracking
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS papers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    doi             TEXT UNIQUE,
    pmid            TEXT UNIQUE,
    pmcid           TEXT,
    title           TEXT NOT NULL,
    abstract_text   TEXT,
    authors         JSONB,
    journal         TEXT,
    pub_date        DATE,
    source          TEXT NOT NULL,
    open_access     BOOLEAN DEFAULT FALSE,
    full_text_url   TEXT,
    retrieval_tier  SMALLINT,           -- 1=PMC XML, 2=Unpaywall PDF, ..., 6=abstract only
    parse_status    TEXT DEFAULT 'pending',
    abstract_simhash BIGINT,            -- for deduplication
    ingested_at     TIMESTAMPTZ DEFAULT NOW(),
    raw_json        JSONB
);
CREATE INDEX IF NOT EXISTS idx_papers_doi    ON papers (doi) WHERE doi IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_papers_pmid   ON papers (pmid) WHERE pmid IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_papers_source ON papers (source);
CREATE INDEX IF NOT EXISTS idx_papers_simhash ON papers (abstract_simhash) WHERE abstract_simhash IS NOT NULL;

-- ---------------------------------------------------------------------------
-- Parsed document chunks
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS paper_chunks (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    paper_id        UUID NOT NULL REFERENCES papers(id) ON DELETE CASCADE,
    chunk_index     INTEGER NOT NULL,
    section_type    TEXT,
    section_heading TEXT,
    content         TEXT NOT NULL,
    token_count     INTEGER,
    page_number     INTEGER,
    embedding       vector(768),        -- BiomedBERT-base dimension
    ts_vector       tsvector,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (paper_id, chunk_index)
);
CREATE INDEX IF NOT EXISTS idx_chunks_paper_id  ON paper_chunks (paper_id);
CREATE INDEX IF NOT EXISTS idx_chunks_ts_vector ON paper_chunks USING GIN (ts_vector);
-- Vector index created after initial bulk load (see migrations/002_vector_indexes.sql)

-- ---------------------------------------------------------------------------
-- Entities
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS entities (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    canonical_id    TEXT,
    entity_type     TEXT NOT NULL,
    name            TEXT NOT NULL,
    aliases         TEXT[],
    external_ids    JSONB,
    embedding       vector(768),
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (canonical_id, entity_type)
);
CREATE INDEX IF NOT EXISTS idx_entities_type        ON entities (entity_type);
CREATE INDEX IF NOT EXISTS idx_entities_canonical   ON entities (canonical_id) WHERE canonical_id IS NOT NULL;

-- Gene-specific extension
CREATE TABLE IF NOT EXISTS ent_genes (
    id              UUID PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    hgnc_id         TEXT UNIQUE,
    symbol          TEXT NOT NULL,
    uniprot_id      TEXT,
    ensembl_id      TEXT,
    entrez_id       TEXT,
    gene_biotype    TEXT,
    chromosome      TEXT,
    strand          SMALLINT,
    oncogene_flag   BOOLEAN DEFAULT FALSE,
    tsg_flag        BOOLEAN DEFAULT FALSE
);
CREATE INDEX IF NOT EXISTS idx_genes_symbol ON ent_genes (symbol);
CREATE INDEX IF NOT EXISTS idx_genes_hgnc   ON ent_genes (hgnc_id) WHERE hgnc_id IS NOT NULL;

-- Mutation-specific extension
CREATE TABLE IF NOT EXISTS ent_mutations (
    id              UUID PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    gene_id         UUID NOT NULL REFERENCES ent_genes(id),
    hgvs_p          TEXT,
    hgvs_c          TEXT,
    rs_id           TEXT,
    aa_ref          TEXT,
    aa_alt          TEXT,
    aa_position     INTEGER,
    oncogenicity    TEXT,
    hotspot_flag    BOOLEAN DEFAULT FALSE,
    vaf_context     TEXT
);
CREATE INDEX IF NOT EXISTS idx_mutations_gene   ON ent_mutations (gene_id);
CREATE INDEX IF NOT EXISTS idx_mutations_hgvs_p ON ent_mutations (hgvs_p) WHERE hgvs_p IS NOT NULL;

-- Cancer type (OncoTree)
CREATE TABLE IF NOT EXISTS ent_cancer_types (
    id              UUID PRIMARY KEY REFERENCES entities(id) ON DELETE CASCADE,
    oncotree_code   TEXT UNIQUE NOT NULL,
    oncotree_name   TEXT NOT NULL,
    icd_o3_code     TEXT,
    tissue          TEXT,
    parent_code     TEXT,
    level           INTEGER
);

-- Structural availability
CREATE TABLE IF NOT EXISTS ent_structures (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene_id         UUID NOT NULL REFERENCES ent_genes(id),
    pdb_ids         TEXT[],
    best_resolution FLOAT,
    exp_method      TEXT,
    af_accession    TEXT,
    af_plddt_mean   FLOAT,
    af_plddt_active FLOAT,
    has_pdb         BOOLEAN DEFAULT FALSE,
    has_alphafold   BOOLEAN DEFAULT FALSE,
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_structures_gene ON ent_structures (gene_id);

-- Druggability scores
CREATE TABLE IF NOT EXISTS ent_druggability (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    structure_id        UUID NOT NULL REFERENCES ent_structures(id),
    fpocket_score       FLOAT,
    fpocket_volume      FLOAT,
    fpocket_pocket_count INTEGER,
    dogsitescorer       FLOAT,
    overall_assessment  TEXT,
    assessed_at         TIMESTAMPTZ DEFAULT NOW()
);

-- Synthetic lethality pairs
CREATE TABLE IF NOT EXISTS ent_synthetic_lethality (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene1_id        UUID NOT NULL REFERENCES ent_genes(id),
    gene2_id        UUID NOT NULL REFERENCES ent_genes(id),
    cancer_id       UUID REFERENCES ent_cancer_types(id),
    evidence_type   TEXT NOT NULL,
    source_db       TEXT,
    screen_id       TEXT,
    effect_size     FLOAT,
    confidence      FLOAT NOT NULL CHECK (confidence BETWEEN 0 AND 1),
    pmid            TEXT,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (gene1_id, gene2_id, cancer_id, source_db)
);
CREATE INDEX IF NOT EXISTS idx_sl_gene1 ON ent_synthetic_lethality (gene1_id);
CREATE INDEX IF NOT EXISTS idx_sl_gene2 ON ent_synthetic_lethality (gene2_id);

-- Compounds / inhibitors
CREATE TABLE IF NOT EXISTS ent_compounds (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chembl_id       TEXT UNIQUE,
    name            TEXT,
    smiles          TEXT,
    inchi_key       TEXT UNIQUE,
    moa             TEXT,
    patent_status   TEXT,
    max_phase       INTEGER,
    target_gene_ids UUID[],
    created_at      TIMESTAMPTZ DEFAULT NOW()
);

-- ---------------------------------------------------------------------------
-- Knowledge graph facts (append-only)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS kg_facts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subject_id      UUID NOT NULL REFERENCES entities(id),
    predicate       TEXT NOT NULL,
    object_id       UUID NOT NULL REFERENCES entities(id),
    confidence      FLOAT NOT NULL CHECK (confidence BETWEEN 0 AND 1),
    evidence_type   TEXT NOT NULL,
    evidence_weight FLOAT NOT NULL,
    source_pmid     TEXT,
    source_doi      TEXT,
    source_db       TEXT,
    sample_size     INTEGER,
    study_type      TEXT,
    contradiction_flag BOOLEAN DEFAULT FALSE,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    valid_from      TIMESTAMPTZ DEFAULT NOW(),
    valid_until     TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_kgfacts_subject    ON kg_facts (subject_id, predicate, object_id);
CREATE INDEX IF NOT EXISTS idx_kgfacts_created    ON kg_facts (created_at);
CREATE INDEX IF NOT EXISTS idx_kgfacts_current    ON kg_facts (subject_id, predicate) WHERE valid_until IS NULL;

-- KG conflicts
CREATE TABLE IF NOT EXISTS kg_conflicts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fact_a_id       UUID NOT NULL REFERENCES kg_facts(id),
    fact_b_id       UUID NOT NULL REFERENCES kg_facts(id),
    conflict_type   TEXT NOT NULL,
    net_confidence  FLOAT NOT NULL,
    resolution      TEXT DEFAULT 'unresolved',
    detected_at     TIMESTAMPTZ DEFAULT NOW()
);

-- ---------------------------------------------------------------------------
-- Entity mentions from NER
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS entity_mentions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chunk_id        UUID REFERENCES paper_chunks(id) ON DELETE CASCADE,
    paper_id        UUID NOT NULL REFERENCES papers(id) ON DELETE CASCADE,
    mention_text    TEXT NOT NULL,
    entity_type     TEXT NOT NULL,
    norm_id         TEXT,
    norm_source     TEXT,
    confidence      FLOAT,
    char_start      INTEGER,
    char_end        INTEGER,
    model_source    TEXT,
    created_at      TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_mentions_paper    ON entity_mentions (paper_id);
CREATE INDEX IF NOT EXISTS idx_mentions_norm_id  ON entity_mentions (norm_id) WHERE norm_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_mentions_type     ON entity_mentions (entity_type, norm_id);

-- ---------------------------------------------------------------------------
-- Target scores (versioned)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS target_scores (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene_entity_id      UUID NOT NULL REFERENCES entities(id),
    cancer_entity_id    UUID NOT NULL REFERENCES entities(id),
    score_version       INTEGER NOT NULL DEFAULT 1,
    composite_score     FLOAT NOT NULL,
    confidence_adj      FLOAT,
    component_scores    JSONB NOT NULL,
    weight_vector       JSONB NOT NULL,
    penalty             FLOAT DEFAULT 0,
    shortlist_tier      TEXT,
    flags               TEXT[],
    is_current          BOOLEAN DEFAULT TRUE,
    scored_at           TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_scores_gene_cancer ON target_scores (gene_entity_id, cancer_entity_id, is_current);
CREATE INDEX IF NOT EXISTS idx_scores_composite   ON target_scores (composite_score DESC) WHERE is_current = TRUE;

-- ---------------------------------------------------------------------------
-- Molecules and docking results
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS molecules (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    smiles          TEXT NOT NULL,
    inchi_key       TEXT UNIQUE,
    chembl_id       TEXT,
    name            TEXT,
    mw              FLOAT,
    logp            FLOAT,
    hbd             INTEGER,
    hba             INTEGER,
    tpsa            FLOAT,
    sa_score        FLOAT,
    source          TEXT,
    parent_id       UUID REFERENCES molecules(id),
    created_at      TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS docking_results (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    molecule_id     UUID NOT NULL REFERENCES molecules(id),
    target_gene_id  UUID NOT NULL REFERENCES entities(id),
    pdb_id          TEXT,
    pocket_id       TEXT,
    vina_score      FLOAT,
    gnina_score     FLOAT,
    pose_file       TEXT,
    admet_scores    JSONB,
    run_params      JSONB,
    docked_at       TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_docking_gene     ON docking_results (target_gene_id);
CREATE INDEX IF NOT EXISTS idx_docking_molecule ON docking_results (molecule_id);

-- ---------------------------------------------------------------------------
-- Self-improvement: feedback events and weight updates
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS feedback_events (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type      TEXT NOT NULL,
    target_gene_id  UUID REFERENCES entities(id),
    cancer_id       UUID REFERENCES entities(id),
    metric_name     TEXT NOT NULL,
    metric_value    FLOAT NOT NULL,
    evidence_source TEXT,
    recorded_at     TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS weight_update_log (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    previous_weights JSONB NOT NULL,
    new_weights      JSONB NOT NULL,
    trigger_event    TEXT,
    algorithm        TEXT,
    approved_by      TEXT,
    delta_summary    JSONB,
    updated_at       TIMESTAMPTZ DEFAULT NOW()
);

-- ---------------------------------------------------------------------------
-- Audit logs
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS llm_audit_log (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id          TEXT,
    model               TEXT NOT NULL,
    backend             TEXT NOT NULL,
    prompt_tokens       INTEGER,
    completion_tokens   INTEGER,
    data_class          TEXT NOT NULL,
    output_hash         TEXT NOT NULL,
    latency_ms          INTEGER,
    called_at           TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS ingestion_audit (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    paper_id        UUID REFERENCES papers(id),
    paper_doi       TEXT,
    paper_pmid      TEXT,
    action          TEXT NOT NULL,
    source          TEXT NOT NULL,
    detail          JSONB,
    occurred_at     TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_ingestion_audit_action ON ingestion_audit (action, occurred_at);

-- ---------------------------------------------------------------------------
-- Schema versioning
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS schema_migrations (
    version     TEXT PRIMARY KEY,
    applied_at  TIMESTAMPTZ DEFAULT NOW(),
    checksum    TEXT
);

INSERT INTO schema_migrations (version, checksum)
VALUES ('001_initial_schema', 'v0.1.0')
ON CONFLICT (version) DO NOTHING;
