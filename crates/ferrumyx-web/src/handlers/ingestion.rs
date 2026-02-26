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
        enable_scihub_fallback: false,
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
<html lang="en" data-bs-theme="dark">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Ferrumyx — Ingestion</title>
    <link rel="stylesheet" href="/static/css/main.css">
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">📥 Ingestion Pipeline</h1>
            <p class="text-muted">Manage literature ingestion from PubMed and Europe PMC</p>
        </div>
    </div>

    {}

    <div class="stats-grid">
        <div class="stat-card"><div class="stat-icon">📄</div>
            <div class="stat-value" id="papers-count">{}</div><div class="stat-label">Total Papers</div></div>
        <div class="stat-card border-success"><div class="stat-icon">✅</div>
            <div class="stat-value text-success">{}</div><div class="stat-label">Parsed</div></div>
        <div class="stat-card border-warning"><div class="stat-icon">⏳</div>
            <div class="stat-value text-warning">{}</div><div class="stat-label">Pending</div></div>
        <div class="stat-card border-danger"><div class="stat-icon">❌</div>
            <div class="stat-value text-danger">{}</div><div class="stat-label">Failed</div></div>
    </div>

    <!-- Pipeline Progress Indicator -->
    <div class="card mt-3" id="pipeline-progress-card" style="display: {};">
        <div class="card-header d-flex justify-content-between align-items-center">
            <h6 class="mb-0">🔄 Pipeline Progress</h6>
            <div>
                <span id="sse-status" class="badge bg-success">● Live</span>
                <span id="pipeline-stage" class="badge bg-primary ms-2">running</span>
            </div>
        </div>
        <div class="card-body">
            <div class="progress mb-2" style="height: 30px;">
                <div id="pipeline-progress" class="progress-bar progress-bar-striped progress-bar-animated bg-success"
                     role="progressbar" style="width: 5%" aria-valuenow="5" aria-valuemin="0" aria-valuemax="100">
                    <span id="progress-text">Starting...</span>
                </div>
            </div>
            <div class="d-flex justify-content-between text-muted small">
                <span id="papers-found">Papers found: 0</span>
                <span id="papers-inserted">Inserted: 0</span>
                <span id="papers-remaining">Remaining: {}</span>
            </div>
            <p id="pipeline-status-text" class="text-muted small mt-2 mb-0">Searching PubMed and Europe PMC...</p>
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
                    document.getElementById('pipeline-status-text').textContent = data.message;
                    document.getElementById('pipeline-stage').textContent = data.stage;
                    
                    if (data.stage === 'searching') {{
                        document.getElementById('pipeline-progress').style.width = '10%';
                        document.getElementById('progress-text').textContent = '10%';
                    }} else if (data.stage === 'complete') {{
                        document.getElementById('pipeline-progress').style.width = '100%';
                        document.getElementById('progress-text').textContent = '100%';
                        document.getElementById('pipeline-stage').className = 'badge bg-success ms-2';
                        document.getElementById('pipeline-stage').textContent = 'complete';
                        setTimeout(() => evtSource.close(), 2000);
                    }}
                }}
                
                if (data.type === 'paper_ingested') {{
                    papersInserted++;
                    document.getElementById('papers-inserted').textContent = 'Inserted: ' + papersInserted;
                    const percent = Math.min(100, Math.round((papersInserted / totalExpected) * 100));
                    document.getElementById('pipeline-progress').style.width = percent + '%';
                    document.getElementById('progress-text').textContent = percent + '%';
                    document.getElementById('papers-remaining').textContent = 'Remaining: ' + Math.max(0, totalExpected - papersInserted);
                }}
            }};
            
            evtSource.onerror = function() {{
                document.getElementById('sse-status').className = 'badge bg-danger';
                document.getElementById('sse-status').textContent = '● Disconnected';
            }};
        </script>
    </div>

    <div class="card mt-4">
        <div class="card-header"><h5 class="mb-0">🚀 Run Ingestion</h5></div>
        <div class="card-body">
            <form method="POST" action="/ingestion/run" class="row g-3">
                <div class="col-md-3">
                    <label class="form-label">Gene</label>
                    <input type="text" name="gene" class="form-control" placeholder="KRAS" value="KRAS" required>
                </div>
                <div class="col-md-2">
                    <label class="form-label">Mutation</label>
                    <input type="text" name="mutation" class="form-control" placeholder="G12D (optional)">
                </div>
                <div class="col-md-3">
                    <label class="form-label">Cancer Type</label>
                    <input type="text" name="cancer" class="form-control" placeholder="pancreatic cancer" value="pancreatic cancer" required>
                </div>
                <div class="col-md-2">
                    <label class="form-label">Max Papers</label>
                    <input type="number" name="max_results" class="form-control" value="100" min="10" max="1000">
                </div>
                <div class="col-md-2">
                    <label class="form-label">Sources</label>
                    <div class="d-flex flex-column gap-1 mt-1">
                        <div class="form-check form-check-sm">
                            <input class="form-check-input" type="checkbox" name="src_pubmed" id="src_pubmed" checked>
                            <label class="form-check-label small" for="src_pubmed">PubMed</label>
                        </div>
                        <div class="form-check form-check-sm">
                            <input class="form-check-input" type="checkbox" name="src_europepmc" id="src_europepmc" checked>
                            <label class="form-check-label small" for="src_europepmc">Europe PMC</label>
                        </div>
                        <div class="form-check form-check-sm">
                            <input class="form-check-input" type="checkbox" name="src_biorxiv" id="src_biorxiv">
                            <label class="form-check-label small" for="src_biorxiv">bioRxiv</label>
                        </div>
                        <div class="form-check form-check-sm">
                            <input class="form-check-input" type="checkbox" name="src_clinicaltrials" id="src_clinicaltrials">
                            <label class="form-check-label small" for="src_clinicaltrials">ClinicalTrials</label>
                        </div>
                        <div class="form-check form-check-sm">
                            <input class="form-check-input" type="checkbox" name="src_crossref" id="src_crossref">
                            <label class="form-check-label small" for="src_crossref">CrossRef</label>
                        </div>
                    </div>
                    <input type="hidden" name="sources" id="sources_hidden" value="pubmed,europepmc">
                </div>
                <div class="col-12">
                    <button type="submit" class="btn btn-success">▶ Start Ingestion</button>
                    <span class="text-muted ms-3 small">Results stream to the Live Activity feed in real-time.</span>
                </div>
            </form>
        </div>
    </div>

    <div class="card mt-4">
        <div class="card-header d-flex justify-content-between">
            <h5 class="mb-0">📋 Recent Ingestion Events</h5>
            <a href="/audit" class="btn btn-sm btn-outline-secondary">Full Audit Log</a>
        </div>
        <div class="card-body p-0">
            <table class="table table-dark table-sm mb-0">
                <thead><tr><th>DOI</th><th>PMID</th><th>Action</th><th>Source</th></tr></thead>
                <tbody>{}</tbody>
            </table>
        </div>
    </div>
</main>
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
