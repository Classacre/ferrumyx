-- migrations/002_phase3_entity_tables.sql
-- Add Phase 3 entity enrichment tables

-- Entity genes table
CREATE TABLE ent_genes (
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

-- Entity mutations table
CREATE TABLE ent_mutations (
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

-- Entity cancer types table
CREATE TABLE ent_cancer_types (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    oncotree_code TEXT,
    oncotree_name TEXT,
    icd_o3_code TEXT,
    tissue TEXT,
    parent_code TEXT,
    level INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Entity pathways table
CREATE TABLE ent_pathways (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    kegg_id TEXT,
    reactome_id TEXT,
    go_term TEXT,
    name TEXT NOT NULL,
    gene_members JSONB,
    source TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Entity clinical evidence table
CREATE TABLE ent_clinical_evidence (
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

-- Entity compounds table
CREATE TABLE ent_compounds (
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

-- Entity structures table
CREATE TABLE ent_structures (
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

-- Entity druggability table
CREATE TABLE ent_druggability (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    structure_id UUID REFERENCES ent_structures(id),
    fpocket_score REAL,
    fpocket_volume REAL,
    fpocket_pocket_count INTEGER,
    dogsitescorer REAL,
    overall_assessment TEXT,
    assessed_at TIMESTAMPTZ DEFAULT NOW()
);

-- Entity synthetic lethality table
CREATE TABLE ent_synthetic_lethality (
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