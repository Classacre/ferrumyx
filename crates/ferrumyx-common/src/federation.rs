//! Federation manifest schema for shared Ferrumyx knowledge-base contributions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

pub const FEDERATION_SCHEMA_VERSION: &str = "ferrumyx.federation.v1";
pub const FEDERATION_SCHEMA_URL: &str = "https://schema.ferrumyx.ai/federation/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionManifest {
    pub schema_version: String,
    pub manifest_id: Uuid,
    pub dataset_id: String,
    pub snapshot_id: String,
    pub parent_snapshot_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub contributor: ContributorIdentity,
    pub provenance: ManifestProvenance,
    pub stats: ManifestStats,
    pub artifacts: Vec<ArtifactDigest>,
    pub quality: QualitySummary,
    pub signature: Option<ManifestSignature>,
    #[serde(default)]
    pub annotations: BTreeMap<String, String>,
}

impl ContributionManifest {
    pub fn template() -> Self {
        Self {
            schema_version: FEDERATION_SCHEMA_VERSION.to_string(),
            manifest_id: Uuid::new_v4(),
            dataset_id: "ferrumyx-public-kb".to_string(),
            snapshot_id: format!("snap-{}", Uuid::new_v4()),
            parent_snapshot_id: None,
            created_at: Utc::now(),
            contributor: ContributorIdentity {
                instance_id: "local-instance".to_string(),
                display_name: "Local Ferrumyx Node".to_string(),
                contact: None,
                public_key_id: None,
            },
            provenance: ManifestProvenance {
                app_version: env!("CARGO_PKG_VERSION").to_string(),
                architecture_version: "1.0.0-mvp".to_string(),
                runtime_profile: "balanced".to_string(),
                generated_by: "ferrumyx-web".to_string(),
            },
            stats: ManifestStats::default(),
            artifacts: Vec::new(),
            quality: QualitySummary::default(),
            signature: None,
            annotations: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorIdentity {
    pub instance_id: String,
    pub display_name: String,
    pub contact: Option<String>,
    pub public_key_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestProvenance {
    pub app_version: String,
    pub architecture_version: String,
    pub runtime_profile: String,
    pub generated_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestStats {
    #[serde(default)]
    pub table_counts: BTreeMap<String, u64>,
    pub unique_predicates: u64,
    pub total_relations: u64,
    pub gene_entities: u64,
    pub generated_at: DateTime<Utc>,
}

impl Default for ManifestStats {
    fn default() -> Self {
        Self {
            table_counts: BTreeMap::new(),
            unique_predicates: 0,
            total_relations: 0,
            gene_entities: 0,
            generated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactDigest {
    pub artifact_kind: String,
    pub relative_path: String,
    pub row_count: u64,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitySummary {
    pub parse_success_rate: f64,
    pub duplicate_identity_rate: f64,
    pub generic_predicate_share: f64,
    #[serde(default)]
    pub notes: Vec<String>,
}

impl Default for QualitySummary {
    fn default() -> Self {
        Self {
            parse_success_rate: 0.0,
            duplicate_identity_rate: 0.0,
            generic_predicate_share: 0.0,
            notes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSignature {
    pub algorithm: String,
    pub key_id: String,
    pub signature_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestValidationReport {
    pub valid: bool,
    pub issues: Vec<ValidationIssue>,
}

impl ManifestValidationReport {
    pub fn ok() -> Self {
        Self {
            valid: true,
            issues: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub code: String,
    pub message: String,
}

impl ValidationIssue {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Error,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Warning,
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationSeverity {
    Error,
    Warning,
}
