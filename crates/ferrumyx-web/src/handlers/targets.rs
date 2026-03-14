//! Target rankings page with score breakdown.

use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::handlers::dashboard::NAV_HTML;
use crate::state::SharedState;
use ferrumyx_common::error::ApiError;
use ferrumyx_db::{
    entities::EntityRepository, kg_facts::KgFactRepository, papers::PaperRepository,
    target_scores::TargetScoreRepository,
};
use ferrumyx_ranker::{
    depmap_provider::{DepMapClientAdapter, DepMapProvider},
    normalise::normalise_ceres,
};

#[derive(Deserialize, Default)]
pub struct TargetFilter {
    pub cancer: Option<String>,
    pub gene: Option<String>,
    pub tier: Option<String>,
    pub page: Option<i64>,
}

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

#[derive(Clone)]
struct TargetRow {
    gene_id: uuid::Uuid,
    gene: String,
    cancer_type: String,
    composite_score: f64,
    confidence_adj: f64,
    tier: String,
    literature_score: Option<f64>,
    crispr_score: Option<f64>,
    mutation_score: Option<f64>,
    evidence_count: u32,
}

pub async fn api_targets(
    State(state): State<SharedState>,
    Query(filter): Query<TargetFilter>,
) -> Result<impl IntoResponse, ApiError> {
    let cancer = filter.cancer.as_deref();
    let gene = filter.gene.as_deref();
    let tier = filter.tier.as_deref();
    let limit = filter.page.unwrap_or(100).clamp(1, 500) as usize;

    let rows = load_target_rows(&state, cancer, gene, tier, 50_000).await?;
    let api_rows: Vec<ApiTarget> = rows
        .into_iter()
        .take(limit)
        .map(|r| ApiTarget {
            gene: r.gene,
            cancer_type: r.cancer_type,
            composite_score: r.composite_score,
            literature_score: r.literature_score,
            crispr_score: r.crispr_score,
            mutation_score: r.mutation_score,
            confidence_adj: Some(r.confidence_adj),
            tier: Some(r.tier),
            evidence_count: r.evidence_count as i32,
        })
        .collect();

    Ok(Json(api_rows))
}

