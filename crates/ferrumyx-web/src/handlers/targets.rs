//! Target rankings page with score breakdown.

use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::handlers::dashboard::NAV_HTML;
use crate::state::SharedState;
use ferrumyx_common::error::ApiError;
use ferrumyx_db::{
    entities::EntityRepository, kg_facts::KgFactRepository, papers::PaperRepository,
    phase4_signals::Phase4SignalRepository, target_scores::TargetScoreRepository,
};
use ferrumyx_ranker::{
    depmap_provider::{DepMapClientAdapter, DepMapProvider},
    normalise::normalise_ceres,
    ProviderRefreshRequest, TargetQueryEngine,
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
    pub provider_cache: Vec<ProviderCacheRow>,
    pub provider_refresh: Vec<ProviderRefreshRow>,
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

#[derive(Debug, Serialize)]
pub struct ProviderCacheRow {
    pub provider: String,
    pub metric: String,
    pub value: String,
    pub source: String,
    pub fetched_at: String,
    pub cache_status: String,
    pub provider_url: Option<String>,
    pub refresh_hint: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProviderRefreshRow {
    pub provider: String,
    pub finished_at: String,
    pub duration_ms: i64,
    pub attempted: i64,
    pub success: i64,
    pub failed: i64,
    pub skipped: i64,
    pub error_rate: f64,
    pub trigger_reason: String,
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
    let detail = load_target_detail_for_page(
        &state,
        &gene,
        filter
            .cancer
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty()),
    )
    .await
    .ok_or_else(|| ApiError::NotFound(format!("Target {gene} not found")))?;
    Ok(Json(detail))
}

pub async fn targets_page(
    State(state): State<SharedState>,
    Query(filter): Query<TargetFilter>,
) -> Html<String> {
    let cancer_input = filter
        .cancer
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .to_string();
    let cancer = if cancer_input.is_empty() {
        None
    } else {
        Some(cancer_input.as_str())
    };
    let gene_input = filter
        .gene
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .to_string();
    let gene_filter = if gene_input.is_empty() {
        None
    } else {
        Some(gene_input.as_str())
    };
    let page = filter.page.unwrap_or(0).max(0);
    let per_page = 25i64;

    let rows = load_target_rows(&state, cancer, gene_filter, filter.tier.as_deref(), 100_000)
        .await
        .unwrap_or_default();

    let mut cancer_options: Vec<String> = rows
        .iter()
        .filter_map(|r| {
            let c = r.cancer_type.trim();
            if c.is_empty() || c.eq_ignore_ascii_case("UNSPECIFIED") {
                None
            } else {
                Some(c.to_string())
            }
        })
        .collect();
    cancer_options.sort();
    cancer_options.dedup();
    let cancer_options_html: String = cancer_options
        .iter()
        .take(40)
        .map(|c| format!(r#"<option value="{}"></option>"#, c))
        .collect();

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
                <td class="target-actions-cell">
                    <div class="d-flex gap-2 target-row-actions">
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

    let selected_cancer = cancer_input.trim();
    let selected_gene = gene_input.trim();
    let insights_html = if !selected_gene.is_empty() {
        match load_target_detail_for_page(
            &state,
            selected_gene,
            if selected_cancer.is_empty() {
                None
            } else {
                Some(selected_cancer)
            },
        )
        .await
        {
            Some(detail) => {
                let mut relation_counts: HashMap<String, usize> = HashMap::new();
                for f in &detail.kg_facts {
                    *relation_counts.entry(f.predicate.clone()).or_insert(0) += 1;
                }
                let mut relation_summary: Vec<(String, usize)> = relation_counts.into_iter().collect();
                relation_summary.sort_by(|a, b| b.1.cmp(&a.1));
                let relation_count = relation_summary.len();
                let relation_rows: String = if relation_summary.is_empty() {
                    r#"<tr><td colspan="2" class="text-muted">No relation evidence found yet.</td></tr>"#
                        .to_string()
                } else {
                    relation_summary
                        .iter()
                        .take(10)
                        .map(|(pred, count)| {
                            format!(
                                r#"<tr><td><span class="badge badge-outline">{}</span></td><td class="text-muted">{}</td></tr>"#,
                                pred, count
                            )
                        })
                        .collect()
                };

                let literature_rows: String = if detail.literature.is_empty() {
                    r#"<div class="text-muted">No paper snippets available yet for this target.</div>"#
                        .to_string()
                } else {
                    detail
                        .literature
                        .iter()
                        .take(8)
                        .map(|hit| {
                            let title = hit.title.clone().unwrap_or_else(|| "Unknown source".to_string());
                            format!(
                                r#"<div class="card mb-2 p-3">
<div style="font-weight:600;">{}</div>
<div class="text-muted" style="font-size:0.9rem; margin-top:0.35rem;">{}</div>
</div>"#,
                                title, hit.snippet
                            )
                        })
                        .collect()
                };

                let provider_cache_rows: String = if detail.provider_cache.is_empty() {
                    r#"<tr><td colspan="6" class="text-muted">No provider cache rows found yet for this target.</td></tr>"#
                        .to_string()
                } else {
                    detail
                        .provider_cache
                        .iter()
                        .map(|row| {
                            let provider_label = if let Some(url) = &row.provider_url {
                                format!(r#"<a href="{}" target="_blank" rel="noopener noreferrer">{}</a>"#, url, row.provider)
                            } else {
                                row.provider.clone()
                            };
                            let health_hint = row
                                .refresh_hint
                                .as_deref()
                                .unwrap_or("Refresh history unavailable");
                            format!(
                                r#"<tr>
<td>{} <span class="badge badge-outline" title="{}" style="padding:0 0.4rem; line-height:1.2;">i</span></td>
<td>{}</td>
<td>{}</td>
<td class="text-muted">{}</td>
<td class="text-muted">{}</td>
<td><span class="badge badge-outline">{}</span></td>
</tr>"#,
                                provider_label,
                                health_hint,
                                row.metric,
                                row.value,
                                row.source,
                                row.fetched_at,
                                row.cache_status
                            )
                        })
                        .collect()
                };

                let provider_cache_count = detail.provider_cache.len();
                let literature_count = detail.literature.len();
                let provider_refresh_summary = if detail.provider_refresh.is_empty() {
                    "No provider refresh runs recorded yet.".to_string()
                } else {
                    detail
                        .provider_refresh
                        .iter()
                        .map(|row| {
                            let trigger = truncate_label(&row.trigger_reason, 26);
                            format!(
                                "{}: {} (ok={}, fail={}, skip={}, attempted={}, err={:.2}, trigger={})",
                                row.provider,
                                row.finished_at,
                                row.success,
                                row.failed,
                                row.skipped,
                                row.attempted,
                                row.error_rate,
                                trigger
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" | ")
                };

                format!(
                    r#"<div class="card mt-4" id="target-insights">
<div class="card-header"><h5 class="mb-0">Target Insights: {}</h5></div>
<div class="card-body">
  <div class="stats-grid" style="grid-template-columns: repeat(5,minmax(0,1fr)); margin-bottom:1rem;">
    <div class="stat-card"><div class="stat-label">Composite</div><div class="stat-value">{:.4}</div></div>
    <div class="stat-card"><div class="stat-label">Confidence Adj</div><div class="stat-value">{:.4}</div></div>
    <div class="stat-card"><div class="stat-label">Literature</div><div class="stat-value">{}</div></div>
    <div class="stat-card"><div class="stat-label">Mutation</div><div class="stat-value">{}</div></div>
    <div class="stat-card"><div class="stat-label">CRISPR</div><div class="stat-value">{}</div></div>
  </div>
  <details class="insight-disclosure">
    <summary>Relation Categories <span class="badge badge-outline">{}</span></summary>
    <div class="insight-disclosure-body">
      <div class="table-container">
        <table class="table"><thead><tr><th>Predicate</th><th>Count</th></tr></thead><tbody>{}</tbody></table>
      </div>
    </div>
  </details>
  <details class="insight-disclosure">
    <summary>Provider Cache Snapshot <span class="badge badge-outline">{}</span> <span class="badge badge-outline" title="{}" style="padding:0 0.45rem; line-height:1.2;">i</span></summary>
    <div class="insight-disclosure-body">
      <div class="table-container">
        <table class="table provider-cache-table">
          <thead>
            <tr>
              <th>Provider</th>
              <th>Metric</th>
              <th>Value</th>
              <th>Source</th>
              <th>Fetched</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>{}</tbody>
        </table>
      </div>
    </div>
  </details>
  <details class="insight-disclosure">
    <summary>Connected Paper Evidence <span class="badge badge-outline">{}</span></summary>
    <div class="insight-disclosure-body">{}</div>
  </details>
</div>
</div>"#,
                    detail.gene,
                    detail.scores.composite,
                    detail.scores.confidence_adj.unwrap_or(0.0),
                    detail
                        .scores
                        .literature
                        .map(|v| format!("{:.4}", v))
                        .unwrap_or_else(|| "n/a".to_string()),
                    detail
                        .scores
                        .mutation
                        .map(|v| format!("{:.4}", v))
                        .unwrap_or_else(|| "n/a".to_string()),
                    detail
                        .scores
                        .crispr
                        .map(|v| format!("{:.4}", v))
                        .unwrap_or_else(|| "n/a".to_string()),
                    relation_count,
                    relation_rows,
                    provider_cache_count,
                    provider_refresh_summary,
                    provider_cache_rows,
                    literature_count,
                    literature_rows
                )
            }
            None => r#"<div class="card mt-4"><div class="card-body text-muted">No detailed insights found for the selected target.</div></div>"#.to_string(),
        }
    } else {
        String::new()
    };

    let pagination = if total > per_page {
        let pages = (total + per_page - 1) / per_page;
        let btns: String = (0..pages)
            .map(|p| {
                format!(
                    r#"<a href="/targets?cancer={}&gene={}&page={}" class="btn btn-sm {}">{}</a>"#,
                    cancer_input,
                    gene_input,
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
    <style>
        .targets-table {{
            min-width: 0;
            table-layout: auto;
        }}
        .targets-table th,
        .targets-table td {{
            padding: 12px 12px;
        }}
        .targets-table th:last-child,
        .targets-table td:last-child {{
            min-width: 154px;
            white-space: nowrap;
        }}
        .target-row-actions .btn {{
            white-space: nowrap;
        }}
        #target-insights .provider-cache-table {{
            min-width: 720px;
        }}
        #target-insights .provider-refresh-table {{
            min-width: 760px;
        }}
        #target-insights .insight-disclosure {{
            border: 1px solid var(--border-glass);
            border-radius: 12px;
            overflow: hidden;
            background: rgba(15, 24, 37, 0.56);
            margin-bottom: 0.85rem;
        }}
        #target-insights .insight-disclosure summary {{
            cursor: pointer;
            list-style: none;
            padding: 0.75rem 0.95rem;
            font-family: 'Outfit', sans-serif;
            font-weight: 600;
            color: var(--text-main);
            border-bottom: 1px solid transparent;
        }}
        #target-insights .insight-disclosure[open] summary {{
            border-bottom-color: var(--border-glass);
        }}
        #target-insights .insight-disclosure-body {{
            padding: 0.8rem 0.95rem 0.95rem;
        }}
        @media (max-width: 1500px) {{
            #target-insights .dashboard-layout {{
                grid-template-columns: 1fr !important;
            }}
        }}
    </style>
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
                <input name="cancer" list="targets-cancer-options" class="form-control" style="width:190px; padding:0.5rem 0.9rem;" placeholder="Any cancer code (optional)" value="{}" />
                <datalist id="targets-cancer-options">{}</datalist>
                <input name="gene" class="form-control" style="width:170px; padding:0.5rem 0.9rem;" placeholder="Gene filter (optional)" value="{}" />
                <button type="submit" class="btn btn-primary">Filter</button>
                <a href="/targets" class="btn btn-outline">Reset</a>
            </form>
        </div>
    </div>

    <div class="card">
        <div class="card-header">
            <h5 class="mb-0">Priority Target Rankings</h5>
            <span class="badge badge-outline">{} verified</span>
        </div>
        <div class="table-container">
            <table class="table targets-table">
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
    {}
