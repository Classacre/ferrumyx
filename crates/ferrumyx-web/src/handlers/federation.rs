//! Federation manifest endpoints (draft schema + draft generation + validation).

use crate::state::SharedState;
use axum::{
    extract::{Query, State},
    http::{header::AUTHORIZATION, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use ferrumyx_common::federation::{
    ContributionManifest, FEDERATION_SCHEMA_URL, FEDERATION_SCHEMA_VERSION,
};
use ferrumyx_db::{
    build_contribution_manifest_draft, export_contribution_package, validate_contribution_manifest,
    validate_contribution_package, sign_contribution_package, submit_package_for_merge,
    decide_merge_queue, list_merge_queue, get_canonical_lineage, list_trusted_signing_keys,
    revoke_trusted_signing_key, upsert_trusted_signing_key, CanonicalSnapshotRecord,
    ManifestDraftRequest, MergeDecisionRequest, MergeSubmitRequest, PackageExportRequest,
    PackageSignRequest, PackageValidationRequest, TrustKeyRecord, TrustKeyRevokeRequest,
    TrustKeyUpsertRequest,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Component, Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Serialize)]
pub struct FederationSchemaResponse {
    pub schema_version: &'static str,
    pub schema_url: &'static str,
    pub required_fields: Vec<&'static str>,
    pub template: ContributionManifest,
}

const DEFAULT_SYNC_CHUNK_BYTES: u64 = 1_048_576;
const DEFAULT_SYNC_TIMEOUT_SECS: u64 = 60;
const DEFAULT_SYNC_PULL_ROOT: &str = "./output/federation/imports";
const DEFAULT_HF_ENABLED: bool = false;
const DEFAULT_HF_BIN: &str = "hf";
const DEFAULT_HF_SNAPSHOTS_PREFIX: &str = "snapshots";
const DEFAULT_HF_TIMEOUT_SECS: u64 = 1_800;
const DEFAULT_HF_PULL_ROOT: &str = "./output/federation/hf_imports";
const DEFAULT_FED_AUTH_ENABLED: bool = false;
const DEFAULT_FED_REPLAY_WINDOW_SECS: u64 = 300;
const DEFAULT_FED_REPLAY_REQUIRED: bool = true;
const DEFAULT_FED_AUDIT_LOG_PATH: &str = "./output/federation/audit.log";

static FED_REPLAY_STATE: OnceLock<Mutex<HashMap<String, i64>>> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
enum FederationScope {
    Read,
    Write,
}

#[derive(Debug, Clone)]
struct FederationAuthConfig {
    enabled: bool,
    read_token: Option<String>,
    write_token: Option<String>,
    replay_required: bool,
    replay_window_secs: i64,
}

#[derive(Debug, Serialize)]
pub struct TrustListResponse {
    pub keys: Vec<TrustKeyRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSnapshotIndexItem {
    pub dataset_id: String,
    pub snapshot_id: String,
    pub parent_snapshot_id: Option<String>,
    pub manifest_id: String,
    pub approved_at: String,
    pub artifact_count: usize,
    pub artifact_bytes_total: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncIndexResponse {
    pub generated_at: String,
    pub snapshots: Vec<SyncSnapshotIndexItem>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncArtifactDescriptor {
    pub artifact_kind: String,
    pub relative_path: String,
    pub row_count: u64,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSnapshotDescriptor {
    pub dataset_id: String,
    pub snapshot_id: String,
    pub parent_snapshot_id: Option<String>,
    pub manifest_id: String,
    pub approved_at: String,
    pub artifacts: Vec<SyncArtifactDescriptor>,
    pub manifest: ContributionManifest,
}

#[derive(Debug, Deserialize)]
pub struct SyncSnapshotQuery {
    pub dataset_id: String,
    pub snapshot_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SyncArtifactQuery {
    pub dataset_id: String,
    pub snapshot_id: String,
    pub relative_path: String,
    pub offset: Option<u64>,
    pub max_bytes: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct SyncPlanRequest {
    pub remote_base_url: String,
    pub dataset_id: Option<String>,
    pub remote_api_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SyncPlanResponse {
    pub remote_base_url: String,
    pub dataset_id: Option<String>,
    pub local_snapshot_count: usize,
    pub remote_snapshot_count: usize,
    pub missing_locally: Vec<SyncSnapshotIndexItem>,
    pub missing_remotely: Vec<SyncSnapshotIndexItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPullRequest {
    pub remote_base_url: String,
    pub dataset_id: String,
    pub snapshot_id: String,
    pub max_chunk_bytes: Option<u64>,
    pub destination_root: Option<String>,
    pub submitted_by: Option<String>,
    pub remote_api_token: Option<String>,
    pub auto_submit_to_merge_queue: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct SyncPullResponse {
    pub package_dir: String,
    pub downloaded_bytes: u64,
    pub artifacts_downloaded: usize,
    pub validation_ok: bool,
    pub merge_queue_entry: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct SyncPushRequest {
    pub remote_base_url: String,
    pub dataset_id: String,
    pub snapshot_id: String,
    pub source_base_url: Option<String>,
    pub max_chunk_bytes: Option<u64>,
    pub submitted_by: Option<String>,
    pub remote_api_token: Option<String>,
    pub source_api_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SyncPushResponse {
    pub remote_base_url: String,
    pub source_base_url: String,
    pub remote_status_code: u16,
    pub remote_response: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct HfSyncStatusQuery {
    pub repo_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HfSyncStatusResponse {
    pub enabled: bool,
    pub hf_binary: String,
    pub hf_available: bool,
    pub hf_version: Option<String>,
    pub repo_id: Option<String>,
    pub snapshots_prefix: String,
    pub revision: Option<String>,
    pub timeout_secs: u64,
    pub pull_root: String,
    pub repo_check_ok: Option<bool>,
    pub repo_private: Option<bool>,
    pub repo_gated: Option<bool>,
    pub repo_info: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HfPublishRequest {
    pub package_dir: String,
    pub repo_id: Option<String>,
    pub path_in_repo: Option<String>,
    pub revision: Option<String>,
    pub commit_message: Option<String>,
    pub create_tag: Option<bool>,
    pub tag_name: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HfPublishResponse {
    pub repo_id: String,
    pub package_dir: String,
    pub snapshot_id: String,
    pub path_in_repo: String,
    pub revision: Option<String>,
    pub upload_stdout: String,
    pub upload_stderr: String,
    pub tag_created: bool,
    pub tag_name: Option<String>,
    pub tag_error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HfPullRequest {
    pub snapshot_id: String,
    pub repo_id: Option<String>,
    pub path_in_repo: Option<String>,
    pub revision: Option<String>,
    pub destination_root: Option<String>,
    pub submitted_by: Option<String>,
    pub auto_submit_to_merge_queue: Option<bool>,
    pub token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HfPullResponse {
    pub repo_id: String,
    pub snapshot_id: String,
    pub path_in_repo: String,
    pub destination_root: String,
    pub package_dir: String,
    pub validation_ok: bool,
    pub merge_queue_entry: Option<serde_json::Value>,
    pub download_stdout: String,
    pub download_stderr: String,
}

#[derive(Debug)]
struct HfCommandResult {
    stdout: String,
    stderr: String,
}

pub struct ArtifactChunkHttpResponse {
    pub chunk: Vec<u8>,
    pub next_offset: Option<u64>,
}

type FederationAuthError = (StatusCode, String);

fn authorize_federation_request(
    headers: &HeaderMap,
    scope: FederationScope,
    require_replay: bool,
) -> Result<(), FederationAuthError> {
    let config = federation_auth_config();
    if !config.enabled {
        return Ok(());
    }

    let provided = extract_bearer_token(headers)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "missing bearer token".to_string()))?;
    let expected = match scope {
        FederationScope::Read => config.read_token.or(config.write_token),
        FederationScope::Write => config.write_token.or(config.read_token),
    }
    .ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "federation auth is enabled but no token is configured".to_string(),
        )
    })?;

    if provided != expected {
        return Err((StatusCode::UNAUTHORIZED, "invalid bearer token".to_string()));
    }

    if require_replay && config.replay_required {
        enforce_replay_guard(headers, config.replay_window_secs)?;
    }

    Ok(())
}

fn federation_auth_config() -> FederationAuthConfig {
    FederationAuthConfig {
        enabled: env_bool("FERRUMYX_FED_AUTH_ENABLED", DEFAULT_FED_AUTH_ENABLED),
        read_token: env_trimmed("FERRUMYX_FED_READ_TOKEN")
            .or_else(|| env_trimmed("FERRUMYX_FED_API_TOKEN")),
        write_token: env_trimmed("FERRUMYX_FED_WRITE_TOKEN")
            .or_else(|| env_trimmed("FERRUMYX_FED_API_TOKEN")),
        replay_required: env_bool("FERRUMYX_FED_REPLAY_REQUIRED", DEFAULT_FED_REPLAY_REQUIRED),
        replay_window_secs: std::env::var("FERRUMYX_FED_REPLAY_WINDOW_SECS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(DEFAULT_FED_REPLAY_WINDOW_SECS as i64)
            .clamp(30, 3_600),
    }
}

fn env_trimmed(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|v| {
            let trimmed = v.trim();
            trimmed == "1" || trimmed.eq_ignore_ascii_case("true")
        })
        .unwrap_or(default)
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .and_then(|raw| raw.strip_prefix("Bearer ").or_else(|| raw.strip_prefix("bearer ")))
        .map(str::trim)
        .map(str::to_string)
        .filter(|v| !v.is_empty())
}

fn enforce_replay_guard(headers: &HeaderMap, replay_window_secs: i64) -> Result<(), FederationAuthError> {
    let nonce = header_text(headers, "x-ferrumyx-nonce")
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "missing x-ferrumyx-nonce header".to_string()))?;
    let ts = header_text(headers, "x-ferrumyx-ts")
        .or_else(|| header_text(headers, "x-ferrumyx-timestamp"))
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "missing x-ferrumyx-ts header".to_string(),
            )
        })
        .and_then(|raw| {
            raw.parse::<i64>().map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    "invalid x-ferrumyx-ts header".to_string(),
                )
            })
        })?;

    let now = unix_now_secs()?;
    if (now - ts).abs() > replay_window_secs {
        return Err((
            StatusCode::UNAUTHORIZED,
            "request timestamp outside replay window".to_string(),
        ));
    }

    let state = FED_REPLAY_STATE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut replay = state
        .lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "replay guard lock poisoned".to_string()))?;
    let ttl = replay_window_secs.max(30).saturating_mul(2);
    let cutoff = now.saturating_sub(ttl);
    replay.retain(|_, seen_at| *seen_at >= cutoff);
    if replay.contains_key(&nonce) {
        return Err((
            StatusCode::CONFLICT,
            "replay detected: nonce already used".to_string(),
        ));
    }
    replay.insert(nonce, now);
    Ok(())
}

fn header_text(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

fn unix_now_secs() -> Result<i64, FederationAuthError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "system clock is before unix epoch".to_string(),
            )
        })
}

