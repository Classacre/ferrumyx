//! Dashboard handler — main landing page with system overview.

use axum::{extract::State, response::Html};
use crate::state::SharedState;
use ferrumyx_db::papers::PaperRepository;
use ferrumyx_db::chunks::ChunkRepository;
use ferrumyx_db::entities::EntityRepository;
use ferrumyx_db::kg_facts::KgFactRepository;

/// Navigation HTML template shared across all pages
pub const NAV_HTML: &str = include_str!("../../templates/nav.html");

pub async fn dashboard(State(state): State<SharedState>) -> Html<String> {
    // Query DB for summary stats using repositories
    let paper_count: u64 = PaperRepository::new(state.db.clone())
        .count().await.unwrap_or(0);

    let chunk_count: u64 = ChunkRepository::new(state.db.clone())
        .count().await.unwrap_or(0);

    let entity_count: u64 = EntityRepository::new(state.db.clone())
        .count().await.unwrap_or(0);

    let kg_fact_count: u64 = KgFactRepository::new(state.db.clone())
        .count().await.unwrap_or(0);

    // For now, return empty top targets until we implement target scoring
    let top_targets: Vec<(String, String, f64)> = Vec::new();

    Html(render_dashboard(paper_count, chunk_count, entity_count, kg_fact_count, top_targets))
}

fn render_dashboard(
    papers: u64, chunks: u64, entities: u64, facts: u64,
    top_targets: Vec<(String, String, f64)>,
) -> String {
    let targets_html = if top_targets.is_empty() {
        r#"<tr><td colspan="3" class="text-center text-muted py-4">No targets scored yet. Run ingestion to populate the knowledge graph.</td></tr>"#.to_string()
    } else {
        top_targets.iter().enumerate().map(|(i, (gene, cancer, score))| {
            let pct = (score * 100.0) as u32;
            let bar_class = if *score > 0.7 { "bg-success" } else if *score > 0.5 { "bg-warning" } else { "bg-danger" };
            format!(r#"
            <tr>
                <td><span class="rank-badge">#{}</span></td>
                <td><a href="/targets?gene={}" class="gene-link fw-bold">{}</a></td>
                <td><span class="badge badge-cancer">{}</span></td>
                <td>
                    <div class="d-flex align-items-center gap-2">
                        <div class="progress flex-grow-1" style="height:8px">
                            <div class="progress-bar {}" style="width:{}%"></div>
                        </div>
                        <span class="score-value">{:.3}</span>
                    </div>
                </td>
                <td><a href="/targets?gene={}&cancer={}" class="btn btn-sm btn-outline-primary">View</a></td>
            </tr>"#, i+1, gene, gene, cancer, bar_class, pct, score, gene, cancer)
        }).collect()
    };

    format!(r#"<!DOCTYPE html>
<html lang="en" data-bs-theme="dark">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Ferrumyx — Oncology Drug Discovery Engine</title>
    <link rel="stylesheet" href="/static/css/main.css">
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">🔬 Dashboard</h1>
            <p class="text-muted">System overview — KRAS G12D PDAC domain</p>
        </div>
        <div class="d-flex gap-2">
            <button class="btn btn-outline-secondary btn-sm" onclick="refreshStats()">↻ Refresh</button>
            <a href="/query" class="btn btn-primary btn-sm">+ New Query</a>
        </div>
    </div>

    <!-- Stat cards -->
    <div class="stats-grid">
        <div class="stat-card">
            <div class="stat-icon">📄</div>
            <div class="stat-value" id="paper-count">{}</div>
            <div class="stat-label">Papers Ingested</div>
        </div>
        <div class="stat-card">
            <div class="stat-icon">🧩</div>
            <div class="stat-value" id="chunk-count">{}</div>
            <div class="stat-label">Indexed Chunks</div>
        </div>
        <div class="stat-card">
            <div class="stat-icon">🧬</div>
            <div class="stat-value" id="entity-count">{}</div>
            <div class="stat-label">Entities Extracted</div>
        </div>
        <div class="stat-card">
            <div class="stat-icon">🔗</div>
            <div class="stat-value" id="fact-count">{}</div>
            <div class="stat-label">KG Facts</div>
        </div>
    </div>

    <!-- Top Targets -->
    <div class="card mt-4">
        <div class="card-header d-flex justify-content-between align-items-center">
            <h5 class="mb-0">🎯 Top Target Scores</h5>
            <a href="/targets" class="btn btn-sm btn-outline-secondary">View All</a>
        </div>
        <div class="card-body">
            <table class="table">
                <thead>
                    <tr>
                        <th>Rank</th>
                        <th>Gene</th>
                        <th>Cancer</th>
                        <th>Score</th>
                        <th></th>
                    </tr>
                </thead>
                <tbody>
                    {}
                </tbody>
            </table>
        </div>
    </div>

    <!-- Quick Actions -->
    <div class="row mt-4">
        <div class="col-md-6">
            <div class="card">
                <div class="card-header">
                    <h5 class="mb-0">📥 Ingestion</h5>
                </div>
                <div class="card-body">
                    <p class="text-muted small">Ingest papers from PubMed, bioRxiv, or PDF uploads.</p>
                    <a href="/ingest" class="btn btn-primary">Start Ingestion</a>
                </div>
            </div>
        </div>
        <div class="col-md-6">
            <div class="card">
                <div class="card-header">
                    <h5 class="mb-0">🔍 Query</h5>
                </div>
                <div class="card-body">
                    <p class="text-muted small">Query the knowledge graph with natural language.</p>
                    <a href="/query" class="btn btn-primary">Ask a Question</a>
                </div>
            </div>
        </div>
    </div>
</main>

<script>
function refreshStats() {{
    location.reload();
}}
</script>
</body>
</html>"#, 
    NAV_HTML,
    papers, chunks, entities, facts,
    targets_html
    )
}
