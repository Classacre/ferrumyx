//! Scientific query interface — NL query → ranked target output.

use axum::{extract::State, response::Html, Form};
use serde::{Deserialize, Serialize};
use crate::state::SharedState;
use crate::handlers::dashboard::nav_html;

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
    // Parse cancer filter
    let cancer_filter = form.cancer_code.as_deref().unwrap_or("PAAD");
    let min_score = form.min_confidence.unwrap_or(0.45);
    let max_results = form.max_results.unwrap_or(20);

    // Query target scores from DB
    let rows: Vec<(String, String, f64, Option<f64>, Option<String>, Option<Vec<String>>)> =
        sqlx::query_as(
            "SELECT eg.symbol, ec.oncotree_code,
                    ts.composite_score, ts.confidence_adj,
                    ts.shortlist_tier, ts.flags
             FROM target_scores ts
             JOIN entities ge ON ts.gene_entity_id = ge.id
             JOIN ent_genes eg ON eg.id = ge.id
             JOIN entities ce ON ts.cancer_entity_id = ce.id
             JOIN ent_cancer_types ec ON ec.id = ce.id
             WHERE ts.is_current = TRUE
               AND ec.oncotree_code = $1
               AND ts.composite_score >= $2
             ORDER BY ts.composite_score DESC
             LIMIT $3"
        )
        .bind(cancer_filter)
        .bind(min_score)
        .bind(max_results as i64)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let results: Vec<QueryResult> = rows.into_iter().enumerate().map(|(i, row)| {
        QueryResult {
            rank: i + 1,
            gene_symbol: row.0,
            cancer_code: row.1,
            composite_score: row.2,
            confidence_adj: row.3.unwrap_or(0.0),
            shortlist_tier: row.4.unwrap_or_else(|| "secondary".to_string()),
            flags: row.5.unwrap_or_default(),
        }
    }).collect();

    Html(render_query_page(Some((&form.query_text, results))))
}

fn render_query_page(results: Option<(&str, Vec<QueryResult>)>) -> String {
    let results_html = match results {
        None => String::new(),
        Some((query, ref targets)) if targets.is_empty() => format!(
            r#"<div class="alert alert-warning mt-4">No targets found for query: <em>{}</em>. Try broadening your filters or running more ingestion.</div>"#,
            query
        ),
        Some((query, targets)) => {
            let rows: String = targets.iter().map(|t| {
                let tier_badge = match t.shortlist_tier.as_str() {
                    "primary" => r#"<span class="badge bg-success">Primary</span>"#,
                    "secondary" => r#"<span class="badge bg-warning text-dark">Secondary</span>"#,
                    _ => r#"<span class="badge bg-secondary">Excluded</span>"#,
                };
                let flags_html = t.flags.iter().map(|f| format!(
                    r#"<span class="badge bg-danger me-1">{}</span>"#, f
                )).collect::<String>();
                format!(r#"
                <tr>
                    <td><span class="rank-badge">#{}</span></td>
                    <td><a href="/targets?gene={}" class="gene-link fw-bold">{}</a></td>
                    <td><span class="badge badge-cancer">{}</span></td>
                    <td><div class="d-flex align-items-center gap-2">
                        <div class="progress flex-grow-1" style="height:6px">
                            <div class="progress-bar bg-primary" style="width:{}%"></div>
                        </div>
                        <code>{:.3}</code>
                    </div></td>
                    <td><code class="text-warning">{:.3}</code></td>
                    <td>{}</td>
                    <td>{}</td>
                    <td>
                        <div class="btn-group btn-group-sm">
                            <a href="/targets?gene={}&cancer={}" class="btn btn-outline-primary">Detail</a>
                            <a href="/molecules?gene={}" class="btn btn-outline-secondary">Dock</a>
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
                    <h5 class="mb-0">Results for: <em class="text-primary">{}</em>
                        <span class="badge bg-secondary ms-2">{} targets</span>
                    </h5>
                </div>
                <div class="card-body p-0">
                    <table class="table table-dark table-hover mb-0">
                        <thead><tr>
                            <th>#</th><th>Gene</th><th>Cancer</th>
                            <th>Score</th><th>Conf. Adj.</th>
                            <th>Tier</th><th>Flags</th><th>Actions</th>
                        </tr></thead>
                        <tbody>{}</tbody>
                    </table>
                </div>
            </div>"#, query, targets.len(), rows)
        }
    };

    format!(r#"<!DOCTYPE html>
<html lang="en" data-bs-theme="dark">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Ferrumyx — Target Query</title>
    <link rel="stylesheet" href="/static/css/main.css">
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">🔍 Target Query</h1>
            <p class="text-muted">Natural language scientific queries with structured filters</p>
        </div>
    </div>

    <div class="card">
        <div class="card-body">
            <form method="POST" action="/query">
                <div class="mb-3">
                    <label class="form-label fw-bold">Research Question</label>
                    <textarea name="query_text" class="form-control form-control-lg query-textarea"
                        rows="3" placeholder="e.g. What are promising synthetic lethal targets in KRAS G12D pancreatic cancer with structural druggability and low prior inhibitor exploration?"
                        required></textarea>
                    <div class="form-text">Ferrumyx will parse entities and map to structured filters below.</div>
                </div>

                <div class="row g-3 mt-1">
                    <div class="col-md-3">
                        <label class="form-label">Cancer Type (OncoTree)</label>
                        <input type="text" name="cancer_code" class="form-control"
                            placeholder="PAAD" value="PAAD">
                    </div>
                    <div class="col-md-2">
                        <label class="form-label">Gene Symbol</label>
                        <input type="text" name="gene" class="form-control" placeholder="KRAS">
                    </div>
                    <div class="col-md-2">
                        <label class="form-label">Mutation</label>
                        <input type="text" name="mutation" class="form-control" placeholder="G12D">
                    </div>
                    <div class="col-md-2">
                        <label class="form-label">Relationship</label>
                        <select name="relationship" class="form-select">
                            <option value="any">Any</option>
                            <option value="synthetic_lethality" selected>Synthetic Lethality</option>
                            <option value="inhibits">Inhibition</option>
                            <option value="activates">Activation</option>
                        </select>
                    </div>
                    <div class="col-md-3">
                        <label class="form-label">Min Confidence</label>
                        <input type="range" name="min_confidence" class="form-range" min="0" max="1" step="0.05" value="0.45"
                            oninput="document.getElementById('conf-val').textContent=this.value">
                        <small>Threshold: <span id="conf-val">0.45</span></small>
                    </div>
                </div>

                <div class="row g-3 mt-1">
                    <div class="col-md-3">
                        <label class="form-label">Min Structural Tractability</label>
                        <input type="number" name="min_structural" class="form-control"
                            min="0" max="1" step="0.1" value="0.4" placeholder="0.4">
                    </div>
                    <div class="col-md-3">
                        <label class="form-label">Max ChEMBL Inhibitors</label>
                        <input type="number" name="max_inhibitors" class="form-control"
                            min="0" max="1000" value="20" placeholder="20">
                        <div class="form-text">Novelty filter</div>
                    </div>
                    <div class="col-md-2">
                        <label class="form-label">Max Results</label>
                        <input type="number" name="max_results" class="form-control"
                            min="1" max="100" value="20">
                    </div>
                    <div class="col-md-4 d-flex align-items-end">
                        <button type="submit" class="btn btn-primary btn-lg w-100">
                            🔬 Run Query
                        </button>
                    </div>
                </div>
            </form>
        </div>
    </div>

    {}
</main>
<script src="/static/js/main.js"></script>
</body>
</html>"#, nav_html(), results_html)
}