fn append_federation_audit(action: &str, status: &str, details: serde_json::Value) {
    let path = std::env::var("FERRUMYX_FED_AUDIT_LOG_PATH")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_FED_AUDIT_LOG_PATH.to_string());
    let path = PathBuf::from(path);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut file = match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let event = serde_json::json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "action": action,
        "status": status,
        "details": details,
    });
    let _ = writeln!(file, "{}", event);
}

pub async fn api_federation_schema(headers: HeaderMap) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Read, false) {
        return err.into_response();
    }
    Json(FederationSchemaResponse {
        schema_version: FEDERATION_SCHEMA_VERSION,
        schema_url: FEDERATION_SCHEMA_URL,
        required_fields: vec![
            "schema_version",
            "manifest_id",
            "dataset_id",
            "snapshot_id",
            "created_at",
            "contributor",
            "provenance",
            "stats",
            "quality",
        ],
        template: ContributionManifest::template(),
    })
    .into_response()
}

pub async fn api_federation_manifest_draft(
    headers: HeaderMap,
    State(state): State<SharedState>,
    Json(req): Json<ManifestDraftRequest>,
) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Read, false) {
        return err.into_response();
    }
    match build_contribution_manifest_draft(state.db.clone(), req).await {
        Ok(manifest) => (StatusCode::OK, Json(manifest)).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

pub async fn api_federation_manifest_validate(
    headers: HeaderMap,
    Json(manifest): Json<ContributionManifest>,
) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Read, false) {
        return err.into_response();
    }
    Json(validate_contribution_manifest(&manifest)).into_response()
}

pub async fn api_federation_package_export(
    headers: HeaderMap,
    State(state): State<SharedState>,
    Json(req): Json<PackageExportRequest>,
) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Write, true) {
        append_federation_audit(
            "package_export",
            "denied",
            serde_json::json!({ "reason": err.1 }),
        );
        return err.into_response();
    }
    match export_contribution_package(state.db.clone(), req).await {
        Ok(result) => {
            append_federation_audit(
                "package_export",
                "ok",
                serde_json::json!({
                    "dataset_id": result.manifest.dataset_id,
                    "snapshot_id": result.manifest.snapshot_id,
                    "package_dir": result.package_dir
                }),
            );
            (StatusCode::OK, Json(result)).into_response()
        }
        Err(err) => {
            append_federation_audit(
                "package_export",
                "error",
                serde_json::json!({ "error": err.to_string() }),
            );
            (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        )
                .into_response()
        }
    }
}

pub async fn api_federation_package_validate(
    headers: HeaderMap,
    Json(req): Json<PackageValidationRequest>,
) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Read, false) {
        return err.into_response();
    }
    match validate_contribution_package(&req.package_dir) {
        Ok(report) => (StatusCode::OK, Json(report)).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

pub async fn api_federation_package_sign(
    headers: HeaderMap,
    Json(req): Json<PackageSignRequest>,
) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Write, true) {
        append_federation_audit(
            "package_sign",
            "denied",
            serde_json::json!({ "reason": err.1 }),
        );
        return err.into_response();
    }
    match sign_contribution_package(req) {
        Ok(result) => {
            append_federation_audit(
                "package_sign",
                "ok",
                serde_json::json!({
                    "key_id": result.key_id,
                    "package_dir": result.package_dir
                }),
            );
            (StatusCode::OK, Json(result)).into_response()
        }
        Err(err) => {
            append_federation_audit(
                "package_sign",
                "error",
                serde_json::json!({ "error": err.to_string() }),
            );
            (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": err.to_string() })),
        )
                .into_response()
        }
    }
}

