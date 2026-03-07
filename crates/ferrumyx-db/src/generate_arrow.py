import sys

def main():
    schemas = [
        ("EntGene", [
            ("id", "Utf8", "false", "uuid", "uuid"),
            ("hgnc_id", "Utf8", "true", "string", "opt_string"),
            ("symbol", "Utf8", "false", "string", "string"),
            ("name", "Utf8", "true", "string", "opt_string"),
            ("uniprot_id", "Utf8", "true", "string", "opt_string"),
            ("ensembl_id", "Utf8", "true", "string", "opt_string"),
            ("entrez_id", "Utf8", "true", "string", "opt_string"),
            ("gene_biotype", "Utf8", "true", "string", "opt_string"),
            ("chromosome", "Utf8", "true", "string", "opt_string"),
            ("strand", "Int16", "true", "int16", "opt_int16"),
            ("aliases", "Utf8", "true", "json_vec_string", "opt_json_vec_string"),
            ("oncogene_flag", "Boolean", "false", "bool", "bool"),
            ("tsg_flag", "Boolean", "false", "bool", "bool"),
            ("created_at", "Utf8", "false", "datetime", "datetime"),
        ]),
        ("EntMutation", [
            ("id", "Utf8", "false", "uuid", "uuid"),
            ("gene_id", "Utf8", "false", "uuid", "uuid"),
            ("hgvs_p", "Utf8", "true", "string", "opt_string"),
            ("hgvs_c", "Utf8", "true", "string", "opt_string"),
            ("rs_id", "Utf8", "true", "string", "opt_string"),
            ("aa_ref", "Utf8", "true", "string", "opt_string"),
            ("aa_alt", "Utf8", "true", "string", "opt_string"),
            ("aa_position", "Int32", "true", "int32", "opt_int32"),
            ("oncogenicity", "Utf8", "true", "string", "opt_string"),
            ("hotspot_flag", "Boolean", "false", "bool", "bool"),
            ("vaf_context", "Utf8", "true", "string", "opt_string"),
            ("created_at", "Utf8", "false", "datetime", "datetime"),
        ]),
        ("EntCancerType", [
            ("id", "Utf8", "false", "uuid", "uuid"),
            ("oncotree_code", "Utf8", "true", "string", "opt_string"),
            ("oncotree_name", "Utf8", "true", "string", "opt_string"),
            ("icd_o3_code", "Utf8", "true", "string", "opt_string"),
            ("tissue", "Utf8", "true", "string", "opt_string"),
            ("parent_code", "Utf8", "true", "string", "opt_string"),
            ("level", "Int32", "true", "int32", "opt_int32"),
            ("created_at", "Utf8", "false", "datetime", "datetime"),
        ]),
        ("EntPathway", [
            ("id", "Utf8", "false", "uuid", "uuid"),
            ("kegg_id", "Utf8", "true", "string", "opt_string"),
            ("reactome_id", "Utf8", "true", "string", "opt_string"),
            ("go_term", "Utf8", "true", "string", "opt_string"),
            ("name", "Utf8", "false", "string", "string"),
            ("gene_members", "Utf8", "true", "json_vec_string", "opt_json_vec_string"),
            ("source", "Utf8", "true", "string", "opt_string"),
            ("created_at", "Utf8", "false", "datetime", "datetime"),
        ]),
        ("EntClinicalEvidence", [
            ("id", "Utf8", "false", "uuid", "uuid"),
            ("nct_id", "Utf8", "true", "string", "opt_string"),
            ("pmid", "Utf8", "true", "string", "opt_string"),
            ("doi", "Utf8", "true", "string", "opt_string"),
            ("phase", "Utf8", "true", "string", "opt_string"),
            ("intervention", "Utf8", "true", "string", "opt_string"),
            ("target_gene_id", "Utf8", "false", "uuid", "uuid"),
            ("cancer_id", "Utf8", "false", "uuid", "uuid"),
            ("primary_endpoint", "Utf8", "true", "string", "opt_string"),
            ("outcome", "Utf8", "true", "string", "opt_string"),
            ("evidence_grade", "Utf8", "true", "string", "opt_string"),
            ("created_at", "Utf8", "false", "datetime", "datetime"),
        ]),
        ("EntCompound", [
            ("id", "Utf8", "false", "uuid", "uuid"),
            ("chembl_id", "Utf8", "true", "string", "opt_string"),
            ("name", "Utf8", "true", "string", "opt_string"),
            ("smiles", "Utf8", "true", "string", "opt_string"),
            ("inchi_key", "Utf8", "true", "string", "opt_string"),
            ("moa", "Utf8", "true", "string", "opt_string"),
            ("patent_status", "Utf8", "true", "string", "opt_string"),
            ("max_phase", "Int32", "true", "int32", "opt_int32"),
            ("target_gene_ids", "Utf8", "true", "json_vec_uuid", "opt_json_vec_uuid"),
            ("created_at", "Utf8", "false", "datetime", "datetime"),
        ]),
        ("EntStructure", [
            ("id", "Utf8", "false", "uuid", "uuid"),
            ("gene_id", "Utf8", "false", "uuid", "uuid"),
            ("pdb_ids", "Utf8", "true", "json_vec_string", "opt_json_vec_string"),
            ("best_resolution", "Float32", "true", "float32", "opt_float32"),
            ("exp_method", "Utf8", "true", "string", "opt_string"),
            ("af_accession", "Utf8", "true", "string", "opt_string"),
            ("af_plddt_mean", "Float32", "true", "float32", "opt_float32"),
            ("af_plddt_active", "Float32", "true", "float32", "opt_float32"),
            ("has_pdb", "Boolean", "false", "bool", "bool"),
            ("has_alphafold", "Boolean", "false", "bool", "bool"),
            ("updated_at", "Utf8", "false", "datetime", "datetime"),
        ]),
        ("EntDruggability", [
            ("id", "Utf8", "false", "uuid", "uuid"),
            ("structure_id", "Utf8", "false", "uuid", "uuid"),
            ("fpocket_score", "Float32", "true", "float32", "opt_float32"),
            ("fpocket_volume", "Float32", "true", "float32", "opt_float32"),
            ("fpocket_pocket_count", "Int32", "true", "int32", "opt_int32"),
            ("dogsitescorer", "Float32", "true", "float32", "opt_float32"),
            ("overall_assessment", "Utf8", "true", "string", "opt_string"),
            ("assessed_at", "Utf8", "false", "datetime", "datetime"),
        ]),
        ("EntSyntheticLethality", [
            ("id", "Utf8", "false", "uuid", "uuid"),
            ("gene1_id", "Utf8", "false", "uuid", "uuid"),
            ("gene2_id", "Utf8", "false", "uuid", "uuid"),
            ("cancer_id", "Utf8", "false", "uuid", "uuid"),
            ("evidence_type", "Utf8", "true", "string", "opt_string"),
            ("source_db", "Utf8", "true", "string", "opt_string"),
            ("screen_id", "Utf8", "true", "string", "opt_string"),
            ("effect_size", "Float32", "true", "float32", "opt_float32"),
            ("confidence", "Float32", "true", "float32", "opt_float32"),
            ("pmid", "Utf8", "true", "string", "opt_string"),
            ("created_at", "Utf8", "false", "datetime", "datetime"),
        ]),
    ]
    
    out = ["\n// ============================================================================="]
    out.append("// Specific Entity Type Conversions (Phase 3)")
    out.append("// =============================================================================")
    
    for struct_name, fields in schemas:
        snake_name = "".join(["_" + c.lower() if c.isupper() else c for c in struct_name]).lstrip("_")
        
        # Schema function
        out.append(f"\npub fn {snake_name}_schema() -> Arc<Schema> {{")
        out.append("    Arc::new(Schema::new(vec![")
        for f in fields:
            out.append(f"        Field::new(\"{f[0]}\", DataType::{f[1]}, {f[2]}),")
        out.append("    ]))\n}")
        
        # To RecordBatch
        out.append(f"\npub fn {snake_name}_to_record(item: &{struct_name}) -> Result<RecordBatch> {{")
        out.append(f"    let schema = {snake_name}_schema();")
        
        for f in fields:
            col, dt, nul, src, rtype = f
            if src == "uuid":
                out.append(f"    let {col} = StringArray::from(vec![item.{col}.to_string()]);")
            elif src == "string":
                if nul == "true":
                    out.append(f"    let {col} = StringArray::from(vec![item.{col}.as_deref()]);")
                else:
                    out.append(f"    let {col} = StringArray::from(vec![item.{col}.as_str()]);")
            elif src == "datetime":
                out.append(f"    let {col} = StringArray::from(vec![item.{col}.to_rfc3339()]);")
            elif src == "bool":
                if nul == "true":
                    out.append(f"    let {col} = arrow_array::BooleanArray::from(vec![item.{col}]);")
                else:
                    out.append(f"    let {col} = arrow_array::BooleanArray::from(vec![Some(item.{col})]);")
            elif src == "int16":
                out.append(f"    let {col} = arrow_array::Int16Array::from(vec![item.{col}]);")
            elif src == "int32":
                out.append(f"    let {col} = arrow_array::Int32Array::from(vec![item.{col}]);")
            elif src == "float32":
                out.append(f"    let {col} = arrow_array::Float32Array::from(vec![item.{col}]);")
            elif "json_vec" in src:
                out.append(f"    let {col} = StringArray::from(vec![item.{col}.as_ref().map(|v| serde_json::to_string(v).unwrap_or_default())]);")
                
        out.append("    RecordBatch::try_new(schema, vec![")
        for f in fields:
            if f[0] == "id":
                out.append(f"        Arc::new({f[0]}) as Arc<dyn Array>,")
            else:
                out.append(f"        Arc::new({f[0]}),")
        out.append("    ]).map_err(|e| DbError::Arrow(e.to_string()))\n}")
        
        # From RecordBatch
        out.append(f"\npub fn record_to_{snake_name}(batch: &RecordBatch, row: usize) -> Result<{struct_name}> {{")
        out.append("    let get_string = |col: usize| -> String { batch.column(col).as_any().downcast_ref::<StringArray>().unwrap().value(row).to_string() };")
        out.append("    let get_opt_string = |col: usize| -> Option<String> { let arr = batch.column(col).as_any().downcast_ref::<StringArray>().unwrap(); if arr.is_null(row) { None } else { Some(arr.value(row).to_string()) } };")
        out.append("    let get_bool = |col: usize| -> bool { let arr = batch.column(col).as_any().downcast_ref::<arrow_array::BooleanArray>().unwrap(); if arr.is_null(row) { false } else { arr.value(row) } };")
        out.append("    let get_opt_i16 = |col: usize| -> Option<i16> { let arr = batch.column(col).as_any().downcast_ref::<arrow_array::Int16Array>().unwrap(); if arr.is_null(row) { None } else { Some(arr.value(row)) } };")
        out.append("    let get_opt_i32 = |col: usize| -> Option<i32> { let arr = batch.column(col).as_any().downcast_ref::<arrow_array::Int32Array>().unwrap(); if arr.is_null(row) { None } else { Some(arr.value(row)) } };")
        out.append("    let get_opt_f32 = |col: usize| -> Option<f32> { let arr = batch.column(col).as_any().downcast_ref::<arrow_array::Float32Array>().unwrap(); if arr.is_null(row) { None } else { Some(arr.value(row)) } };")

        out.append(f"    Ok({struct_name} {{")
        for i, f in enumerate(fields):
            col, dt, nul, src, rtype = f
            if rtype == "uuid":
                out.append(f"        {col}: uuid::Uuid::parse_str(&get_string({i})).map_err(|e| DbError::InvalidQuery(e.to_string()))?,")
            elif rtype == "string":
                out.append(f"        {col}: get_string({i}),")
            elif rtype == "opt_string":
                out.append(f"        {col}: get_opt_string({i}),")
            elif rtype == "datetime":
                out.append(f"        {col}: chrono::DateTime::parse_from_rfc3339(&get_string({i})).map(|dt| dt.with_timezone(&chrono::Utc)).unwrap_or_else(|_| chrono::Utc::now()),")
            elif rtype == "bool":
                out.append(f"        {col}: get_bool({i}),")
            elif rtype == "opt_int16":
                out.append(f"        {col}: get_opt_i16({i}),")
            elif rtype == "opt_int32":
                out.append(f"        {col}: get_opt_i32({i}),")
            elif rtype == "opt_float32":
                out.append(f"        {col}: get_opt_f32({i}),")
            elif "json_vec_string" in rtype:
                out.append(f"        {col}: get_opt_string({i}).and_then(|s| serde_json::from_str(&s).ok()),")
            elif "json_vec_uuid" in rtype:
                out.append(f"        {col}: get_opt_string({i}).and_then(|s| serde_json::from_str(&s).ok()),")
        out.append("    })\n}")

    with open("d:\\AI\\Ferrumyx\\crates\\ferrumyx-db\\src\\schema_arrow.rs", "a") as f:
        f.write("\n".join(out))
        f.write("\n")

if __name__ == "__main__":
    main()