</main>
<script src="/static/js/main.js"></script>
</body>
</html>"#,
        NAV_HTML,
        if selected_cancer.is_empty() {
            "all cancers".to_string()
        } else {
            selected_cancer.to_string()
        },
        cancer_input,
        cancer_options_html,
        gene_input,
        total,
        rows_html,
        pagination,
        insights_html
    ))
}

async fn load_target_detail_for_page(
    state: &SharedState,
    gene: &str,
    cancer: Option<&str>,
) -> Option<ApiTargetDetail> {
    let rows = load_target_rows(state, cancer, Some(gene), None, 50_000)
        .await
        .ok()?;
    let row = rows
        .into_iter()
        .find(|r| r.gene.eq_ignore_ascii_case(gene))?;

    let kg_repo = KgFactRepository::new(state.db.clone());
    let paper_repo = PaperRepository::new(state.db.clone());

    let facts = kg_repo.find_by_subject(row.gene_id).await.ok()?;
    let paper_ids: Vec<uuid::Uuid> = facts
        .iter()
        .take(120)
        .filter(|f| !f.paper_id.is_nil())
        .map(|f| f.paper_id)
        .collect();
    let paper_title_cache: HashMap<uuid::Uuid, String> = paper_repo
        .find_titles_by_ids(&paper_ids)
        .await
        .unwrap_or_default();

    let mut kg_facts = Vec::new();
    let mut literature_by_paper: HashMap<uuid::Uuid, LiteratureHit> = HashMap::new();
    let mut literature_misc = Vec::new();
    for fact in facts.into_iter().take(120) {
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

        let fallback_snippet = format!(
            "{} {} {}",
            fact.subject_name,
            normalize_predicate_label(&fact.predicate),
            fact.object_name
        );
        let snippet = fact
            .evidence
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(fallback_snippet);

        if !fact.paper_id.is_nil() {
            literature_by_paper
                .entry(fact.paper_id)
                .or_insert(LiteratureHit {
                    pmid: None,
                    title: Some(source),
                    snippet,
                });
        } else if !snippet.trim().is_empty() {
            literature_misc.push(LiteratureHit {
                pmid: None,
                title: Some(source),
                snippet,
            });
        }
    }
    let mut literature: Vec<LiteratureHit> = literature_by_paper.into_values().collect();
    literature.sort_by(|a, b| {
        a.title
            .as_deref()
            .unwrap_or("")
            .cmp(b.title.as_deref().unwrap_or(""))
    });
    literature.extend(literature_misc.into_iter().take(12));
    let mut deduped_literature = Vec::new();
    let mut seen_literature = std::collections::HashSet::new();
    for hit in literature {
        let title_key = canonical_literature_key(hit.title.as_deref().unwrap_or(""));
        let key = if title_key.is_empty() {
            canonical_literature_key(&hit.snippet)
        } else {
            title_key
        };
        if key.is_empty() || seen_literature.insert(key) {
            deduped_literature.push(hit);
        }
    }
    let literature = deduped_literature;

    let (provider_cache, provider_refresh) =
        load_provider_cache_data(state, &row.gene, &row.cancer_type).await;

    Some(ApiTargetDetail {
        gene: row.gene,
        cancer_type: row.cancer_type,
        scores: ScoreBreakdown {
            composite: row.composite_score,
            literature: row.literature_score,
            crispr: row.crispr_score,
            mutation: row.mutation_score,
            confidence_adj: Some(row.confidence_adj),
        },
        kg_facts,
        literature,
        provider_cache,
        provider_refresh,
    })
}