pub async fn api_federation_merge_submit(
    headers: HeaderMap,
    Json(req): Json<MergeSubmitRequest>,
) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Write, true) {
        append_federation_audit(
            "merge_submit",
            "denied",
            serde_json::json!({ "reason": err.1 }),
        );
        return err.into_response();
    }
    match submit_package_for_merge(req) {
        Ok(result) => {
            append_federation_audit(
                "merge_submit",
                "ok",
                serde_json::json!({
                    "queue_id": result.entry.queue_id,
                    "dataset_id": result.entry.dataset_id,
                    "snapshot_id": result.entry.snapshot_id,
                    "status": format!("{:?}", result.entry.status)
                }),
            );
            (StatusCode::OK, Json(result)).into_response()
        }
        Err(err) => {
            append_federation_audit(
                "merge_submit",
                "error",
                serde_json::json!({ "error": err.to_string() }),
            );
            (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": err.to_string() })),
        )
                .into_response()
        }
    }
}

pub async fn api_federation_merge_queue(headers: HeaderMap) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Read, false) {
        return err.into_response();
    }
    match list_merge_queue() {
        Ok(queue) => (StatusCode::OK, Json(queue)).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

pub async fn api_federation_merge_decide(
    headers: HeaderMap,
    Json(req): Json<MergeDecisionRequest>,
) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Write, true) {
        append_federation_audit(
            "merge_decide",
            "denied",
            serde_json::json!({ "reason": err.1 }),
        );
        return err.into_response();
    }
    match decide_merge_queue(req) {
        Ok(result) => {
            append_federation_audit(
                "merge_decide",
                "ok",
                serde_json::json!({
                    "queue_id": result.entry.queue_id,
                    "status": format!("{:?}", result.entry.status),
                    "dataset_id": result.entry.dataset_id,
                    "snapshot_id": result.entry.snapshot_id
                }),
            );
            (StatusCode::OK, Json(result)).into_response()
        }
        Err(err) => {
            append_federation_audit(
                "merge_decide",
                "error",
                serde_json::json!({ "error": err.to_string() }),
            );
            (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": err.to_string() })),
        )
                .into_response()
        }
    }
}

pub async fn api_federation_canonical_lineage(headers: HeaderMap) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Read, false) {
        return err.into_response();
    }
    match get_canonical_lineage() {
        Ok(lineage) => (StatusCode::OK, Json(lineage)).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

pub async fn api_federation_trust_list(headers: HeaderMap) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Read, false) {
        return err.into_response();
    }
    match list_trusted_signing_keys() {
        Ok(keys) => (StatusCode::OK, Json(TrustListResponse { keys })).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

pub async fn api_federation_trust_upsert(
    headers: HeaderMap,
    Json(req): Json<TrustKeyUpsertRequest>,
) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Write, true) {
        append_federation_audit(
            "trust_upsert",
            "denied",
            serde_json::json!({ "reason": err.1 }),
        );
        return err.into_response();
    }
    match upsert_trusted_signing_key(req) {
        Ok(record) => {
            append_federation_audit(
                "trust_upsert",
                "ok",
                serde_json::json!({ "key_id": record.key_id }),
            );
            (StatusCode::OK, Json(record)).into_response()
        }
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

pub async fn api_federation_trust_revoke(
    headers: HeaderMap,
    Json(req): Json<TrustKeyRevokeRequest>,
) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Write, true) {
        append_federation_audit(
            "trust_revoke",
            "denied",
            serde_json::json!({ "reason": err.1 }),
        );
        return err.into_response();
    }
    match revoke_trusted_signing_key(req) {
        Ok(removed) => {
            append_federation_audit(
                "trust_revoke",
                "ok",
                serde_json::json!({ "removed": removed }),
            );
            (
                StatusCode::OK,
                Json(serde_json::json!({ "removed": removed })),
            )
                .into_response()
        }
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

pub async fn api_federation_sync_index(headers: HeaderMap) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Read, false) {
        return err.into_response();
    }
    match build_local_sync_index() {
        Ok(index) => (StatusCode::OK, Json(index)).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err })),
        )
            .into_response(),
    }
}

pub async fn api_federation_sync_snapshot(
    headers: HeaderMap,
    Query(query): Query<SyncSnapshotQuery>,
) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Read, false) {
        return err.into_response();
    }
    match find_snapshot_descriptor(&query.dataset_id, &query.snapshot_id) {
        Ok(descriptor) => (StatusCode::OK, Json(descriptor)).into_response(),
        Err(err) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": err })),
        )
            .into_response(),
    }
}

pub async fn api_federation_sync_artifact(
    headers: HeaderMap,
    Query(query): Query<SyncArtifactQuery>,
) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Read, false) {
        return err.into_response();
    }
    match read_artifact_chunk(&query) {
        Ok(chunk) => chunk,
        Err((status, message)) => (
            status,
            Json(serde_json::json!({ "error": message })),
        )
            .into_response(),
    }
}

pub async fn api_federation_sync_plan(
    headers: HeaderMap,
    Json(req): Json<SyncPlanRequest>,
) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Read, false) {
        return err.into_response();
    }
    let remote_base = normalize_base_url(&req.remote_base_url);
    if remote_base.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "remote_base_url is required" })),
        )
            .into_response();
    }

    let local_index = match build_local_sync_index() {
        Ok(v) => v,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": err })),
            )
                .into_response()
        }
    };

    let client = match Client::builder()
        .timeout(Duration::from_secs(sync_timeout_secs()))
        .build()
    {
        Ok(c) => c,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("http client init failed: {err}") })),
            )
                .into_response()
        }
    };

    let remote_token = resolve_remote_api_token(req.remote_api_token.as_deref());
    let remote_index = match fetch_remote_sync_index(&client, &remote_base, remote_token.as_deref())
        .await
    {
            Ok(v) => v,
            Err(err) => {
                return (
                    StatusCode::BAD_GATEWAY,
                    Json(serde_json::json!({ "error": err })),
                )
                    .into_response()
            }
        };

    let dataset_filter = req
        .dataset_id
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let mut local_set = HashSet::new();
    let mut remote_set = HashSet::new();
    for snap in &local_index.snapshots {
        if dataset_filter
            .as_deref()
            .is_none_or(|ds| ds.eq_ignore_ascii_case(&snap.dataset_id))
        {
            local_set.insert((snap.dataset_id.clone(), snap.snapshot_id.clone()));
        }
    }
    for snap in &remote_index.snapshots {
        if dataset_filter
            .as_deref()
            .is_none_or(|ds| ds.eq_ignore_ascii_case(&snap.dataset_id))
        {
            remote_set.insert((snap.dataset_id.clone(), snap.snapshot_id.clone()));
        }
    }

    let missing_locally = remote_index
        .snapshots
        .into_iter()
        .filter(|snap| {
            dataset_filter
                .as_deref()
                .is_none_or(|ds| ds.eq_ignore_ascii_case(&snap.dataset_id))
                && !local_set.contains(&(snap.dataset_id.clone(), snap.snapshot_id.clone()))
        })
        .collect::<Vec<_>>();
    let missing_remotely = local_index
        .snapshots
        .into_iter()
        .filter(|snap| {
            dataset_filter
                .as_deref()
                .is_none_or(|ds| ds.eq_ignore_ascii_case(&snap.dataset_id))
                && !remote_set.contains(&(snap.dataset_id.clone(), snap.snapshot_id.clone()))
        })
        .collect::<Vec<_>>();

    (
        StatusCode::OK,
        Json(SyncPlanResponse {
            remote_base_url: remote_base,
            dataset_id: dataset_filter,
            local_snapshot_count: local_set.len(),
            remote_snapshot_count: remote_set.len(),
            missing_locally,
            missing_remotely,
        }),
    )
        .into_response()
}

