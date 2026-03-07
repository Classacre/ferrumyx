//! Comprehensive biomedical entity database for fast dictionary-based NER.

use std::collections::{HashMap, HashSet};
use ahash::AHashMap;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct EntityDatabase {
    pub genes: GeneDatabase,
    pub diseases: DiseaseDatabase,
    pub chemicals: ChemicalDatabase,
    pub cancer_types: CancerTypeDatabase,
}

#[derive(Debug, Clone, Default)]
pub struct GeneDatabase {
    pub approved_symbols: AHashMap<String, GeneInfo>,
    pub aliases: AHashMap<String, String>,
    pub all_symbols: HashSet<String>,
    pub matcher: Option<Regex>,
}

#[derive(Debug, Clone)]
pub struct GeneInfo {
    pub hgnc_id: String,
    pub approved_symbol: String,
    pub approved_name: String,
    pub aliases: Vec<String>,
    pub entrez_id: Option<String>,
    pub ensembl_id: Option<String>,
    pub uniprot_ids: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DiseaseDatabase {
    pub by_mesh_id: AHashMap<String, DiseaseInfo>,
    pub by_name: AHashMap<String, String>,
    pub all_terms: HashSet<String>,
    pub matcher: Option<Regex>,
}

#[derive(Debug, Clone)]
pub struct DiseaseInfo {
    pub mesh_id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub tree_numbers: Vec<String>,
    pub category: crate::ner::entity_loader::DiseaseCategory,
}

#[derive(Debug, Clone, Default)]
pub struct ChemicalDatabase {
    pub by_chembl_id: AHashMap<String, ChemicalInfo>,
    pub by_name: AHashMap<String, String>,
    pub all_names: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct ChemicalInfo {
    pub chembl_id: String,
    pub name: String,
    pub synonyms: Vec<String>,
    pub smiles: Option<String>,
    pub inchi_key: Option<String>,
    pub is_drug: bool,
}

#[derive(Debug, Clone, Default)]
pub struct CancerTypeDatabase {
    pub types: HashSet<String>,
    pub subtypes: AHashMap<String, String>,
}

impl EntityDatabase {
    pub fn with_defaults() -> Self {
        Self {
            genes: GeneDatabase::default(),
            diseases: DiseaseDatabase::default(),
            chemicals: ChemicalDatabase::default(),
            cancer_types: CancerTypeDatabase::default(),
        }
    }
}
