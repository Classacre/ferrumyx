//! Target rankings page with score breakdown.

use axum::{
    extract::{State, Query, Path},
    response::{Html, IntoResponse},
    Json,
};
use serde::{Deserialize, Serialize};
use crate::state::SharedState;
use crate::handlers::dashboard::nav_html;
use ferrumyx_common::error::ApiError;

#[derive(Deserialize, Default)]
pub struct TargetFilter {
    pub cancer: Option<String>,
    pub gene: Option<String>,
    pub tier: Option<String>,
    pub page: Option<i64>,
}

// === API Types ===

#[derive(Debug, Serialize, sqlx::FromRow)]
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

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ScoreBreakdown {
    pub composite: f64,
    pub literature: Option<f64>,
    pub crispr: Option<f64>,
    pub mutation: Option<f64>,
    pub confidence_adj: Option<f64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct KgFactBrief {
    pub predicate: String,
    pub object: String,
    pub confidence: f64,
    pub source: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LiteratureHit {
    pub pmid: Option<String>,
    pub title: Option<String>,
    pub snippet: String,
}

// === API Endpoints ===

/// GET /api/targets - List all scored targets
pub async fn api_targets(
    State(state): State<SharedState>,
    Query(filter): Query<TargetFilter>,
) -> Result<impl IntoResponse, ApiError> {
    let cancer = filter.cancer.as_deref().unwrap_or("PAAD");
    let limit = filter.page.unwrap_or(100).max(1).min(500) as i32;

    let rows = sqlx::query_as::<_, ApiTarget>(
        r#"
        SELECT 
            eg.symbol as gene,
            ec.oncotree_code as cancer_type,
            ts.composite_score,
            ts.literature_score,
            ts.crispr_score,
            ts.mutation_score,
            ts.confidence_adj,
            ts.shortlist_tier as tier,
            ts.evidence_count
        FROM target_scores ts
        JOIN entities ge ON ts.gene_entity_id = ge.id
        JOIN ent_genes eg ON eg.id = ge.id
        JOIN entities ce ON ts.cancer_entity_id = ce.id
        JOIN ent_cancer_types ec ON ec.id = ce.id
        WHERE ts.is_current = TRUE
          AND ec.oncotree_code = $1
        ORDER BY ts.composite_score DESC
        LIMIT $2
        "#
    )
    .bind(cancer)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows))
}

/// GET /api/targets/:gene - Single gene details
pub async fn api_target_detail(
    State(state): State<SharedState>,
    Path(gene): Path<String>,
    Query(filter): Query<TargetFilter>,
) -> Result<impl IntoResponse, ApiError> {
    let cancer = filter.cancer.as_deref().unwrap_or("PAAD");

    // Get scores
    let scores = sqlx::query_as::<_, ScoreBreakdown>(
        r#"
        SELECT 
            ts.composite_score as composite,
            ts.literature_score,
            ts.crispr_score,
            ts.mutation_score,
            ts.confidence_adj
        FROM target_scores ts
        JOIN entities ge ON ts.gene_entity_id = ge.id
        JOIN ent_genes eg ON eg.id = ge.id
        JOIN entities ce ON ts.cancer_entity_id = ce.id
        JOIN ent_cancer_types ec ON ec.id = ce.id
        WHERE ts.is_current = TRUE
          AND eg.symbol = $1
          AND ec.oncotree_code = $2
        "#
    )
    .bind(&gene)
    .bind(cancer)
    .fetch_optional(&state.db)
    .await?;

    let scores = match scores {
        Some(s) => s,
        None => return Err(ApiError::NotFound(format!("No scores found for gene {}", gene))),
    };

    // Get KG facts
    let kg_facts = sqlx::query_as::<_, KgFactBrief>(
        r#"
        SELECT 
            kf.predicate,
            COALESCE(eg2.symbol, e2.name) as object,
            kf.confidence,
            COALESCE(kf.source_pmid, kf.source_db, 'unknown') as source
        FROM kg_facts kf
        JOIN entities e1 ON kf.subject_id = e1.id
        LEFT JOIN ent_genes eg1 ON eg1.id = e1.id
        JOIN entities e2 ON kf.object_id = e2.id
        LEFT JOIN ent_genes eg2 ON eg2.id = e2.id
        WHERE kf.valid_until IS NULL
          AND eg1.symbol = $1
        ORDER BY kf.confidence DESC
        LIMIT 20
        "#
    )
    .bind(&gene)
    .fetch_all(&state.db)
    .await?;

    // Get literature hits
    let literature = sqlx::query_as::<_, LiteratureHit>(
        r#"
        SELECT 
            p.pmid,
            p.title,
            LEFT(pc.content, 200) as snippet
        FROM paper_chunks pc
        JOIN papers p ON p.id = pc.paper_id
        WHERE LOWER(pc.content) LIKE '%' || LOWER($1) || '%'
        ORDER BY pc.chunk_index
        LIMIT 10
        "#
    )
    .bind(&gene)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(ApiTargetDetail {
        gene: gene.clone(),
        cancer_type: cancer.to_string(),
        scores,
        kg_facts,
        literature,
    }))
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