pub async fn api_federation_sync_pull(
    headers: HeaderMap,
    Json(req): Json<SyncPullRequest>,
) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Write, true) {
        append_federation_audit(
            "sync_pull",
            "denied",
            serde_json::json!({ "reason": err.1 }),
        );
        return err.into_response();
    }
    append_federation_audit(
        "sync_pull",
        "start",
        serde_json::json!({
            "remote_base_url": req.remote_base_url,
            "dataset_id": req.dataset_id,
            "snapshot_id": req.snapshot_id
        }),
    );
    let remote_base = normalize_base_url(&req.remote_base_url);
    if remote_base.is_empty()
        || req.dataset_id.trim().is_empty()
        || req.snapshot_id.trim().is_empty()
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "remote_base_url, dataset_id, and snapshot_id are required" })),
        )
            .into_response();
    }

    let client = match Client::builder()
        .timeout(Duration::from_secs(sync_timeout_secs()))
        .build()
    {
        Ok(c) => c,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("http client init failed: {err}") })),
            )
                .into_response()
        }
    };

    let descriptor = match fetch_remote_snapshot_descriptor(
        &client,
        &remote_base,
        &req.dataset_id,
        &req.snapshot_id,
        resolve_remote_api_token(req.remote_api_token.as_deref()).as_deref(),
    )
    .await
    {
        Ok(v) => v,
        Err(err) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({ "error": err })),
            )
                .into_response()
        }
    };

    let pull_root = req
        .destination_root
        .clone()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(default_sync_pull_root);
    let package_dir = PathBuf::from(pull_root).join(&descriptor.snapshot_id);
    if let Err(err) = fs::create_dir_all(&package_dir) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("create pull directory failed: {err}") })),
        )
            .into_response();
    }

    let mut downloaded_bytes = 0_u64;
    let chunk_bytes = req
        .max_chunk_bytes
        .unwrap_or_else(sync_chunk_bytes)
        .clamp(4096, 16 * 1024 * 1024);

    for artifact in &descriptor.artifacts {
        if !is_safe_relative_path(&artifact.relative_path) {
            return (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({ "error": format!("remote artifact path '{}' is unsafe", artifact.relative_path) })),
            )
                .into_response();
        }

        let artifact_path = package_dir.join(&artifact.relative_path);
        if let Some(parent) = artifact_path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": format!("create artifact parent failed: {err}") })),
                )
                    .into_response();
            }
        }

        let mut offset = fs::metadata(&artifact_path).map(|m| m.len()).unwrap_or(0);
        if offset > artifact.bytes {
            if let Err(err) = fs::write(&artifact_path, []) {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": format!("reset oversized artifact failed: {err}") })),
                )
                    .into_response();
            }
            offset = 0;
        }

        while offset < artifact.bytes {
            let response = match fetch_remote_artifact_chunk(
                &client,
                &remote_base,
                &req.dataset_id,
                &req.snapshot_id,
                &artifact.relative_path,
                offset,
                chunk_bytes,
                resolve_remote_api_token(req.remote_api_token.as_deref()).as_deref(),
            )
            .await
            {
                Ok(v) => v,
                Err(err) => {
                    return (
                        StatusCode::BAD_GATEWAY,
                        Json(serde_json::json!({ "error": err })),
                    )
                        .into_response()
                }
            };

            if response.chunk.is_empty() {
                return (
                    StatusCode::BAD_GATEWAY,
                    Json(serde_json::json!({ "error": format!("empty chunk received for '{}' at offset {}", artifact.relative_path, offset) })),
                )
                    .into_response();
            }

            match OpenOptions::new().create(true).append(true).open(&artifact_path) {
                Ok(mut file) => {
                    if let Err(err) = file.write_all(&response.chunk) {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(serde_json::json!({ "error": format!("write artifact chunk failed: {err}") })),
                        )
                            .into_response();
                    }
                }
                Err(err) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": format!("open artifact for append failed: {err}") })),
                    )
                        .into_response();
                }
            }

            let fallback_next = offset.saturating_add(response.chunk.len() as u64);
            let next_offset = response.next_offset.unwrap_or(fallback_next);
            if next_offset <= offset {
                return (
                    StatusCode::BAD_GATEWAY,
                    Json(serde_json::json!({ "error": format!("remote chunk offset did not advance for '{}'", artifact.relative_path) })),
                )
                    .into_response();
            }
            offset = next_offset.min(artifact.bytes);
        }
        downloaded_bytes = downloaded_bytes.saturating_add(artifact.bytes);
    }

    let manifest_path = package_dir.join("manifest.json");
    let manifest_pretty = match serde_json::to_string_pretty(&descriptor.manifest) {
        Ok(v) => v,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("serialize manifest failed: {err}") })),
            )
                .into_response()
        }
    };
    if let Err(err) = fs::write(&manifest_path, manifest_pretty) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("write manifest failed: {err}") })),
        )
            .into_response();
    }

    let validation = match validate_contribution_package(&package_dir) {
        Ok(v) => v,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("package validation failed: {err}") })),
            )
                .into_response()
        }
    };
    if !validation.valid {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "pulled package failed validation",
                "validation": validation
            })),
        )
            .into_response();
    }

    let auto_submit = req
        .auto_submit_to_merge_queue
        .unwrap_or_else(default_auto_submit_on_pull);
    let merge_queue_entry = if auto_submit {
        match submit_package_for_merge(MergeSubmitRequest {
            package_dir: package_dir.to_string_lossy().to_string(),
            submitted_by: req.submitted_by.clone(),
        }) {
            Ok(submit) => Some(serde_json::to_value(submit.entry).unwrap_or_default()),
            Err(err) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": format!("merge queue submit failed: {err}") })),
                )
                    .into_response()
            }
        }
    } else {
        None
    };

    (
        StatusCode::OK,
        Json(SyncPullResponse {
            package_dir: package_dir.to_string_lossy().to_string(),
            downloaded_bytes,
            artifacts_downloaded: descriptor.artifacts.len(),
            validation_ok: true,
            merge_queue_entry,
        }),
    )
        .into_response()
}

