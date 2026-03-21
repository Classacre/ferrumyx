//! Federation manifest endpoints (draft schema + draft generation + validation).

use crate::state::SharedState;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use ferrumyx_common::federation::{
    ContributionManifest, FEDERATION_SCHEMA_URL, FEDERATION_SCHEMA_VERSION, ManifestValidationReport,
};
use ferrumyx_db::{
    build_contribution_manifest_draft, export_contribution_package, validate_contribution_manifest,
    validate_contribution_package, sign_contribution_package, ManifestDraftRequest,
    PackageExportRequest, PackageSignRequest, PackageValidationRequest,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct FederationSchemaResponse {
    pub schema_version: &'static str,
    pub schema_url: &'static str,
    pub required_fields: Vec<&'static str>,
    pub template: ContributionManifest,
}

pub async fn api_federation_schema() -> Json<FederationSchemaResponse> {
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
}

pub async fn api_federation_manifest_draft(
    State(state): State<SharedState>,
    Json(req): Json<ManifestDraftRequest>,
) -> impl IntoResponse {
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
    Json(manifest): Json<ContributionManifest>,
) -> Json<ManifestValidationReport> {
    Json(validate_contribution_manifest(&manifest))
}

pub async fn api_federation_package_export(
    State(state): State<SharedState>,
    Json(req): Json<PackageExportRequest>,
) -> impl IntoResponse {
    match export_contribution_package(state.db.clone(), req).await {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}

pub async fn api_federation_package_validate(
    Json(req): Json<PackageValidationRequest>,
) -> impl IntoResponse {
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
    Json(req): Json<PackageSignRequest>,
) -> impl IntoResponse {
    match sign_contribution_package(req) {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": err.to_string() })),
        )
            .into_response(),
    }
}
