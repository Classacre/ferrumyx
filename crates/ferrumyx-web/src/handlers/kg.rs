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
            No KG facts found for <strong>{}</strong>. Run ingestion first.
        </td></tr>"#, gene)
    } else {
        display_facts.iter().map(|(subj, pred, obj, conf, src)| {
            let conf_class = if *conf > 0.7 { "text-success" }
                             else if *conf > 0.4 { "text-warning" }
                             else { "text-danger" };
            let pred_badge = format!(r#"<span class="badge badge-predicate">{}</span>"#, pred);
            format!(r#"<tr>
                <td class="fw-bold">{}</td>
                <td>{}</td>
                <td class="fw-bold">{}</td>
                <td class="{}">{:.3}</td>
                <td class="small text-muted">{}</td>
            </tr>"#, subj, pred_badge, obj, conf_class, conf, src)
        }).collect()
    };

    Html(format!(r#"<!DOCTYPE html>
<html lang="en" data-bs-theme="dark">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Ferrumyx - Knowledge Graph</title>
    <link rel="stylesheet" href="/static/css/main.css">
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">Web Knowledge Graph Explorer</h1>
            <p class="text-muted">Browse KG facts, confidence scores, and evidence provenance</p>
        </div>
    </div>

    <form class="d-flex gap-2 mb-4" method="GET" action="/kg">
        <input type="text" name="gene" class="form-control" style="max-width:200px"
               placeholder="Gene symbol..." value="{}">
        <button type="submit" class="btn btn-primary">Search</button>
    </form>

    <div class="card">
        <div class="card-header">
            <h6 class="mb-0">Facts involving <span class="text-primary">{}</span>
                <span class="badge bg-secondary ms-2">{} facts</span>
            </h6>
        </div>
        <div class="card-body p-0">
            <table class="table table-dark table-hover mb-0">
                <thead>
                    <tr>
                        <th>Subject</th><th>Predicate</th><th>Object</th>
                        <th>Confidence</th><th>Source</th>
                    </tr>
                </thead>
                <tbody>{}</tbody>
            </table>
        </div>
    </div>
</main>
<script src="/static/js/main.js"></script>
</body>
</html>"#, NAV_HTML, gene, gene, display_facts.len(), fact_rows))
}
