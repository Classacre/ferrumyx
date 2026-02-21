/// Core entity types mirroring the knowledge graph schema.
/// These are Rust representations of the PostgreSQL entity tables.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Gene / Protein
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gene {
    pub id: Uuid,
    pub hgnc_id: Option<String>,
    pub symbol: String,
    pub name: Option<String>,
    pub uniprot_id: Option<String>,
    pub ensembl_id: Option<String>,
    pub entrez_id: Option<String>,
    pub gene_biotype: Option<String>,
    pub chromosome: Option<String>,
    pub strand: Option<i16>,
    pub aliases: Vec<String>,
    pub oncogene_flag: bool,
    pub tsg_flag: bool,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Mutation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mutation {
    pub id: Uuid,
    pub gene_id: Uuid,
    pub hgvs_p: Option<String>,  // e.g. p.Gly12Asp
    pub hgvs_c: Option<String>,  // e.g. c.35G>A
    pub rs_id: Option<String>,   // e.g. rs121913529
    pub aa_ref: Option<String>,
    pub aa_alt: Option<String>,
    pub aa_position: Option<i32>,
    pub oncogenicity: Option<String>,
    pub hotspot_flag: bool,
    pub vaf_context: Option<String>, // somatic | germline | unknown
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Cancer Type (OncoTree)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancerType {
    pub id: Uuid,
    pub oncotree_code: String,  // e.g. PAAD
    pub oncotree_name: String,
    pub icd_o3_code: Option<String>,
    pub tissue: Option<String>,
    pub parent_code: Option<String>,
    pub level: Option<i32>,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Evidence type enum (used in KG facts)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceType {
    ExperimentalInVivo,
    ExperimentalInVitro,
    ClinicalTrialPhase3Plus,
    ClinicalTrialPhase12,
    ComputationalMl,
    ComputationalRuleBased,
    TextMined,
    DatabaseAssertion,
}

impl EvidenceType {
    /// Base confidence weight for this evidence type.
    /// See ARCHITECTURE.md §3.2
    pub fn base_weight(&self) -> f64 {
        match self {
            EvidenceType::ExperimentalInVivo         => 1.00,
            EvidenceType::ExperimentalInVitro        => 0.85,
            EvidenceType::ClinicalTrialPhase3Plus    => 1.00,
            EvidenceType::ClinicalTrialPhase12       => 0.75,
            EvidenceType::ComputationalMl            => 0.50,
            EvidenceType::ComputationalRuleBased     => 0.35,
            EvidenceType::TextMined                  => 0.30,
            EvidenceType::DatabaseAssertion          => 0.40,
        }
    }
}

// ---------------------------------------------------------------------------
// KG Fact
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgFact {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub predicate: String,
    pub object_id: Uuid,
    pub confidence: f64,
    pub evidence_type: EvidenceType,
    pub evidence_weight: f64,
    pub source_pmid: Option<String>,
    pub source_doi: Option<String>,
    pub source_db: Option<String>,
    pub sample_size: Option<i32>,
    pub study_type: Option<String>,
    pub created_at: DateTime<Utc>,
    pub valid_from: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>,
}

impl KgFact {
    /// Is this fact currently valid?
    pub fn is_current(&self) -> bool {
        self.valid_until.is_none()
    }
}

impl EvidenceType {
    /// Serialize to the string stored in the DB.
    pub fn as_str(&self) -> &'static str {
        match self {
            EvidenceType::ExperimentalInVivo         => "experimental_in_vivo",
            EvidenceType::ExperimentalInVitro        => "experimental_in_vitro",
            EvidenceType::ClinicalTrialPhase3Plus    => "clinical_trial_phase3_plus",
            EvidenceType::ClinicalTrialPhase12       => "clinical_trial_phase1_2",
            EvidenceType::ComputationalMl            => "computational_ml",
            EvidenceType::ComputationalRuleBased     => "computational_rule_based",
            EvidenceType::TextMined                  => "text_mined",
            EvidenceType::DatabaseAssertion          => "database_assertion",
        }
    }

    /// Parse from the string stored in the DB.
    pub fn from_str(s: &str) -> Self {
        match s {
            "experimental_in_vivo"        => EvidenceType::ExperimentalInVivo,
            "experimental_in_vitro"       => EvidenceType::ExperimentalInVitro,
            "clinical_trial_phase3_plus"  => EvidenceType::ClinicalTrialPhase3Plus,
            "clinical_trial_phase1_2"     => EvidenceType::ClinicalTrialPhase12,
            "computational_ml"            => EvidenceType::ComputationalMl,
            "computational_rule_based"    => EvidenceType::ComputationalRuleBased,
            "text_mined"                  => EvidenceType::TextMined,
            _                             => EvidenceType::DatabaseAssertion,
        }
    }
}