async fn load_provider_cache_data(
    state: &SharedState,
    gene: &str,
    cancer: &str,
) -> (Vec<ProviderCacheRow>, Vec<ProviderRefreshRow>) {
    let signal_repo = Phase4SignalRepository::new(state.db.clone());
    let gene_symbol = gene.trim().to_uppercase();
    if gene_symbol.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let provider_cancer = normalize_provider_cancer_code(cancer);
    let mut provider_cache = Vec::new();

    let cbio_row = if let Some(code) = provider_cancer.as_deref() {
        match signal_repo
            .find_cbio_mutation_frequency(&gene_symbol, code)
            .await
            .ok()
            .flatten()
        {
            Some(row) => Some(row),
            None => signal_repo
                .find_cbio_mutation_frequency_any_cancer(&gene_symbol)
                .await
                .ok()
                .flatten(),
        }
    } else {
        signal_repo
            .find_cbio_mutation_frequency_any_cancer(&gene_symbol)
            .await
            .ok()
            .flatten()
    };
    provider_cache.push(match cbio_row {
        Some(row) => provider_cache_row(
            "cBioPortal",
            format!("Mutation Frequency ({})", row.cancer_code),
            format!(
                "{:.4} ({} / {} samples)",
                row.mutation_frequency,
                row.mutated_sample_count.max(0),
                row.profiled_sample_count.max(0)
            ),
            row.source,
            row.fetched_at,
        ),
        None => provider_cache_missing("cBioPortal", "Mutation Frequency"),
    });

    let cosmic_row = if let Some(code) = provider_cancer.as_deref() {
        match signal_repo
            .find_cosmic_mutation_frequency(&gene_symbol, code)
            .await
            .ok()
            .flatten()
        {
            Some(row) => Some(row),
            None => signal_repo
                .find_cosmic_mutation_frequency_any_cancer(&gene_symbol)
                .await
                .ok()
                .flatten(),
        }
    } else {
        signal_repo
            .find_cosmic_mutation_frequency_any_cancer(&gene_symbol)
            .await
            .ok()
            .flatten()
    };
    provider_cache.push(match cosmic_row {
        Some(row) => provider_cache_row(
            "COSMIC",
            format!("Mutation Frequency ({})", row.cancer_code),
            format!(
                "{:.4} ({} / {} samples)",
                row.mutation_frequency,
                row.mutated_sample_count.max(0),
                row.profiled_sample_count.max(0)
            ),
            row.source,
            row.fetched_at,
        ),
        None => provider_cache_missing("COSMIC", "Mutation Frequency"),
    });

    let tcga_row = if let Some(code) = provider_cancer.as_deref() {
        signal_repo
            .find_tcga_survival(&gene_symbol, code)
            .await
            .ok()
            .flatten()
    } else {
        None
    };
    provider_cache.push(match tcga_row {
        Some(row) => provider_cache_row(
            "TCGA",
            format!("Survival Score ({})", row.cancer_code),
            format!("{:.4}", row.survival_score),
            row.source,
            row.fetched_at,
        ),
        None => provider_cache_missing("TCGA", "Survival Score"),
    });

    let gtex_row = signal_repo
        .find_gtex_expression(&gene_symbol)
        .await
        .ok()
        .flatten();
    provider_cache.push(match gtex_row {
        Some(row) => provider_cache_row(
            "GTEx",
            "Expression Specificity".to_string(),
            format!("{:.4}", row.expression_score),
            row.source,
            row.fetched_at,
        ),
        None => provider_cache_missing("GTEx", "Expression Specificity"),
    });

    let chembl_row = signal_repo
        .find_chembl_target(&gene_symbol)
        .await
        .ok()
        .flatten();
    provider_cache.push(match chembl_row {
        Some(row) => provider_cache_row(
            "ChEMBL",
            "Inhibitor Count".to_string(),
            row.inhibitor_count.max(0).to_string(),
            row.source,
            row.fetched_at,
        ),
        None => provider_cache_missing("ChEMBL", "Inhibitor Count"),
    });

    let reactome_row = signal_repo
        .find_reactome_gene(&gene_symbol)
        .await
        .ok()
        .flatten();
    provider_cache.push(match reactome_row {
        Some(row) => provider_cache_row(
            "Reactome",
            "Pathway Count".to_string(),
            row.pathway_count.max(0).to_string(),
            row.source,
            row.fetched_at,
        ),
        None => provider_cache_missing("Reactome", "Pathway Count"),
    });

    let mut provider_refresh = Vec::new();
    for (provider_key, provider_name) in [
        ("cbioportal", "cBioPortal"),
        ("cosmic", "COSMIC"),
        ("gtex", "GTEx"),
        ("tcga", "TCGA"),
        ("chembl", "ChEMBL"),
        ("reactome", "Reactome"),
    ] {
        let row = signal_repo
            .latest_provider_refresh_run(provider_key)
            .await
            .ok()
            .flatten();
        provider_refresh.push(match row {
            Some(r) => ProviderRefreshRow {
                provider: provider_name.to_string(),
                finished_at: format_cache_time(r.finished_at),
                duration_ms: r.duration_ms.max(0),
                attempted: r.attempted.max(0),
                success: r.success.max(0),
                failed: r.failed.max(0),
                skipped: r.skipped.max(0),
                error_rate: r.error_rate.clamp(0.0, 1.0),
                trigger_reason: if r.trigger_reason.trim().is_empty() {
                    "n/a".to_string()
                } else {
                    r.trigger_reason
                },
            },
            None => ProviderRefreshRow {
                provider: provider_name.to_string(),
                finished_at: "n/a".to_string(),
                duration_ms: 0,
                attempted: 0,
                success: 0,
                failed: 0,
                skipped: 0,
                error_rate: 0.0,
                trigger_reason: "no runs recorded".to_string(),
            },
        });
    }

    let refresh_hint_by_provider: HashMap<String, String> = provider_refresh
        .iter()
        .map(|row| {
            (
                row.provider.to_ascii_lowercase(),
                format!(
                    "Last run: {} | success={} failed={} skipped={} attempted={} | error={:.2}",
                    row.finished_at,
                    row.success,
                    row.failed,
                    row.skipped,
                    row.attempted,
                    row.error_rate
                ),
            )
        })
        .collect();

    for row in &mut provider_cache {
        row.provider_url =
            provider_external_url(&row.provider, &gene_symbol, provider_cancer.as_deref());
        row.refresh_hint = refresh_hint_by_provider
            .get(&row.provider.to_ascii_lowercase())
            .cloned();
    }

    let missing_rows = provider_cache
        .iter()
        .filter(|r| r.cache_status.eq_ignore_ascii_case("missing"))
        .count();
    if missing_rows > 0 {
        maybe_spawn_provider_cache_warmup(
            state,
            &gene_symbol,
            provider_cancer.as_deref(),
            "targets_insights_cache_miss",
        );
    }

    (provider_cache, provider_refresh)
}