pub async fn api_federation_sync_push(
    headers: HeaderMap,
    Json(req): Json<SyncPushRequest>,
) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Write, true) {
        append_federation_audit(
            "sync_push",
            "denied",
            serde_json::json!({ "reason": err.1 }),
        );
        return err.into_response();
    }
    append_federation_audit(
        "sync_push",
        "start",
        serde_json::json!({
            "remote_base_url": req.remote_base_url,
            "dataset_id": req.dataset_id,
            "snapshot_id": req.snapshot_id
        }),
    );
    if req.remote_base_url.trim().is_empty()
        || req.dataset_id.trim().is_empty()
        || req.snapshot_id.trim().is_empty()
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "remote_base_url, dataset_id, and snapshot_id are required" })),
        )
            .into_response();
    }

    let source_base_url = req
        .source_base_url
        .clone()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| std::env::var("FERRUMYX_FED_NODE_PUBLIC_BASE_URL").ok())
        .unwrap_or_default();
    if source_base_url.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "source_base_url is required (or set FERRUMYX_FED_NODE_PUBLIC_BASE_URL)" })),
        )
            .into_response();
    }

    let target_base = normalize_base_url(&req.remote_base_url);
    let source_base = normalize_base_url(&source_base_url);
    let client = match Client::builder()
        .timeout(Duration::from_secs(sync_timeout_secs()))
        .build()
    {
        Ok(c) => c,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("http client init failed: {err}") })),
            )
                .into_response()
        }
    };

    let body = SyncPullRequest {
        remote_base_url: source_base.clone(),
        dataset_id: req.dataset_id.clone(),
        snapshot_id: req.snapshot_id.clone(),
        max_chunk_bytes: req.max_chunk_bytes,
        destination_root: None,
        submitted_by: req.submitted_by.clone(),
        remote_api_token: req.source_api_token.clone(),
        auto_submit_to_merge_queue: Some(true),
    };
    let url = format!("{}/api/federation/sync/pull", target_base);
    let mut request = client.post(url).json(&body);
    let remote_token = resolve_remote_api_token(req.remote_api_token.as_deref());
    if let Some(token) = remote_token.as_deref() {
        request = request.header(AUTHORIZATION, format!("Bearer {}", token.trim()));
    }

    let response = match request.send().await {
        Ok(v) => v,
        Err(err) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({ "error": format!("remote push-trigger request failed: {err}") })),
            )
                .into_response()
        }
    };
    let status_code = response.status();
    let parsed = response
        .json::<serde_json::Value>()
        .await
        .unwrap_or_else(|_| serde_json::json!({ "error": "remote returned non-json response" }));

    (
        if status_code.is_success() {
            StatusCode::OK
        } else {
            StatusCode::BAD_GATEWAY
        },
        Json(SyncPushResponse {
            remote_base_url: target_base,
            source_base_url: source_base,
            remote_status_code: status_code.as_u16(),
            remote_response: parsed,
        }),
    )
        .into_response()
}

pub async fn api_federation_hf_status(
    headers: HeaderMap,
    Query(query): Query<HfSyncStatusQuery>,
) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Read, false) {
        return err.into_response();
    }

    let enabled = hf_sync_enabled();
    let hf_binary = hf_binary();
    let repo_id = resolve_hf_repo_id(query.repo_id.as_deref());
    let snapshots_prefix = hf_snapshots_prefix();
    let revision = hf_default_revision();
    let timeout_secs = hf_timeout_secs();
    let pull_root = hf_pull_root();

    let version_args = vec!["--version".to_string()];
    let version = match run_hf_command(&version_args).await {
        Ok(v) => Some(v.stdout.trim().to_string()),
        Err(err) => {
            return (
                StatusCode::OK,
                Json(HfSyncStatusResponse {
                    enabled,
                    hf_binary,
                    hf_available: false,
                    hf_version: None,
                    repo_id,
                    snapshots_prefix,
                    revision,
                    timeout_secs,
                    pull_root,
                    repo_check_ok: None,
                    repo_private: None,
                    repo_gated: None,
                    repo_info: None,
                    error: Some(err),
                }),
            )
                .into_response();
        }
    };

    let mut repo_check_ok = None;
    let mut repo_private = None;
    let mut repo_gated = None;
    let mut repo_info = None;
    let mut error = None;
    if let Some(repo) = repo_id.as_ref() {
        let mut info_args = vec![
            "datasets".to_string(),
            "info".to_string(),
            repo.to_string(),
        ];
        if let Some(token) = resolve_hf_token(None) {
            info_args.push("--token".to_string());
            info_args.push(token);
        }
        match run_hf_command(&info_args).await {
            Ok(out) => {
                repo_check_ok = Some(true);
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&out.stdout) {
                    repo_private = value.get("private").and_then(|v| v.as_bool());
                    repo_gated = value.get("gated").and_then(|v| v.as_bool());
                    repo_info = Some(value);
                }
            }
            Err(err_text) => {
                repo_check_ok = Some(false);
                error = Some(err_text);
            }
        }
    }

    (
        StatusCode::OK,
        Json(HfSyncStatusResponse {
            enabled,
            hf_binary,
            hf_available: true,
            hf_version: version,
            repo_id,
            snapshots_prefix,
            revision,
            timeout_secs,
            pull_root,
            repo_check_ok,
            repo_private,
            repo_gated,
            repo_info,
            error,
        }),
    )
        .into_response()
}

