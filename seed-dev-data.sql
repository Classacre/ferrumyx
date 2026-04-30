-- seed-dev-data.sql - Development seed data for Ferrumyx
-- This inserts sample oncology papers and entities for testing

-- Insert sample papers
INSERT INTO papers (doi, pmid, title, abstract_text, source, published_at, authors, journal, open_access) VALUES
('10.1038/nature12345', '12345678', 'Targeting KRAS Mutations in Pancreatic Cancer',
 'Recent advances in targeting KRAS mutations have shown promising results in preclinical models...',
 'PubMed Central', '2023-06-15 00:00:00+00'::timestamptz,
 '["Smith J", "Johnson A", "Williams B"]', 'Nature', true),

('10.1126/science.456789', '23456789', 'BRCA1/2 Mutations and PARP Inhibitors in Ovarian Cancer',
 'PARP inhibitors have revolutionized treatment of BRCA-mutated ovarian cancers...',
 'PubMed Central', '2023-08-22 00:00:00+00'::timestamptz,
 '["Brown K", "Davis M", "Garcia L"]', 'Science', true),

('10.1056/NEJMoa2301234', '34567890', 'PD-1 Blockade in Advanced Melanoma: Long-term Outcomes',
 'Five-year survival data from KEYNOTE-001 trial shows durable responses...',
 'NEJM', '2023-09-10 00:00:00+00'::timestamptz,
 '["Chen R", "Wilson P", "Taylor S"]', 'New England Journal of Medicine', false);

-- Insert sample entities
INSERT INTO entities (paper_id, entity_type, entity_text, normalized_id) VALUES
((SELECT id FROM papers WHERE doi = '10.1038/nature12345'), 'GENE', 'KRAS', 'HGNC:6407'),
((SELECT id FROM papers WHERE doi = '10.1038/nature12345'), 'DISEASE', 'Pancreatic Cancer', 'DOID:1793'),
((SELECT id FROM papers WHERE doi = '10.1126/science.456789'), 'GENE', 'BRCA1', 'HGNC:983'),
((SELECT id FROM papers WHERE doi = '10.1126/science.456789'), 'GENE', 'BRCA2', 'HGNC:984'),
((SELECT id FROM papers WHERE doi = '10.1126/science.456789'), 'DISEASE', 'Ovarian Cancer', 'DOID:2394'),
((SELECT id FROM papers WHERE doi = '10.1126/science.456789'), 'CHEMICAL', 'Olaparib', 'CHEMBL521965'),
((SELECT id FROM papers WHERE doi = '10.1056/NEJMoa2301234'), 'DISEASE', 'Melanoma', 'DOID:1909'),
((SELECT id FROM papers WHERE doi = '10.1056/NEJMoa2301234'), 'CHEMICAL', 'Pembrolizumab', 'CHEMBL3137343');

-- Insert sample chunks with embeddings (simplified 768-dim vectors)
INSERT INTO chunks (paper_id, chunk_index, content, section, created_at, embedding) VALUES
((SELECT id FROM papers WHERE doi = '10.1038/nature12345'), 0,
 'Recent advances in targeting KRAS mutations have shown promising results in preclinical models of pancreatic cancer.',
 'Abstract', NOW(), '[0.1, 0.2, 0.3]' || string_agg(',' || (random()::text), '') FROM generate_series(4,767)),

((SELECT id FROM papers WHERE doi = '10.1126/science.456789'), 0,
 'PARP inhibitors have revolutionized treatment of BRCA-mutated ovarian cancers through synthetic lethality.',
 'Abstract', NOW(), '[0.2, 0.3, 0.4]' || string_agg(',' || (random()::text), '') FROM generate_series(4,767)),

((SELECT id FROM papers WHERE doi = '10.1056/NEJMoa2301234'), 0,
 'Five-year survival data shows durable responses with PD-1 blockade in advanced melanoma.',
 'Abstract', NOW(), '[0.3, 0.4, 0.5]' || string_agg(',' || (random()::text), '') FROM generate_series(4,767));

-- Insert sample entity mentions
INSERT INTO entity_mentions (entity_id, chunk_id, paper_id, start_offset, end_offset, text) VALUES
((SELECT e.id FROM entities e JOIN papers p ON e.paper_id = p.id WHERE p.doi = '10.1038/nature12345' AND e.entity_text = 'KRAS'),
 (SELECT c.id FROM chunks c JOIN papers p ON c.paper_id = p.id WHERE p.doi = '10.1038/nature12345' AND c.chunk_index = 0),
 (SELECT id FROM papers WHERE doi = '10.1038/nature12345'), 30, 34, 'KRAS');

