-- migrations/003_external_data_providers.sql
-- Add tables for external data provider integrations

-- TCGA survival data
CREATE TABLE ent_tcga_survival (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene_symbol TEXT NOT NULL,
    cancer_code TEXT NOT NULL,
    tcga_project_id TEXT NOT NULL,
    survival_score DOUBLE PRECISION NOT NULL,
    source TEXT NOT NULL,
    fetched_at TIMESTAMPTZ DEFAULT NOW()
);

-- cBioPortal mutation frequency data
CREATE TABLE ent_cbio_mutation_frequency (
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

-- COSMIC mutation frequency data
CREATE TABLE ent_cosmic_mutation_frequency (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene_symbol TEXT NOT NULL,
    cancer_code TEXT NOT NULL,
    mutated_sample_count BIGINT NOT NULL,
    profiled_sample_count BIGINT NOT NULL,
    mutation_frequency DOUBLE PRECISION NOT NULL,
    source TEXT NOT NULL,
    fetched_at TIMESTAMPTZ DEFAULT NOW()
);

-- GTEx expression data
CREATE TABLE ent_gtex_expression (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene_symbol TEXT NOT NULL,
    expression_score DOUBLE PRECISION NOT NULL,
    source TEXT NOT NULL,
    fetched_at TIMESTAMPTZ DEFAULT NOW()
);

-- ChEMBL target data
CREATE TABLE ent_chembl_targets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene_symbol TEXT NOT NULL,
    inhibitor_count BIGINT NOT NULL,
    source TEXT NOT NULL,
    fetched_at TIMESTAMPTZ DEFAULT NOW()
);

-- Reactome pathway data
CREATE TABLE ent_reactome_genes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene_symbol TEXT NOT NULL,
    pathway_count BIGINT NOT NULL,
    source TEXT NOT NULL,
    fetched_at TIMESTAMPTZ DEFAULT NOW()
);

-- Provider refresh tracking
CREATE TABLE ent_provider_refresh_runs (
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

-- Add indexes for performance
CREATE INDEX idx_ent_tcga_gene_cancer ON ent_tcga_survival(gene_symbol, cancer_code);
CREATE INDEX idx_ent_cbio_gene_cancer ON ent_cbio_mutation_frequency(gene_symbol, cancer_code);
CREATE INDEX idx_ent_cosmic_gene_cancer ON ent_cosmic_mutation_frequency(gene_symbol, cancer_code);
CREATE INDEX idx_ent_gtex_gene ON ent_gtex_expression(gene_symbol);
CREATE INDEX idx_ent_chembl_gene ON ent_chembl_targets(gene_symbol);
CREATE INDEX idx_ent_reactome_gene ON ent_reactome_genes(gene_symbol);
CREATE INDEX idx_ent_provider_runs_provider_started ON ent_provider_refresh_runs(provider, started_at);