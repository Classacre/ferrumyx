//! Knowledge graph explorer.

use axum::{
    extract::{State, Query},
    response::{Html, IntoResponse},
    Json,
};
use serde::{Deserialize, Serialize};
use crate::state::SharedState;
use crate::handlers::dashboard::nav_html;
use ferrumyx_common::error::ApiError;

#[derive(Deserialize, Default)]
pub struct KgFilter { pub gene: Option<String> }

// === API Types ===

#[derive(Debug, Serialize, sqlx::FromRow)]
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
    pub entity_count: i64,
    pub fact_count: i64,
    pub gene_count: i64,
    pub cancer_count: i64,
}

// === API Endpoints ===

/// GET /api/kg - List KG facts
pub async fn api_kg_facts(
    State(state): State<SharedState>,
    Query(filter): Query<KgFilter>,
) -> Result<impl IntoResponse, ApiError> {
    let gene = filter.gene.as_deref().unwrap_or("KRAS");

    let facts = sqlx::query_as::<_, ApiKgFact>(
        r#"
        SELECT 
            COALESCE(eg1.symbol, e1.name) as subject,
            kf.predicate,
            COALESCE(eg2.symbol, e2.name) as object,
            kf.confidence,
            COALESCE(kf.source_pmid, kf.source_db, 'unknown') as source,
            kf.evidence_count
        FROM kg_facts kf
        JOIN entities e1 ON kf.subject_id = e1.id
        LEFT JOIN ent_genes eg1 ON eg1.id = e1.id
        JOIN entities e2 ON kf.object_id = e2.id
        LEFT JOIN ent_genes eg2 ON eg2.id = e2.id
        WHERE kf.valid_until IS NULL
          AND (eg1.symbol = $1 OR eg2.symbol = $1)
        ORDER BY kf.confidence DESC
        LIMIT 100
        "#
    )
    .bind(gene)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(facts))
}

/// GET /api/kg/stats - KG statistics
pub async fn api_kg_stats(
    State(state): State<SharedState>,
) -> Result<impl IntoResponse, ApiError> {
    let entity_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM entities")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let fact_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM kg_facts WHERE valid_until IS NULL")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let gene_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ent_genes")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let cancer_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ent_cancer_types")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    Ok(Json(ApiKgStats {
        entity_count,
        fact_count,
        gene_count,
        cancer_count,
    }))
}

pub async fn kg_page(
    State(state): State<SharedState>,
    Query(filter): Query<KgFilter>,
) -> Html<String> {
    let gene = filter.gene.as_deref().unwrap_or("KRAS");

    let facts: Vec<(String, String, String, f64, String)> = sqlx::query_as(
        "SELECT
            COALESCE(eg1.symbol, e1.name) AS subject,
            kf.predicate,
            COALESCE(eg2.symbol, e2.name) AS object,
            kf.confidence,
            COALESCE(kf.source_pmid, kf.source_db, '—') AS source
         FROM kg_facts kf
         JOIN entities e1 ON kf.subject_id = e1.id
         LEFT JOIN ent_genes eg1 ON eg1.id = e1.id
         JOIN entities e2 ON kf.object_id = e2.id
         LEFT JOIN ent_genes eg2 ON eg2.id = e2.id
         WHERE kf.valid_until IS NULL
           AND (eg1.symbol = $1 OR eg2.symbol = $1)
         ORDER BY kf.confidence DESC
         LIMIT 100"
    ).bind(gene)
     .fetch_all(&state.db)
     .await
     .unwrap_or_default();

    let fact_rows: String = if facts.is_empty() {
        format!(r#"<tr><td colspan="5" class="text-center text-muted py-4">
            No KG facts found for <strong>{}</strong>. Run ingestion first.
        </td></tr>"#, gene)
    } else {
        facts.iter().map(|(subj, pred, obj, conf, src)| {
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
    <title>Ferrumyx — Knowledge Graph</title>
    <link rel="stylesheet" href="/static/css/main.css">
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">🕸️ Knowledge Graph Explorer</h1>
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
</html>"#, nav_html(), gene, gene, facts.len(), fact_rows))
}