pub async fn api_federation_hf_publish(
    headers: HeaderMap,
    Json(req): Json<HfPublishRequest>,
) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Write, true) {
        append_federation_audit(
            "hf_publish",
            "denied",
            serde_json::json!({ "reason": err.1 }),
        );
        return err.into_response();
    }
    if !hf_sync_enabled() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "HF sync is disabled (set FERRUMYX_FED_HF_ENABLED=1 or enable in settings)" })),
        )
            .into_response();
    }

    let package_dir = PathBuf::from(req.package_dir.trim());
    if req.package_dir.trim().is_empty() || !package_dir.exists() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "package_dir is required and must exist" })),
        )
            .into_response();
    }
    let validation = match validate_contribution_package(&package_dir) {
        Ok(v) => v,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("package validation failed: {err}") })),
            )
                .into_response()
        }
    };
    if !validation.valid {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "package is invalid and cannot be published",
                "validation": validation
            })),
        )
            .into_response();
    }

    let manifest_path = package_dir.join("manifest.json");
    let manifest_raw = match fs::read_to_string(&manifest_path) {
        Ok(v) => v,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("read manifest failed: {err}") })),
            )
                .into_response()
        }
    };
    let manifest = match serde_json::from_str::<ContributionManifest>(&manifest_raw) {
        Ok(v) => v,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("parse manifest failed: {err}") })),
            )
                .into_response()
        }
    };
    let snapshot_id = manifest.snapshot_id.clone();

    let repo_id = match resolve_hf_repo_id(req.repo_id.as_deref()) {
        Some(v) => v,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "HF repo_id is required (set in settings or request)" })),
            )
                .into_response()
        }
    };
    let path_in_repo = req
        .path_in_repo
        .as_deref()
        .map(sanitize_repo_subpath)
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| format!("{}/{}", hf_snapshots_prefix(), snapshot_id));
    if !is_safe_relative_path(&path_in_repo) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "path_in_repo is unsafe" })),
        )
            .into_response();
    }
    let revision = req
        .revision
        .as_deref()
        .and_then(trimmed_nonempty)
        .map(str::to_string)
        .or_else(hf_default_revision);
    let token = resolve_hf_token(req.token.as_deref());

    let mut upload_args = vec![
        "upload".to_string(),
        repo_id.clone(),
        package_dir.to_string_lossy().to_string(),
        path_in_repo.clone(),
        "--repo-type".to_string(),
        "dataset".to_string(),
    ];
    if let Some(value) = revision.as_ref() {
        upload_args.push("--revision".to_string());
        upload_args.push(value.clone());
    }
    if let Some(value) = req
        .commit_message
        .as_deref()
        .and_then(trimmed_nonempty)
        .map(str::to_string)
    {
        upload_args.push("--commit-message".to_string());
        upload_args.push(value);
    }
    if let Some(value) = token.as_ref() {
        upload_args.push("--token".to_string());
        upload_args.push(value.clone());
    }

    let upload = match run_hf_command(&upload_args).await {
        Ok(v) => v,
        Err(err) => {
            append_federation_audit(
                "hf_publish",
                "error",
                serde_json::json!({ "repo_id": repo_id, "snapshot_id": snapshot_id, "error": err }),
            );
            return (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({ "error": err })),
            )
                .into_response();
        }
    };

    let create_tag = req.create_tag.unwrap_or(true);
    let mut tag_created = false;
    let mut tag_name = None;
    let mut tag_error = None;
    if create_tag {
        let tag = req
            .tag_name
            .as_deref()
            .and_then(trimmed_nonempty)
            .unwrap_or(&snapshot_id)
            .to_string();
        let mut tag_args = vec![
            "repos".to_string(),
            "tag".to_string(),
            "create".to_string(),
            repo_id.clone(),
            tag.clone(),
            "--repo-type".to_string(),
            "dataset".to_string(),
        ];
        if let Some(value) = revision.as_ref() {
            tag_args.push("--revision".to_string());
            tag_args.push(value.clone());
        }
        if let Some(value) = token.as_ref() {
            tag_args.push("--token".to_string());
            tag_args.push(value.clone());
        }
        match run_hf_command(&tag_args).await {
            Ok(_) => {
                tag_created = true;
                tag_name = Some(tag);
            }
            Err(err) => {
                tag_name = Some(tag);
                tag_error = Some(err);
            }
        }
    }

    append_federation_audit(
        "hf_publish",
        "ok",
        serde_json::json!({
            "repo_id": repo_id,
            "snapshot_id": snapshot_id,
            "path_in_repo": path_in_repo,
            "tag_created": tag_created
        }),
    );
    (
        StatusCode::OK,
        Json(HfPublishResponse {
            repo_id,
            package_dir: package_dir.to_string_lossy().to_string(),
            snapshot_id,
            path_in_repo,
            revision,
            upload_stdout: upload.stdout,
            upload_stderr: upload.stderr,
            tag_created,
            tag_name,
            tag_error,
        }),
    )
        .into_response()
}

pub async fn api_federation_hf_pull(
    headers: HeaderMap,
    Json(req): Json<HfPullRequest>,
) -> impl IntoResponse {
    if let Err(err) = authorize_federation_request(&headers, FederationScope::Write, true) {
        append_federation_audit(
            "hf_pull",
            "denied",
            serde_json::json!({ "reason": err.1 }),
        );
        return err.into_response();
    }
    if !hf_sync_enabled() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "HF sync is disabled (set FERRUMYX_FED_HF_ENABLED=1 or enable in settings)" })),
        )
            .into_response();
    }

    let snapshot_id = match trimmed_nonempty(&req.snapshot_id) {
        Some(v) => v.to_string(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "snapshot_id is required" })),
            )
                .into_response()
        }
    };
    let repo_id = match resolve_hf_repo_id(req.repo_id.as_deref()) {
        Some(v) => v,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "HF repo_id is required (set in settings or request)" })),
            )
                .into_response()
        }
    };
    let path_in_repo = req
        .path_in_repo
        .as_deref()
        .map(sanitize_repo_subpath)
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| format!("{}/{}", hf_snapshots_prefix(), snapshot_id));
    if !is_safe_relative_path(&path_in_repo) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "path_in_repo is unsafe" })),
        )
            .into_response();
    }
    let revision = req
        .revision
        .as_deref()
        .and_then(trimmed_nonempty)
        .map(str::to_string)
        .or_else(hf_default_revision);
    let destination_root = req
        .destination_root
        .as_deref()
        .and_then(trimmed_nonempty)
        .map(str::to_string)
        .unwrap_or_else(hf_pull_root);
    let destination_root_path = PathBuf::from(&destination_root);
    if let Err(err) = fs::create_dir_all(&destination_root_path) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("create destination_root failed: {err}") })),
        )
            .into_response();
    }
    let token = resolve_hf_token(req.token.as_deref());
    let include = format!("{}/*", path_in_repo);
    let mut download_args = vec![
        "download".to_string(),
        repo_id.clone(),
        "--repo-type".to_string(),
        "dataset".to_string(),
        "--include".to_string(),
        include,
        "--local-dir".to_string(),
        destination_root.clone(),
    ];
    if let Some(value) = revision.as_ref() {
        download_args.push("--revision".to_string());
        download_args.push(value.clone());
    }
    if let Some(value) = token.as_ref() {
        download_args.push("--token".to_string());
        download_args.push(value.clone());
    }

    let download = match run_hf_command(&download_args).await {
        Ok(v) => v,
        Err(err) => {
            append_federation_audit(
                "hf_pull",
                "error",
                serde_json::json!({ "repo_id": repo_id, "snapshot_id": snapshot_id, "error": err }),
            );
            return (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({ "error": err })),
            )
                .into_response();
        }
    };

    let mut package_dir = destination_root_path.join(&path_in_repo);
    if !package_dir.exists() {
        let fallback = destination_root_path.join(&snapshot_id);
        if fallback.join("manifest.json").exists() {
            package_dir = fallback;
        } else if let Some(found) =
            locate_downloaded_package_dir(&destination_root_path, &snapshot_id)
        {
            package_dir = found;
        }
    }
    if !package_dir.exists() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("download completed but package path '{}' was not found", package_dir.to_string_lossy())
            })),
        )
            .into_response();
    }
    let validation = match validate_contribution_package(&package_dir) {
        Ok(v) => v,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("downloaded package validation failed: {err}") })),
            )
                .into_response()
        }
    };
    if !validation.valid {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "downloaded package failed validation",
                "validation": validation
            })),
        )
            .into_response();
    }

    let auto_submit = req
        .auto_submit_to_merge_queue
        .unwrap_or_else(default_auto_submit_on_pull);
    let merge_queue_entry = if auto_submit {
        match submit_package_for_merge(MergeSubmitRequest {
            package_dir: package_dir.to_string_lossy().to_string(),
            submitted_by: req.submitted_by.clone(),
        }) {
            Ok(submit) => Some(serde_json::to_value(submit.entry).unwrap_or_default()),
            Err(err) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": format!("merge queue submit failed: {err}") })),
                )
                    .into_response()
            }
        }
    } else {
        None
    };

    append_federation_audit(
        "hf_pull",
        "ok",
        serde_json::json!({
            "repo_id": repo_id,
            "snapshot_id": snapshot_id,
            "path_in_repo": path_in_repo,
            "merge_queue_submitted": merge_queue_entry.is_some()
        }),
    );
    (
        StatusCode::OK,
        Json(HfPullResponse {
            repo_id,
            snapshot_id,
            path_in_repo,
            destination_root,
            package_dir: package_dir.to_string_lossy().to_string(),
            validation_ok: true,
            merge_queue_entry,
            download_stdout: download.stdout,
            download_stderr: download.stderr,
        }),
    )
        .into_response()
}

