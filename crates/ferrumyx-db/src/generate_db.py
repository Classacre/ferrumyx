import sys

def main():
    schemas = [
        ("EntGene", "TABLE_ENT_GENES", "create_ent_genes_table", [
            ("id", "Utf8", "false"), ("hgnc_id", "Utf8", "true"), ("symbol", "Utf8", "false"), ("name", "Utf8", "true"),
            ("uniprot_id", "Utf8", "true"), ("ensembl_id", "Utf8", "true"), ("entrez_id", "Utf8", "true"), 
            ("gene_biotype", "Utf8", "true"), ("chromosome", "Utf8", "true"), ("strand", "Int16", "true"),
            ("aliases", "Utf8", "true"), ("oncogene_flag", "Boolean", "false"), ("tsg_flag", "Boolean", "false"),
            ("created_at", "Utf8", "false")
        ]),
        ("EntMutation", "TABLE_ENT_MUTATIONS", "create_ent_mutations_table", [
            ("id", "Utf8", "false"), ("gene_id", "Utf8", "false"), ("hgvs_p", "Utf8", "true"), ("hgvs_c", "Utf8", "true"),
            ("rs_id", "Utf8", "true"), ("aa_ref", "Utf8", "true"), ("aa_alt", "Utf8", "true"), ("aa_position", "Int32", "true"),
            ("oncogenicity", "Utf8", "true"), ("hotspot_flag", "Boolean", "false"), ("vaf_context", "Utf8", "true"),
            ("created_at", "Utf8", "false")
        ]),
        ("EntCancerType", "TABLE_ENT_CANCER_TYPES", "create_ent_cancer_types_table", [
            ("id", "Utf8", "false"), ("oncotree_code", "Utf8", "true"), ("oncotree_name", "Utf8", "true"),
            ("icd_o3_code", "Utf8", "true"), ("tissue", "Utf8", "true"), ("parent_code", "Utf8", "true"),
            ("level", "Int32", "true"), ("created_at", "Utf8", "false")
        ]),
        ("EntPathway", "TABLE_ENT_PATHWAYS", "create_ent_pathways_table", [
            ("id", "Utf8", "false"), ("kegg_id", "Utf8", "true"), ("reactome_id", "Utf8", "true"), ("go_term", "Utf8", "true"),
            ("name", "Utf8", "false"), ("gene_members", "Utf8", "true"), ("source", "Utf8", "true"),
            ("created_at", "Utf8", "false")
        ]),
        ("EntClinicalEvidence", "TABLE_ENT_CLINICAL_EVIDENCE", "create_ent_clinical_evidence_table", [
            ("id", "Utf8", "false"), ("nct_id", "Utf8", "true"), ("pmid", "Utf8", "true"), ("doi", "Utf8", "true"),
            ("phase", "Utf8", "true"), ("intervention", "Utf8", "true"), ("target_gene_id", "Utf8", "false"),
            ("cancer_id", "Utf8", "false"), ("primary_endpoint", "Utf8", "true"), ("outcome", "Utf8", "true"),
            ("evidence_grade", "Utf8", "true"), ("created_at", "Utf8", "false")
        ]),
        ("EntCompound", "TABLE_ENT_COMPOUNDS", "create_ent_compounds_table", [
            ("id", "Utf8", "false"), ("chembl_id", "Utf8", "true"), ("name", "Utf8", "true"), ("smiles", "Utf8", "true"),
            ("inchi_key", "Utf8", "true"), ("moa", "Utf8", "true"), ("patent_status", "Utf8", "true"),
            ("max_phase", "Int32", "true"), ("target_gene_ids", "Utf8", "true"), ("created_at", "Utf8", "false")
        ]),
        ("EntStructure", "TABLE_ENT_STRUCTURES", "create_ent_structures_table", [
            ("id", "Utf8", "false"), ("gene_id", "Utf8", "false"), ("pdb_ids", "Utf8", "true"), ("best_resolution", "Float32", "true"),
            ("exp_method", "Utf8", "true"), ("af_accession", "Utf8", "true"), ("af_plddt_mean", "Float32", "true"),
            ("af_plddt_active", "Float32", "true"), ("has_pdb", "Boolean", "false"), ("has_alphafold", "Boolean", "false"),
            ("updated_at", "Utf8", "false")
        ]),
        ("EntDruggability", "TABLE_ENT_DRUGGABILITY", "create_ent_druggability_table", [
            ("id", "Utf8", "false"), ("structure_id", "Utf8", "false"), ("fpocket_score", "Float32", "true"),
            ("fpocket_volume", "Float32", "true"), ("fpocket_pocket_count", "Int32", "true"), ("dogsitescorer", "Float32", "true"),
            ("overall_assessment", "Utf8", "true"), ("assessed_at", "Utf8", "false")
        ]),
        ("EntSyntheticLethality", "TABLE_ENT_SYNTHETIC_LETHALITY", "create_ent_synthetic_lethality_table", [
            ("id", "Utf8", "false"), ("gene1_id", "Utf8", "false"), ("gene2_id", "Utf8", "false"), ("cancer_id", "Utf8", "false"),
            ("evidence_type", "Utf8", "true"), ("source_db", "Utf8", "true"), ("screen_id", "Utf8", "true"),
            ("effect_size", "Float32", "true"), ("confidence", "Float32", "true"), ("pmid", "Utf8", "true"),
            ("created_at", "Utf8", "false")
        ])
    ]

    out = ["\n// ============================================================================="]
    out.append("// Phase 3 Entity Table Creation")
    out.append("// =============================================================================")

    out.append("impl Database {")

    for struct_name, const_name, method_name, fields in schemas:
        out.append(f"    pub async fn {method_name}(&self) -> Result<()> {{")
        out.append("        let fields: Fields = vec![")
        for f in fields:
            out.append(f"            Field::new(\"{f[0]}\", DataType::{f[1]}, {f[2]}),")
        out.append("        ].into();")
        out.append("        let schema = Arc::new(Schema::new(fields));")
        out.append("        let empty_iter = RecordBatchIterator::new(vec![], schema);")
        out.append(f"        self.conn.create_table(schema::{const_name}, empty_iter).execute().await?;")
        out.append("        Ok(())")
        out.append("    }\n")

    out.append("}")

    with open("d:\\AI\\Ferrumyx\\crates\\ferrumyx-db\\src\\database.rs", "a") as f:
        f.write("\n".join(out))
        f.write("\n")

if __name__ == "__main__":
    main()
