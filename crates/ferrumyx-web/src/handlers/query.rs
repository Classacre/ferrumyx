//! Scientific query interface — NL query → ranked target output.

use axum::{extract::State, response::Html, Form};
use serde::{Deserialize, Serialize};
use crate::state::SharedState;
use crate::handlers::dashboard::NAV_HTML;
use ferrumyx_db::entities::EntityRepository;
use ferrumyx_db::kg_facts::KgFactRepository;

#[derive(Deserialize)]
pub struct QueryForm {
    pub query_text: String,
    pub cancer_code: Option<String>,
    pub gene: Option<String>,
    pub mutation: Option<String>,
    pub min_structural: Option<f64>,
    pub max_inhibitors: Option<i32>,
    pub min_confidence: Option<f64>,
    pub relationship: Option<String>,
    pub max_results: Option<usize>,
}

#[derive(Serialize)]
pub struct QueryResult {
    pub rank: usize,
    pub gene_symbol: String,
    pub cancer_code: String,
    pub composite_score: f64,
    pub confidence_adj: f64,
    pub shortlist_tier: String,
    pub flags: Vec<String>,
}

pub async fn query_page(State(_state): State<SharedState>) -> Html<String> {
    Html(render_query_page(None))
}

pub async fn query_submit(
    State(state): State<SharedState>,
    Form(form): Form<QueryForm>,
) -> Html<String> {
    // Use repositories to query data
    let entity_repo = EntityRepository::new(state.db.clone());
    let kg_repo = KgFactRepository::new(state.db.clone());
    
    // Get entities and facts for basic scoring
    let entities = entity_repo.list(0, 100).await.unwrap_or_default();
    let facts = kg_repo.list(0, 100).await.unwrap_or_default();
    
    // Build simple results from KG facts
    let gene_filter = form.gene.as_deref().unwrap_or("");
    let max_results = form.max_results.unwrap_or(20);
    
    let results: Vec<QueryResult> = facts
        .iter()
        .filter(|f| gene_filter.is_empty() || f.subject_name.contains(gene_filter))
        .enumerate()
        .take(max_results)
        .map(|(i, f)| QueryResult {
            rank: i + 1,
            gene_symbol: f.subject_name.clone(),
            cancer_code: form.cancer_code.clone().unwrap_or_else(|| "PAAD".to_string()),
            composite_score: f.confidence.map(|c| c as f64).unwrap_or(0.5),
            confidence_adj: f.confidence.map(|c| c as f64).unwrap_or(0.5),
            shortlist_tier: "secondary".to_string(),
            flags: vec![],
        })
        .collect();

    Html(render_query_page(Some((&form.query_text, results))))
}