fn provider_cache_row(
    provider: &str,
    metric: String,
    value: String,
    source: String,
    fetched_at: chrono::DateTime<chrono::Utc>,
) -> ProviderCacheRow {
    ProviderCacheRow {
        provider: provider.to_string(),
        metric,
        value,
        source: if source.trim().is_empty() {
            "unknown".to_string()
        } else {
            source
        },
        fetched_at: format_cache_time(fetched_at),
        cache_status: provider_cache_status(fetched_at),
        provider_url: None,
        refresh_hint: None,
    }
}

fn provider_cache_missing(provider: &str, metric: &str) -> ProviderCacheRow {
    ProviderCacheRow {
        provider: provider.to_string(),
        metric: metric.to_string(),
        value: "n/a".to_string(),
        source: "not_cached".to_string(),
        fetched_at: "n/a".to_string(),
        cache_status: "missing".to_string(),
        provider_url: None,
        refresh_hint: None,
    }
}

fn provider_cache_status(fetched_at: chrono::DateTime<chrono::Utc>) -> String {
    const PROVIDER_TTL_DAYS: i64 = 14;
    let age_days = (chrono::Utc::now() - fetched_at).num_days();
    if age_days <= PROVIDER_TTL_DAYS {
        "fresh".to_string()
    } else {
        "stale".to_string()
    }
}

