//! System status, audit log, and LLM backend health.

use axum::{extract::State, response::Html};
use crate::state::SharedState;
use crate::handlers::dashboard::nav_html;

pub async fn system_page(State(state): State<SharedState>) -> Html<String> {
    let llm_calls: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM llm_audit_log")
        .fetch_one(&state.db).await.unwrap_or(0);
    let internal_calls: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM llm_audit_log WHERE data_class != 'PUBLIC'"
    ).fetch_one(&state.db).await.unwrap_or(0);

    let recent_llm: Vec<(String, String, String, i32, i32, String)> = sqlx::query_as(
        "SELECT model, backend, data_class,
                COALESCE(prompt_tokens,0), COALESCE(completion_tokens,0),
                called_at::TEXT
         FROM llm_audit_log
         ORDER BY called_at DESC LIMIT 20"
    ).fetch_all(&state.db).await.unwrap_or_default();

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
                <div class="stat-icon">🏠</div>
                <div class="stat-value text-success">Local Only</div>
                <div class="stat-label">LLM Mode</div>
            </div>
        </div>
    </div>

    <div class="card mb-4">
        <div class="card-header">
            <h6 class="mb-0">🔑 Security Policy</h6>
        </div>
        <div class="card-body">
            <ul class="list-unstyled mb-0">
                <li>✅ <strong>CONFIDENTIAL data:</strong> Hard blocked from all remote LLMs</li>
                <li>✅ <strong>INTERNAL data:</strong> Local Ollama only (remote requires explicit override)</li>
                <li>✅ <strong>PUBLIC data:</strong> Any backend (default: local)</li>
                <li>✅ <strong>Audit log:</strong> All LLM calls logged with data classification + output hash</li>
                <li>✅ <strong>Weight updates:</strong> Require human operator approval</li>
            </ul>
        </div>
    </div>

    <div class="card">
        <div class="card-header d-flex justify-content-between">
            <h6 class="mb-0">🔍 LLM Audit Log (recent 20)</h6>
            <span class="text-muted small">Append-only · never deleted</span>
        </div>
        <div class="card-body p-0">
            <table class="table table-dark table-sm table-hover mb-0">
                <thead>
                    <tr>
                        <th>Model</th><th>Backend</th><th>Data Class</th>
                        <th class="text-end">Prompt Tokens</th>
                        <th class="text-end">Completion Tokens</th>
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
</html>"#, nav_html(), llm_calls, internal_calls, llm_rows))
}
