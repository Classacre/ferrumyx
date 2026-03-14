//! Phase 4 enrichment reader for `ent_*` tables.
//!
//! Provides batched, bounded lookups so ranker can replace proxy values with
//! source-backed signals when entity-stage tables are populated.

use crate::database::Database;
use crate::error::Result;
use crate::schema;
use crate::schema_arrow::{
    ent_druggability_to_record, ent_structure_to_record, record_to_ent_compound,
    record_to_ent_druggability, record_to_ent_gene, record_to_ent_mutation, record_to_ent_pathway,
    record_to_ent_structure,
};
use futures::StreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

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
        let table = self
            .db
            .connection()
            .open_table(schema::TABLE_ENT_STRUCTURES)
            .execute()
            .await?;
        let record = ent_structure_to_record(structure)?;
        let schema = record.schema();
        let iter = arrow_array::RecordBatchIterator::new(vec![Ok(record)], schema);
        let mut builder = table.merge_insert(&["gene_id"]);
        builder.when_matched_update_all(None);
        builder.execute(Box::new(iter)).await?;
        Ok(())
    }

    pub async fn upsert_druggability_signal(
        &self,
        druggability: &crate::schema::EntDruggability,
    ) -> Result<()> {
        let table = self
            .db
            .connection()
            .open_table(schema::TABLE_ENT_DRUGGABILITY)
            .execute()
            .await?;
        let record = ent_druggability_to_record(druggability)?;
        let schema = record.schema();
        let iter = arrow_array::RecordBatchIterator::new(vec![Ok(record)], schema);
        let mut builder = table.merge_insert(&["structure_id"]);
        builder.when_matched_update_all(None);
        builder.execute(Box::new(iter)).await?;
        Ok(())
    }

    async fn fetch_genes_by_symbol(
        &self,
        symbols: &[String],
    ) -> Result<Vec<crate::schema::EntGene>> {
        if !self.db.table_exists(schema::TABLE_ENT_GENES).await? {
            return Ok(Vec::new());
        }
        let table = self
            .db
            .connection()
            .open_table(schema::TABLE_ENT_GENES)
            .execute()
            .await?;
        let mut out = Vec::new();
        for chunk in symbols.chunks(150) {
            let filter = chunk
                .iter()
                .map(|s| format!("symbol = '{}'", escape_sql(s)))
                .collect::<Vec<_>>()
                .join(" OR ");
            let mut stream = table.query().only_if(&filter).execute().await?;
            while let Some(batch) = stream.next().await {
                let batch = batch?;
                for row in 0..batch.num_rows() {
                    out.push(record_to_ent_gene(&batch, row)?);
                }
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
        let table = self
            .db
            .connection()
            .open_table(schema::TABLE_ENT_MUTATIONS)
            .execute()
            .await?;
        let mut counts: HashMap<uuid::Uuid, u32> = HashMap::new();
        for chunk in gene_ids.chunks(150) {
            let filter = chunk
                .iter()
                .map(|id| format!("gene_id = '{}'", id))
                .collect::<Vec<_>>()
                .join(" OR ");
            let mut stream = table.query().only_if(&filter).execute().await?;
            while let Some(batch) = stream.next().await {
                let batch = batch?;
                for row in 0..batch.num_rows() {
                    let m = record_to_ent_mutation(&batch, row)?;
                    *counts.entry(m.gene_id).or_insert(0) += 1;
                }
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
        let table = self
            .db
            .connection()
            .open_table(schema::TABLE_ENT_STRUCTURES)
            .execute()
            .await?;
        let mut pdb_count: HashMap<uuid::Uuid, u32> = HashMap::new();
        let mut plddt_mean: HashMap<uuid::Uuid, Option<f64>> = HashMap::new();
        let mut structure_ids: HashMap<uuid::Uuid, Vec<uuid::Uuid>> = HashMap::new();
        for chunk in gene_ids.chunks(150) {
            let filter = chunk
                .iter()
                .map(|id| format!("gene_id = '{}'", id))
                .collect::<Vec<_>>()
                .join(" OR ");
            let mut stream = table.query().only_if(&filter).execute().await?;
            while let Some(batch) = stream.next().await {
                let batch = batch?;
                for row in 0..batch.num_rows() {
                    let s = record_to_ent_structure(&batch, row)?;
                    if s.has_pdb {
                        *pdb_count.entry(s.gene_id).or_insert(0) += 1;
                    }
                    let incoming = s.af_plddt_mean.map(|v| v as f64);
                    plddt_mean
                        .entry(s.gene_id)
                        .and_modify(|cur| {
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
        }
        Ok((pdb_count, plddt_mean, structure_ids))
    }

    async fn fetch_fpocket_by_structure_ids(
        &self,
        structure_ids: &[uuid::Uuid],
    ) -> Result<HashMap<uuid::Uuid, f64>> {
        if structure_ids.is_empty() || !self.db.table_exists(schema::TABLE_ENT_DRUGGABILITY).await?
        {
            return Ok(HashMap::new());
        }
        let table = self
            .db
            .connection()
            .open_table(schema::TABLE_ENT_DRUGGABILITY)
            .execute()
            .await?;
        let mut out: HashMap<uuid::Uuid, f64> = HashMap::new();
        for chunk in structure_ids.chunks(150) {
            let filter = chunk
                .iter()
                .map(|id| format!("structure_id = '{}'", id))
                .collect::<Vec<_>>()
                .join(" OR ");
            let mut stream = table.query().only_if(&filter).execute().await?;
            while let Some(batch) = stream.next().await {
                let batch = batch?;
                for row in 0..batch.num_rows() {
                    let d = record_to_ent_druggability(&batch, row)?;
                    if let Some(score) = d.fpocket_score {
                        out.entry(d.structure_id)
                            .and_modify(|v| *v = v.max(score as f64))
                            .or_insert(score as f64);
                    }
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
        let table = self
            .db
            .connection()
            .open_table(schema::TABLE_ENT_COMPOUNDS)
            .execute()
            .await?;
        let wanted: HashSet<uuid::Uuid> = gene_ids.iter().copied().collect();
        let mut seen_pairs: HashSet<(uuid::Uuid, uuid::Uuid)> = HashSet::new();
        let mut counts: HashMap<uuid::Uuid, u32> = HashMap::new();
        for chunk in gene_ids.chunks(80) {
            let filter = chunk
                .iter()
                .map(|id| format!("target_gene_ids LIKE '%{}%'", id))
                .collect::<Vec<_>>()
                .join(" OR ");
            let mut stream = table.query().only_if(&filter).execute().await?;
            while let Some(batch) = stream.next().await {
                let batch = batch?;
                for row in 0..batch.num_rows() {
                    let c = record_to_ent_compound(&batch, row)?;
                    let Some(targets) = c.target_gene_ids else {
                        continue;
                    };
                    for gid in targets {
                        if !wanted.contains(&gid) {
                            continue;
                        }
                        if seen_pairs.insert((c.id, gid)) {
                            *counts.entry(gid).or_insert(0) += 1;
                        }
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
        let table = self
            .db
            .connection()
            .open_table(schema::TABLE_ENT_PATHWAYS)
            .execute()
            .await?;
        let mut counts: HashMap<String, u32> = HashMap::new();
        let wanted: HashSet<String> = symbols.iter().cloned().collect();
        for chunk in symbols.chunks(80) {
            let filter = chunk
                .iter()
                .map(|s| format!("gene_members LIKE '%\"{}\"%'", escape_sql(s)))
                .collect::<Vec<_>>()
                .join(" OR ");
            let mut stream = table.query().only_if(&filter).execute().await?;
            while let Some(batch) = stream.next().await {
                let batch = batch?;
                for row in 0..batch.num_rows() {
                    let p = record_to_ent_pathway(&batch, row)?;
                    let Some(members) = p.gene_members else {
                        continue;
                    };
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
