//! Ingestion pipeline monitor and trigger — wired to real pipeline.

use axum::{
    extract::State,
    response::Html,
    Form,
};
use serde::Deserialize;
use std::sync::Arc;

use ferrumyx_ingestion::pipeline::{
    IngestionJob, IngestionSourceSpec, run_ingestion,
};
use ferrumyx_ingestion::repository::IngestionRepository;

use crate::state::{SharedState, AppEvent};
use crate::handlers::dashboard::NAV_HTML;
use ferrumyx_db::papers::PaperRepository;

// ── Form input ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct IngestionForm {
    pub gene: String,
    pub mutation: Option<String>,
    pub cancer: String,
    pub max_results: Option<usize>,
    /// Comma-separated source list: "pubmed,europepmc"
    pub sources: Option<String>,
    /// Optional embedding backend: "openai" | "gemini" | "biomedbert" | "" (skip)
    pub embed_backend: Option<String>,
    pub embed_api_key: Option<String>,
    pub embed_model: Option<String>,
    pub enable_scihub: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn ingestion_page(State(state): State<SharedState>) -> Html<String> {
    let stats = load_stats(&state).await;
    Html(render_page(stats, None))
}

pub async fn ingestion_run(
    State(state): State<SharedState>,
    Form(form): Form<IngestionForm>,
) -> Html<String> {
    // Parse source list from the multi-select
    let sources = parse_sources(form.sources.as_deref().unwrap_or("pubmed"));

    let job = IngestionJob {
        gene:           form.gene.clone(),
        mutation:       form.mutation.clone().filter(|m| !m.is_empty()),
        cancer_type:    form.cancer.clone(),
        max_results:    form.max_results.unwrap_or(100),
        sources,
        pubmed_api_key: None,
        embedding_cfg:  None,
        enable_scihub_fallback: form.enable_scihub.is_some() && form.enable_scihub.as_deref() == Some("on"),
    };

    // Emit SSE start event immediately
    let _ = state.event_tx.send(AppEvent::PipelineStatus {
        stage:   "search".to_string(),
        message: format!("Starting ingestion: {} {} in {}", job.gene, job.mutation.as_deref().unwrap_or(""), job.cancer_type),
        count:   0,
    });

    // Spawn ingestion in background task so we can return immediately
    let event_tx = state.event_tx.clone();
    let db = state.db.clone();
    
    tokio::spawn(async move {
        let repo = Arc::new(IngestionRepository::new(db));
        
        // Update progress before starting
        let _ = event_tx.send(AppEvent::PipelineStatus {
            stage:   "searching".to_string(),
            message: "Searching PubMed and Europe PMC...".to_string(),
            count:   0,
        });

        let result = run_ingestion(job, repo, None).await;

        // Emit SSE completion events
        let _ = event_tx.send(AppEvent::PipelineStatus {
            stage:   "complete".to_string(),
            message: format!(
                "Ingestion complete — {} papers found, {} inserted, {} chunks",
                result.papers_found, result.papers_inserted, result.chunks_inserted
            ),
            count:   result.papers_inserted as u64,
        });

        // Also emit individual PaperIngested events
        for i in 0..result.papers_inserted.min(10) {
            let _ = event_tx.send(AppEvent::PaperIngested {
                paper_id: format!("{}-{}", result.job_id, i),
                title:    format!("Paper #{} ingested from {}", i + 1, result.query),
                source:   "ingestion".to_string(),
            });
        }
    });

    // Return immediately with status that job is running
    let stats = load_stats(&state).await;
    let summary = format!(
        "🔄 Ingestion job started for {} {} in {}. Check the Live Activity feed for real-time progress.",
        form.gene,
        form.mutation.as_deref().unwrap_or(""),
        form.cancer
    );

    Html(render_page_with_progress(stats, &summary, form.max_results.unwrap_or(100) as i64))
}

// ── Stats loader ──────────────────────────────────────────────────────────────

struct PageStats {
    total: i64,
    parsed: i64,
    pending: i64,
    failed: i64,
    recent_audit: Vec<(String, String, String, String)>,
}

async fn load_stats(state: &SharedState) -> PageStats {
    // Use repository to get paper count
    let paper_repo = PaperRepository::new(state.db.clone());
    let total = paper_repo.count().await.unwrap_or(0) as i64;
    
    // For now, return placeholder values for parsed/pending/failed
    // In a full implementation, we'd track these in the database
    PageStats { 
        total, 
        parsed: total,  // All papers are considered parsed for now
        pending: 0, 
        failed: 0, 
        recent_audit: vec![]  // No audit log in LanceDB yet
    }
}

