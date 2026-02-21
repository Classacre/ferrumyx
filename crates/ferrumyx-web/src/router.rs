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
    targets::targets_page,
    ingestion::ingestion_page,
    molecules::molecules_page,
    kg::kg_page,
    metrics::metrics_page,
    system::system_page,
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
        .route("/molecules",  get(molecules_page))
        .route("/kg",         get(kg_page))
        .route("/metrics",    get(metrics_page))
        .route("/system",     get(system_page))
        .route("/audit",      get(system_page)) // alias for now

        // SSE streaming
        .route("/api/events", get(sse_handler))

        // API stubs (will be expanded in later phases)
        .route("/api/ingestion/run", post(api_stub))
        .route("/api/targets",       get(api_stub))
        .route("/api/kg/facts",      get(api_stub))

        // Static files
        .nest_service("/static", ServeDir::new("static"))

        // Middleware
        .layer(CorsLayer::permissive())
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(shared)
}

/// Placeholder for API endpoints not yet implemented.
async fn api_stub() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "not_implemented",
        "message": "This API endpoint will be implemented in a future phase."
    }))
}
