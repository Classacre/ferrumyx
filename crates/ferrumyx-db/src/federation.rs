//! Federation draft + validation helpers.

use crate::{
    chunks::ChunkRepository,
    database::Database,
    entities::EntityRepository,
    entity_mentions::EntityMentionRepository,
    kg_facts::KgFactRepository,
    papers::PaperRepository,
    schema::{
        EntityType, TABLE_CHUNKS, TABLE_ENTITIES, TABLE_ENTITY_MENTIONS, TABLE_INGESTION_AUDIT,
        TABLE_KG_CONFLICTS, TABLE_KG_FACTS, TABLE_PAPERS, TABLE_TARGET_SCORES,
    },
    target_scores::TargetScoreRepository,
    Result,
};
use base64::Engine;
use chrono::{Duration, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use ferrumyx_common::federation::{
    ArtifactDigest, ContributionManifest, ContributorIdentity, FEDERATION_SCHEMA_VERSION,
    ManifestSignature, ManifestStats, ManifestValidationReport, QualitySummary, ValidationIssue,
    ValidationSeverity,
};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const QUALITY_PAPER_SCAN_LIMIT: usize = 20_000;
const QUALITY_FACT_SCAN_LIMIT: usize = 20_000;
const EXPORT_PAGE_SIZE: usize = 1_000;
const DEFAULT_KEYS_DIR: &str = "./output/federation/keys";
const DEFAULT_TRUST_REGISTRY_PATH: &str = "./output/federation/trust_registry.json";
const DEFAULT_MERGE_QUEUE_PATH: &str = "./output/federation/merge_queue.json";
const DEFAULT_CANONICAL_LINEAGE_PATH: &str = "./output/federation/canonical_lineage.json";
const SIGNATURE_ALGORITHM_ED25519: &str = "ed25519";
const DEFAULT_REQUIRE_SIGNATURE_FOR_QUEUE: bool = true;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ManifestDraftRequest {
    pub dataset_id: Option<String>,
    pub snapshot_id: Option<String>,
    pub parent_snapshot_id: Option<String>,
    pub contributor_instance_id: Option<String>,
    pub contributor_name: Option<String>,
    pub contributor_contact: Option<String>,
    pub runtime_profile: Option<String>,
}

impl Default for ManifestDraftRequest {
    fn default() -> Self {
        Self {
            dataset_id: Some("ferrumyx-public-kb".to_string()),
            snapshot_id: None,
            parent_snapshot_id: None,
            contributor_instance_id: Some("local-instance".to_string()),
            contributor_name: Some("Local Ferrumyx Node".to_string()),
            contributor_contact: None,
            runtime_profile: Some("balanced".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageExportRequest {
    #[serde(default)]
    pub draft: ManifestDraftRequest,
    pub output_root: Option<String>,
    #[serde(default)]
    pub include_heavy_artifacts: bool,
}

impl Default for PackageExportRequest {
    fn default() -> Self {
        Self {
            draft: ManifestDraftRequest::default(),
            output_root: Some(default_package_root()),
            include_heavy_artifacts: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageExportResult {
    pub package_dir: String,
    pub manifest_path: String,
    pub manifest: ContributionManifest,
    pub validation: PackageValidationReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageValidationRequest {
    pub package_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSignRequest {
    pub package_dir: String,
    pub key_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSignResult {
    pub package_dir: String,
    pub key_id: String,
    pub public_key_base64: String,
    pub signature_base64: String,
    pub signed_manifest_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustKeyRecord {
    pub key_id: String,
    pub algorithm: String,
    pub public_key_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustKeyUpsertRequest {
    pub key_id: String,
    pub algorithm: String,
    pub public_key_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustKeyRevokeRequest {
    pub key_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MergeQueueStatus {
    PendingReview,
    Approved,
    Rejected,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeQueueEntry {
    pub queue_id: String,
    pub submitted_at: String,
    pub submitted_by: Option<String>,
    pub package_dir: String,
    pub dataset_id: String,
    pub snapshot_id: String,
    pub parent_snapshot_id: Option<String>,
    pub manifest_id: String,
    pub status: MergeQueueStatus,
    pub validation: PackageValidationReport,
    pub decision_at: Option<String>,
    pub decision_by: Option<String>,
    pub decision_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MergeQueueStore {
    #[serde(default)]
    pub entries: Vec<MergeQueueEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeSubmitRequest {
    pub package_dir: String,
    pub submitted_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeSubmitResult {
    pub entry: MergeQueueEntry,
    pub queue_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeDecisionRequest {
    pub queue_id: String,
    pub approve: bool,
    pub decision_by: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeDecisionResult {
    pub entry: MergeQueueEntry,
    pub canonical_lineage_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalSnapshotRecord {
    pub dataset_id: String,
    pub snapshot_id: String,
    pub parent_snapshot_id: Option<String>,
    pub manifest_id: String,
    pub package_dir: String,
    pub approved_at: String,
    pub approved_by: Option<String>,
    pub quality: QualitySummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CanonicalLineageStore {
    #[serde(default)]
    pub snapshots: Vec<CanonicalSnapshotRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureValidationResult {
    pub present: bool,
    pub valid: bool,
    pub key_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactValidationResult {
    pub relative_path: String,
    pub exists: bool,
    pub expected_sha256: String,
    pub actual_sha256: Option<String>,
    pub sha256_match: bool,
    pub expected_bytes: u64,
    pub actual_bytes: Option<u64>,
    pub bytes_match: bool,
    pub expected_row_count: u64,
    pub actual_row_count: Option<u64>,
    pub row_count_match: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageValidationReport {
    pub valid: bool,
    pub manifest_validation: ManifestValidationReport,
    pub signature_validation: SignatureValidationResult,
    pub artifact_checks: Vec<ArtifactValidationResult>,
}

pub async fn build_contribution_manifest_draft(
    db: Arc<Database>,
    request: ManifestDraftRequest,
) -> Result<ContributionManifest> {
    let mut manifest = ContributionManifest::template();

    let dataset_id = request
        .dataset_id
        .unwrap_or_else(|| "ferrumyx-public-kb".to_string())
        .trim()
        .to_string();
    manifest.dataset_id = if dataset_id.is_empty() {
        "ferrumyx-public-kb".to_string()
    } else {
        dataset_id
    };

    manifest.snapshot_id = request
        .snapshot_id
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| format!("snap-{}", uuid::Uuid::new_v4()));
    manifest.parent_snapshot_id = request.parent_snapshot_id.filter(|v| !v.trim().is_empty());
    manifest.created_at = Utc::now();
    manifest.contributor = ContributorIdentity {
        instance_id: request
            .contributor_instance_id
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "local-instance".to_string()),
        display_name: request
            .contributor_name
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "Local Ferrumyx Node".to_string()),
        contact: request.contributor_contact.filter(|v| !v.trim().is_empty()),
        public_key_id: None,
    };
    manifest.provenance.runtime_profile = request
        .runtime_profile
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "balanced".to_string());
    manifest.provenance.generated_by = "ferrumyx-db".to_string();

    let table_counts = collect_table_counts(&db).await?;
    let relation_repo = KgFactRepository::new(db.clone());
    let entity_repo = EntityRepository::new(db.clone());

    manifest.stats = ManifestStats {
        table_counts,
        unique_predicates: relation_repo.get_predicates().await.unwrap_or_default().len() as u64,
        total_relations: relation_repo.count().await.unwrap_or(0),
        gene_entities: entity_repo.count_by_type(EntityType::Gene).await.unwrap_or(0),
        generated_at: Utc::now(),
    };
    manifest.quality = collect_quality_summary(db).await?;

    Ok(manifest)
}

pub async fn export_contribution_package(
    db: Arc<Database>,
    request: PackageExportRequest,
) -> Result<PackageExportResult> {
    let mut manifest = build_contribution_manifest_draft(db.clone(), request.draft).await?;
    let package_root = request.output_root.unwrap_or_else(default_package_root);
    let package_dir = PathBuf::from(package_root).join(&manifest.snapshot_id);
    std::fs::create_dir_all(&package_dir)?;

    let mut artifacts: Vec<ArtifactDigest> = Vec::new();
    artifacts.push(export_table_papers(&db, &package_dir).await?);
    artifacts.push(export_table_entities(&db, &package_dir).await?);
    artifacts.push(export_table_kg_facts(&db, &package_dir).await?);
    artifacts.push(export_table_target_scores(&db, &package_dir).await?);

    if request.include_heavy_artifacts {
        artifacts.push(export_table_chunks(&db, &package_dir).await?);
        artifacts.push(export_table_entity_mentions(&db, &package_dir).await?);
    }

    manifest.artifacts = artifacts;
    manifest
        .annotations
        .insert("export_mode".to_string(), "jsonl".to_string());
    manifest.annotations.insert(
        "include_heavy_artifacts".to_string(),
        if request.include_heavy_artifacts {
            "true".to_string()
        } else {
            "false".to_string()
        },
    );

    let manifest_path = package_dir.join("manifest.json");
    write_manifest(&manifest_path, &manifest)?;

    let validation = validate_contribution_package(&package_dir)?;
    Ok(PackageExportResult {
        package_dir: package_dir.to_string_lossy().to_string(),
        manifest_path: manifest_path.to_string_lossy().to_string(),
        manifest,
        validation,
    })
}

pub fn sign_contribution_package(request: PackageSignRequest) -> Result<PackageSignResult> {
    let package_dir = PathBuf::from(request.package_dir.trim());
    if !package_dir.exists() {
        return Err(crate::error::DbError::NotFound(format!(
            "package_dir '{}' does not exist",
            package_dir.to_string_lossy()
        )));
    }

    let manifest_path = package_dir.join("manifest.json");
    let raw = std::fs::read_to_string(&manifest_path)?;
    let mut manifest: ContributionManifest = serde_json::from_str(&raw)?;
    manifest.signature = None;

    let key_name = request
        .key_name
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "default".to_string());

    let keys_dir = federation_keys_dir_path();
    let key_entry = ensure_signing_key(&keys_dir, &key_name)?;
    let payload = manifest_signing_payload(&manifest)?;
    let signature = key_entry.signing_key.sign(&payload);
    let signature_base64 =
        base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());

    manifest.signature = Some(ManifestSignature {
        algorithm: SIGNATURE_ALGORITHM_ED25519.to_string(),
        key_id: key_entry.key_id.clone(),
        signature_base64: signature_base64.clone(),
    });
    write_manifest(&manifest_path, &manifest)?;

    upsert_trust_registry_key(
        &federation_trust_registry_path(),
        TrustRegistryEntry {
            key_id: key_entry.key_id.clone(),
            algorithm: SIGNATURE_ALGORITHM_ED25519.to_string(),
            public_key_base64: key_entry.public_key_base64.clone(),
        },
    )?;

    Ok(PackageSignResult {
        package_dir: package_dir.to_string_lossy().to_string(),
        key_id: key_entry.key_id,
        public_key_base64: key_entry.public_key_base64,
        signature_base64,
        signed_manifest_path: manifest_path.to_string_lossy().to_string(),
    })
}

pub fn validate_contribution_package(package_dir: impl AsRef<Path>) -> Result<PackageValidationReport> {
    let package_dir = package_dir.as_ref();
    let manifest_path = package_dir.join("manifest.json");
    let raw = std::fs::read_to_string(&manifest_path)?;
    let manifest: ContributionManifest = serde_json::from_str(&raw)?;

    let manifest_validation = validate_contribution_manifest(&manifest);
    let signature_validation = verify_manifest_signature(
        &manifest,
        &federation_trust_registry_path(),
    )?;
    let mut artifact_checks = Vec::with_capacity(manifest.artifacts.len());

    for artifact in &manifest.artifacts {
        let path = package_dir.join(&artifact.relative_path);
        if !path.exists() {
            artifact_checks.push(ArtifactValidationResult {
                relative_path: artifact.relative_path.clone(),
                exists: false,
                expected_sha256: artifact.sha256.clone(),
                actual_sha256: None,
                sha256_match: false,
                expected_bytes: artifact.bytes,
                actual_bytes: None,
                bytes_match: false,
                expected_row_count: artifact.row_count,
                actual_row_count: None,
                row_count_match: false,
            });
            continue;
        }

        let actual = digest_file(&path)?;
        let actual_rows = count_file_lines(&path)?;
        let actual_sha = actual.sha256;
        let actual_bytes = actual.bytes;
        let sha_match = actual_sha == artifact.sha256;
        let bytes_match = actual_bytes == artifact.bytes;
        artifact_checks.push(ArtifactValidationResult {
            relative_path: artifact.relative_path.clone(),
            exists: true,
            expected_sha256: artifact.sha256.clone(),
            actual_sha256: Some(actual_sha),
            sha256_match: sha_match,
            expected_bytes: artifact.bytes,
            actual_bytes: Some(actual_bytes),
            bytes_match,
            expected_row_count: artifact.row_count,
            actual_row_count: Some(actual_rows),
            row_count_match: actual_rows == artifact.row_count,
        });
    }

    let artifacts_ok = artifact_checks
        .iter()
        .all(|check| check.exists && check.sha256_match && check.bytes_match && check.row_count_match);
    Ok(PackageValidationReport {
        valid: manifest_validation.valid && artifacts_ok && signature_validation.valid,
        manifest_validation,
        signature_validation,
        artifact_checks,
    })
}

pub fn submit_package_for_merge(request: MergeSubmitRequest) -> Result<MergeSubmitResult> {
    let package_dir = request.package_dir.trim();
    if package_dir.is_empty() {
        return Err(crate::error::DbError::InvalidQuery(
            "package_dir is required".to_string(),
        ));
    }

    let package_path = PathBuf::from(package_dir);
    if !package_path.exists() {
        return Err(crate::error::DbError::NotFound(format!(
            "package_dir '{}' does not exist",
            package_dir
        )));
    }

    let manifest = read_manifest_from_package(&package_path)?;
    let validation = validate_contribution_package(&package_path)?;

    let require_signature = require_signature_for_queue();
    let signature_ok = validation.signature_validation.present && validation.signature_validation.valid;
    let mut effective_validation = validation.clone();
    let is_valid = if require_signature {
        let ok = validation.valid && signature_ok;
        if !ok && (!validation.signature_validation.present || !validation.signature_validation.valid) {
            effective_validation
                .manifest_validation
                .issues
                .push(ValidationIssue::error(
                    "signature_required",
                    "signed manifest from trusted key is required for merge queue admission",
                ));
            effective_validation.valid = false;
        }
        ok
    } else {
        validation.valid
    };
    let status = if is_valid {
        MergeQueueStatus::PendingReview
    } else {
        MergeQueueStatus::Invalid
    };
    let now = Utc::now().to_rfc3339();
    let entry = MergeQueueEntry {
        queue_id: format!("mq-{}", uuid::Uuid::new_v4()),
        submitted_at: now.clone(),
        submitted_by: request.submitted_by.filter(|v| !v.trim().is_empty()),
        package_dir: package_path.to_string_lossy().to_string(),
        dataset_id: manifest.dataset_id.clone(),
        snapshot_id: manifest.snapshot_id.clone(),
        parent_snapshot_id: manifest.parent_snapshot_id.clone(),
        manifest_id: manifest.manifest_id.to_string(),
        status,
        validation: effective_validation,
        decision_at: if !is_valid { Some(now) } else { None },
        decision_by: if is_valid {
            None
        } else {
            Some("auto-validator".to_string())
        },
        decision_reason: if is_valid {
            None
        } else {
            Some(if require_signature {
                "package failed validation/signature policy; cannot enter moderation queue"
                    .to_string()
            } else {
                "package failed validation; cannot enter moderation queue".to_string()
            })
        },
    };

    let queue_path = federation_merge_queue_path();
    let mut queue = load_merge_queue(&queue_path)?;
    if queue.entries.iter().any(|existing| {
        existing.dataset_id == entry.dataset_id
            && existing.snapshot_id == entry.snapshot_id
            && existing.status != MergeQueueStatus::Rejected
    }) {
        return Err(crate::error::DbError::InvalidQuery(format!(
            "snapshot '{}' for dataset '{}' is already queued",
            entry.snapshot_id, entry.dataset_id
        )));
    }
    queue.entries.push(entry.clone());
    save_merge_queue(&queue_path, &queue)?;

    Ok(MergeSubmitResult {
        entry,
        queue_size: queue.entries.len(),
    })
}

pub fn list_merge_queue() -> Result<MergeQueueStore> {
    load_merge_queue(&federation_merge_queue_path())
}

pub fn decide_merge_queue(request: MergeDecisionRequest) -> Result<MergeDecisionResult> {
    let queue_path = federation_merge_queue_path();
    let mut queue = load_merge_queue(&queue_path)?;
    let idx = queue
        .entries
        .iter()
        .position(|entry| entry.queue_id == request.queue_id)
        .ok_or_else(|| {
            crate::error::DbError::NotFound(format!("queue_id '{}' not found", request.queue_id))
        })?;

    let entry = queue
        .entries
        .get_mut(idx)
        .ok_or_else(|| crate::error::DbError::NotFound("queue entry missing".to_string()))?;
    if entry.status != MergeQueueStatus::PendingReview {
        return Err(crate::error::DbError::InvalidQuery(format!(
            "queue entry '{}' is not pending review (current: {:?})",
            entry.queue_id, entry.status
        )));
    }

    entry.status = if request.approve {
        MergeQueueStatus::Approved
    } else {
        MergeQueueStatus::Rejected
    };
    entry.decision_at = Some(Utc::now().to_rfc3339());
    entry.decision_by = request.decision_by.filter(|v| !v.trim().is_empty());
    entry.decision_reason = request.reason.filter(|v| !v.trim().is_empty());
    let updated = entry.clone();

    save_merge_queue(&queue_path, &queue)?;

    let mut lineage = load_canonical_lineage(&federation_canonical_lineage_path())?;
    if request.approve {
        if !lineage.snapshots.iter().any(|snap| {
            snap.dataset_id == updated.dataset_id && snap.snapshot_id == updated.snapshot_id
        }) {
            let manifest = read_manifest_from_package(Path::new(&updated.package_dir))?;
            lineage.snapshots.push(CanonicalSnapshotRecord {
                dataset_id: updated.dataset_id.clone(),
                snapshot_id: updated.snapshot_id.clone(),
                parent_snapshot_id: updated.parent_snapshot_id.clone(),
                manifest_id: updated.manifest_id.clone(),
                package_dir: updated.package_dir.clone(),
                approved_at: updated
                    .decision_at
                    .clone()
                    .unwrap_or_else(|| Utc::now().to_rfc3339()),
                approved_by: updated.decision_by.clone(),
                quality: manifest.quality,
            });
            save_canonical_lineage(&federation_canonical_lineage_path(), &lineage)?;
        }
    }

    Ok(MergeDecisionResult {
        entry: updated,
        canonical_lineage_size: lineage.snapshots.len(),
    })
}

pub fn get_canonical_lineage() -> Result<CanonicalLineageStore> {
    load_canonical_lineage(&federation_canonical_lineage_path())
}

pub fn list_trusted_signing_keys() -> Result<Vec<TrustKeyRecord>> {
    let registry = load_trust_registry(&federation_trust_registry_path())?;
    Ok(registry
        .keys
        .into_iter()
        .map(|entry| TrustKeyRecord {
            key_id: entry.key_id,
            algorithm: entry.algorithm,
            public_key_base64: entry.public_key_base64,
        })
        .collect())
}

pub fn upsert_trusted_signing_key(request: TrustKeyUpsertRequest) -> Result<TrustKeyRecord> {
    let key_id = request.key_id.trim().to_string();
    if key_id.is_empty() {
        return Err(crate::error::DbError::InvalidQuery(
            "key_id is required".to_string(),
        ));
    }
    let algorithm = request.algorithm.trim().to_ascii_lowercase();
    if algorithm != SIGNATURE_ALGORITHM_ED25519 {
        return Err(crate::error::DbError::InvalidQuery(format!(
            "unsupported algorithm '{}'; expected '{}'",
            request.algorithm, SIGNATURE_ALGORITHM_ED25519
        )));
    }
    let encoded = request.public_key_base64.trim().to_string();
    if encoded.is_empty() {
        return Err(crate::error::DbError::InvalidQuery(
            "public_key_base64 is required".to_string(),
        ));
    }
    let _ = decode_base64_fixed::<32>(&encoded)?;

    let entry = TrustRegistryEntry {
        key_id: key_id.clone(),
        algorithm,
        public_key_base64: encoded.clone(),
    };
    upsert_trust_registry_key(&federation_trust_registry_path(), entry)?;
    Ok(TrustKeyRecord {
        key_id,
        algorithm: SIGNATURE_ALGORITHM_ED25519.to_string(),
        public_key_base64: encoded,
    })
}

pub fn revoke_trusted_signing_key(request: TrustKeyRevokeRequest) -> Result<bool> {
    let key_id = request.key_id.trim();
    if key_id.is_empty() {
        return Err(crate::error::DbError::InvalidQuery(
            "key_id is required".to_string(),
        ));
    }
    let path = federation_trust_registry_path();
    let mut registry = load_trust_registry(&path)?;
    let before = registry.keys.len();
    registry.keys.retain(|entry| entry.key_id != key_id);
    let removed = registry.keys.len() != before;
    if removed {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let pretty = serde_json::to_string_pretty(&registry)?;
        std::fs::write(path, pretty)?;
    }
    Ok(removed)
}

pub fn validate_contribution_manifest(manifest: &ContributionManifest) -> ManifestValidationReport {
    let mut issues: Vec<ValidationIssue> = Vec::new();

    if manifest.schema_version.trim() != FEDERATION_SCHEMA_VERSION {
        issues.push(ValidationIssue::error(
            "schema_version_mismatch",
            format!(
                "schema_version must be '{}', got '{}'",
                FEDERATION_SCHEMA_VERSION, manifest.schema_version
            ),
        ));
    }

    if manifest.dataset_id.trim().is_empty() {
        issues.push(ValidationIssue::error(
            "dataset_id_missing",
            "dataset_id must be non-empty",
        ));
    }

    if manifest.snapshot_id.trim().is_empty() {
        issues.push(ValidationIssue::error(
            "snapshot_id_missing",
            "snapshot_id must be non-empty",
        ));
    }

    if manifest.parent_snapshot_id.as_deref() == Some(manifest.snapshot_id.as_str()) {
        issues.push(ValidationIssue::error(
            "parent_snapshot_cycle",
            "parent_snapshot_id cannot equal snapshot_id",
        ));
    }

    let has_any_rows = manifest.stats.table_counts.values().any(|count| *count > 0);
    if !has_any_rows {
        issues.push(ValidationIssue::warning(
            "empty_snapshot",
            "all table_counts are zero; snapshot is likely a dry run",
        ));
    }

    validate_ratio(
        &mut issues,
        "parse_success_rate",
        manifest.quality.parse_success_rate,
    );
    validate_ratio(
        &mut issues,
        "duplicate_identity_rate",
        manifest.quality.duplicate_identity_rate,
    );
    validate_ratio(
        &mut issues,
        "generic_predicate_share",
        manifest.quality.generic_predicate_share,
    );

    let mut seen_paths = HashSet::new();
    for artifact in &manifest.artifacts {
        if artifact.relative_path.trim().is_empty() {
            issues.push(ValidationIssue::error(
                "artifact_path_missing",
                "artifact relative_path must be non-empty",
            ));
        }
        if !seen_paths.insert(artifact.relative_path.clone()) {
            issues.push(ValidationIssue::error(
                "artifact_path_duplicate",
                format!("duplicate artifact path '{}'", artifact.relative_path),
            ));
        }
        if !artifact.sha256.is_empty() && !is_sha256_hex(&artifact.sha256) {
            issues.push(ValidationIssue::error(
                "artifact_sha_invalid",
                format!(
                    "artifact '{}' has invalid sha256 digest",
                    artifact.relative_path
                ),
            ));
        }
    }

    if let Some(sig) = &manifest.signature {
        if sig.algorithm.trim().is_empty() {
            issues.push(ValidationIssue::error(
                "signature_algorithm_missing",
                "signature.algorithm must be non-empty",
            ));
        }
        if sig.key_id.trim().is_empty() {
            issues.push(ValidationIssue::error(
                "signature_key_id_missing",
                "signature.key_id must be non-empty",
            ));
        }
        if sig.signature_base64.trim().is_empty() {
            issues.push(ValidationIssue::error(
                "signature_data_missing",
                "signature.signature_base64 must be non-empty",
            ));
        }
    }

    let now_plus_tolerance = Utc::now() + Duration::minutes(5);
    if manifest.created_at > now_plus_tolerance {
        issues.push(ValidationIssue::warning(
            "created_at_future",
            "created_at is in the future beyond tolerance; check clock skew",
        ));
    }

    let valid = !issues
        .iter()
        .any(|issue| issue.severity == ValidationSeverity::Error);
    ManifestValidationReport { valid, issues }
}

fn default_package_root() -> String {
    "./output/federation".to_string()
}

fn default_keys_dir() -> String {
    DEFAULT_KEYS_DIR.to_string()
}

fn default_merge_queue_path() -> String {
    DEFAULT_MERGE_QUEUE_PATH.to_string()
}

fn require_signature_for_queue() -> bool {
    std::env::var("FERRUMYX_FED_REQUIRE_SIGNATURE_FOR_QUEUE")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(DEFAULT_REQUIRE_SIGNATURE_FOR_QUEUE)
}

fn federation_keys_dir_path() -> PathBuf {
    if let Ok(path) = std::env::var("FERRUMYX_FED_KEYS_DIR") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    PathBuf::from(default_keys_dir())
}

fn federation_trust_registry_path() -> PathBuf {
    if let Ok(path) = std::env::var("FERRUMYX_FED_TRUST_REGISTRY_PATH") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    PathBuf::from(DEFAULT_TRUST_REGISTRY_PATH)
}

fn federation_merge_queue_path() -> PathBuf {
    if let Ok(path) = std::env::var("FERRUMYX_FED_MERGE_QUEUE_PATH") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    PathBuf::from(default_merge_queue_path())
}

fn federation_canonical_lineage_path() -> PathBuf {
    if let Ok(path) = std::env::var("FERRUMYX_FED_CANONICAL_LINEAGE_PATH") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    PathBuf::from(DEFAULT_CANONICAL_LINEAGE_PATH)
}

fn read_manifest_from_package(package_dir: &Path) -> Result<ContributionManifest> {
    let manifest_path = package_dir.join("manifest.json");
    let raw = std::fs::read_to_string(&manifest_path)?;
    let manifest: ContributionManifest = serde_json::from_str(&raw)?;
    Ok(manifest)
}

fn load_merge_queue(path: &Path) -> Result<MergeQueueStore> {
    if !path.exists() {
        return Ok(MergeQueueStore::default());
    }
    let raw = std::fs::read_to_string(path)?;
    let queue: MergeQueueStore = serde_json::from_str(&raw)?;
    Ok(queue)
}

fn save_merge_queue(path: &Path, queue: &MergeQueueStore) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let pretty = serde_json::to_string_pretty(queue)?;
    std::fs::write(path, pretty)?;
    Ok(())
}

fn load_canonical_lineage(path: &Path) -> Result<CanonicalLineageStore> {
    if !path.exists() {
        return Ok(CanonicalLineageStore::default());
    }
    let raw = std::fs::read_to_string(path)?;
    let lineage: CanonicalLineageStore = serde_json::from_str(&raw)?;
    Ok(lineage)
}

fn save_canonical_lineage(path: &Path, lineage: &CanonicalLineageStore) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let pretty = serde_json::to_string_pretty(lineage)?;
    std::fs::write(path, pretty)?;
    Ok(())
}

fn write_manifest(path: &Path, manifest: &ContributionManifest) -> Result<()> {
    let pretty = serde_json::to_string_pretty(manifest)?;
    std::fs::write(path, pretty)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredSigningKey {
    key_name: String,
    key_id: String,
    secret_key_base64: String,
    public_key_base64: String,
    created_at: String,
}

#[derive(Debug, Clone)]
struct SigningKeyEntry {
    key_id: String,
    signing_key: SigningKey,
    public_key_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TrustRegistry {
    #[serde(default)]
    keys: Vec<TrustRegistryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TrustRegistryEntry {
    key_id: String,
    algorithm: String,
    public_key_base64: String,
}

impl Default for TrustRegistry {
    fn default() -> Self {
        Self { keys: Vec::new() }
    }
}

fn ensure_signing_key(keys_dir: &Path, key_name: &str) -> Result<SigningKeyEntry> {
    std::fs::create_dir_all(keys_dir)?;
    let path = keys_dir.join(format!("{}.key.json", sanitize_file_stem(key_name)));
    if path.exists() {
        let raw = std::fs::read_to_string(&path)?;
        let stored: StoredSigningKey = serde_json::from_str(&raw)?;
        let secret_bytes = decode_base64_fixed::<32>(&stored.secret_key_base64)?;
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        return Ok(SigningKeyEntry {
            key_id: stored.key_id,
            signing_key,
            public_key_base64: stored.public_key_base64,
        });
    }

    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    let public_key_bytes = verifying_key.to_bytes();
    let public_key_base64 = base64::engine::general_purpose::STANDARD.encode(public_key_bytes);
    let key_id = format!(
        "local:{}:{}",
        sanitize_key_id(key_name),
        &hex::encode(public_key_bytes)[..16]
    );
    let stored = StoredSigningKey {
        key_name: key_name.to_string(),
        key_id: key_id.clone(),
        secret_key_base64: base64::engine::general_purpose::STANDARD.encode(signing_key.to_bytes()),
        public_key_base64: public_key_base64.clone(),
        created_at: Utc::now().to_rfc3339(),
    };
    let pretty = serde_json::to_string_pretty(&stored)?;
    std::fs::write(&path, pretty)?;
    Ok(SigningKeyEntry {
        key_id,
        signing_key,
        public_key_base64,
    })
}

fn verify_manifest_signature(
    manifest: &ContributionManifest,
    trust_registry_path: &Path,
) -> Result<SignatureValidationResult> {
    let Some(signature) = manifest.signature.as_ref() else {
        return Ok(SignatureValidationResult {
            present: false,
            valid: true,
            key_id: None,
            message: "manifest is unsigned".to_string(),
        });
    };

    if signature.algorithm.trim() != SIGNATURE_ALGORITHM_ED25519 {
        return Ok(SignatureValidationResult {
            present: true,
            valid: false,
            key_id: Some(signature.key_id.clone()),
            message: format!(
                "unsupported signature algorithm '{}'",
                signature.algorithm
            ),
        });
    }

    let registry = load_trust_registry(trust_registry_path)?;
    let Some(key) = registry
        .keys
        .iter()
        .find(|entry| entry.key_id == signature.key_id)
    else {
        return Ok(SignatureValidationResult {
            present: true,
            valid: false,
            key_id: Some(signature.key_id.clone()),
            message: "signature key_id not found in trust registry".to_string(),
        });
    };

    let public_key_bytes = decode_base64_fixed::<32>(&key.public_key_base64)?;
    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes).map_err(|err| {
        crate::error::DbError::InvalidQuery(format!("invalid trusted public key: {err}"))
    })?;

    let sig_bytes = decode_base64_fixed::<64>(&signature.signature_base64)?;
    let sig = Signature::from_bytes(&sig_bytes);
    let payload = manifest_signing_payload(manifest)?;
    let verified = verifying_key.verify(&payload, &sig).is_ok();
    Ok(SignatureValidationResult {
        present: true,
        valid: verified,
        key_id: Some(signature.key_id.clone()),
        message: if verified {
            "signature verified".to_string()
        } else {
            "signature verification failed".to_string()
        },
    })
}

fn load_trust_registry(path: &Path) -> Result<TrustRegistry> {
    if !path.exists() {
        return Ok(TrustRegistry::default());
    }
    let raw = std::fs::read_to_string(path)?;
    let registry: TrustRegistry = serde_json::from_str(&raw)?;
    Ok(registry)
}

fn upsert_trust_registry_key(path: &Path, entry: TrustRegistryEntry) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut registry = load_trust_registry(path)?;
    if let Some(existing) = registry.keys.iter_mut().find(|k| k.key_id == entry.key_id) {
        *existing = entry;
    } else {
        registry.keys.push(entry);
    }
    let pretty = serde_json::to_string_pretty(&registry)?;
    std::fs::write(path, pretty)?;
    Ok(())
}

fn manifest_signing_payload(manifest: &ContributionManifest) -> Result<Vec<u8>> {
    let mut unsigned = manifest.clone();
    unsigned.signature = None;
    let value = serde_json::to_value(&unsigned)?;
    let canonical = canonical_json(&value);
    Ok(canonical.into_bytes())
}

fn canonical_json(value: &serde_json::Value) -> String {
    fn write_value(value: &serde_json::Value, out: &mut String) {
        match value {
            serde_json::Value::Null => out.push_str("null"),
            serde_json::Value::Bool(v) => out.push_str(if *v { "true" } else { "false" }),
            serde_json::Value::Number(n) => out.push_str(&n.to_string()),
            serde_json::Value::String(s) => out.push_str(
                &serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string()),
            ),
            serde_json::Value::Array(arr) => {
                out.push('[');
                let mut first = true;
                for item in arr {
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    write_value(item, out);
                }
                out.push(']');
            }
            serde_json::Value::Object(map) => {
                out.push('{');
                let mut keys = map.keys().collect::<Vec<_>>();
                keys.sort_unstable();
                let mut first = true;
                for key in keys {
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    let key_json = serde_json::to_string(key).unwrap_or_else(|_| "\"\"".to_string());
                    out.push_str(&key_json);
                    out.push(':');
                    if let Some(v) = map.get(key) {
                        write_value(v, out);
                    } else {
                        out.push_str("null");
                    }
                }
                out.push('}');
            }
        }
    }

    let mut out = String::new();
    write_value(value, &mut out);
    out
}

fn decode_base64_fixed<const N: usize>(encoded: &str) -> Result<[u8; N]> {
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded.as_bytes())
        .map_err(|err| crate::error::DbError::InvalidQuery(format!("invalid base64: {err}")))?;
    if decoded.len() != N {
        return Err(crate::error::DbError::InvalidQuery(format!(
            "invalid decoded byte length: expected {N}, got {}",
            decoded.len()
        )));
    }
    let mut out = [0_u8; N];
    out.copy_from_slice(&decoded);
    Ok(out)
}

fn sanitize_file_stem(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.trim_matches('_').is_empty() {
        "default".to_string()
    } else {
        out
    }
}

fn sanitize_key_id(name: &str) -> String {
    sanitize_file_stem(name).trim_matches('_').to_string()
}

async fn export_table_papers(db: &Arc<Database>, package_dir: &Path) -> Result<ArtifactDigest> {
    let repo = PaperRepository::new(db.clone());
    let path = package_dir.join("papers.jsonl");
    let mut writer = BufWriter::new(std::fs::File::create(&path)?);
    let mut offset = 0usize;
    let mut row_count = 0u64;
    loop {
        let rows = repo.list(offset, EXPORT_PAGE_SIZE).await?;
        if rows.is_empty() {
            break;
        }
        offset += rows.len();
        for row in rows {
            write_jsonl_row(&mut writer, &row)?;
            row_count += 1;
        }
    }
    writer.flush()?;
    build_table_artifact(&path, "papers", row_count)
}

async fn export_table_chunks(db: &Arc<Database>, package_dir: &Path) -> Result<ArtifactDigest> {
    let repo = ChunkRepository::new(db.clone());
    let path = package_dir.join("chunks.jsonl");
    let mut writer = BufWriter::new(std::fs::File::create(&path)?);
    let mut offset = 0usize;
    let mut row_count = 0u64;
    loop {
        let rows = repo.list(offset, EXPORT_PAGE_SIZE).await?;
        if rows.is_empty() {
            break;
        }
        offset += rows.len();
        for row in rows {
            write_jsonl_row(&mut writer, &row)?;
            row_count += 1;
        }
    }
    writer.flush()?;
    build_table_artifact(&path, "chunks", row_count)
}

async fn export_table_entities(db: &Arc<Database>, package_dir: &Path) -> Result<ArtifactDigest> {
    let repo = EntityRepository::new(db.clone());
    let path = package_dir.join("entities.jsonl");
    let mut writer = BufWriter::new(std::fs::File::create(&path)?);
    let mut offset = 0usize;
    let mut row_count = 0u64;
    loop {
        let rows = repo.list(offset, EXPORT_PAGE_SIZE).await?;
        if rows.is_empty() {
            break;
        }
        offset += rows.len();
        for row in rows {
            write_jsonl_row(&mut writer, &row)?;
            row_count += 1;
        }
    }
    writer.flush()?;
    build_table_artifact(&path, "entities", row_count)
}

async fn export_table_entity_mentions(
    db: &Arc<Database>,
    package_dir: &Path,
) -> Result<ArtifactDigest> {
    let repo = EntityMentionRepository::new(db.clone());
    let path = package_dir.join("entity_mentions.jsonl");
    let mut writer = BufWriter::new(std::fs::File::create(&path)?);
    let mut offset = 0usize;
    let mut row_count = 0u64;
    loop {
        let rows = repo.list(offset, EXPORT_PAGE_SIZE).await?;
        if rows.is_empty() {
            break;
        }
        offset += rows.len();
        for row in rows {
            write_jsonl_row(&mut writer, &row)?;
            row_count += 1;
        }
    }
    writer.flush()?;
    build_table_artifact(&path, "entity_mentions", row_count)
}

async fn export_table_kg_facts(db: &Arc<Database>, package_dir: &Path) -> Result<ArtifactDigest> {
    let repo = KgFactRepository::new(db.clone());
    let path = package_dir.join("kg_facts.jsonl");
    let mut writer = BufWriter::new(std::fs::File::create(&path)?);
    let mut offset = 0usize;
    let mut row_count = 0u64;
    loop {
        let rows = repo.list(offset, EXPORT_PAGE_SIZE).await?;
        if rows.is_empty() {
            break;
        }
        offset += rows.len();
        for row in rows {
            write_jsonl_row(&mut writer, &row)?;
            row_count += 1;
        }
    }
    writer.flush()?;
    build_table_artifact(&path, "kg_facts", row_count)
}

async fn export_table_target_scores(
    db: &Arc<Database>,
    package_dir: &Path,
) -> Result<ArtifactDigest> {
    let repo = TargetScoreRepository::new(db.clone());
    let path = package_dir.join("target_scores.jsonl");
    let mut writer = BufWriter::new(std::fs::File::create(&path)?);
    let mut offset = 0usize;
    let mut row_count = 0u64;
    loop {
        let rows = repo.list(offset, EXPORT_PAGE_SIZE).await?;
        if rows.is_empty() {
            break;
        }
        offset += rows.len();
        for row in rows {
            write_jsonl_row(&mut writer, &row)?;
            row_count += 1;
        }
    }
    writer.flush()?;
    build_table_artifact(&path, "target_scores", row_count)
}

fn write_jsonl_row<T: Serialize>(writer: &mut BufWriter<std::fs::File>, row: &T) -> Result<()> {
    let mut line = serde_json::to_vec(row)?;
    line.push(b'\n');
    writer.write_all(&line)?;
    Ok(())
}

fn build_table_artifact(path: &Path, table_name: &str, row_count: u64) -> Result<ArtifactDigest> {
    let digest = digest_file(path)?;
    Ok(ArtifactDigest {
        artifact_kind: "table_jsonl".to_string(),
        relative_path: path
            .file_name()
            .map(|v| v.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("{table_name}.jsonl")),
        row_count,
        sha256: digest.sha256,
        bytes: digest.bytes,
    })
}

#[derive(Debug, Clone)]
struct FileDigest {
    sha256: String,
    bytes: u64,
}

fn digest_file(path: &Path) -> Result<FileDigest> {
    let file = std::fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 8192];
    let mut total = 0u64;

    loop {
        let read = reader.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
        total += read as u64;
    }

    Ok(FileDigest {
        sha256: format!("{:x}", hasher.finalize()),
        bytes: total,
    })
}

fn count_file_lines(path: &Path) -> Result<u64> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut count = 0u64;
    for line in reader.lines() {
        let _ = line?;
        count += 1;
    }
    Ok(count)
}

async fn collect_table_counts(db: &Database) -> Result<BTreeMap<String, u64>> {
    let names = [
        TABLE_PAPERS,
        TABLE_CHUNKS,
        TABLE_ENTITIES,
        TABLE_ENTITY_MENTIONS,
        TABLE_KG_FACTS,
        TABLE_TARGET_SCORES,
        TABLE_KG_CONFLICTS,
        TABLE_INGESTION_AUDIT,
        crate::schema::TABLE_ENT_GENES,
        crate::schema::TABLE_ENT_MUTATIONS,
        crate::schema::TABLE_ENT_CANCER_TYPES,
        crate::schema::TABLE_ENT_PATHWAYS,
        crate::schema::TABLE_ENT_CLINICAL_EVIDENCE,
        crate::schema::TABLE_ENT_COMPOUNDS,
        crate::schema::TABLE_ENT_STRUCTURES,
        crate::schema::TABLE_ENT_DRUGGABILITY,
        crate::schema::TABLE_ENT_SYNTHETIC_LETHALITY,
        crate::schema::TABLE_ENT_TCGA_SURVIVAL,
        crate::schema::TABLE_ENT_CBIO_MUTATION_FREQUENCY,
        crate::schema::TABLE_ENT_COSMIC_MUTATION_FREQUENCY,
        crate::schema::TABLE_ENT_GTEX_EXPRESSION,
        crate::schema::TABLE_ENT_CHEMBL_TARGETS,
        crate::schema::TABLE_ENT_REACTOME_GENES,
        crate::schema::TABLE_ENT_PROVIDER_REFRESH_RUNS,
    ];

    let mut out = BTreeMap::new();
    for name in names {
        let count = if db.table_exists(name).await.unwrap_or(false) {
            let table = db.connection().open_table(name).execute().await?;
            table.count_rows(None).await? as u64
        } else {
            0
        };
        out.insert(name.to_string(), count);
    }
    Ok(out)
}

async fn collect_quality_summary(db: Arc<Database>) -> Result<QualitySummary> {
    let papers = PaperRepository::new(db.clone());
    let kg = KgFactRepository::new(db.clone());
    let chunks = ChunkRepository::new(db.clone());
    let scores = TargetScoreRepository::new(db);

    let parsed = papers.count_by_parse_status("parsed").await.unwrap_or(0)
        + papers.count_by_parse_status("parsed_fast").await.unwrap_or(0)
        + papers.count_by_parse_status("parsed_light").await.unwrap_or(0);
    let failed = papers.count_by_parse_status("failed").await.unwrap_or(0);
    let parse_success_rate = if parsed + failed > 0 {
        parsed as f64 / (parsed + failed) as f64
    } else {
        0.0
    };

    let duplicate_identity_rate = estimate_duplicate_doi_rate(&papers).await?;
    let generic_predicate_share = estimate_generic_predicate_share(&kg).await?;

    let mut notes = Vec::new();
    if chunks.count().await.unwrap_or(0) == 0 {
        notes.push("chunks table has no rows".to_string());
    }
    if scores.count().await.unwrap_or(0) == 0 {
        notes.push("target_scores table has no rows".to_string());
    }

    Ok(QualitySummary {
        parse_success_rate,
        duplicate_identity_rate,
        generic_predicate_share,
        notes,
    })
}

async fn estimate_duplicate_doi_rate(repo: &PaperRepository) -> Result<f64> {
    let mut offset = 0usize;
    let page_size = 500usize;
    let mut seen = HashSet::new();
    let mut with_doi = 0usize;
    let mut duplicates = 0usize;

    while with_doi < QUALITY_PAPER_SCAN_LIMIT {
        let batch = repo.list(offset, page_size).await?;
        if batch.is_empty() {
            break;
        }
        offset += batch.len();
        for paper in batch {
            let Some(doi) = paper.doi else { continue };
            let normalized = normalize_identity(&doi);
            if normalized.is_empty() {
                continue;
            }
            with_doi += 1;
            if !seen.insert(normalized) {
                duplicates += 1;
            }
            if with_doi >= QUALITY_PAPER_SCAN_LIMIT {
                break;
            }
        }
    }

    if with_doi == 0 {
        Ok(0.0)
    } else {
        Ok((duplicates as f64 / with_doi as f64).clamp(0.0, 1.0))
    }
}

async fn estimate_generic_predicate_share(repo: &KgFactRepository) -> Result<f64> {
    let facts = repo.list(0, QUALITY_FACT_SCAN_LIMIT).await?;
    if facts.is_empty() {
        return Ok(0.0);
    }
    let generic: HashSet<&'static str> = HashSet::from([
        "mentions",
        "associated_with",
        "related_to",
        "linked_to",
        "correlated_with",
    ]);
    let generic_count = facts
        .iter()
        .filter(|fact| generic.contains(fact.predicate.trim().to_ascii_lowercase().as_str()))
        .count();
    Ok((generic_count as f64 / facts.len() as f64).clamp(0.0, 1.0))
}

fn normalize_identity(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace("doi:", "")
}

fn validate_ratio(issues: &mut Vec<ValidationIssue>, field: &str, value: f64) {
    if !value.is_finite() {
        issues.push(ValidationIssue::error(
            format!("{field}_invalid"),
            format!("{field} must be a finite value in [0,1]"),
        ));
        return;
    }
    if !(0.0..=1.0).contains(&value) {
        issues.push(ValidationIssue::error(
            format!("{field}_out_of_range"),
            format!("{field} must be in [0,1], got {value}"),
        ));
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrumyx_common::federation::{ArtifactDigest, ContributionManifest};
    use std::sync::{Mutex, OnceLock};

    fn federation_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn validate_rejects_bad_schema() {
        let _guard = federation_test_lock().lock().expect("lock federation tests");
        let mut manifest = ContributionManifest::template();
        manifest.schema_version = "wrong.schema".to_string();
        let report = validate_contribution_manifest(&manifest);
        assert!(!report.valid);
        assert!(report.issues.iter().any(|issue| issue.code == "schema_version_mismatch"));
    }

    #[test]
    fn validate_accepts_template_manifest() {
        let _guard = federation_test_lock().lock().expect("lock federation tests");
        let manifest = ContributionManifest::template();
        let report = validate_contribution_manifest(&manifest);
        assert!(report.valid);
    }

    #[test]
    fn package_validation_checks_artifact_hashes() {
        let _guard = federation_test_lock().lock().expect("lock federation tests");
        let base = std::env::temp_dir().join(format!("ferrumyx-fed-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base).expect("temp dir");

        let artifact_path = base.join("papers.jsonl");
        std::fs::write(&artifact_path, b"{\"title\":\"x\"}\n").expect("write artifact");
        let digest = digest_file(&artifact_path).expect("digest artifact");
        let rows = count_file_lines(&artifact_path).expect("row count");

        let mut manifest = ContributionManifest::template();
        manifest
            .stats
            .table_counts
            .insert("papers".to_string(), 1);
        manifest.artifacts = vec![ArtifactDigest {
            artifact_kind: "table_jsonl".to_string(),
            relative_path: "papers.jsonl".to_string(),
            row_count: rows,
            sha256: digest.sha256.clone(),
            bytes: digest.bytes,
        }];
        write_manifest(&base.join("manifest.json"), &manifest).expect("write manifest");

        let report = validate_contribution_package(&base).expect("validate package");
        assert!(report.valid);
        assert!(report.artifact_checks.iter().all(|c| c.sha256_match));

        std::fs::write(&artifact_path, b"{\"title\":\"changed\"}\n").expect("rewrite artifact");
        let report_bad = validate_contribution_package(&base).expect("validate package mismatch");
        assert!(!report_bad.valid);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn export_package_writes_manifest_and_artifacts() {
        let _guard = federation_test_lock().lock().expect("lock federation tests");
        let base = std::env::temp_dir().join(format!("ferrumyx-fed-export-{}", uuid::Uuid::new_v4()));
        let db_dir = base.join("db");
        let export_dir = base.join("export");
        std::fs::create_dir_all(&db_dir).expect("create db dir");
        std::fs::create_dir_all(&export_dir).expect("create export dir");

        let db = Arc::new(Database::open(&db_dir).await.expect("open db"));
        db.initialize().await.expect("init db");

        let result = export_contribution_package(
            db,
            PackageExportRequest {
                output_root: Some(export_dir.to_string_lossy().to_string()),
                ..PackageExportRequest::default()
            },
        )
        .await
        .expect("export package");

        assert!(Path::new(&result.manifest_path).exists());
        assert!(Path::new(&result.package_dir).exists());
        assert!(result.manifest.artifacts.iter().any(|a| a.relative_path == "papers.jsonl"));
        assert!(result.validation.valid);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn sign_and_verify_package_manifest() {
        let _guard = federation_test_lock().lock().expect("lock federation tests");
        let base = std::env::temp_dir().join(format!("ferrumyx-fed-sign-{}", uuid::Uuid::new_v4()));
        let db_dir = base.join("db");
        let export_dir = base.join("export");
        let keys_dir = base.join("keys");
        let trust_registry = base.join("trust_registry.json");
        std::fs::create_dir_all(&db_dir).expect("create db dir");
        std::fs::create_dir_all(&export_dir).expect("create export dir");
        std::env::set_var("FERRUMYX_FED_KEYS_DIR", keys_dir.to_string_lossy().to_string());
        std::env::set_var(
            "FERRUMYX_FED_TRUST_REGISTRY_PATH",
            trust_registry.to_string_lossy().to_string(),
        );

        let db = Arc::new(Database::open(&db_dir).await.expect("open db"));
        db.initialize().await.expect("init db");
        let export = export_contribution_package(
            db,
            PackageExportRequest {
                output_root: Some(export_dir.to_string_lossy().to_string()),
                ..PackageExportRequest::default()
            },
        )
        .await
        .expect("export package");

        let signed = sign_contribution_package(PackageSignRequest {
            package_dir: export.package_dir.clone(),
            key_name: Some("test-sign".to_string()),
        })
        .expect("sign package");
        assert!(!signed.signature_base64.is_empty());

        let verified = validate_contribution_package(&export.package_dir).expect("validate signed");
        assert!(verified.valid);
        assert!(verified.signature_validation.present);
        assert!(verified.signature_validation.valid);

        let manifest_path = Path::new(&export.package_dir).join("manifest.json");
        let mut value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).expect("read manifest"))
                .expect("parse manifest");
        value["dataset_id"] = serde_json::Value::String("tampered".to_string());
        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&value).expect("serialize tampered"),
        )
        .expect("write tampered");

        let tampered = validate_contribution_package(&export.package_dir).expect("validate tampered");
        assert!(!tampered.valid);
        assert!(!tampered.signature_validation.valid);

        std::env::remove_var("FERRUMYX_FED_KEYS_DIR");
        std::env::remove_var("FERRUMYX_FED_TRUST_REGISTRY_PATH");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn merge_queue_submit_and_approve_updates_lineage() {
        let _guard = federation_test_lock().lock().expect("lock federation tests");
        let base = std::env::temp_dir().join(format!(
            "ferrumyx-fed-merge-{}",
            uuid::Uuid::new_v4()
        ));
        let db_dir = base.join("db");
        let export_dir = base.join("export");
        let keys_dir = base.join("keys");
        let trust_registry = base.join("trust_registry.json");
        let merge_queue = base.join("merge_queue.json");
        let lineage = base.join("canonical_lineage.json");
        std::fs::create_dir_all(&db_dir).expect("create db dir");
        std::fs::create_dir_all(&export_dir).expect("create export dir");
        std::env::set_var("FERRUMYX_FED_KEYS_DIR", keys_dir.to_string_lossy().to_string());
        std::env::set_var(
            "FERRUMYX_FED_TRUST_REGISTRY_PATH",
            trust_registry.to_string_lossy().to_string(),
        );
        std::env::set_var(
            "FERRUMYX_FED_MERGE_QUEUE_PATH",
            merge_queue.to_string_lossy().to_string(),
        );
        std::env::set_var(
            "FERRUMYX_FED_CANONICAL_LINEAGE_PATH",
            lineage.to_string_lossy().to_string(),
        );

        let db = Arc::new(Database::open(&db_dir).await.expect("open db"));
        db.initialize().await.expect("init db");
        let export = export_contribution_package(
            db,
            PackageExportRequest {
                output_root: Some(export_dir.to_string_lossy().to_string()),
                ..PackageExportRequest::default()
            },
        )
        .await
        .expect("export package");
        sign_contribution_package(PackageSignRequest {
            package_dir: export.package_dir.clone(),
            key_name: Some("merge-key".to_string()),
        })
        .expect("sign package");

        let submitted = submit_package_for_merge(MergeSubmitRequest {
            package_dir: export.package_dir.clone(),
            submitted_by: Some("tester".to_string()),
        })
        .expect("submit package");
        assert_eq!(submitted.entry.status, MergeQueueStatus::PendingReview);

        let decision = decide_merge_queue(MergeDecisionRequest {
            queue_id: submitted.entry.queue_id.clone(),
            approve: true,
            decision_by: Some("moderator".to_string()),
            reason: Some("quality checks passed".to_string()),
        })
        .expect("approve package");
        assert_eq!(decision.entry.status, MergeQueueStatus::Approved);
        assert!(decision.canonical_lineage_size >= 1);

        let listed = get_canonical_lineage().expect("read lineage");
        assert!(listed
            .snapshots
            .iter()
            .any(|snap| snap.snapshot_id == submitted.entry.snapshot_id));

        std::env::remove_var("FERRUMYX_FED_KEYS_DIR");
        std::env::remove_var("FERRUMYX_FED_TRUST_REGISTRY_PATH");
        std::env::remove_var("FERRUMYX_FED_MERGE_QUEUE_PATH");
        std::env::remove_var("FERRUMYX_FED_CANONICAL_LINEAGE_PATH");
        let _ = std::fs::remove_dir_all(&base);
    }
}
