//! Target rankings page with score breakdown.

use axum::{extract::{State, Query}, response::Html};
use serde::Deserialize;
use crate::state::SharedState;
use crate::handlers::dashboard::nav_html;

#[derive(Deserialize, Default)]
pub struct TargetFilter {
    pub cancer: Option<String>,
    pub gene: Option<String>,
    pub tier: Option<String>,
    pub page: Option<i64>,
}

pub async fn targets_page(
    State(state): State<SharedState>,
    Query(filter): Query<TargetFilter>,
) -> Html<String> {
    let cancer = filter.cancer.as_deref().unwrap_or("PAAD");
    let page = filter.page.unwrap_or(0);
    let per_page = 25i64;

    let rows: Vec<(String, String, f64, Option<f64>, Option<String>, i32)> =
        sqlx::query_as(
            "SELECT eg.symbol, ec.oncotree_code,
                    ts.composite_score, ts.confidence_adj,
                    ts.shortlist_tier,
                    ts.score_version
             FROM target_scores ts
             JOIN entities ge ON ts.gene_entity_id = ge.id
             JOIN ent_genes eg ON eg.id = ge.id
             JOIN entities ce ON ts.cancer_entity_id = ce.id
             JOIN ent_cancer_types ec ON ec.id = ce.id
             WHERE ts.is_current = TRUE
               AND ec.oncotree_code = $1
             ORDER BY ts.composite_score DESC
             LIMIT $2 OFFSET $3"
        )
        .bind(cancer)
        .bind(per_page)
        .bind(page * per_page)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM target_scores ts
         JOIN entities ce ON ts.cancer_entity_id = ce.id
         JOIN ent_cancer_types ec ON ec.id = ce.id
         WHERE ts.is_current = TRUE AND ec.oncotree_code = $1"
    ).bind(cancer).fetch_one(&state.db).await.unwrap_or(0);

    let rows_html: String = if rows.is_empty() {
        r#"<tr><td colspan="7" class="text-center text-muted py-5">
            No targets scored yet for this cancer type.<br>
            <a href="/ingestion" class="btn btn-primary mt-2">Start Ingestion</a>
        </td></tr>"#.to_string()
    } else {
        rows.iter().enumerate().map(|(i, (gene, cancer_code, score, conf_adj, tier, version))| {
            let rank = page * per_page + i as i64 + 1;
            let tier_badge = match tier.as_deref() {
                Some("primary")   => r#"<span class="badge bg-success">Primary</span>"#,
                Some("secondary") => r#"<span class="badge bg-warning text-dark">Secondary</span>"#,
                _                 => r#"<span class="badge bg-secondary">—</span>"#,
            };
            let bar = (score * 100.0) as u32;
            format!(r#"
            <tr>
                <td class="text-muted">{}</td>
                <td><a href="/targets?gene={}&cancer={}" class="gene-link fw-bold">{}</a></td>
                <td><span class="badge badge-cancer">{}</span></td>
                <td>
                    <div class="d-flex align-items-center gap-2">
                        <div class="progress flex-grow-1" style="height:6px">
                            <div class="progress-bar bg-primary" style="width:{}%"></div>
                        </div>
                        <code>{:.4}</code>
                    </div>
                </td>
                <td><code class="text-warning">{:.4}</code></td>
                <td>{}</td>
                <td class="text-muted small">v{}</td>
                <td>
                    <div class="btn-group btn-group-sm">
                        <a href="/targets?gene={}&cancer={}" class="btn btn-outline-primary">Detail</a>
                        <a href="/molecules?gene={}" class="btn btn-outline-secondary">Dock</a>
                        <a href="/kg?gene={}" class="btn btn-outline-info">KG</a>
                    </div>
                </td>
            </tr>"#,
            rank, gene, cancer_code, gene, cancer_code,
            bar, score, conf_adj.unwrap_or(0.0), tier_badge, version,
            gene, cancer_code, gene, gene)
        }).collect()
    };

    let pagination = if total > per_page {
        let pages = (total + per_page - 1) / per_page;
        let btns: String = (0..pages).map(|p| format!(
            r#"<a href="/targets?cancer={}&page={}" class="btn btn-sm {}">{}</a>"#,
            cancer, p,
            if p == page { "btn-primary" } else { "btn-outline-secondary" },
            p + 1
        )).collect();
        format!(r#"<div class="d-flex justify-content-center gap-1 mt-3">{}</div>"#, btns)
    } else { String::new() };

    Html(format!(r#"<!DOCTYPE html>
<html lang="en" data-bs-theme="dark">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Ferrumyx — Target Rankings</title>
    <link rel="stylesheet" href="/static/css/main.css">
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">🎯 Target Rankings</h1>
            <p class="text-muted">{} scored targets · Cancer: <strong>{}</strong></p>
        </div>
        <div class="d-flex gap-2">
            <select class="form-select form-select-sm" style="width:140px"
                onchange="window.location='/targets?cancer='+this.value">
                <option value="PAAD" {}>PAAD</option>
                <option value="LUAD" {}>LUAD</option>
                <option value="BRCA" {}>BRCA</option>
            </select>
            <a href="/query" class="btn btn-primary btn-sm">+ New Query</a>
        </div>
    </div>

    <div class="card">
        <div class="card-body p-0">
            <table class="table table-dark table-hover mb-0">
                <thead>
                    <tr>
                        <th width="50">#</th>
                        <th>Gene</th><th>Cancer</th>
                        <th>Composite Score</th>
                        <th>Confidence Adj.</th>
                        <th>Tier</th>
                        <th>Version</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>{}</tbody>
            </table>
        </div>
    </div>
    {}
</main>
<script src="/static/js/main.js"></script>
</body>
</html>"#,
    nav_html(), total, cancer,
    if cancer == "PAAD" { "selected" } else { "" },
    if cancer == "LUAD" { "selected" } else { "" },
    if cancer == "BRCA" { "selected" } else { "" },
    rows_html, pagination))
}