fn render_query_page(results: Option<(&str, Vec<QueryResult>)>) -> String {
    let results_html = match results {
        None => String::new(),
        Some((query, ref targets)) if targets.is_empty() => format!(
            r#"<div class="card mt-4 p-4 text-center">
                <div class="text-warning mb-2"><svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24"><path d="M1 21h22L12 2 1 21zm12-3h-2v-2h2v2zm0-4h-2v-4h2v4z"/></svg></div>
                <div class="text-muted">No targets found for query: <em class="text-main">{}</em>. Try broadening your filters or running more ingestion.</div>
            </div>"#,
            query
        ),
        Some((query, targets)) => {
            let rows: String = targets.iter().map(|t| {
                let tier_badge = match t.shortlist_tier.as_str() {
                    "primary" => r#"<span class="badge badge-success">Primary</span>"#,
                    "secondary" => r#"<span class="badge badge-warning">Secondary</span>"#,
                    _ => r#"<span class="badge badge-outline">Excluded</span>"#,
                };
                let flags_html = t.flags.iter().map(|f| format!(
                    r#"<span class="badge badge-danger" style="margin-right:4px">{}</span>"#, f
                )).collect::<String>();
                format!(r#"
                <tr>
                    <td><span class="rank-badge">#{}</span></td>
                    <td><a href="/targets?gene={}" style="font-weight:700;">{}</a></td>
                    <td><span class="badge badge-outline">{}</span></td>
                    <td>
                        <div class="d-flex align-center gap-3">
                            <div class="progress-track" style="width: 100px;">
                                <div class="progress-bar brand" style="width:{}%"></div>
                            </div>
                            <span class="score-value">{:.3}</span>
                        </div>
                    </td>
                    <td><span style="color:var(--warning); font-family:'Inter',sans-serif; font-weight:600;">{:.3}</span></td>
                    <td>{}</td>
                    <td>{}</td>
                    <td>
                        <div class="d-flex gap-2">
                            <a href="/targets?gene={}&cancer={}" class="btn btn-outline btn-sm">Insights</a>
                            <a href="/molecules?gene={}" class="btn btn-outline btn-sm">Dock</a>
                        </div>
                    </td>
                </tr>"#,
                t.rank, t.gene_symbol, t.gene_symbol, t.cancer_code,
                (t.composite_score * 100.0) as u32, t.composite_score,
                t.confidence_adj, tier_badge, flags_html,
                t.gene_symbol, t.cancer_code, t.gene_symbol)
            }).collect();

            format!(r#"
            <div class="card mt-4">
                <div class="card-header">
                    <div>Results for: <em class="text-gradient">{}</em></div>
                    <span class="badge badge-outline">{} targets</span>
                </div>
                <div class="table-container">
                    <table class="table">
                        <thead>
                            <tr>
                                <th>Rank</th>
                                <th>Gene Target</th>
                                <th>Indication</th>
                                <th>Priority Score</th>
                                <th>Conf. Adj.</th>
                                <th>Tier</th>
                                <th>Flags</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>{}</tbody>
                    </table>
                </div>
            </div>"#, query, targets.len(), rows)
        }
    };

    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Target Query — Ferrumyx</title>
    <link rel="stylesheet" href="/static/css/main.css">
    <style>
        .query-textarea {{
            min-height: 120px;
            resize: vertical;
        }}
        .form-row {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 1.5rem;
            margin-top: 1.5rem;
        }}
    </style>
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"/></svg>
                Target Query Engine
            </h1>
            <p class="text-muted">Natural language scientific queries with structured semantic filters</p>
        </div>
    </div>

    <div class="card">
        <form method="POST" action="/query" class="d-flex flex-column gap-4">
            <div>
                <label class="form-label" style="font-family:'Outfit',sans-serif; font-size:1.1rem; color:var(--text-main);">Research Question</label>
                <textarea name="query_text" class="form-control query-textarea"
                    placeholder="e.g. What are promising synthetic lethal targets in KRAS G12D pancreatic cancer with structural druggability and low prior inhibitor exploration?"
                    required></textarea>
                <div class="text-muted mt-2" style="font-size:0.85rem;">Ferrumyx will parse entities and map to structured filters below.</div>
            </div>

            <div class="form-row">
                <div>
                    <label class="form-label">Cancer Type (OncoTree)</label>
                    <input type="text" name="cancer_code" class="form-control" placeholder="PAAD" value="PAAD">
                </div>
                <div>
                    <label class="form-label">Gene Symbol</label>
                    <input type="text" name="gene" class="form-control" placeholder="KRAS">
                </div>
                <div>
                    <label class="form-label">Mutation Indicator</label>
                    <input type="text" name="mutation" class="form-control" placeholder="G12D">
                </div>
                <div>
                    <label class="form-label">Target Relationship</label>
                    <select name="relationship" class="form-control">
                        <option value="any">Any Pipeline Edge</option>
                        <option value="synthetic_lethality" selected>Synthetic Lethality</option>
                        <option value="inhibits">Therapeutic Inhibition</option>
                        <option value="activates">Therapeutic Activation</option>
                    </select>
                </div>
            </div>

            <div class="form-row">
                <div>
                    <label class="form-label d-flex justify-between">
                        <span>Min Confidence</span>
                        <span id="conf-val" class="text-gradient" style="font-weight:600;">0.45</span>
                    </label>
                    <input type="range" name="min_confidence" style="width:100%; margin-top:0.75rem;" min="0" max="1" step="0.05" value="0.45"
                        oninput="document.getElementById('conf-val').textContent=this.value">
                </div>
                <div>
                    <label class="form-label">Min Structural Tractability</label>
                    <input type="number" name="min_structural" class="form-control"
                        min="0" max="1" step="0.1" value="0.4" placeholder="0.4">
                </div>
                <div>
                    <label class="form-label d-flex justify-between">
                        <span>Max ChEMBL Inhibitors</span>
                        <span class="badge badge-primary">Novelty</span>
                    </label>
                    <input type="number" name="max_inhibitors" class="form-control"
                        min="0" max="1000" value="20" placeholder="20">
                </div>
                <div class="d-flex align-center" style="margin-top: 1.5rem;">
                    <button type="submit" class="btn btn-primary w-100" style="padding: 0.75rem; font-size: 1.05rem;">
                        Execution Core
                    </button>
                </div>
            </div>
        </form>
    </div>

    {}
</main>
<script src="/static/js/main.js"></script>
</body>
</html>"#, NAV_HTML, results_html)
}