pub async fn api_target_detail(
    State(state): State<SharedState>,
    Path(gene): Path<String>,
    Query(filter): Query<TargetFilter>,
) -> Result<impl IntoResponse, ApiError> {
    let rows =
        load_target_rows(&state, filter.cancer.as_deref(), Some(&gene), None, 50_000).await?;

    let row = rows
        .into_iter()
        .find(|r| r.gene.eq_ignore_ascii_case(&gene))
        .ok_or_else(|| ApiError::NotFound(format!("Target {gene} not found")))?;

    let kg_repo = KgFactRepository::new(state.db.clone());
    let paper_repo = PaperRepository::new(state.db.clone());

    let facts = kg_repo
        .find_by_subject(row.gene_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let paper_ids: Vec<uuid::Uuid> = facts
        .iter()
        .take(80)
        .filter(|f| !f.paper_id.is_nil())
        .map(|f| f.paper_id)
        .collect();
    let paper_title_cache: HashMap<uuid::Uuid, String> = paper_repo
        .find_titles_by_ids(&paper_ids)
        .await
        .unwrap_or_default();
    let mut kg_facts = Vec::new();
    let mut literature = Vec::new();

    for fact in facts.into_iter().take(80) {
        let source = if fact.paper_id.is_nil() {
            "unknown".to_string()
        } else {
            paper_title_cache
                .get(&fact.paper_id)
                .cloned()
                .unwrap_or_else(|| fact.paper_id.to_string())
        };

        kg_facts.push(KgFactBrief {
            predicate: fact.predicate.clone(),
            object: fact.object_name.clone(),
            confidence: fact.confidence as f64,
            source: source.clone(),
        });

        if let Some(evidence) = fact.evidence.clone().filter(|s| !s.trim().is_empty()) {
            literature.push(LiteratureHit {
                pmid: None,
                title: Some(source),
                snippet: evidence,
            });
        }
    }

    let scores = ScoreBreakdown {
        composite: row.composite_score,
        literature: row.literature_score,
        crispr: row.crispr_score,
        mutation: row.mutation_score,
        confidence_adj: Some(row.confidence_adj),
    };

    Ok(Json(ApiTargetDetail {
        gene: row.gene,
        cancer_type: row.cancer_type,
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
    let page = filter.page.unwrap_or(0).max(0);
    let per_page = 25i64;

    let rows = load_target_rows(
        &state,
        Some(cancer),
        filter.gene.as_deref(),
        filter.tier.as_deref(),
        100_000,
    )
    .await
    .unwrap_or_default();

    let total = rows.len() as i64;
    let start = (page * per_page) as usize;
    let end = ((page + 1) * per_page) as usize;
    let page_rows = if start < rows.len() {
        &rows[start..rows.len().min(end)]
    } else {
        &[]
    };

    let rows_html: String = if page_rows.is_empty() {
        r#"<tr><td colspan="8" class="text-center text-muted">
            No targets scored yet for this cancer type.<br><br>
            <a href="/ingestion" class="btn btn-primary">Initialize Ingestion Pipeline</a>
        </td></tr>"#
            .to_string()
    } else {
        page_rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let rank = page * per_page + i as i64 + 1;
                let tier_badge = match row.tier.as_str() {
                    "primary" => r#"<span class="badge badge-success">Primary</span>"#,
                    "secondary" => r#"<span class="badge badge-warning">Secondary</span>"#,
                    _ => r#"<span class="badge badge-outline">—</span>"#,
                };
                let bar = (row.composite_score * 100.0) as u32;
                let bar_class = if row.composite_score > 0.7 {
                    "success"
                } else if row.composite_score > 0.5 {
                    "warning"
                } else {
                    "danger"
                };
                format!(
                    r#"
            <tr>
                <td class="text-muted rank-badge">#{}</td>
                <td><a href="/targets?gene={}&cancer={}" style="font-weight:700;">{}</a></td>
                <td><span class="badge badge-outline">{}</span></td>
                <td>
                    <div class="d-flex align-center gap-3">
                        <div class="progress-track" style="width: 100px;">
                            <div class="progress-bar {}" style="width:{}%"></div>
                        </div>
                        <span class="score-value">{:.4}</span>
                    </div>
                </td>
                <td><span style="color:var(--warning); font-family:'Inter',sans-serif; font-weight:600;">{:.4}</span></td>
                <td>{}</td>
                <td class="text-muted text-center">v{}</td>
                <td>
                    <div class="d-flex gap-2">
                        <a href="/targets?gene={}&cancer={}" class="btn btn-outline btn-sm">Insights</a>
                        <a href="/molecules?gene={}" class="btn btn-outline btn-sm">Dock</a>
                    </div>
                </td>
            </tr>"#,
                    rank,
                    row.gene,
                    row.cancer_type,
                    row.gene,
                    row.cancer_type,
                    bar_class,
                    bar,
                    row.composite_score,
                    row.confidence_adj,
                    tier_badge,
                    1,
                    row.gene,
                    row.cancer_type,
                    row.gene
                )
            })
            .collect()
    };

    let pagination = if total > per_page {
        let pages = (total + per_page - 1) / per_page;
        let btns: String = (0..pages)
            .map(|p| {
                format!(
                    r#"<a href="/targets?cancer={}&page={}" class="btn btn-sm {}">{}</a>"#,
                    cancer,
                    p,
                    if p == page {
                        "btn-primary"
                    } else {
                        "btn-outline-secondary"
                    },
                    p + 1
                )
            })
            .collect();
        format!(
            r#"<div class="d-flex justify-content-center gap-1 mt-3">{}</div>"#,
            btns
        )
    } else {
        String::new()
    };

    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Targets — Ferrumyx</title>
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@400;500;600;700&family=Inter:wght@400;500;600&display=swap" rel="stylesheet">
    <link rel="stylesheet" href="/static/css/main.css?v=1.0.3">
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">
                <svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"/></svg>
                Therapeutic Targets
            </h1>
            <p class="text-muted">Computed priority scores for {} candidates</p>
        </div>
        <div class="d-flex gap-3">
            <form method="GET" class="d-flex gap-3 align-center">
                <select name="cancer" class="form-control" style="width:180px; padding: 0.5rem 1rem;">
                    <option value="PAAD" {}>PAAD (Pancreatic)</option>
                    <option value="NSCLC" {}>NSCLC (Lung)</option>
                    <option value="BRCA" {}>BRCA (Breast)</option>
                    <option value="COAD" {}>COAD (Colon)</option>
                </select>
                <button type="submit" class="btn btn-primary">Filter</button>
            </form>
        </div>
    </div>

    <div class="card">
        <div class="card-header">
            <h5 class="mb-0">Priority Target Rankings</h5>
            <span class="badge badge-outline">{} verified</span>
        </div>
        <div class="table-container">
            <table class="table">
                <thead>
                    <tr>
                        <th>Rank</th>
                        <th>Gene Target</th>
                        <th>Indication</th>
                        <th>Priority Score</th>
                        <th>Confidence Adj</th>
                        <th>Tier</th>
                        <th class="text-center">Pipeline</th>
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

async fn load_target_rows(
    state: &SharedState,
    cancer_filter: Option<&str>,
    gene_filter: Option<&str>,
    tier_filter: Option<&str>,
    limit: usize,
) -> Result<Vec<TargetRow>, ApiError> {
    let score_repo = TargetScoreRepository::new(state.db.clone());
    let entity_repo = EntityRepository::new(state.db.clone());
    let kg_repo = KgFactRepository::new(state.db.clone());
    let depmap = DepMapClientAdapter::init().await.ok();

    let mut scores = score_repo
        .list(0, limit.min(10_000))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    scores.sort_by(|a, b| {
        b.confidence_adjusted_score
            .partial_cmp(&a.confidence_adjusted_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let gene_ids: Vec<uuid::Uuid> = scores.iter().map(|s| s.gene_id).collect();
    let evidence_by_gene: HashMap<uuid::Uuid, u32> = kg_repo
        .count_by_subject_ids(&gene_ids, 300)
        .await
        .unwrap_or_default();

    let mut name_cache: HashMap<uuid::Uuid, String> = HashMap::new();
    let mut rows = Vec::new();

    for s in scores {
        let raw_json: serde_json::Value =
            serde_json::from_str(&s.components_raw).unwrap_or_default();
        let norm_json: serde_json::Value =
            serde_json::from_str(&s.components_normed).unwrap_or_default();

        let gene = if let Some(v) = raw_json.get("gene").and_then(|v| v.as_str()) {
            v.to_string()
        } else if let Some(cached) = name_cache.get(&s.gene_id) {
            cached.clone()
        } else if let Ok(Some(ent)) = entity_repo.find_by_id(s.gene_id).await {
            name_cache.insert(s.gene_id, ent.name.clone());
            ent.name
        } else {
            s.gene_id.to_string()
        };

        let cancer_type = raw_json
            .get("cancer_code")
            .and_then(|v| v.as_str())
            .unwrap_or("UNSPECIFIED")
            .to_string();

        if let Some(cf) = cancer_filter {
            if !cancer_type.eq_ignore_ascii_case(cf) {
                continue;
            }
        }

        if let Some(gf) = gene_filter {
            if !gene.to_lowercase().contains(&gf.to_lowercase()) {
                continue;
            }
        }

        if let Some(tf) = tier_filter {
            if !s.shortlist_tier.eq_ignore_ascii_case(tf) {
                continue;
            }
        }

        let mut crispr_score = None;
        if let Some(depmap) = &depmap {
            if let Some(ceres) = depmap.get_mean_ceres(&gene, &cancer_type) {
                crispr_score = Some(normalise_ceres(ceres));
            }
        }

        rows.push(TargetRow {
            gene_id: s.gene_id,
            gene,
            cancer_type,
            composite_score: s.composite_score,
            confidence_adj: s.confidence_adjusted_score,
            tier: s.shortlist_tier,
            literature_score: norm_json.get("literature_score").and_then(|v| v.as_f64()),
            mutation_score: norm_json.get("mutation_score").and_then(|v| v.as_f64()),
            crispr_score,
            evidence_count: evidence_by_gene.get(&s.gene_id).copied().unwrap_or(0),
        });
    }

    Ok(rows)
}
