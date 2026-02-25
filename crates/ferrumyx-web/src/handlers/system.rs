//! System status, audit log, and LLM backend health.

use axum::{extract::State, response::Html};
use crate::state::SharedState;
use crate::handlers::dashboard::NAV_HTML;

pub async fn system_page(State(_state): State<SharedState>) -> Html<String> {
    // Placeholder values - would need llm_audit_log table
    let llm_calls: u64 = 0;
    let internal_calls: u64 = 0;
    let recent_llm: Vec<(String, String, String, i32, i32, String)> = Vec::new();

    let llm_rows: String = if recent_llm.is_empty() {
        r#"<tr><td colspan="6" class="text-center text-muted py-3">No LLM calls logged yet.</td></tr>"#.to_string()
    } else {
        recent_llm.iter().map(|(model, backend, class, prompt_tok, comp_tok, ts)| {
            let class_badge = match class.as_str() {
                "PUBLIC"       => r#"<span class="badge bg-success">PUBLIC</span>"#,
                "INTERNAL"     => r#"<span class="badge bg-warning text-dark">INTERNAL</span>"#,
                "CONFIDENTIAL" => r#"<span class="badge bg-danger">CONFIDENTIAL</span>"#,
                _              => r#"<span class="badge bg-secondary">—</span>"#,
            };
            let backend_badge = if backend == "ollama" {
                r#"<span class="badge bg-info text-dark">🏠 local</span>"#
            } else {
                r#"<span class="badge bg-warning text-dark">☁ remote</span>"#
            };
            format!(r#"<tr>
                <td class="small">{}</td>
                <td>{}</td>
                <td>{}</td>
                <td class="text-end">{}</td>
                <td class="text-end">{}</td>
                <td class="text-muted small">{}</td>
            </tr>"#, model, backend_badge, class_badge, prompt_tok, comp_tok, ts)
        }).collect()
    };

    Html(format!(r#"<!DOCTYPE html>
<html lang="en" data-bs-theme="dark">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Ferrumyx — System</title>
    <link rel="stylesheet" href="/static/css/main.css">
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">⚙️ System</h1>
            <p class="text-muted">LLM backend status, audit log, security overview</p>
        </div>
    </div>

    <div class="row g-3 mb-4">
        <div class="col-md-4">
            <div class="stat-card">
                <div class="stat-icon">🤖</div>
                <div class="stat-value">{}</div>
                <div class="stat-label">Total LLM Calls</div>
            </div>
        </div>
        <div class="col-md-4">
            <div class="stat-card">
                <div class="stat-icon">🔒</div>
                <div class="stat-value text-warning">{}</div>
                <div class="stat-label">Non-PUBLIC Calls (INTERNAL+)</div>
            </div>
        </div>
        <div class="col-md-4">
            <div class="stat-card border-success">
                <div class="stat-icon">🌐</div>
                <div class="stat-value text-success">Multi-Backend</div>
                <div class="stat-label">LLM Mode</div>
            </div>
        </div>
    </div>

    <!-- Backend registry -->
    <div class="card mb-4">
        <div class="card-header"><h6 class="mb-0">🤖 LLM Backends</h6></div>
        <div class="card-body">
            <div class="row g-3">
                <div class="col-md-4">
                    <div class="backend-card">
                        <div class="d-flex justify-content-between align-items-center mb-2">
                            <strong>Ollama</strong>
                            <span class="badge bg-info text-dark">🏠 local</span>
                        </div>
                        <p class="text-muted small mb-0">Local inference via Ollama. Default for development.</p>
                    </div>
                </div>
                <div class="col-md-4">
                    <div class="backend-card">
                        <div class="d-flex justify-content-between align-items-center mb-2">
                            <strong>OpenAI</strong>
                            <span class="badge bg-warning text-dark">☁ remote</span>
                        </div>
                        <p class="text-muted small mb-0">OpenAI API for production workloads.</p>
                    </div>
                </div>
                <div class="col-md-4">
                    <div class="backend-card">
                        <div class="d-flex justify-content-between align-items-center mb-2">
                            <strong>Rust Native</strong>
                            <span class="badge bg-success">🏠 local</span>
                        </div>
                        <p class="text-muted small mb-0">Candle ML for embeddings. No external service.</p>
                    </div>
                </div>
            </div>
        </div>
    </div>

    <!-- Recent LLM calls -->
    <div class="card">
        <div class="card-header"><h6 class="mb-0">📋 Recent LLM Calls</h6></div>
        <div class="card-body p-0">
            <table class="table table-dark table-sm mb-0">
                <thead>
                    <tr>
                        <th>Model</th>
                        <th>Backend</th>
                        <th>Data Class</th>
                        <th class="text-end">Prompt Tok</th>
                        <th class="text-end">Completion Tok</th>
                        <th>Timestamp</th>
                    </tr>
                </thead>
                <tbody>{}</tbody>
            </table>
        </div>
    </div>
</main>
<script src="/static/js/main.js"></script>
</body>
</html>"#, NAV_HTML, llm_calls, internal_calls, llm_rows))
}
