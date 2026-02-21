//! Ingestion pipeline monitor and control panel.

use axum::{extract::State, response::Html};
use crate::state::SharedState;
use crate::handlers::dashboard::nav_html;

pub async fn ingestion_page(State(state): State<SharedState>) -> Html<String> {
    let total_papers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM papers")
        .fetch_one(&state.db).await.unwrap_or(0);
    let parsed: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM papers WHERE parse_status='parsed'")
        .fetch_one(&state.db).await.unwrap_or(0);
    let pending: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM papers WHERE parse_status='pending'")
        .fetch_one(&state.db).await.unwrap_or(0);
    let failed: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM papers WHERE parse_status='failed'")
        .fetch_one(&state.db).await.unwrap_or(0);

    let recent_audit: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT COALESCE(paper_doi,'—'), COALESCE(paper_pmid,'—'),
                action, source
         FROM ingestion_audit
         ORDER BY occurred_at DESC LIMIT 20"
    ).fetch_all(&state.db).await.unwrap_or_default();

    let audit_rows: String = recent_audit.iter().map(|(doi, pmid, action, source)| {
        let action_badge = match action.as_str() {
            "parsed"       => r#"<span class="badge bg-success">parsed</span>"#,
            "discovered"   => r#"<span class="badge bg-info text-dark">discovered</span>"#,
            "failed"       => r#"<span class="badge bg-danger">failed</span>"#,
            "deduplicated" => r#"<span class="badge bg-secondary">deduplicated</span>"#,
            _              => r#"<span class="badge bg-secondary">other</span>"#,
        };
        format!(r#"<tr><td class="font-monospace small">{}</td><td class="font-monospace small">{}</td><td>{}</td><td>{}</td></tr>"#,
            doi, pmid, action_badge, source)
    }).collect();

    Html(format!(r#"<!DOCTYPE html>
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
            <p class="text-muted">Manage literature ingestion from all sources</p>
        </div>
    </div>

    <!-- Status cards -->
    <div class="stats-grid">
        <div class="stat-card">
            <div class="stat-icon">📄</div>
            <div class="stat-value">{}</div>
            <div class="stat-label">Total Papers</div>
        </div>
        <div class="stat-card border-success">
            <div class="stat-icon">✅</div>
            <div class="stat-value text-success">{}</div>
            <div class="stat-label">Parsed</div>
        </div>
        <div class="stat-card border-warning">
            <div class="stat-icon">⏳</div>
            <div class="stat-value text-warning">{}</div>
            <div class="stat-label">Pending</div>
        </div>
        <div class="stat-card border-danger">
            <div class="stat-icon">❌</div>
            <div class="stat-value text-danger">{}</div>
            <div class="stat-label">Failed</div>
        </div>
    </div>

    <!-- Trigger ingestion form -->
    <div class="card mt-4">
        <div class="card-header"><h5 class="mb-0">🚀 Run Ingestion</h5></div>
        <div class="card-body">
            <form method="POST" action="/api/ingestion/run" class="row g-3">
                <div class="col-md-3">
                    <label class="form-label">Gene</label>
                    <input type="text" name="gene" class="form-control" placeholder="KRAS" value="KRAS">
                </div>
                <div class="col-md-2">
                    <label class="form-label">Mutation</label>
                    <input type="text" name="mutation" class="form-control" placeholder="G12D" value="G12D">
                </div>
                <div class="col-md-3">
                    <label class="form-label">Cancer Type</label>
                    <input type="text" name="cancer" class="form-control" placeholder="pancreatic cancer" value="pancreatic cancer">
                </div>
                <div class="col-md-2">
                    <label class="form-label">Max Papers</label>
                    <input type="number" name="max_results" class="form-control" value="100" min="10" max="1000">
                </div>
                <div class="col-md-2">
                    <label class="form-label">Sources</label>
                    <select name="sources" class="form-select" multiple>
                        <option value="pubmed" selected>PubMed</option>
                        <option value="europepmc" selected>Europe PMC</option>
                        <option value="biorxiv">bioRxiv</option>
                        <option value="semanticscholar">Semantic Scholar</option>
                    </select>
                </div>
                <div class="col-12">
                    <button type="submit" class="btn btn-success">
                        ▶ Start Ingestion
                    </button>
                    <span class="text-muted ms-3 small">
                        Results will appear in the Live Activity feed on the dashboard.
                    </span>
                </div>
            </form>
        </div>
    </div>

    <!-- Recent audit log -->
    <div class="card mt-4">
        <div class="card-header d-flex justify-content-between">
            <h5 class="mb-0">📋 Recent Ingestion Events</h5>
            <a href="/audit" class="btn btn-sm btn-outline-secondary">Full Audit Log</a>
        </div>
        <div class="card-body p-0">
            <table class="table table-dark table-sm mb-0">
                <thead>
                    <tr><th>DOI</th><th>PMID</th><th>Action</th><th>Source</th></tr>
                </thead>
                <tbody>{}</tbody>
            </table>
        </div>
    </div>
</main>
<script src="/static/js/main.js"></script>
</body>
</html>"#,
    nav_html(), total_papers, parsed, pending, failed,
    if audit_rows.is_empty() {
        r#"<tr><td colspan="4" class="text-center text-muted py-3">No ingestion events yet.</td></tr>"#.to_string()
    } else { audit_rows }))
}