fn format_cache_time(ts: chrono::DateTime<chrono::Utc>) -> String {
    ts.format("%Y-%m-%d %H:%M UTC").to_string()
}

fn provider_external_url(provider: &str, gene: &str, cancer: Option<&str>) -> Option<String> {
    let g = gene.trim().to_uppercase();
    if g.is_empty() {
        return None;
    }
    let c = cancer
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("ALL");
    let url = match provider.to_ascii_lowercase().as_str() {
        "cbioportal" => format!(
            "https://www.cbioportal.org/results/oncoprint?gene_list={}&cancer_study_list={}",
            g, c
        ),
        "cosmic" => format!("https://cancer.sanger.ac.uk/cosmic/search?q={}", g),
        "gtex" => format!("https://gtexportal.org/home/gene/{}", g),
        "tcga" => format!(
            "https://portal.gdc.cancer.gov/exploration?searchTableTab=genes&geneSymbol={}",
            g
        ),
        "chembl" => format!(
            "https://www.ebi.ac.uk/chembl/g/#search_results/all/query={}",
            g
        ),
        "reactome" => format!("https://reactome.org/content/query?q={}", g),
        _ => return None,
    };
    Some(url)
}

fn provider_warmup_gate() -> &'static Mutex<HashMap<String, Instant>> {
    static GATE: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();
    GATE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn maybe_spawn_provider_cache_warmup(
    state: &SharedState,
    gene: &str,
    cancer_code: Option<&str>,
    trigger_reason: &str,
) {
    let gene_symbol = gene.trim().to_uppercase();
    if gene_symbol.is_empty() {
        return;
    }
    let cancer = cancer_code
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string());
    let gate_key = format!(
        "{}:{}",
        gene_symbol,
        cancer.clone().unwrap_or_else(|| "ALL".to_string())
    );
    let now = Instant::now();
    let mut gate = match provider_warmup_gate().lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    if let Some(last) = gate.get(&gate_key) {
        if now.saturating_duration_since(*last) < Duration::from_secs(10 * 60) {
            return;
        }
    }
    gate.insert(gate_key, now);
    drop(gate);

    let db = state.db.clone();
    let trigger = trigger_reason.to_string();
    tokio::spawn(async move {
        let engine = TargetQueryEngine::new(db);
        let _ = engine
            .refresh_provider_signals(ProviderRefreshRequest {
                genes: vec![gene_symbol],
                cancer_code: cancer,
                max_genes: 1,
                batch_size: 1,
                retries: 1,
                offline_strict: false,
            })
            .await
            .map_err(|e| {
                tracing::debug!("provider cache warmup trigger='{}' failed: {}", trigger, e);
                e
            });
    });
}

