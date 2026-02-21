//! Knowledge graph explorer.

use axum::{extract::{State, Query}, response::Html};
use serde::Deserialize;
use crate::state::SharedState;
use crate::handlers::dashboard::nav_html;

#[derive(Deserialize, Default)]
pub struct KgFilter { pub gene: Option<String> }

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