fn locate_downloaded_package_dir(root: &Path, snapshot_id: &str) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(v) => v,
                Err(_) => continue,
            };
            if !file_type.is_dir() {
                continue;
            }
            let name = entry.file_name();
            if name.to_string_lossy() == snapshot_id && path.join("manifest.json").exists() {
                return Some(path);
            }
            stack.push(path);
        }
    }
    None
}

fn build_local_sync_index() -> Result<SyncIndexResponse, String> {
    let lineage = get_canonical_lineage().map_err(|err| err.to_string())?;
    let mut snapshots = Vec::with_capacity(lineage.snapshots.len());
    let mut warnings = Vec::new();
    for record in lineage.snapshots {
        match read_manifest_for_snapshot(&record) {
            Ok(manifest) => {
                let artifact_bytes_total = manifest
                    .artifacts
                    .iter()
                    .fold(0_u64, |acc, item| acc.saturating_add(item.bytes));
                snapshots.push(SyncSnapshotIndexItem {
                    dataset_id: record.dataset_id,
                    snapshot_id: record.snapshot_id,
                    parent_snapshot_id: record.parent_snapshot_id,
                    manifest_id: record.manifest_id,
                    approved_at: record.approved_at,
                    artifact_count: manifest.artifacts.len(),
                    artifact_bytes_total,
                });
            }
            Err(err) => warnings.push(format!(
                "snapshot '{}' skipped: {}",
                record.snapshot_id, err
            )),
        }
    }
    snapshots.sort_by(|a, b| a.approved_at.cmp(&b.approved_at));
    Ok(SyncIndexResponse {
        generated_at: chrono::Utc::now().to_rfc3339(),
        snapshots,
        warnings,
    })
}

fn find_snapshot_record(dataset_id: &str, snapshot_id: &str) -> Result<CanonicalSnapshotRecord, String> {
    let lineage = get_canonical_lineage().map_err(|err| err.to_string())?;
    lineage
        .snapshots
        .into_iter()
        .find(|row| {
            row.dataset_id.eq_ignore_ascii_case(dataset_id)
                && row.snapshot_id.eq_ignore_ascii_case(snapshot_id)
        })
        .ok_or_else(|| {
            format!(
                "snapshot '{}' for dataset '{}' not found in canonical lineage",
                snapshot_id, dataset_id
            )
        })
}

fn find_snapshot_descriptor(dataset_id: &str, snapshot_id: &str) -> Result<SyncSnapshotDescriptor, String> {
    let record = find_snapshot_record(dataset_id, snapshot_id)?;
    let manifest = read_manifest_for_snapshot(&record)?;
    let artifacts = manifest
        .artifacts
        .iter()
        .map(|row| SyncArtifactDescriptor {
            artifact_kind: row.artifact_kind.clone(),
            relative_path: row.relative_path.clone(),
            row_count: row.row_count,
            sha256: row.sha256.clone(),
            bytes: row.bytes,
        })
        .collect::<Vec<_>>();
    Ok(SyncSnapshotDescriptor {
        dataset_id: record.dataset_id,
        snapshot_id: record.snapshot_id,
        parent_snapshot_id: record.parent_snapshot_id,
        manifest_id: record.manifest_id,
        approved_at: record.approved_at,
        artifacts,
        manifest,
    })
}

fn read_manifest_for_snapshot(record: &CanonicalSnapshotRecord) -> Result<ContributionManifest, String> {
    let manifest_path = Path::new(&record.package_dir).join("manifest.json");
    let raw = fs::read_to_string(&manifest_path)
        .map_err(|err| format!("read manifest '{}': {err}", manifest_path.to_string_lossy()))?;
    serde_json::from_str::<ContributionManifest>(&raw).map_err(|err| {
        format!(
            "parse manifest '{}': {err}",
            manifest_path.to_string_lossy()
        )
    })
}

fn read_artifact_chunk(query: &SyncArtifactQuery) -> Result<axum::response::Response, (StatusCode, String)> {
    if query.relative_path.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "relative_path is required".to_string()));
    }
    if !is_safe_relative_path(&query.relative_path) {
        return Err((StatusCode::BAD_REQUEST, "relative_path is unsafe".to_string()));
    }

    let descriptor = find_snapshot_descriptor(&query.dataset_id, &query.snapshot_id)
        .map_err(|err| (StatusCode::NOT_FOUND, err))?;
    let artifact = descriptor
        .artifacts
        .iter()
        .find(|row| row.relative_path == query.relative_path)
        .cloned()
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!(
                    "artifact '{}' not found in snapshot '{}'",
                    query.relative_path, query.snapshot_id
                ),
            )
        })?;

    let package_dir = PathBuf::from(
        default_package_dir_for_descriptor(&descriptor)
            .map_err(|err| (StatusCode::NOT_FOUND, err))?,
    );
    let path = package_dir.join(&artifact.relative_path);
    if !path.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            format!("artifact file '{}' does not exist", path.to_string_lossy()),
        ));
    }

    let total_bytes = artifact.bytes;
    let offset = query.offset.unwrap_or(0).min(total_bytes);
    let max_bytes = query
        .max_bytes
        .unwrap_or_else(sync_chunk_bytes)
        .clamp(4096, 16 * 1024 * 1024);
    let remaining = total_bytes.saturating_sub(offset);
    let read_len = remaining.min(max_bytes) as usize;

    let mut file = fs::File::open(&path)
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, format!("open artifact failed: {err}")))?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, format!("seek artifact failed: {err}")))?;
    let mut chunk = vec![0_u8; read_len];
    let read = file
        .read(&mut chunk)
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, format!("read artifact failed: {err}")))?;
    chunk.truncate(read);
    let next_offset = offset.saturating_add(read as u64);
    let complete = next_offset >= total_bytes;

    let mut headers = HeaderMap::new();
    insert_header_u64(&mut headers, "x-ferrumyx-total-bytes", total_bytes);
    insert_header_u64(&mut headers, "x-ferrumyx-next-offset", next_offset);
    insert_header_str(
        &mut headers,
        "x-ferrumyx-complete",
        if complete { "1" } else { "0" },
    );
    insert_header_str(&mut headers, "x-ferrumyx-sha256", &artifact.sha256);
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );

    Ok((StatusCode::OK, headers, chunk).into_response())
}

fn default_package_dir_for_descriptor(descriptor: &SyncSnapshotDescriptor) -> Result<String, String> {
    let record = find_snapshot_record(&descriptor.dataset_id, &descriptor.snapshot_id)?;
    Ok(record.package_dir)
}

fn is_safe_relative_path(raw: &str) -> bool {
    let path = Path::new(raw);
    if path.is_absolute() {
        return false;
    }
    !path
        .components()
        .any(|comp| matches!(comp, Component::ParentDir | Component::Prefix(_)))
}

