//! Unified biomedical entity database loader.

use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::{Context, Result};

pub struct BiomedicalDatabase {
    pub genes: HashMap<String, String>,
}

impl BiomedicalDatabase {
    pub async fn load() -> Result<Self> {
        Ok(Self { genes: HashMap::new() })
    }
}

#[derive(Debug, Clone)]
pub struct GeneEntry {
    pub symbol: String,
    pub hgnc_id: String,
}

#[derive(Debug, Clone)]
pub struct DiseaseEntry {
    pub name: String,
    pub mesh_id: String,
}

#[derive(Debug, Clone)]
pub struct ChemicalEntry {
    pub name: String,
    pub chembl_id: String,
}

#[derive(Debug, Clone, Copy)]
pub enum DiseaseCategory {
    Neoplasm,
    Other,
}
