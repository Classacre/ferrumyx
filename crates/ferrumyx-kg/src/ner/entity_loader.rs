//! Unified biomedical entity database loader.

use anyhow::Result;
use tracing::{info, warn};

pub struct BiomedicalDatabase {
    pub genes: Option<crate::ner::hgnc::HgncNormaliser>,
    pub cancers: Option<crate::ner::cancer_normaliser::CancerNormaliser>,
    pub mutations: crate::ner::hgvs::HgvsMutationNormaliser,
}

impl BiomedicalDatabase {
    pub async fn load() -> Result<Self> {
        info!("Loading biomedical databases...");
        let genes = match crate::ner::hgnc::HgncNormaliser::from_download().await {
            Ok(g) => Some(g),
            Err(e) => {
                warn!(
                    "Failed to download HGNC database: {}. Proceeding without genes.",
                    e
                );
                None
            }
        };

        let cancers = match crate::ner::cancer_normaliser::CancerNormaliser::from_download().await {
            Ok(c) => Some(c),
            Err(e) => {
                warn!(
                    "Failed to download OncoTree database: {}. Proceeding without cancers.",
                    e
                );
                None
            }
        };

        Ok(Self {
            genes,
            cancers,
            mutations: crate::ner::hgvs::HgvsMutationNormaliser::new(),
        })
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
