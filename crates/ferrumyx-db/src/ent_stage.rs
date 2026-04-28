//! Phase 4 enrichment reader for `ent_*` tables.
//!
//! Provides batched, bounded lookups so ranker can replace proxy values with
//! source-backed signals when entity-stage tables are populated.

use crate::database::Database;
use crate::error::Result;
use crate::schema;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio_postgres::Row;

#[derive(Debug, Clone, Default)]
pub struct EntEnrichment {
    pub mutation_count: u32,
    pub pdb_structure_count: u32,
    pub af_plddt_mean: Option<f64>,
    pub fpocket_best_score: Option<f64>,
    pub chembl_inhibitor_count: u32,
    pub pathway_count: u32,
}

#[derive(Clone)]
pub struct EntStageRepository {
    db: Arc<Database>,
}

impl EntStageRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn get_enrichment_by_symbol(
        &self,
        symbols: &[String],
    ) -> Result<HashMap<String, EntEnrichment>> {
        let clean_symbols: Vec<String> = symbols
            .iter()
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty())
            .collect();
        if clean_symbols.is_empty() {
            return Ok(HashMap::new());
        }

        let genes = self.fetch_genes_by_symbol(&clean_symbols).await?;
        let mut gene_id_by_symbol: HashMap<String, uuid::Uuid> = HashMap::new();
        for g in genes {
            gene_id_by_symbol.insert(g.symbol.to_uppercase(), g.id);
        }
        let gene_ids: Vec<uuid::Uuid> = gene_id_by_symbol.values().copied().collect();
        let mut_by_gene = self.count_mutations_by_gene_ids(&gene_ids).await?;
        let (pdb_by_gene, plddt_by_gene, structure_ids_by_gene) =
            self.fetch_structure_metrics_by_gene_ids(&gene_ids).await?;
        let fpocket_by_structure = self
            .fetch_fpocket_by_structure_ids(
                &structure_ids_by_gene
                    .values()
                    .flat_map(|v| v.iter().copied())
                    .collect::<Vec<_>>(),
            )
            .await?;
        let compounds_by_gene = self.count_compounds_by_gene_ids(&gene_ids).await?;
        let pathways_by_symbol = self.count_pathways_by_symbol(&clean_symbols).await?;

        let mut out: HashMap<String, EntEnrichment> = HashMap::new();
        for symbol in clean_symbols {
            let mut e = EntEnrichment::default();
            if let Some(gid) = gene_id_by_symbol.get(&symbol) {
                e.mutation_count = *mut_by_gene.get(gid).unwrap_or(&0);
                e.pdb_structure_count = *pdb_by_gene.get(gid).unwrap_or(&0);
                e.af_plddt_mean = plddt_by_gene.get(gid).copied().flatten();
                e.chembl_inhibitor_count = *compounds_by_gene.get(gid).unwrap_or(&0);
                if let Some(structure_ids) = structure_ids_by_gene.get(gid) {
                    let mut best: Option<f64> = None;
                    for sid in structure_ids {
                        if let Some(score) = fpocket_by_structure.get(sid) {
                            best = Some(best.map(|v| v.max(*score)).unwrap_or(*score));
                        }
                    }
                    e.fpocket_best_score = best;
                }
            }
            e.pathway_count = *pathways_by_symbol.get(&symbol).unwrap_or(&0);
            out.insert(symbol, e);
        }
        Ok(out)
    }

    pub async fn find_genes_by_symbol(
        &self,
        symbols: &[String],
    ) -> Result<HashMap<String, crate::schema::EntGene>> {
        let clean_symbols: Vec<String> = symbols
            .iter()
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty())
            .collect();
        if clean_symbols.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = self.fetch_genes_by_symbol(&clean_symbols).await?;
        let mut out = HashMap::new();
        for row in rows {
            out.insert(row.symbol.to_uppercase(), row);
        }
        Ok(out)
    }

    pub async fn upsert_structure_signal(
        &self,
        structure: &crate::schema::EntStructure,
    ) -> Result<()> {
        let client = self.db.client();
        client.execute(
            "INSERT INTO ent_structures (id, gene_id, pdb_ids, best_resolution, exp_method, af_accession, af_plddt_mean, af_plddt_active, has_pdb, has_alphafold, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW()) \
             ON CONFLICT (gene_id) DO UPDATE SET \
             pdb_ids = EXCLUDED.pdb_ids, best_resolution = EXCLUDED.best_resolution, \
             exp_method = EXCLUDED.exp_method, af_accession = EXCLUDED.af_accession, \
             af_plddt_mean = EXCLUDED.af_plddt_mean, af_plddt_active = EXCLUDED.af_plddt_active, \
             has_pdb = EXCLUDED.has_pdb, has_alphafold = EXCLUDED.has_alphafold, updated_at = EXCLUDED.updated_at",
            &[
                &structure.id, &structure.gene_id, &structure.pdb_ids,
                &structure.best_resolution, &structure.exp_method,
                &structure.af_accession, &structure.af_plddt_mean,
                &structure.af_plddt_active, &structure.has_pdb, &structure.has_alphafold,
            ],
        ).await?;
        Ok(())
    }

    pub async fn upsert_druggability_signal(
        &self,
        druggability: &crate::schema::EntDruggability,
    ) -> Result<()> {
        let client = self.db.client();
        client.execute(
            "INSERT INTO ent_druggability (id, structure_id, fpocket_score, fpocket_volume, fpocket_pocket_count, dogsitescorer, overall_assessment, assessed_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW()) \
             ON CONFLICT (structure_id) DO UPDATE SET \
             fpocket_score = EXCLUDED.fpocket_score, fpocket_volume = EXCLUDED.fpocket_volume, \
             fpocket_pocket_count = EXCLUDED.fpocket_pocket_count, dogsitescorer = EXCLUDED.dogsitescorer, \
             overall_assessment = EXCLUDED.overall_assessment, assessed_at = EXCLUDED.assessed_at",
            &[
                &druggability.id, &druggability.structure_id, &druggability.fpocket_score,
                &druggability.fpocket_volume, &druggability.fpocket_pocket_count,
                &druggability.dogsitescorer, &druggability.overall_assessment,
            ],
        ).await?;
        Ok(())
    }

    async fn fetch_genes_by_symbol(
        &self,
        symbols: &[String],
    ) -> Result<Vec<crate::schema::EntGene>> {
        if !self.db.table_exists(schema::TABLE_ENT_GENES).await? {
            return Ok(Vec::new());
        }
        let client = self.db.client();
        let mut out = Vec::new();
        for chunk in symbols.chunks(150) {
            if chunk.is_empty() { continue; }
            let placeholders = (1..=chunk.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(",");
            let sql = format!("SELECT * FROM ent_genes WHERE symbol IN ({})", placeholders);
            let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = chunk.iter().map(|s| s as &(dyn tokio_postgres::types::ToSql + Sync)).collect();
            let rows = client.query(&sql, params.as_slice()).await?;
            for row in rows {
                out.push(row_to_ent_gene(&row));
            }
        }
        Ok(out)
    }

    async fn count_mutations_by_gene_ids(
        &self,
        gene_ids: &[uuid::Uuid],
    ) -> Result<HashMap<uuid::Uuid, u32>> {
        if gene_ids.is_empty() || !self.db.table_exists(schema::TABLE_ENT_MUTATIONS).await? {
            return Ok(HashMap::new());
        }
        let client = self.db.client();
        let mut counts = HashMap::new();
        for chunk in gene_ids.chunks(150) {
            if chunk.is_empty() { continue; }
            let placeholders = (1..=chunk.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(",");
            let sql = format!("SELECT gene_id, COUNT(*) as cnt FROM ent_mutations WHERE gene_id IN ({}) GROUP BY gene_id", placeholders);
            let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = chunk.iter().map(|id| id as &(dyn tokio_postgres::types::ToSql + Sync)).collect();
            let rows = client.query(&sql, params.as_slice()).await?;
            for row in rows {
                let gene_id: uuid::Uuid = row.get("gene_id");
                let cnt: i64 = row.get("cnt");
                *counts.entry(gene_id).or_insert(0) += cnt as u32;
            }
        }
        Ok(counts)
    }

    async fn fetch_structure_metrics_by_gene_ids(
        &self,
        gene_ids: &[uuid::Uuid],
    ) -> Result<(
        HashMap<uuid::Uuid, u32>,
        HashMap<uuid::Uuid, Option<f64>>,
        HashMap<uuid::Uuid, Vec<uuid::Uuid>>,
    )> {
        if gene_ids.is_empty() || !self.db.table_exists(schema::TABLE_ENT_STRUCTURES).await? {
            return Ok((HashMap::new(), HashMap::new(), HashMap::new()));
        }
        let client = self.db.client();
        let mut pdb_count = HashMap::new();
        let mut plddt_mean = HashMap::new();
        let mut structure_ids: HashMap<uuid::Uuid, Vec<uuid::Uuid>> = HashMap::new();
        for chunk in gene_ids.chunks(150) {
            if chunk.is_empty() { continue; }
            let placeholders = (1..=chunk.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(",");
            let sql = format!("SELECT * FROM ent_structures WHERE gene_id IN ({})", placeholders);
            let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = chunk.iter().map(|id| id as &(dyn tokio_postgres::types::ToSql + Sync)).collect();
            let rows = client.query(&sql, params.as_slice()).await?;
            for row in rows {
                let s = row_to_ent_structure(&row);
                if s.has_pdb {
                    *pdb_count.entry(s.gene_id).or_insert(0) += 1;
                }
                let incoming = s.af_plddt_mean.map(|v| v as f64);
                plddt_mean
                    .entry(s.gene_id)
                    .and_modify(|cur: &mut Option<f64>| {
                        *cur = match (*cur, incoming) {
                            (Some(a), Some(b)) => Some(a.max(b)),
                            (None, Some(b)) => Some(b),
                            (x, None) => x,
                        }
                    })
                    .or_insert(incoming);
                structure_ids.entry(s.gene_id).or_default().push(s.id);
            }
        }
        Ok((pdb_count, plddt_mean, structure_ids))
    }

    async fn fetch_fpocket_by_structure_ids(
        &self,
        structure_ids: &[uuid::Uuid],
    ) -> Result<HashMap<uuid::Uuid, f64>> {
        if structure_ids.is_empty() || !self.db.table_exists(schema::TABLE_ENT_DRUGGABILITY).await? {
            return Ok(HashMap::new());
        }
        let client = self.db.client();
        let mut out = HashMap::new();
        for chunk in structure_ids.chunks(150) {
            if chunk.is_empty() { continue; }
            let placeholders = (1..=chunk.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(",");
            let sql = format!("SELECT structure_id, fpocket_score FROM ent_druggability WHERE structure_id IN ({})", placeholders);
            let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = chunk.iter().map(|id| id as &(dyn tokio_postgres::types::ToSql + Sync)).collect();
            let rows = client.query(&sql, params.as_slice()).await?;
            for row in rows {
                let structure_id: uuid::Uuid = row.get("structure_id");
                let score: Option<f32> = row.get("fpocket_score");
                if let Some(score) = score {
                    out.entry(structure_id)
                        .and_modify(|v: &mut f64| *v = v.max(score as f64))
                        .or_insert(score as f64);
                }
            }
        }
        Ok(out)
    }

    async fn count_compounds_by_gene_ids(
        &self,
        gene_ids: &[uuid::Uuid],
    ) -> Result<HashMap<uuid::Uuid, u32>> {
        if gene_ids.is_empty() || !self.db.table_exists(schema::TABLE_ENT_COMPOUNDS).await? {
            return Ok(HashMap::new());
        }
        let client = self.db.client();
        let wanted: HashSet<uuid::Uuid> = gene_ids.iter().copied().collect();
        let mut counts = HashMap::new();
        for chunk in gene_ids.chunks(80) {
            if chunk.is_empty() { continue; }
            let placeholders = (1..=chunk.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(",");
            let sql = format!("SELECT id, target_gene_ids FROM ent_compounds WHERE target_gene_ids IS NOT NULL");
            let rows = client.query(&sql, &[]).await?;
            for row in rows {
                let c = row_to_ent_compound(&row);
                if let Some(targets) = c.target_gene_ids {
                    for gid in targets {
                        if !wanted.contains(&gid) {
                            continue;
                        }
                        *counts.entry(gid).or_insert(0) += 1;
                    }
                }
            }
        }
        Ok(counts)
    }

    async fn count_pathways_by_symbol(&self, symbols: &[String]) -> Result<HashMap<String, u32>> {
        if symbols.is_empty() || !self.db.table_exists(schema::TABLE_ENT_PATHWAYS).await? {
            return Ok(HashMap::new());
        }
        let client = self.db.client();
        let mut counts = HashMap::new();
        let wanted: HashSet<String> = symbols.iter().cloned().collect();
        for chunk in symbols.chunks(80) {
            if chunk.is_empty() { continue; }
            let placeholders = (1..=chunk.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(",");
            let sql = format!("SELECT gene_members FROM ent_pathways WHERE gene_members IS NOT NULL");
            let rows = client.query(&sql, &[]).await?;
            for row in rows {
                let p = row_to_ent_pathway(&row);
                if let Some(members) = p.gene_members {
                    for m in members {
                        let key = m.trim().to_uppercase();
                        if wanted.contains(&key) {
                            *counts.entry(key).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
        Ok(counts)
    }
}

fn escape_sql(input: &str) -> String {
    input.replace('\'', "''")
}

// Row conversion helpers
fn row_to_ent_gene(row: &Row) -> crate::schema::EntGene {
    crate::schema::EntGene {
        id: row.get("id"),
        hgnc_id: row.get("hgnc_id"),
        symbol: row.get("symbol"),
        name: row.get("name"),
        uniprot_id: row.get("uniprot_id"),
        ensembl_id: row.get("ensembl_id"),
        entrez_id: row.get("entrez_id"),
        gene_biotype: row.get("gene_biotype"),
        chromosome: row.get("chromosome"),
        strand: Some(row.get::<_, i16>("strand")),
        aliases: row.get("aliases"),
        oncogene_flag: row.get::<_, bool>("oncogene_flag"),
        tsg_flag: row.get::<_, bool>("tsg_flag"),
        created_at: row.get::<_, chrono::DateTime<chrono::Utc>>("created_at"),
    }
}

fn row_to_ent_mutation(row: &Row) -> crate::schema::EntMutation {
    crate::schema::EntMutation {
        id: row.get("id"),
        gene_id: row.get("gene_id"),
        hgvs_p: row.get("hgvs_p"),
        hgvs_c: row.get("hgvs_c"),
        rs_id: row.get("rs_id"),
        aa_ref: row.get("aa_ref"),
        aa_alt: row.get("aa_alt"),
        aa_position: Some(row.get::<_, i32>("aa_position")),
        oncogenicity: row.get("oncogenicity"),
        hotspot_flag: row.get::<_, bool>("hotspot_flag"),
        vaf_context: row.get("vaf_context"),
        created_at: row.get::<_, chrono::DateTime<chrono::Utc>>("created_at"),
    }
}

fn row_to_ent_structure(row: &Row) -> crate::schema::EntStructure {
    crate::schema::EntStructure {
        id: row.get("id"),
        gene_id: row.get("gene_id"),
        pdb_ids: row.get("pdb_ids"),
        best_resolution: row.get("best_resolution"),
        exp_method: row.get("exp_method"),
        af_accession: row.get("af_accession"),
        af_plddt_mean: row.get("af_plddt_mean"),
        af_plddt_active: row.get("af_plddt_active"),
        has_pdb: row.get::<_, bool>("has_pdb"),
        has_alphafold: row.get::<_, bool>("has_alphafold"),
        updated_at: row.get::<_, chrono::DateTime<chrono::Utc>>("updated_at"),
    }
}

fn row_to_ent_druggability(row: &Row) -> crate::schema::EntDruggability {
    crate::schema::EntDruggability {
        id: row.get("id"),
        structure_id: row.get("structure_id"),
        fpocket_score: row.get("fpocket_score"),
        fpocket_volume: row.get("fpocket_volume"),
        fpocket_pocket_count: Some(row.get::<_, i32>("fpocket_pocket_count")),
        dogsitescorer: row.get("dogsitescorer"),
        overall_assessment: row.get("overall_assessment"),
        assessed_at: row.get::<_, chrono::DateTime<chrono::Utc>>("assessed_at"),
    }
}

fn row_to_ent_compound(row: &Row) -> crate::schema::EntCompound {
    crate::schema::EntCompound {
        id: row.get("id"),
        chembl_id: row.get("chembl_id"),
        name: row.get("name"),
        smiles: row.get("smiles"),
        inchi_key: row.get("inchi_key"),
        moa: row.get("moa"),
        patent_status: row.get("patent_status"),
        max_phase: Some(row.get::<_, i32>("max_phase")),
        target_gene_ids: row.get::<_, Option<Vec<uuid::Uuid>>>("target_gene_ids"),
        created_at: row.get::<_, chrono::DateTime<chrono::Utc>>("created_at"),
    }
}

fn row_to_ent_pathway(row: &Row) -> crate::schema::EntPathway {
    crate::schema::EntPathway {
        id: row.get("id"),
        kegg_id: row.get("kegg_id"),
        reactome_id: row.get("reactome_id"),
        go_term: row.get("go_term"),
        name: row.get("name"),
        gene_members: row.get::<_, Option<Vec<String>>>("gene_members"),
        source: row.get("source"),
        created_at: row.get::<_, chrono::DateTime<chrono::Utc>>("created_at"),
    }
}
