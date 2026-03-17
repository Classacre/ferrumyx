//! Axum router — maps all URL paths to handlers.

use crate::handlers::{
    chat::{
        chat_events_proxy, chat_history, chat_lab_monitor, chat_page, chat_submit, chat_thread_new,
        chat_threads,
    },
    dashboard::dashboard,
    depmap::{api_depmap_celllines, api_depmap_gene, depmap_page},
    ingestion::{ingestion_page, ingestion_run},
    kg::{api_entity_suggest, api_kg_facts, api_kg_stats, kg_page},
    metrics::{metrics_page, metrics_perf_api},
    molecules::{api_molecules_run, molecules_page},
    ner::{api_ner_extract, api_ner_stats, ner_extract, ner_page},
    query::{query_page, query_submit},
    ranker::{api_ranker_score, api_ranker_stats, api_ranker_top, ranker_page},
    search::hybrid_search,
    settings::{settings_get, settings_page, settings_save},
    system::system_page,
    targets::{api_target_detail, api_targets, targets_page},
};
use crate::sse::sse_handler;
use crate::state::{AppState, SharedState};
use axum::{
    response::Redirect,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::{
    compression::CompressionLayer, cors::CorsLayer, services::ServeDir, trace::TraceLayer,
};

/// Build and return the full Axum router.
pub fn build_router(state: AppState) -> Router {
    let shared: SharedState = Arc::new(state);

    Router::new()
        // Pages
        .route("/", get(dashboard))
        .route("/query", get(query_page).post(query_submit))
        .route("/targets", get(targets_page))
        .route("/ingestion", get(ingestion_page))
        .route("/ingestion/run", post(ingestion_run))
        .route("/molecules", get(molecules_page))
        .route("/kg", get(kg_page))
        .route("/metrics", get(metrics_page))
        .route("/system", get(system_page))
        .route("/audit", get(system_page)) // alias for now
        .route("/ner", get(ner_page).post(ner_extract))
        .route("/depmap", get(depmap_page))
        .route("/ranker", get(ranker_page))
        .route("/settings", get(settings_page))
        .route("/chat", get(chat_page))
        .route(
            "/favicon.ico",
            get(|| async { Redirect::permanent("/static/logo.svg") }),
        )
        // SSE streaming
        .route("/api/events", get(sse_handler))
        // API endpoints
        .route("/api/targets", get(api_targets))
        .route("/api/targets/{gene}", get(api_target_detail))
        .route("/api/kg", get(api_kg_facts))
        .route("/api/kg/stats", get(api_kg_stats))
        .route("/api/entities/suggest", get(api_entity_suggest))
        .route("/api/search", get(hybrid_search))
        .route("/api/ner/stats", get(api_ner_stats))
        .route("/api/ner/extract", post(api_ner_extract))
        .route("/api/molecules/run", post(api_molecules_run))
        .route("/api/depmap/gene", get(api_depmap_gene))
        .route("/api/depmap/celllines", get(api_depmap_celllines))
        .route("/api/ranker/score", get(api_ranker_score))
        .route("/api/ranker/top", get(api_ranker_top))
        .route("/api/ranker/stats", get(api_ranker_stats))
        .route("/api/metrics/perf", get(metrics_perf_api))
        .route("/api/chat", post(chat_submit))
        .route("/api/chat/history", get(chat_history))
        .route("/api/chat/threads", get(chat_threads))
        .route("/api/chat/thread/new", post(chat_thread_new))
        .route("/api/chat/lab-monitor", get(chat_lab_monitor))
        .route("/api/chat/events", get(chat_events_proxy))
        .route("/api/settings", get(settings_get).post(settings_save))
        // Static files
        .nest_service(
            "/static",
            ServeDir::new(format!("{}/static", env!("CARGO_MANIFEST_DIR"))),
        )
        // Middleware
        .layer(CorsLayer::permissive())
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(shared)
}
