//! Knowledge graph explorer.

use axum::{
    extract::{State, Query},
    response::{Html, IntoResponse},
    Json,
};
use serde::{Deserialize, Serialize};
use crate::state::SharedState;
use crate::handlers::dashboard::NAV_HTML;
use ferrumyx_common::error::ApiError;
use ferrumyx_db::entities::EntityRepository;
use ferrumyx_db::kg_facts::KgFactRepository;

#[derive(Deserialize, Default)]
pub struct KgFilter { pub gene: Option<String> }

// === API Types ===

#[derive(Debug, Serialize)]
pub struct ApiKgFact {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f64,
    pub source: String,
    pub evidence_count: i32,
}

#[derive(Debug, Serialize)]
pub struct ApiKgStats {
    pub entity_count: u64,
    pub fact_count: u64,
    pub gene_count: u64,
    pub cancer_count: u64,
}

// === API Endpoints ===

/// GET /api/kg - List KG facts
pub async fn api_kg_facts(
    State(state): State<SharedState>,
    Query(filter): Query<KgFilter>,
) -> Result<impl IntoResponse, ApiError> {
    let _gene = filter.gene.as_deref().unwrap_or("KRAS");

    // Use KgFactRepository to get facts
    let fact_repo = KgFactRepository::new(state.db.clone());
    let facts = fact_repo.list(0, 100).await.unwrap_or_default();
    
    // Convert to API format
    let api_facts: Vec<ApiKgFact> = facts.iter().map(|f| ApiKgFact {
        subject: f.subject_name.clone(),
        predicate: f.predicate.clone(),
        object: f.object_name.clone(),
        confidence: f.confidence.map(|c| c as f64).unwrap_or(0.5),
        source: "unknown".to_string(),
        evidence_count: 1,
    }).collect();

    Ok(Json(api_facts))
}

/// GET /api/kg/stats - KG statistics
pub async fn api_kg_stats(
    State(state): State<SharedState>,
) -> Result<impl IntoResponse, ApiError> {
    let entity_repo = EntityRepository::new(state.db.clone());
    let fact_repo = KgFactRepository::new(state.db.clone());

    let entity_count = entity_repo.count().await.unwrap_or(0);
    let fact_count = fact_repo.count().await.unwrap_or(0);

    // For now, return placeholder values for gene/cancer counts
    // These would require additional repository methods
    Ok(Json(ApiKgStats {
        entity_count,
        fact_count,
        gene_count: 0,
        cancer_count: 0,
    }))
}

pub async fn kg_page(
    State(state): State<SharedState>,
    Query(filter): Query<KgFilter>,
) -> Html<String> {
    let gene = filter.gene.as_deref().unwrap_or("KRAS");

    // Use KgFactRepository to get facts
    let fact_repo = KgFactRepository::new(state.db.clone());
    let facts = fact_repo.list(0, 100).await.unwrap_or_default();
    
    // Convert to display format
    let display_facts: Vec<(String, String, String, f64, String)> = facts.iter().map(|f| {
        (f.subject_name.clone(), f.predicate.clone(), f.object_name.clone(), 
         f.confidence.map(|c| c as f64).unwrap_or(0.5), "unknown".to_string())
    }).collect();

    let fact_rows: String = if display_facts.is_empty() {
        format!(r#"<tr><td colspan="5" class="text-center text-muted py-4">
            No KG facts found for <strong style="color:var(--text-main);">{}</strong>. Run the ingestion pipeline first.
        </td></tr>"#, gene)
    } else {
        display_facts.iter().map(|(subj, pred, obj, conf, src)| {
            let conf_class = if *conf > 0.7 { "success" }
                             else if *conf > 0.4 { "warning" }
                             else { "danger" };
            let pred_badge = format!(r#"<span class="badge badge-outline">{}</span>"#, pred);
            format!(r#"<tr>
                <td style="font-weight:600; color:var(--text-main);">{}</td>
                <td>{}</td>
                <td style="font-weight:600; color:var(--text-main);">{}</td>
                <td><span class="badge badge-{}">{:.3}</span></td>
                <td class="text-muted">{}</td>
            </tr>"#, subj, pred_badge, obj, conf_class, conf, src)
        }).collect()
    };

    Html(format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Knowledge Graph — Ferrumyx</title>
    <link rel="stylesheet" href="/static/css/main.css">
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M16 11c1.66 0 2.99-1.34 2.99-3S17.66 5 16 5c-1.66 0-3 1.34-3 3v.15l-3.32 1.62A2.97 2.97 0 0 0 8 9c-1.66 0-3 1.34-3 3s1.34 3 3 3c.6 0 1.15-.18 1.61-.48l3.36 1.64c-.01.12-.04.24-.04.37 0 1.66 1.34 3 3 3s3-1.34 3-3-1.34-3-3-3c-.62 0-1.18.19-1.64.5l-3.32-1.62C10.96 12.15 11 12.04 11 11.91V11.9z"/></svg>
                Knowledge Graph
            </h1>
            <p class="text-muted">Browse multi-modal KG facts, evidence confidence, and provenance</p>
        </div>
    </div>

    <form class="d-flex gap-3 mb-4 align-center" method="GET" action="/kg">
        <input type="text" name="gene" class="form-control" style="max-width:300px"
               placeholder="Search entities (e.g. KRAS)..." value="{}">
        <button type="submit" class="btn btn-primary">Locate Node</button>
    </form>

    <div class="card">
        <div class="card-header">
            <div>Edges connected to <span class="text-gradient" style="font-weight:700">{}</span></div>
            <span class="badge badge-outline">{} connections</span>
        </div>
        <div class="table-container">
            <table class="table">
                <thead>
                    <tr>
                        <th>Subject Entity</th>
                        <th>Predicate Relation</th>
                        <th>Object Entity</th>
                        <th>Trust Score</th>
                        <th>Provenance</th>
                    </tr>
                </thead>
                <tbody>
                    {}
                </tbody>
            </table>
        </div>
    </div>
</main>
<script src="/static/js/main.js"></script>
</body>
</html>"#, NAV_HTML, gene, gene, display_facts.len(), fact_rows))
}