fn normalize_provider_cancer_code(cancer_code: &str) -> Option<String> {
    let mut code = cancer_code.trim().to_uppercase();
    if code.is_empty() || code == "UNSPECIFIED" {
        return None;
    }
    if let Some(stripped) = code.strip_prefix("TCGA-") {
        code = stripped.to_string();
    }
    if code == "NSCLC" {
        code = "LUAD".to_string();
    }
    if code.len() < 3 || code.len() > 12 {
        return None;
    }
    if !code
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
        return None;
    }
    Some(code)
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
    let depmap = if std::env::var("FERRUMYX_WEB_TARGETS_LIVE_DEPMAP")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
    {
        DepMapClientAdapter::init().await.ok()
    } else {
        None
    };

    let mut scores = score_repo
        .list(0, limit.min(3_000))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    scores.sort_by(|a, b| {
        b.confidence_adjusted_score
            .partial_cmp(&a.confidence_adjusted_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let gene_ids: Vec<uuid::Uuid> = scores.iter().map(|s| s.gene_id).collect();
    let evidence_gene_ids: Vec<uuid::Uuid> = gene_ids.iter().copied().take(1_200).collect();
    let evidence_by_gene: HashMap<uuid::Uuid, u32> = kg_repo
        .count_by_subject_ids(&evidence_gene_ids, 32)
        .await
        .unwrap_or_default();

    let mut name_cache: HashMap<uuid::Uuid, String> = HashMap::new();
    let mut rows = Vec::new();

    for s in scores {
        let mut gene = if let Some(raw_gene) = extract_json_string_field(&s.components_raw, "gene")
            .or_else(|| extract_json_string_field(&s.components_raw, "gene_symbol"))
        {
            raw_gene
        } else if let Some(cached) = name_cache.get(&s.gene_id) {
            cached.clone()
        } else if let Ok(Some(ent)) = entity_repo.find_by_id(s.gene_id).await {
            name_cache.insert(s.gene_id, ent.name.clone());
            ent.name
        } else {
            s.gene_id.to_string()
        };
        gene = gene.trim().to_uppercase();
        if looks_like_uuid(&gene) {
            continue;
        }

        let cancer_type = extract_json_string_field(&s.components_raw, "cancer_code")
            .or_else(|| cancer_filter.map(str::to_string))
            .unwrap_or_else(|| "UNSPECIFIED".to_string());

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

        let literature_score = extract_json_number_field(&s.components_normed, "literature_score");
        let mutation_score = extract_json_number_field(&s.components_normed, "mutation_score");
        let mut crispr_score = extract_json_number_field(&s.components_normed, "crispr_score");
        if crispr_score.is_none() {
            if let Some(depmap) = &depmap {
                if let Some(ceres) = depmap.get_mean_ceres(&gene, &cancer_type) {
                    crispr_score = Some(normalise_ceres(ceres));
                }
            }
        }
        let evidence_count = evidence_by_gene.get(&s.gene_id).copied().unwrap_or(0);
        let (composite_score, confidence_adj, tier) = recalibrate_target_scores(
            s.composite_score,
            s.confidence_adjusted_score,
            literature_score,
            mutation_score,
            crispr_score,
            evidence_count,
            &s.shortlist_tier,
        );

        rows.push(TargetRow {
            gene_id: s.gene_id,
            gene,
            cancer_type,
            composite_score,
            confidence_adj,
            tier,
            literature_score,
            mutation_score,
            crispr_score,
            evidence_count,
        });
    }

    rows.sort_by(|a, b| {
        b.confidence_adj
            .partial_cmp(&a.confidence_adj)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                b.composite_score
                    .partial_cmp(&a.composite_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| b.evidence_count.cmp(&a.evidence_count))
    });

    Ok(rows)
}

fn extract_json_string_field(raw: &str, key: &str) -> Option<String> {
    if raw.is_empty() || raw.len() > 256_000 {
        return None;
    }
    let marker = format!("\"{}\"", key);
    let idx = raw.find(&marker)?;
    let rest = &raw[idx + marker.len()..];
    let colon = rest.find(':')?;
    let mut value = rest[colon + 1..].trim_start();
    if !value.starts_with('"') {
        return None;
    }
    value = &value[1..];
    let end = value.find('"')?;
    let extracted = value[..end].trim();
    if extracted.is_empty() {
        None
    } else {
        Some(extracted.to_string())
    }
}

fn extract_json_number_field(raw: &str, key: &str) -> Option<f64> {
    if raw.is_empty() || raw.len() > 256_000 {
        return None;
    }
    let marker = format!("\"{}\"", key);
    let idx = raw.find(&marker)?;
    let rest = &raw[idx + marker.len()..];
    let colon = rest.find(':')?;
    let value = rest[colon + 1..].trim_start();
    let end = value
        .find(|c: char| [',', '}', ']', ' '].contains(&c))
        .unwrap_or(value.len());
    value[..end].trim().parse::<f64>().ok()
}

fn looks_like_uuid(value: &str) -> bool {
    let v = value.trim();
    if v.len() != 36 {
        return false;
    }
    uuid::Uuid::parse_str(v).is_ok()
}

fn recalibrate_target_scores(
    raw_composite: f64,
    raw_adjusted: f64,
    literature: Option<f64>,
    mutation: Option<f64>,
    crispr: Option<f64>,
    evidence_count: u32,
    raw_tier: &str,
) -> (f64, f64, String) {
    let composite = raw_composite.clamp(0.0, 0.98);
    let adjusted = raw_adjusted.clamp(0.0, 0.95);
    let saturated = raw_composite >= 0.999 || raw_adjusted >= 0.999;
    if !saturated {
        return (composite, adjusted, normalize_tier(raw_tier, adjusted));
    }

    let lit = literature.unwrap_or(0.0).clamp(0.0, 1.0);
    let mutn = mutation.unwrap_or(0.0).clamp(0.0, 1.0);
    let dep = crispr.unwrap_or(0.5).clamp(0.0, 1.0);
    let base = (0.50 * lit + 0.30 * mutn + 0.20 * dep).clamp(0.0, 1.0);
    let diversity = [lit, mutn, dep].iter().filter(|v| **v >= 0.15).count() as f64;
    let diversity_factor = (0.72 + 0.08 * diversity).clamp(0.72, 1.0);
    let recomputed = ((1.0 - (-1.9 * base).exp()) * diversity_factor).clamp(0.05, 0.98);
    let evidence_factor = ((evidence_count as f64 + 1.0).ln() / (401.0_f64).ln()).clamp(0.35, 1.0);
    let adjusted_recomputed = (recomputed * (0.55 + 0.45 * evidence_factor)).clamp(0.05, 0.95);
    (
        recomputed,
        adjusted_recomputed,
        normalize_tier(raw_tier, adjusted_recomputed),
    )
}

fn normalize_tier(raw_tier: &str, adjusted: f64) -> String {
    if raw_tier.eq_ignore_ascii_case("primary")
        || raw_tier.eq_ignore_ascii_case("secondary")
        || raw_tier.eq_ignore_ascii_case("excluded")
    {
        return raw_tier.to_ascii_lowercase();
    }
    if adjusted >= 0.65 {
        "primary".to_string()
    } else if adjusted >= 0.50 {
        "secondary".to_string()
    } else {
        "excluded".to_string()
    }
}

fn normalize_predicate_label(raw: &str) -> String {
    let cleaned = raw.trim().replace('_', " ");
    if cleaned.is_empty() {
        "is related to".to_string()
    } else {
        cleaned
    }
}

fn canonical_literature_key(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_space = false;
    for ch in raw.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            ' '
        };
        if mapped == ' ' {
            if prev_space {
                continue;
            }
            prev_space = true;
            out.push(' ');
        } else {
            prev_space = false;
            out.push(mapped);
        }
    }
    out.trim().to_string()
}

fn truncate_label(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    let mut out = String::with_capacity(max_chars + 1);
    for (idx, ch) in input.chars().enumerate() {
        if idx >= max_chars.saturating_sub(1) {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}