// ── Source parser ─────────────────────────────────────────────────────────────

fn parse_sources(s: &str) -> Vec<IngestionSourceSpec> {
    let v: Vec<IngestionSourceSpec> = s.split(',')
        .filter_map(|x| match x.trim() {
            "pubmed"         => Some(IngestionSourceSpec::PubMed),
            "europepmc"      => Some(IngestionSourceSpec::EuropePmc),
            "biorxiv"        => Some(IngestionSourceSpec::BioRxiv),
            "medrxiv"        => Some(IngestionSourceSpec::MedRxiv),
            "clinicaltrials" => Some(IngestionSourceSpec::ClinicalTrials),
            "crossref"       => Some(IngestionSourceSpec::CrossRef),
            _                => None,
        })
        .collect();
    if v.is_empty() { vec![IngestionSourceSpec::PubMed] } else { v }
}

// ── Renderer ──────────────────────────────────────────────────────────────────

fn render_page(stats: PageStats, result_banner: Option<(&str, &Vec<String>)>) -> String {
    render_page_with_progress(stats, result_banner.map(|(s, _)| s).unwrap_or(""), 0)
}

fn render_page_with_progress(stats: PageStats, summary: &str, total_expected: i64) -> String {
    let banner = if summary.is_empty() {
        String::new()
    } else {
        let is_running = summary.contains("started");
        let alert_class = if is_running { "alert-info" } else { "alert-success" };
        format!(r#"
        <div class="alert {} alert-dismissible mt-3">
            {}
            <button type="button" class="btn-close" data-bs-dismiss="alert"></button>
        </div>"#, alert_class, summary)
    };
    
    let progress_display = if total_expected > 0 { "block" } else { "none" };

    let audit_rows: String = if stats.recent_audit.is_empty() {
        r#"<tr><td colspan="4" class="text-center text-muted py-3">No ingestion events yet.</td></tr>"#.to_string()
    } else {
        stats.recent_audit.iter().map(|(doi, pmid, action, source)| {
            let badge = match action.as_str() {
                "parsed"       => r#"<span class="badge bg-success">parsed</span>"#,
                "discovered"   => r#"<span class="badge bg-info text-dark">discovered</span>"#,
                "failed"       => r#"<span class="badge bg-danger">failed</span>"#,
                "deduplicated" => r#"<span class="badge bg-secondary">dup</span>"#,
                _              => r#"<span class="badge bg-secondary">other</span>"#,
            };
            format!(r#"<tr><td class="font-monospace small">{}</td><td class="font-monospace small">{}</td><td>{}</td><td>{}</td></tr>"#,
                doi, pmid, badge, source)
        }).collect()
    };

    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Ingestion — Ferrumyx</title>
    <link rel="stylesheet" href="/static/css/main.css?v=1.0.1">
    <style>
        .sse-live {{ display: inline-flex; align-items: center; gap: 6px; }}
        .sse-dot {{ width: 8px; height: 8px; border-radius: 50%; background: var(--success); animation: pulse 2s infinite; }}
        @keyframes pulse {{ 0% {{ opacity: 1; }} 50% {{ opacity: 0.4; }} 100% {{ opacity: 1; }} }}
    </style>
</head>
<body>
<div class="app-container">
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">
                <svg width="36" height="36" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M19 9h-4V3H9v6H5l7 7 7-7zM5 18v2h14v-2H5z"/></svg>
                Ingestion Pipeline
            </h1>
            <p class="text-muted">Manage knowledge ingestion from PubMed and Europe PMC</p>
        </div>
    </div>

    {}

    <div class="stats-grid mt-4">
        <div class="stat-card card-hover">
            <div class="stat-icon"><svg width="32" height="32" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M14 2H6c-1.1 0-1.99.9-1.99 2L4 20c0 1.1.89 2 1.99 2H18c1.1 0 2-.9 2-2V8l-6-6zm2 16H8v-2h8v2zm0-4H8v-2h8v2zm-3-5V3.5L18.5 9H13z"/></svg></div>
            <div class="stat-value text-gradient" id="papers-count">{}</div><div class="stat-label">Total Literature</div></div>
        <div class="stat-card card-hover" style="border-bottom: 2px solid var(--success);">
            <div class="stat-icon"><svg width="32" height="32" fill="var(--success)" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/></svg></div>
            <div class="stat-value" style="color:var(--success)">{}</div><div class="stat-label">Parsed Successfully</div></div>
        <div class="stat-card card-hover" style="border-bottom: 2px solid var(--warning);">
            <div class="stat-icon"><svg width="32" height="32" fill="var(--warning)" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M11.99 2C6.47 2 2 6.48 2 12s4.47 10 9.99 10C17.52 22 22 17.52 22 12S17.52 2 11.99 2zM12 20c-4.42 0-8-3.58-8-8s3.58-8 8-8 8 3.58 8 8-3.58 8-8 8zm.5-13H11v6l5.25 3.15.75-1.23-4.5-2.67z"/></svg></div>
            <div class="stat-value" style="color:var(--warning)">{}</div><div class="stat-label">Pending Queues</div></div>
        <div class="stat-card card-hover" style="border-bottom: 2px solid var(--danger);">
            <div class="stat-icon"><svg width="32" height="32" fill="var(--danger)" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/></svg></div>
            <div class="stat-value" style="color:var(--danger)">{}</div><div class="stat-label">Parsing Failures</div></div>
    </div>

    <!-- Pipeline Progress Indicator -->
    <div class="card mt-4" id="pipeline-progress-card" style="display: {};">
        <div class="card-header d-flex justify-between align-center">
            <div>Pipeline Subroutine Active</div>
            <div class="d-flex align-center gap-2">
                <span id="sse-status" class="badge badge-outline text-muted sse-live"><div class="sse-dot"></div> Live</span>
                <span id="pipeline-stage" class="badge badge-primary">Running Operations</span>
            </div>
        </div>
        <div>
            <div class="progress-track" style="height: 12px; margin-bottom: 1rem;">
                <div id="pipeline-progress" class="progress-bar brand" style="width: 5%">
                </div>
            </div>
            <div class="d-flex justify-between text-muted" style="font-size: 0.9rem; font-family: 'Outfit';">
                <span id="papers-found">Vectors Decoded: 0</span>
                <span>|</span>
                <span id="papers-inserted">Inserted: 0 / <span id="progress-text">5%</span></span>
                <span>|</span>
                <span id="papers-remaining">Remaining: {}</span>
            </div>
            <p id="pipeline-status-text" class="text-brand mt-4 mb-0" style="font-family: monospace; font-size: 0.95rem; color:var(--brand-cyan);">> Initiating search sequence for PubMed and Europe PMC interfaces...</p>
        </div>
        <script>
            // Auto-show progress card and start listening to SSE
            document.getElementById('pipeline-progress-card').style.display = 'block';
            
            let papersFound = 0;
            let papersInserted = 0;
            const totalExpected = {};
            
            // Connect to SSE
            const evtSource = new EventSource('/api/events');
            evtSource.onmessage = function(e) {{
                const data = JSON.parse(e.data);
                
                if (data.type === 'pipeline_status') {{
                    document.getElementById('pipeline-status-text').textContent = '> ' + data.message;
                    document.getElementById('pipeline-stage').textContent = data.stage;
                    
                    if (data.stage === 'searching') {{
                        document.getElementById('pipeline-progress').style.width = '10%';
                        document.getElementById('progress-text').textContent = '10%';
                    }} else if (data.stage === 'complete') {{
                        document.getElementById('pipeline-progress').style.width = '100%';
                        document.getElementById('pipeline-progress').classList.add('success');
                        document.getElementById('progress-text').textContent = '100%';
                        document.getElementById('pipeline-stage').className = 'badge badge-success';
                        document.getElementById('pipeline-stage').textContent = 'Job Completed';
                        setTimeout(() => evtSource.close(), 2000);
                    }}
                }}
                
                if (data.type === 'paper_ingested') {{
                    papersInserted++;
                    document.getElementById('papers-inserted').innerHTML = 'Inserted: ' + papersInserted + ' / <span id="progress-text"></span>';
                    const percent = Math.min(100, Math.round((papersInserted / totalExpected) * 100));
                    document.getElementById('pipeline-progress').style.width = percent + '%';
                    document.getElementById('progress-text').textContent = percent + '%';
                    document.getElementById('papers-remaining').textContent = 'Remaining: ' + Math.max(0, totalExpected - papersInserted);
                }}
            }};
            
            evtSource.onerror = function() {{
                document.getElementById('sse-status').className = 'badge badge-outline';
                document.getElementById('sse-status').innerHTML = '<div class="sse-dot" style="background:var(--danger)"></div> Disconnected';
            }};
        </script>
    </div>

    <div class="grid-2 mt-4">
        <div class="card">
            <div class="card-header">Initialize Mining Sequence</div>
            <form method="POST" action="/ingestion/run" class="d-flex flex-column gap-3">
                <div class="d-flex gap-3">
                    <div style="flex:1">
                        <label class="form-label">Target Gene / Domain</label>
                        <input type="text" name="gene" class="form-control" placeholder="KRAS" value="KRAS" required>
                    </div>
                    <div style="flex:1">
                        <label class="form-label">Mutation Filter</label>
                        <input type="text" name="mutation" class="form-control" placeholder="G12D (optional)">
                    </div>
                </div>
                <div class="d-flex gap-3 mt-2">
                    <div style="flex:2">
                        <label class="form-label">Cancer Domain Specification</label>
                        <input type="text" name="cancer" class="form-control" placeholder="pancreatic cancer" value="pancreatic cancer" required>
                    </div>
                    <div style="flex:1">
                        <label class="form-label">Max Articles Limit</label>
                        <input type="number" name="max_results" class="form-control" value="100" min="10" max="1000">
                    </div>
                </div>
                <div class="mt-2">
                    <label class="form-label">Distributed Source Targets</label>
                    <div class="d-flex flex-wrap gap-2 mt-1">
                        <label style="display:flex; align-items:center; gap:0.5rem; background:var(--bg-hover); padding:0.5rem 1rem; border-radius:8px; cursor:pointer border:1px solid var(--border-glass)">
                            <input type="checkbox" name="src_pubmed" id="src_pubmed" checked> <span style="font-weight:500">PubMed</span>
                        </label>
                        <label style="display:flex; align-items:center; gap:0.5rem; background:var(--bg-hover); padding:0.5rem 1rem; border-radius:8px; cursor:pointer border:1px solid var(--border-glass)">
                            <input type="checkbox" name="src_europepmc" id="src_europepmc" checked> <span style="font-weight:500">Europe PMC</span>
                        </label>
                        <label style="display:flex; align-items:center; gap:0.5rem; background:var(--bg-hover); padding:0.5rem 1rem; border-radius:8px; cursor:pointer border:1px solid var(--border-glass)">
                            <input type="checkbox" name="src_biorxiv" id="src_biorxiv"> <span style="font-weight:500">bioRxiv</span>
                        </label>
                    </div>
                    <input type="hidden" name="sources" id="sources_hidden" value="pubmed,europepmc">
                </div>
                <div class="mt-2">
                    <label style="display:flex; align-items:center; gap:0.5rem; cursor:pointer;">
                        <input type="checkbox" name="enable_scihub" id="enable_scihub" checked> <span style="font-weight:500; color: var(--brand-purple);">Enable Sci-Hub Fallback (Full-text PDFs)</span>
                    </label>
                </div>
                <div class="mt-4 pt-4" style="border-top:1px solid var(--border-glass)">
                    <button type="submit" class="btn btn-primary w-100" style="padding: 1rem; font-size: 1.1rem; justify-content:center;">
                        <svg width="20" height="20" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M8 5v14l11-7z"/></svg>
                        Execute Ingestion Routine
                    </button>
                    <div class="text-muted text-center mt-2 small">Results stream to the Live Activity feed mapped above in real-time via Server-Sent Events.</div>
                </div>
            </form>
        </div>

        <div class="card">
            <div class="card-header d-flex justify-between">
                <div>Recent Ingestion Subroutines</div>
                <a href="/audit" class="btn btn-sm btn-outline">Query Logs</a>
            </div>
            <div class="table-container p-0">
                <table class="table mb-0">
                    <thead><tr><th>Document Auth DOI</th><th>PMID</th><th>State</th><th>Node</th></tr></thead>
                    <tbody>{}</tbody>
                </table>
            </div>
        </div>
    </div>
</main>
</div>
<script src="/static/js/main.js"></script>
</body>
</html>"#,
        NAV_HTML,
        banner,
        stats.total, stats.parsed, stats.pending, stats.failed,
        progress_display,
        total_expected,
        total_expected,
        audit_rows)
}
