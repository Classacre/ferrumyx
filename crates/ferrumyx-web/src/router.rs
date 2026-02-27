//! Axum router — maps all URL paths to handlers.

use axum::{
    Router,
    routing::{get, post},
};
use tower_http::{
    services::ServeDir,
    cors::CorsLayer,
    trace::TraceLayer,
    compression::CompressionLayer,
};
use std::sync::Arc;
use crate::state::{AppState, SharedState};
use crate::handlers::{
    dashboard::dashboard,
    query::{query_page, query_submit},
    targets::{targets_page, api_targets, api_target_detail},
    ingestion::{ingestion_page, ingestion_run},
    molecules::{molecules_page, api_molecules_run},
    kg::{kg_page, api_kg_facts, api_kg_stats},
    metrics::metrics_page,
    search::hybrid_search,
    ner::{ner_page, ner_extract, api_ner_stats, api_ner_extract},
    depmap::{depmap_page, api_depmap_gene, api_depmap_celllines},
    ranker::{ranker_page, api_ranker_score, api_ranker_top, api_ranker_stats},
    settings::settings_page,
};
use crate::sse::sse_handler;

/// Build and return the full Axum router.
pub fn build_router(state: AppState) -> Router {
    let shared: SharedState = Arc::new(state);

    Router::new()
        // Pages
        .route("/",           get(dashboard))
        .route("/query",      get(query_page).post(query_submit))
        .route("/targets",    get(targets_page))
        .route("/ingestion",  get(ingestion_page))
        .route("/ingestion/run", post(ingestion_run))
        .route("/molecules",  get(molecules_page))
        .route("/kg",         get(kg_page))
        .route("/metrics",    get(metrics_page))
        .route("/ner",        get(ner_page).post(ner_extract))
        .route("/depmap",     get(depmap_page))
        .route("/ranker",     get(ranker_page))
        .route("/settings",   get(settings_page))

        // SSE streaming
        .route("/api/events", get(sse_handler))

        // API endpoints
        .route("/api/targets",       get(api_targets))
        .route("/api/targets/{gene}", get(api_target_detail))
        .route("/api/kg",            get(api_kg_facts))
        .route("/api/kg/stats",      get(api_kg_stats))
        .route("/api/search",        get(hybrid_search))
        .route("/api/ner/stats",     get(api_ner_stats))
        .route("/api/ner/extract",   post(api_ner_extract))
        .route("/api/molecules/run", post(api_molecules_run))
        .route("/api/depmap/gene",   get(api_depmap_gene))
        .route("/api/depmap/celllines", get(api_depmap_celllines))
        .route("/api/ranker/score",  get(api_ranker_score))
        .route("/api/ranker/top",    get(api_ranker_top))
        .route("/api/ranker/stats",  get(api_ranker_stats))

        // Static files
        .nest_service("/static", ServeDir::new("static"))

        // Middleware
        .layer(CorsLayer::permissive())
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(shared)
}