fn normalize_base_url(raw: &str) -> String {
    raw.trim().trim_end_matches('/').to_string()
}

fn resolve_remote_api_token(raw: Option<&str>) -> Option<String> {
    raw.map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .or_else(|| {
            std::env::var("FERRUMYX_FED_REMOTE_API_TOKEN")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        })
}

fn sync_chunk_bytes() -> u64 {
    std::env::var("FERRUMYX_FED_SYNC_CHUNK_BYTES")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_SYNC_CHUNK_BYTES)
        .clamp(4096, 16 * 1024 * 1024)
}

fn sync_timeout_secs() -> u64 {
    std::env::var("FERRUMYX_FED_SYNC_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_SYNC_TIMEOUT_SECS)
        .clamp(5, 600)
}

fn default_sync_pull_root() -> String {
    std::env::var("FERRUMYX_FED_SYNC_PULL_ROOT")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SYNC_PULL_ROOT.to_string())
}

fn default_auto_submit_on_pull() -> bool {
    std::env::var("FERRUMYX_FED_PULL_AUTO_SUBMIT")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(true)
}

fn hf_sync_enabled() -> bool {
    env_bool("FERRUMYX_FED_HF_ENABLED", DEFAULT_HF_ENABLED)
}

fn hf_binary() -> String {
    env_trimmed("FERRUMYX_FED_HF_BIN").unwrap_or_else(|| DEFAULT_HF_BIN.to_string())
}

fn hf_snapshots_prefix() -> String {
    env_trimmed("FERRUMYX_FED_HF_SNAPSHOTS_PREFIX")
        .unwrap_or_else(|| DEFAULT_HF_SNAPSHOTS_PREFIX.to_string())
        .trim_matches('/')
        .to_string()
}

fn hf_default_revision() -> Option<String> {
    env_trimmed("FERRUMYX_FED_HF_REVISION")
}

fn hf_timeout_secs() -> u64 {
    std::env::var("FERRUMYX_FED_HF_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_HF_TIMEOUT_SECS)
        .clamp(30, 7_200)
}

fn hf_pull_root() -> String {
    env_trimmed("FERRUMYX_FED_HF_PULL_ROOT").unwrap_or_else(|| DEFAULT_HF_PULL_ROOT.to_string())
}

fn resolve_hf_repo_id(raw: Option<&str>) -> Option<String> {
    raw.and_then(trimmed_nonempty)
        .map(str::to_string)
        .or_else(|| env_trimmed("FERRUMYX_FED_HF_REPO_ID"))
}

fn resolve_hf_token(raw: Option<&str>) -> Option<String> {
    raw.and_then(trimmed_nonempty)
        .map(str::to_string)
        .or_else(|| env_trimmed("FERRUMYX_FED_HF_TOKEN"))
}

fn sanitize_repo_subpath(raw: &str) -> String {
    raw.trim().trim_matches('/').replace('\\', "/")
}

fn trimmed_nonempty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

async fn run_hf_command(args: &[String]) -> Result<HfCommandResult, String> {
    let mut command = tokio::process::Command::new(hf_binary());
    command.args(args);
    command.kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(hf_timeout_secs()), command.output())
        .await
        .map_err(|_| {
            format!(
                "hf command timed out after {} seconds",
                hf_timeout_secs()
            )
        })?
        .map_err(|err| format!("failed to run hf command: {err}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        let code = output
            .status
            .code()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        return Err(format!(
            "hf command failed (exit code {code}){}{}",
            if stdout.is_empty() { "" } else { ": " },
            if stderr.is_empty() { stdout } else { stderr }
        ));
    }
    Ok(HfCommandResult { stdout, stderr })
}

fn insert_header_u64(headers: &mut HeaderMap, name: &'static str, value: u64) {
    insert_header_str(headers, name, &value.to_string());
}

fn insert_header_str(headers: &mut HeaderMap, name: &'static str, value: &str) {
    if let Ok(v) = HeaderValue::from_str(value) {
        headers.insert(name, v);
    }
}

async fn fetch_remote_sync_index(
    client: &Client,
    remote_base_url: &str,
    token: Option<&str>,
) -> Result<SyncIndexResponse, String> {
    let url = format!("{}/api/federation/sync/index", remote_base_url);
    let mut req = client.get(url);
    if let Some(value) = token.filter(|v| !v.trim().is_empty()) {
        req = req.header(AUTHORIZATION, format!("Bearer {}", value.trim()));
    }
    let resp = req
        .send()
        .await
        .map_err(|err| format!("remote sync index request failed: {err}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!(
            "remote sync index request failed: status {} body {}",
            status, body
        ));
    }
    resp.json::<SyncIndexResponse>()
        .await
        .map_err(|err| format!("parse remote sync index failed: {err}"))
}

async fn fetch_remote_snapshot_descriptor(
    client: &Client,
    remote_base_url: &str,
    dataset_id: &str,
    snapshot_id: &str,
    token: Option<&str>,
) -> Result<SyncSnapshotDescriptor, String> {
    let url = format!("{}/api/federation/sync/snapshot", remote_base_url);
    let mut req = client
        .get(url)
        .query(&[("dataset_id", dataset_id), ("snapshot_id", snapshot_id)]);
    if let Some(value) = token.filter(|v| !v.trim().is_empty()) {
        req = req.header(AUTHORIZATION, format!("Bearer {}", value.trim()));
    }
    let resp = req
        .send()
        .await
        .map_err(|err| format!("remote snapshot descriptor request failed: {err}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!(
            "remote snapshot descriptor request failed: status {} body {}",
            status, body
        ));
    }
    resp.json::<SyncSnapshotDescriptor>()
        .await
        .map_err(|err| format!("parse remote snapshot descriptor failed: {err}"))
}

async fn fetch_remote_artifact_chunk(
    client: &Client,
    remote_base_url: &str,
    dataset_id: &str,
    snapshot_id: &str,
    relative_path: &str,
    offset: u64,
    max_bytes: u64,
    token: Option<&str>,
) -> Result<ArtifactChunkHttpResponse, String> {
    let url = format!("{}/api/federation/sync/artifact", remote_base_url);
    let mut req = client.get(url).query(&[
        ("dataset_id", dataset_id.to_string()),
        ("snapshot_id", snapshot_id.to_string()),
        ("relative_path", relative_path.to_string()),
        ("offset", offset.to_string()),
        ("max_bytes", max_bytes.to_string()),
    ]);
    if let Some(value) = token.filter(|v| !v.trim().is_empty()) {
        req = req.header(AUTHORIZATION, format!("Bearer {}", value.trim()));
    }
    let resp = req
        .send()
        .await
        .map_err(|err| format!("remote artifact chunk request failed: {err}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!(
            "remote artifact chunk request failed: status {} body {}",
            status, body
        ));
    }
    let next_offset = resp
        .headers()
        .get("x-ferrumyx-next-offset")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());
    let bytes = resp
        .bytes()
        .await
        .map_err(|err| format!("read remote artifact chunk body failed: {err}"))?;
    Ok(ArtifactChunkHttpResponse {
        chunk: bytes.to_vec(),
        next_offset,
    })
}
