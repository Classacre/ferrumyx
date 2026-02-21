//! Dashboard handler — main landing page with system overview.

use axum::{extract::State, response::Html};
use crate::state::SharedState;

pub async fn dashboard(State(state): State<SharedState>) -> Html<String> {
    // Query DB for summary stats
    let paper_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM papers")
        .fetch_one(&state.db).await.unwrap_or(0);

    let chunk_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM paper_chunks")
        .fetch_one(&state.db).await.unwrap_or(0);

    let entity_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM entities")
        .fetch_one(&state.db).await.unwrap_or(0);

    let kg_fact_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kg_facts WHERE valid_until IS NULL"
    ).fetch_one(&state.db).await.unwrap_or(0);

    let top_targets: Vec<(String, String, f64)> = sqlx::query_as(
        "SELECT eg.symbol, ec.oncotree_code, ts.composite_score
         FROM target_scores ts
         JOIN entities ge ON ts.gene_entity_id = ge.id
         JOIN ent_genes eg ON eg.id = ge.id
         JOIN entities ce ON ts.cancer_entity_id = ce.id
         JOIN ent_cancer_types ec ON ec.id = ce.id
         WHERE ts.is_current = TRUE
         ORDER BY ts.composite_score DESC
         LIMIT 10"
    ).fetch_all(&state.db).await.unwrap_or_default();

    Html(render_dashboard(paper_count, chunk_count, entity_count, kg_fact_count, top_targets))
}

fn render_dashboard(
    papers: i64, chunks: i64, entities: i64, facts: i64,
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
            <div class="stat-label">Entities</div>
        </div>
        <div class="stat-card">
            <div class="stat-icon">🕸️</div>
            <div class="stat-value" id="fact-count">{}</div>
            <div class="stat-label">KG Facts (active)</div>
        </div>
    </div>

    <!-- Top targets table -->
    <div class="card mt-4">
        <div class="card-header d-flex justify-content-between align-items-center">
            <h5 class="mb-0">🎯 Top Ranked Targets</h5>
            <a href="/targets" class="btn btn-sm btn-outline-primary">View All</a>
        </div>
        <div class="card-body p-0">
            <table class="table table-dark table-hover mb-0">
                <thead>
                    <tr>
                        <th width="50">#</th>
                        <th>Gene</th>
                        <th>Cancer</th>
                        <th>Composite Score</th>
                        <th width="80">Action</th>
                    </tr>
                </thead>
                <tbody>{}</tbody>
            </table>
        </div>
    </div>

    <!-- Live activity feed -->
    <div class="card mt-4">
        <div class="card-header d-flex justify-content-between align-items-center">
            <h5 class="mb-0">⚡ Live Activity</h5>
            <span class="badge bg-success" id="sse-status">● Connected</span>
        </div>
        <div class="card-body">
            <div id="activity-feed" class="activity-feed">
                <div class="activity-item text-muted">Waiting for events...</div>
            </div>
        </div>
    </div>
</main>
<script src="/static/js/main.js"></script>
<script>
    // Connect to SSE and update activity feed
    const evtSource = new EventSource('/api/events');
    evtSource.onmessage = (e) => {{
        const data = JSON.parse(e.data);
        addActivity(data);
        if (data.type === 'paper_ingested') updateStat('paper-count');
        if (data.type === 'target_scored') updateStat('entity-count');
    }};
    evtSource.onerror = () => {{
        document.getElementById('sse-status').className = 'badge bg-danger';
        document.getElementById('sse-status').textContent = '● Disconnected';
    }};

    function addActivity(data) {{
        const feed = document.getElementById('activity-feed');
        const item = document.createElement('div');
        item.className = 'activity-item';
        item.innerHTML = `<span class="activity-time">${{new Date().toLocaleTimeString()}}</span> ${{formatEvent(data)}}`;
        feed.insertBefore(item, feed.firstChild);
        if (feed.children.length > 50) feed.removeChild(feed.lastChild);
    }}

    function formatEvent(data) {{
        switch(data.type) {{
            case 'paper_ingested': return `📄 Paper ingested: <strong>${{data.title}}</strong> (via ${{data.source}})`;
            case 'target_scored': return `🎯 Target scored: <strong>${{data.gene}}</strong> in ${{data.cancer}} — ${{data.score.toFixed(3)}}`;
            case 'docking_complete': return `⚗️ Docking complete: ${{data.gene}} — Vina: ${{data.vina_score.toFixed(2)}} kcal/mol`;
            case 'pipeline_status': return `🔄 [${{data.stage}}] ${{data.message}}`;
            case 'feedback_metric': return `📊 Metric: ${{data.metric}} = ${{data.value.toFixed(4)}}`;
            case 'notification': return `${{data.level === 'error' ? '🔴' : 'ℹ️'}} ${{data.message}}`;
            default: return JSON.stringify(data);
        }}
    }}
</script>
</body>
</html>"#,
        nav_html(), papers, chunks, entities, facts, targets_html)
}

pub fn nav_html() -> &'static str {
    r#"<nav class="sidebar">
    <div class="sidebar-brand">
        <span class="brand-icon">⚗️</span>
        <span class="brand-name">Ferrumyx</span>
        <span class="brand-version">v0.1</span>
    </div>
    <ul class="sidebar-nav">
        <li class="nav-section">Research</li>
        <li><a href="/" class="nav-link active"><span class="nav-icon">🏠</span> Dashboard</a></li>
        <li><a href="/query" class="nav-link"><span class="nav-icon">🔍</span> Target Query</a></li>
        <li><a href="/targets" class="nav-link"><span class="nav-icon">🎯</span> Target Rankings</a></li>
        <li><a href="/kg" class="nav-link"><span class="nav-icon">🕸️</span> Knowledge Graph</a></li>
        <li><a href="/molecules" class="nav-link"><span class="nav-icon">⚗️</span> Molecules</a></li>
        <li class="nav-section">Operations</li>
        <li><a href="/ingestion" class="nav-link"><span class="nav-icon">📥</span> Ingestion</a></li>
        <li><a href="/metrics" class="nav-link"><span class="nav-icon">📊</span> Self-Improvement</a></li>
        <li><a href="/system" class="nav-link"><span class="nav-icon">⚙️</span> System</a></li>
        <li><a href="/audit" class="nav-link"><span class="nav-icon">🔒</span> Audit Log</a></li>
    </ul>
</nav>"#
}
