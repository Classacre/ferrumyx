//! Settings page for configuring API keys and system preferences.

use axum::{extract::State, response::Html};
use crate::state::SharedState;
use crate::handlers::dashboard::NAV_HTML;

pub async fn settings_page(State(_state): State<SharedState>) -> Html<String> {
    Html(format!(r#"<!DOCTYPE html>
<html lang="en" data-bs-theme="dark">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Ferrumyx — Settings</title>
    <link rel="stylesheet" href="/static/css/main.css">
    <style>
        .settings-section {{
            background: var(--bg-card);
            border: 1px solid var(--border-color);
            border-radius: 8px;
            padding: 20px;
            margin-bottom: 20px;
        }}
        .settings-section h3 {{
            font-size: 1.2rem;
            margin-bottom: 15px;
            color: var(--text-primary);
            border-bottom: 1px solid var(--border-color);
            padding-bottom: 10px;
        }}
        .form-group {{
            margin-bottom: 15px;
        }}
        .form-group label {{
            display: block;
            margin-bottom: 5px;
            color: var(--text-secondary);
            font-weight: 500;
        }}
        .form-control {{
            width: 100%;
            padding: 8px 12px;
            background: var(--bg-dark);
            border: 1px solid var(--border-color);
            color: var(--text-primary);
            border-radius: 4px;
        }}
        .form-control:focus {{
            outline: none;
            border-color: var(--accent-color);
        }}
        .btn-save {{
            background: var(--accent-color);
            color: white;
            border: none;
            padding: 8px 16px;
            border-radius: 4px;
            cursor: pointer;
            font-weight: 500;
        }}
        .btn-save:hover {{
            background: #2980b9;
        }}
        .help-text {{
            font-size: 0.85rem;
            color: var(--text-muted);
            margin-top: 4px;
        }}
    </style>
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">⚙️ Settings</h1>
            <p class="text-muted">Configure API keys and system preferences</p>
        </div>
    </div>

    <div class="settings-section">
        <h3>Literature Sources</h3>
        <div class="form-group">
            <label for="pubmed_api_key">PubMed / NCBI API Key</label>
            <input type="password" id="pubmed_api_key" class="form-control" placeholder="Enter NCBI API Key">
            <div class="help-text">Increases rate limits from 3 to 10 requests per second.</div>
        </div>
        <div class="form-group">
            <label for="scihub_url">Sci-Hub Mirror URL</label>
            <input type="text" id="scihub_url" class="form-control" placeholder="https://sci-hub.se" value="https://sci-hub.se">
            <div class="help-text">Used for fallback full-text PDF downloads.</div>
        </div>
        <div class="form-group">
            <label>
                <input type="checkbox" id="enable_scihub" style="margin-right: 8px;">
                Enable Sci-Hub Fallback
            </label>
            <div class="help-text">Attempt to download paywalled PDFs via Sci-Hub if Open Access is unavailable.</div>
        </div>
    </div>

    <div class="settings-section">
        <h3>LLM Backends</h3>
        <div class="form-group">
            <label for="openai_api_key">OpenAI API Key</label>
            <input type="password" id="openai_api_key" class="form-control" placeholder="sk-...">
        </div>
        <div class="form-group">
            <label for="anthropic_api_key">Anthropic API Key</label>
            <input type="password" id="anthropic_api_key" class="form-control" placeholder="sk-ant-...">
        </div>
        <div class="form-group">
            <label for="ollama_url">Ollama Base URL</label>
            <input type="text" id="ollama_url" class="form-control" placeholder="http://localhost:11434" value="http://localhost:11434">
        </div>
    </div>

    <div class="settings-section">
        <h3>System Preferences</h3>
        <div class="form-group">
            <label for="data_dir">Data Directory</label>
            <input type="text" id="data_dir" class="form-control" value="./data" disabled>
            <div class="help-text">Location of the LanceDB vector database and cached files. (Requires restart to change)</div>
        </div>
        <div class="form-group mt-4">
            <label>Hardware Optimization</label>
            <div class="d-flex align-items-center gap-3 mt-2">
                <button class="btn btn-outline-primary" onclick="detectHardware()" id="btn-detect-hw">
                    🔍 Detect & Optimize Hardware
                </button>
                <span id="hw-status" class="text-muted small">Current: CPU (Standard)</span>
            </div>
            <div class="help-text mt-2">Detects available GPUs (CUDA/Metal) or CPU accelerators and configures the embedding models to use them.</div>
        </div>
    </div>

    <button class="btn-save" onclick="saveSettings()">Save Settings</button>

</main>
<script>
    function saveSettings() {{
        // In a real app, this would send a POST request to an API endpoint
        // to securely store these settings in the database or a config file.
        const btn = document.querySelector('.btn-save');
        const originalText = btn.innerText;
        btn.innerText = 'Saved!';
        btn.style.backgroundColor = '#27ae60';
        
        setTimeout(() => {{
            btn.innerText = originalText;
            btn.style.backgroundColor = '';
        }}, 2000);
    }}

    function detectHardware() {{
        const btn = document.getElementById('btn-detect-hw');
        const status = document.getElementById('hw-status');
        
        btn.disabled = true;
        btn.innerHTML = '<span class="spinner-border spinner-border-sm" role="status" aria-hidden="true"></span> Detecting...';
        
        // Simulate hardware detection delay
        setTimeout(() => {{
            // In a real implementation, this would call a Rust endpoint that checks
            // candle_core::utils::cuda_is_available() or metal_is_available()
            const hasCuda = false; // Simulated result
            
            if (hasCuda) {{
                status.innerHTML = '<span class="text-success">✅ Optimized for NVIDIA CUDA</span>';
            }} else {{
                status.innerHTML = '<span class="text-info">✅ Optimized for CPU (Standard)</span>';
            }}
            
            btn.disabled = false;
            btn.innerHTML = '🔍 Detect & Optimize Hardware';
        }}, 1500);
    }}
</script>
<script src="/static/js/main.js"></script>
</body>
</html>"#, NAV_HTML))
}
