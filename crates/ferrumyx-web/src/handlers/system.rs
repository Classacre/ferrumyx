//! System status and pipeline health.

use axum::{extract::State, response::Html};

use crate::handlers::dashboard::NAV_HTML;
use crate::state::SharedState;
use ferrumyx_db::{papers::PaperRepository, target_scores::TargetScoreRepository};

pub async fn system_page(State(state): State<SharedState>) -> Html<String> {
    let paper_repo = PaperRepository::new(state.db.clone());
    let score_repo = TargetScoreRepository::new(state.db.clone());

    let stats = state.db.stats().await.unwrap_or_default();
    let pending = paper_repo
        .count_by_parse_status("pending")
        .await
        .unwrap_or(0);
    let parsed = paper_repo
        .count_by_parse_status("parsed")
        .await
        .unwrap_or(0);
    let failed = paper_repo
        .count_by_parse_status("failed")
        .await
        .unwrap_or(0);
    let score_rows = score_repo.count().await.unwrap_or(0);

    let mut recent_papers = paper_repo.list(0, 40).await.unwrap_or_default();
    recent_papers.sort_by(|a, b| b.ingested_at.cmp(&a.ingested_at));
    recent_papers.truncate(12);

    let paper_rows = if recent_papers.is_empty() {
        r#"<tr><td colspan="4" class="text-center text-muted py-4">No paper ingestion events recorded yet.</td></tr>"#.to_string()
    } else {
        recent_papers
            .iter()
            .map(|p| {
                format!(
                    r#"<tr>
                <td title="{}">{}</td>
                <td><span class="badge badge-outline">{}</span></td>
                <td>{}</td>
                <td class="text-muted small">{}</td>
            </tr>"#,
                    html_escape(&p.title),
                    html_escape(&truncate(&p.title, 72)),
                    html_escape(&p.parse_status),
                    html_escape(&p.source),
                    p.ingested_at.format("%Y-%m-%d %H:%M").to_string(),
                )
            })
            .collect()
    };

    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>System - Ferrumyx</title>
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@400;500;600;700&family=Inter:wght@400;500;600&display=swap" rel="stylesheet">
    <link rel="stylesheet" href="/static/css/main.css?v=1.0.3">
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">
                <svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" viewBox="0 0 24 24"><path d="M19.14,12.94c0.04-0.3,0.06-0.61,0.06-0.94c0-0.32-0.02-0.64-0.06-0.94l2.03-1.58c0.18-0.14,0.23-0.41,0.12-0.61 l-1.92-3.32c-0.12-0.22-0.37-0.29-0.59-0.22l-2.39,0.96c-0.5-0.38-1.03-0.7-1.62-0.94L14.4,2.81c-0.04-0.24-0.24-0.41-0.48-0.41 h-3.84c-0.24,0-0.43,0.17-0.47,0.41L9.25,5.35C8.66,5.59,8.12,5.92,7.63,6.29L5.24,5.33c-0.22-0.08-0.47,0-0.59,0.22L2.73,8.87 C2.62,9.08,2.66,9.34,2.86,9.48l2.03,1.58C4.84,11.36,4.8,11.69,4.8,12s0.02,0.64,0.06,0.94l-2.03,1.58 c-0.18,0.14-0.23,0.41-0.12,0.61l1.92,3.32c0.12,0.22,0.37,0.29,0.59,0.22l2.39-0.96c0.5,0.38,1.03,0.7,1.62,0.94l0.36,2.54 c0.05,0.24,0.24,0.41,0.48,0.41h3.84c0.24,0,0.43-0.17,0.47-0.41l0.36-2.54c0.59-0.24,1.13-0.56,1.62-0.94l2.39,0.96 c0.22,0.08,0.47,0,0.59-0.22l1.92-3.32c0.12-0.22,0.07-0.49-0.12-0.61L19.14,12.94z"/></svg>
                System Core Topology
            </h1>
            <p class="text-muted">Autonomous pipeline telemetry from live database tables</p>
        </div>
    </div>

    <div class="grid-3 mb-4">
        <div class="card p-4 text-center">
            <div class="font-outfit" style="font-size:2.4rem; font-weight:800; color:var(--text-main)">{}</div>
            <div class="text-muted text-uppercase mt-2" style="font-size:0.8rem; letter-spacing:1px">Papers</div>
        </div>
        <div class="card p-4 text-center">
            <div class="font-outfit" style="font-size:2.4rem; font-weight:800; color:var(--text-main)">{}</div>
            <div class="text-muted text-uppercase mt-2" style="font-size:0.8rem; letter-spacing:1px">KG Facts</div>
        </div>
        <div class="card p-4 text-center">
            <div class="font-outfit text-gradient" style="font-size:2rem; font-weight:800; line-height:1">Autonomous</div>
            <div class="text-muted text-uppercase mt-2" style="font-size:0.8rem; letter-spacing:1px">Control Mode</div>
        </div>
    </div>

    <div class="grid-2 mb-4">
        <div class="card">
            <div class="card-header">Pipeline Status</div>
            <div class="table-container p-0">
                <table class="table mb-0">
                    <tbody>
                        <tr><td>Pending Parse</td><td class="text-end">{}</td></tr>
                        <tr><td>Parsed</td><td class="text-end">{}</td></tr>
                        <tr><td>Failed Parse</td><td class="text-end">{}</td></tr>
                        <tr><td>Target Scores</td><td class="text-end">{}</td></tr>
                    </tbody>
                </table>
            </div>
        </div>
        <div class="card">
            <div class="card-header">Database Footprint</div>
            <div class="table-container p-0">
                <table class="table mb-0">
                    <tbody>
                        <tr><td>Chunks</td><td class="text-end">{}</td></tr>
                        <tr><td>Entities</td><td class="text-end">{}</td></tr>
                        <tr><td>Entity Mentions</td><td class="text-end">{}</td></tr>
                        <tr><td>Ingestion Audit Rows</td><td class="text-end">{}</td></tr>
                    </tbody>
                </table>
            </div>
        </div>
    </div>

    <div class="card">
        <div class="card-header">Recent Ingested Papers</div>
        <div class="table-container p-0">
            <table class="table mb-0">
                <thead>
                    <tr>
                        <th>Title</th>
                        <th>Parse Status</th>
                        <th>Source</th>
                        <th>Ingested At</th>
                    </tr>
                </thead>
                <tbody>{}</tbody>
            </table>
        </div>
    </div>
</main>
<script src="/static/js/main.js"></script>
</body>
</html>"#,
        NAV_HTML,
        stats.papers,
        stats.kg_facts,
        pending,
        parsed,
        failed,
        score_rows,
        stats.chunks,
        stats.entities,
        stats.entity_mentions,
        stats.ingestion_audit,
        paper_rows
    ))
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out = s.chars().take(max).collect::<String>();
        out.push_str("...");
        out
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
