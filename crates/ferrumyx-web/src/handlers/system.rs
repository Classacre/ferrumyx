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
        r#"<tr><td colspan="6" class="text-center text-muted py-4">No model invocations logged in the audit trail.</td></tr>"#.to_string()
    } else {
        recent_llm.iter().map(|(model, backend, class, prompt_tok, comp_tok, ts)| {
            let class_badge = match class.as_str() {
                "PUBLIC"       => r#"<span class="badge badge-success">PUBLIC</span>"#,
                "INTERNAL"     => r#"<span class="badge badge-warning">INTERNAL</span>"#,
                "CONFIDENTIAL" => r#"<span class="badge badge-danger">CONFIDENTIAL</span>"#,
                _              => r#"<span class="badge badge-outline">—</span>"#,
            };
            let backend_badge = if backend == "ollama" {
                r#"<span class="badge badge-outline" style="color:var(--brand-blue); border-color:rgba(59,130,246,0.3)">Local</span>"#
            } else {
                r#"<span class="badge badge-outline" style="color:var(--brand-purple); border-color:rgba(139,92,246,0.3)">Cloud</span>"#
            };
            format!(r#"<tr>
                <td class="font-outfit" style="color:var(--text-main); font-weight:500;">{}</td>
                <td>{}</td>
                <td>{}</td>
                <td class="text-end font-outfit" style="color:var(--text-muted)">{}</td>
                <td class="text-end font-outfit" style="color:var(--text-muted)">{}</td>
                <td class="text-end text-muted small">{}</td>
            </tr>"#, model, backend_badge, class_badge, prompt_tok, comp_tok, ts)
        }).collect()
    };

    Html(format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>System & Audit — Ferrumyx</title>
    <link rel="stylesheet" href="/static/css/main.css">
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M19.14,12.94c0.04-0.3,0.06-0.61,0.06-0.94c0-0.32-0.02-0.64-0.06-0.94l2.03-1.58c0.18-0.14,0.23-0.41,0.12-0.61 l-1.92-3.32c-0.12-0.22-0.37-0.29-0.59-0.22l-2.39,0.96c-0.5-0.38-1.03-0.7-1.62-0.94L14.4,2.81c-0.04-0.24-0.24-0.41-0.48-0.41 h-3.84c-0.24,0-0.43,0.17-0.47,0.41L9.25,5.35C8.66,5.59,8.12,5.92,7.63,6.29L5.24,5.33c-0.22-0.08-0.47,0-0.59,0.22L2.73,8.87 C2.62,9.08,2.66,9.34,2.86,9.48l2.03,1.58C4.84,11.36,4.8,11.69,4.8,12s0.02,0.64,0.06,0.94l-2.03,1.58 c-0.18,0.14-0.23,0.41-0.12,0.61l1.92,3.32c0.12,0.22,0.37,0.29,0.59,0.22l2.39-0.96c0.5,0.38,1.03,0.7,1.62,0.94l0.36,2.54 c0.05,0.24,0.24,0.41,0.48,0.41h3.84c0.24,0,0.43-0.17,0.47-0.41l0.36-2.54c0.59-0.24,1.13-0.56,1.62-0.94l2.39,0.96 c0.22,0.08,0.47,0,0.59-0.22l1.92-3.32c0.12-0.22,0.07-0.49-0.12-0.61L19.14,12.94z M12,15.6c-1.98,0-3.6-1.62-3.6-3.6 s1.62-3.6,3.6-3.6s3.6,1.62,3.6,3.6S13.98,15.6,12,15.6z"/></svg>
                System Core Topology
            </h1>
            <p class="text-muted">LLM backend utilization, distributed model infrastructure, and audit trails</p>
        </div>
    </div>

    <div class="grid-3 mb-4">
        <div class="card p-4 d-flex flex-column align-center justify-center text-center">
            <svg class="mb-3" style="width:36px; height:36px; fill:var(--brand-blue)" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"/></svg>
            <div class="font-outfit" style="font-size:2.5rem; font-weight:800; color:var(--text-main); line-height:1">{}</div>
            <div class="text-muted text-uppercase mt-2" style="font-size:0.8rem; letter-spacing:1px">Total Inference Calls</div>
        </div>
        <div class="card p-4 d-flex flex-column align-center justify-center text-center" style="border-bottom: 3px solid var(--warning)">
            <svg class="mb-3" style="width:36px; height:36px; fill:var(--warning)" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M18 8h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2zM9 6c0-1.66 1.34-3 3-3s3 1.34 3 3v2H9V6zm9 14H6V10h12v10zm-6-3c1.1 0 2-.9 2-2s-.9-2-2-2-2 .9-2 2 .9 2 2 2z"/></svg>
            <div class="font-outfit" style="font-size:2.5rem; font-weight:800; color:var(--text-main); line-height:1">{}</div>
            <div class="text-muted text-uppercase mt-2" style="font-size:0.8rem; letter-spacing:1px">Protected Invocations</div>
        </div>
        <div class="card p-4 d-flex flex-column align-center justify-center text-center" style="border-bottom: 3px solid var(--success)">
            <svg class="mb-3" style="width:36px; height:36px; fill:var(--success)" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.56c-.59-.59-1.54-.59-2.12 0-.31.31-.83.31-1.14 0-.31-.31-.31-.83 0-1.14.59-.59.59-1.54 0-2.12-.31-.31-.83-.31-1.14 0l-1.42 1.42c-.31.31-.83.31-1.14 0-.31-.31-.31-.83 0-1.14.59-.59.59-1.54 0-2.12-.31-.31-.83-.31-1.14 0-.31.31-.31.83 0 1.14-.59.59-1.54.59-2.12 0l-1.42-1.42C2.19 10.74 3.06 7.53 5.46 5.46c2.4-2.07 5.76-2.58 8.61-1.32 2.85 1.25 4.7 4.09 4.7 7.22 0 1.93-.78 3.73-2.18 5.06l-.69-.69z"/></svg>
            <div class="font-outfit text-gradient" style="font-size:2rem; font-weight:800; line-height:1">Distributed</div>
            <div class="text-muted text-uppercase mt-2" style="font-size:0.8rem; letter-spacing:1px">Operational Mode</div>
        </div>
    </div>

    <!-- Backend registry -->
    <div class="card mb-4">
        <div class="card-header">Model Execution Backends</div>
        <div class="card-body bg-body p-0">
            <div class="grid-3 p-3 gap-3">
                <div class="card bg-surface border-glass p-3 h-100 d-flex flex-column transition-fast hover-lift">
                    <div class="d-flex justify-between align-center border-bottom border-glass pb-2 mb-3">
                        <strong class="font-outfit" style="font-size:1.1rem; color:var(--text-main)">Ollama</strong>
                        <span class="badge badge-outline" style="color:var(--brand-blue); border-color:rgba(59,130,246,0.3)">Local Topology</span>
                    </div>
                    <p class="text-muted small mb-0 flex-1">Dedicated local inference pipeline. Hardware-accelerated execution layer set as default.</p>
                </div>
                <div class="card bg-surface border-glass p-3 h-100 d-flex flex-column transition-fast hover-lift">
                    <div class="d-flex justify-between align-center border-bottom border-glass pb-2 mb-3">
                        <strong class="font-outfit" style="font-size:1.1rem; color:var(--text-main)">OpenAI Target</strong>
                        <span class="badge badge-outline" style="color:var(--brand-purple); border-color:rgba(139,92,246,0.3)">Cloud Connect</span>
                    </div>
                    <p class="text-muted small mb-0 flex-1">Remote inference bridging for complex generation tasks. Utilized when local resources are constrained.</p>
                </div>
                <div class="card bg-surface border-glass p-3 h-100 d-flex flex-column transition-fast hover-lift">
                    <div class="d-flex justify-between align-center border-bottom border-glass pb-2 mb-3">
                        <strong class="font-outfit" style="font-size:1.1rem; color:var(--text-main)">Rust Multi-Threaded</strong>
                        <span class="badge badge-outline" style="color:var(--brand-blue); border-color:rgba(59,130,246,0.3)">Local Topology</span>
                    </div>
                    <p class="text-muted small mb-0 flex-1">Embedded Candle ML framework execution. High-performance tensor bridging with zero external hops.</p>
                </div>
            </div>
        </div>
    </div>

    <!-- Recent LLM calls -->
    <div class="card">
        <div class="card-header d-flex justify-between align-center">
            <span>LLM Invocation Audit Trail</span>
            <span class="badge badge-outline">Latest Transactions</span>
        </div>
        <div class="table-container p-0">
            <table class="table mb-0">
                <thead>
                    <tr>
                        <th>Assigned Model</th>
                        <th>Network Node</th>
                        <th>Security Context</th>
                        <th class="text-end">Ingest Tokens</th>
                        <th class="text-end">Emission Tokens</th>
                        <th class="text-end">Timestamp Registry</th>
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