-- Insert sample KG facts
INSERT INTO kg_facts (paper_id, subject_id, subject_name, predicate, object_id, object_name, confidence, evidence_type) VALUES
((SELECT id FROM papers WHERE doi = '10.1038/nature12345'),
 gen_random_uuid(), 'KRAS', 'associated_with', gen_random_uuid(), 'Pancreatic Cancer', 0.85, 'literature'),

((SELECT id FROM papers WHERE doi = '10.1126/science.456789'),
 gen_random_uuid(), 'BRCA1', 'inhibits', gen_random_uuid(), 'PARP', 0.92, 'experimental');

-- Insert sample target scores
INSERT INTO target_scores (gene_id, cancer_id, composite_score, confidence_adjusted_score, penalty_score, shortlist_tier, components_raw, components_normed) VALUES
(gen_random_uuid(), gen_random_uuid(), 8.5, 8.2, 0.3, 'Tier 1', '{"expression": 0.9, "mutation": 0.8}', '{"expression": 0.45, "mutation": 0.4}'),
(gen_random_uuid(), gen_random_uuid(), 6.2, 5.8, 0.4, 'Tier 2', '{"expression": 0.7, "mutation": 0.6}', '{"expression": 0.35, "mutation": 0.3}');

-- Insert sample external data
INSERT INTO ent_tcga_survival (gene_symbol, cancer_code, tcga_project_id, survival_score, source) VALUES
('TP53', 'BRCA', 'TCGA-BRCA', 0.75, 'TCGA'),
('EGFR', 'LUAD', 'TCGA-LUAD', 0.82, 'TCGA');

INSERT INTO ent_cbio_mutation_frequency (gene_symbol, cancer_code, study_id, molecular_profile_id, sample_list_id, mutated_sample_count, profiled_sample_count, mutation_frequency, source) VALUES
('KRAS', 'PAAD', 'paad_tcga', 'paad_tcga_mutations', 'paad_tcga_all', 125, 185, 0.68, 'cBioPortal'),
('BRCA1', 'OV', 'ov_tcga', 'ov_tcga_mutations', 'ov_tcga_all', 89, 316, 0.28, 'cBioPortal');

INSERT INTO ent_cosmic_mutation_frequency (gene_symbol, cancer_code, mutated_sample_count, profiled_sample_count, mutation_frequency, source) VALUES
('TP53', 'LUAD', 2450, 8900, 0.275, 'COSMIC'),
('KRAS', 'COAD', 1890, 3200, 0.59, 'COSMIC');

INSERT INTO ent_gtex_expression (gene_symbol, expression_score, source) VALUES
('GAPDH', 12.5, 'GTEx'),
('ACTB', 11.8, 'GTEx');

INSERT INTO ent_chembl_targets (gene_symbol, inhibitor_count, source) VALUES
('EGFR', 245, 'ChEMBL'),
('BRCA1', 67, 'ChEMBL');

INSERT INTO ent_reactome_genes (gene_symbol, pathway_count, source) VALUES
('TP53', 89, 'Reactome'),
('MYC', 156, 'Reactome');

-- Insert sample Phase 3 entities
INSERT INTO ent_genes (symbol, name, hgnc_id, uniprot_id, oncogene_flag, tsg_flag) VALUES
('TP53', 'Tumor Protein P53', 'HGNC:11998', 'P04637', false, true),
('KRAS', 'KRAS Proto-Oncogene', 'HGNC:6407', 'P01116', true, false),
('EGFR', 'Epidermal Growth Factor Receptor', 'HGNC:3236', 'P00533', true, false);

INSERT INTO ent_cancer_types (oncotree_code, oncotree_name, tissue) VALUES
('PAAD', 'Pancreatic Adenocarcinoma', 'Pancreas'),
('LUAD', 'Lung Adenocarcinoma', 'Lung'),
('BRCA', 'Invasive Breast Carcinoma', 'Breast');

INSERT INTO ent_compounds (name, chembl_id, moa, max_phase) VALUES
('Olaparib', 'CHEMBL521965', 'PARP inhibitor', 4),
('Pembrolizumab', 'CHEMBL3137343', 'PD-1 inhibitor', 4),
('Trametinib', 'CHEMBL210387', 'MEK inhibitor', 4);