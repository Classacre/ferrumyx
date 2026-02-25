//! Target rankings page with score breakdown.

use axum::{
    extract::{State, Query, Path},
    response::{Html, IntoResponse},
    Json,
};
use serde::{Deserialize, Serialize};
use crate::state::SharedState;
use crate::handlers::dashboard::NAV_HTML;
use ferrumyx_common::error::ApiError;

#[derive(Deserialize, Default)]
pub struct TargetFilter {
    pub cancer: Option<String>,
    pub gene: Option<String>,
    pub tier: Option<String>,
    pub page: Option<i64>,
}

// === API Types ===

#[derive(Debug, Serialize)]
pub struct ApiTarget {
    pub gene: String,
    pub cancer_type: String,
    pub composite_score: f64,
    pub literature_score: Option<f64>,
    pub crispr_score: Option<f64>,
    pub mutation_score: Option<f64>,
    pub confidence_adj: Option<f64>,
    pub tier: Option<String>,
    pub evidence_count: i32,
}

#[derive(Debug, Serialize)]
pub struct ApiTargetDetail {
    pub gene: String,
    pub cancer_type: String,
    pub scores: ScoreBreakdown,
    pub kg_facts: Vec<KgFactBrief>,
    pub literature: Vec<LiteratureHit>,
}

#[derive(Debug, Serialize)]
pub struct ScoreBreakdown {
    pub composite: f64,
    pub literature: Option<f64>,
    pub crispr: Option<f64>,
    pub mutation: Option<f64>,
    pub confidence_adj: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct KgFactBrief {
    pub predicate: String,
    pub object: String,
    pub confidence: f64,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct LiteratureHit {
    pub pmid: Option<String>,
    pub title: Option<String>,
    pub snippet: String,
}

// === API Endpoints ===

/// GET /api/targets - List all scored targets
pub async fn api_targets(
    State(_state): State<SharedState>,
    Query(filter): Query<TargetFilter>,
) -> Result<impl IntoResponse, ApiError> {
    let _cancer = filter.cancer.as_deref().unwrap_or("PAAD");
    let _limit = filter.page.unwrap_or(100).max(1).min(500) as i32;

    // Placeholder - would need target_scores table implementation
    let rows: Vec<ApiTarget> = Vec::new();

    Ok(Json(rows))
}

/// GET /api/targets/:gene - Single gene details
pub async fn api_target_detail(
    State(_state): State<SharedState>,
    Path(gene): Path<String>,
    Query(filter): Query<TargetFilter>,
) -> Result<impl IntoResponse, ApiError> {
    let _cancer = filter.cancer.as_deref().unwrap_or("PAAD");

    // Placeholder - would need target_scores table implementation
    let scores = ScoreBreakdown {
        composite: 0.0,
        literature: None,
        crispr: None,
        mutation: None,
        confidence_adj: None,
    };

    Ok(Json(ApiTargetDetail {
        gene: gene.clone(),
        cancer_type: _cancer.to_string(),
        scores,
        kg_facts: Vec::new(),
        literature: Vec::new(),
    }))
}

pub async fn targets_page(
    State(_state): State<SharedState>,
    Query(filter): Query<TargetFilter>,
) -> Html<String> {
    let cancer = filter.cancer.as_deref().unwrap_or("PAAD");
    let page = filter.page.unwrap_or(0);
    let _per_page = 25i64;

    // Placeholder - would need target_scores table implementation
    let rows: Vec<(String, String, f64, Option<f64>, Option<String>, i32)> = Vec::new();
    let total: i64 = 0;

    let rows_html: String = if rows.is_empty() {
        r#"<tr><td colspan="7" class="text-center text-muted py-5">
            No targets scored yet for this cancer type.<br>
            <a href="/ingestion" class="btn btn-primary mt-2">Start Ingestion</a>
        </td></tr>"#.to_string()
    } else {
        rows.iter().enumerate().map(|(i, (gene, cancer_code, score, conf_adj, tier, version))| {
            let rank = page * _per_page + i as i64 + 1;
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

    let pagination = if total > _per_page {
        let pages = (total + _per_page - 1) / _per_page;
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
            <p class="text-muted">Scored targets for {} — sorted by composite score</p>
        </div>
        <div class="d-flex gap-2">
            <form method="GET" class="d-flex gap-2">
                <select name="cancer" class="form-select form-select-sm" style="width:150px">
                    <option value="PAAD" {}>PAAD (Pancreatic)</option>
                    <option value="NSCLC" {}>NSCLC (Lung)</option>
                    <option value="BRCA" {}>BRCA (Breast)</option>
                    <option value="COAD" {}>COAD (Colon)</option>
                </select>
                <button type="submit" class="btn btn-sm btn-outline-secondary">Filter</button>
            </form>
        </div>
    </div>

    <div class="card">
        <div class="card-header d-flex justify-content-between align-items-center">
            <h6 class="mb-0">Scored Targets</h6>
            <span class="badge bg-secondary">{} total</span>
        </div>
        <div class="card-body p-0">
            <table class="table table-dark table-hover mb-0">
                <thead>
                    <tr>
                        <th>#</th>
                        <th>Gene</th>
                        <th>Cancer</th>
                        <th>Composite Score</th>
                        <th>Confidence Adj</th>
                        <th>Tier</th>
                        <th>Ver</th>
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
    NAV_HTML,
    cancer,
    if cancer == "PAAD" { "selected" } else { "" },
    if cancer == "NSCLC" { "selected" } else { "" },
    if cancer == "BRCA" { "selected" } else { "" },
    if cancer == "COAD" { "selected" } else { "" },
    total,
    rows_html,
    pagination
    ))
}
