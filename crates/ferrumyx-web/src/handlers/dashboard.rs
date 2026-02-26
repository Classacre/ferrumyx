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
        r#"<tr><td colspan="5" class="text-center text-muted">No targets scored yet. Run ingestion to populate the knowledge graph.</td></tr>"#.to_string()
    } else {
        top_targets.iter().enumerate().map(|(i, (gene, cancer, score))| {
            let pct = (score * 100.0) as u32;
            let bar_class = if *score > 0.7 { "success" } else if *score > 0.5 { "warning" } else { "danger" };
            format!(r#"
            <tr>
                <td><span class="rank-badge">#{}</span></td>
                <td><a href="/targets?gene={}" style="font-weight: 700;">{}</a></td>
                <td><span class="badge badge-outline">{}</span></td>
                <td>
                    <div class="d-flex align-center gap-3">
                        <div class="progress-track" style="width: 140px;">
                            <div class="progress-bar {}" style="width:{}%"></div>
                        </div>
                        <span class="score-value">{:.3}</span>
                    </div>
                </td>
                <td><a href="/targets?gene={}&cancer={}" class="btn btn-outline btn-sm">Insights</a></td>
            </tr>"#, i+1, gene, gene, cancer, bar_class, pct, score, gene, cancer)
        }).collect()
    };

    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Dashboard — Ferrumyx</title>
    <link rel="stylesheet" href="/static/css/main.css">
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M3 13h8V3H3v10zm0 8h8v-6H3v6zm10 0h8V11h-8v10zm0-18v6h8V3h-8z"/></svg>
                Dashboard Analytics
            </h1>
            <p class="text-muted">High-level overview of knowledge extraction and drug target scoring</p>
        </div>
        <div class="d-flex gap-3">
            <button class="btn btn-outline" onclick="location.reload()">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M17.65 6.35A7.958 7.958 0 0012 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08A5.99 5.99 0 0112 18c-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z"/></svg>
                Sync
            </button>
            <a href="/query" class="btn btn-primary">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"/></svg>
                New Search
            </a>
        </div>
    </div>

    <!-- Stat cards -->
    <div class="stats-grid">
        <div class="stat-card card-hover">
            <div class="stat-icon">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M14 2H6c-1.1 0-1.99.9-1.99 2L4 20c0 1.1.89 2 1.99 2H18c1.1 0 2-.9 2-2V8l-6-6zm2 16H8v-2h8v2zm0-4H8v-2h8v2zm-3-5V3.5L18.5 9H13z"/></svg>
            </div>
            <div class="stat-value text-gradient">{}</div>
            <div class="stat-label">Scientific Papers</div>
        </div>
        <div class="stat-card card-hover">
            <div class="stat-icon">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M4 6H2v14c0 1.1.9 2 2 2h14v-2H4V6zm16-4H8c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2zm-1 9H9V9h10v2zm-4 4H9v-2h6v2zm4-8H9V5h10v2z"/></svg>
            </div>
            <div class="stat-value text-gradient">{}</div>
            <div class="stat-label">Vector Chunks</div>
        </div>
        <div class="stat-card card-hover">
            <div class="stat-icon">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"/></svg>
            </div>
            <div class="stat-value text-gradient">{}</div>
            <div class="stat-label">Bio-Entities Extracted</div>
        </div>
        <div class="stat-card card-hover">
            <div class="stat-icon">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M16 11c1.66 0 2.99-1.34 2.99-3S17.66 5 16 5c-1.66 0-3 1.34-3 3v.15l-3.32 1.62A2.97 2.97 0 0 0 8 9c-1.66 0-3 1.34-3 3s1.34 3 3 3c.6 0 1.15-.18 1.61-.48l3.36 1.64c-.01.12-.04.24-.04.37 0 1.66 1.34 3 3 3s3-1.34 3-3-1.34-3-3-3c-.62 0-1.18.19-1.64.5l-3.32-1.62C10.96 12.15 11 12.04 11 11.91V11.9z"/></svg>
            </div>
            <div class="stat-value text-gradient">{}</div>
            <div class="stat-label">KG Relations</div>
        </div>
    </div>

    <div class="grid-2">
        <!-- Top Targets -->
        <div class="card">
            <div class="card-header">
                <div>Top Therapeutic Targets</div>
                <a href="/targets" class="btn btn-outline btn-sm">Full Report</a>
            </div>
            <div class="table-container">
                <table class="table">
                    <thead>
                        <tr>
                            <th>Rank</th>
                            <th>Gene Target</th>
                            <th>Indication</th>
                            <th>Priority Score</th>
                            <th>Action</th>
                        </tr>
                    </thead>
                    <tbody>
                        {}
                    </tbody>
                </table>
            </div>
        </div>

        <!-- Quick Actions -->
        <div class="d-flex flex-column gap-3">
            <div class="card">
                <div class="card-header">
                    Data Source Ingestion Pipeline
                </div>
                <p class="text-muted mb-4">Ingest raw literature from PubMed, bioRxiv, or manual PDF drops into the Knowledge Graph.</p>
                <a href="/ingestion" class="btn btn-outline">
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M5 4v2h14V4H5zm0 10h4v6h6v-6h4l-7-7-7 7z"/></svg>
                    Initialize Pipeline
                </a>
            </div>
            <div class="card">
                <div class="card-header">
                    Molecular Docking Engine
                </div>
                <p class="text-muted mb-4">Run IronClaw inference to isolate target pockets and generate ADMET-verified ligand conformations.</p>
                <a href="/molecules" class="btn btn-outline">
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M11 2v4.07C7.38 6.55 4.55 9.38 4.07 13H2v-2c0-3.86 3.14-7 7-7zm.3 6V2.3A9.975 9.975 0 0 1 20.3 11H16.3c-.45-1.92-2-3.47-3.92-3.92zM15 11v2h5.7c-.42 3.86-3.42 6.86-7.28 7.28V15h-2v5.7C5.56 20.28 2 16.56 2 12V6.3c.42-3.86 3.42-6.86 7.28-7.28v2h2v-2C16.44 2.72 20 6.44 20 11h-5z"/></svg>
                    Run Vina Module
                </a>
            </div>
        </div>
    </div>
</main>
</body>
</html>"#, 
    NAV_HTML,
    papers, chunks, entities, facts,
    targets_html
    )
}
