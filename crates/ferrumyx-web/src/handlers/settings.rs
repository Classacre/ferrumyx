//! Settings page for configuring API keys and system preferences.

use axum::{extract::State, http::StatusCode, response::Html, Json};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

use crate::handlers::dashboard::NAV_HTML;
use crate::state::SharedState;

const SETTINGS_SCRIPT: &str = r#"
function tabInit() {
  document.querySelectorAll('.tab-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      document.querySelectorAll('.tab-btn').forEach((x) => x.classList.remove('active'));
      document.querySelectorAll('.tab-panel').forEach((x) => x.classList.remove('active'));
      btn.classList.add('active');
      const panel = document.getElementById(btn.dataset.tab);
      if (panel) panel.classList.add('active');
    });
  });
}

function setProviderState(id, hasKey) {
  const el = document.getElementById(id);
  if (!el) return;
  el.textContent = hasKey ? 'Configured' : 'Not Set';
  el.style.color = hasKey ? 'var(--success)' : 'var(--text-muted)';
}

function setSyncState(id, ok) {
  const el = document.getElementById(id);
  if (!el) return;
  el.textContent = ok ? 'Synced' : 'Missing';
  el.style.color = ok ? 'var(--success)' : 'var(--warning)';
}

function byId(id) { return document.getElementById(id); }

async function loadSettings() {
  const res = await fetch('/api/settings');
  const data = await res.json();

  byId('llm_mode').value = data.llm_mode;
  byId('llm_default_backend').value = data.llm_default_backend;
  byId('llm_local_backend').value = data.llm_local_backend;
  byId('ollama_base_url').value = data.ollama_base_url;
  byId('ollama_model').value = data.ollama_model;
  byId('openai_model').value = data.openai_model;
  byId('anthropic_model').value = data.anthropic_model;
  byId('gemini_model').value = data.gemini_model;
  byId('compat_base_url').value = data.compat_base_url;
  byId('compat_model').value = data.compat_model;
  byId('compat_cached_chat').checked = data.compat_cached_chat;
  byId('embedding_backend').value = data.embedding_backend;
  byId('embedding_model').value = data.embedding_model;
  byId('embedding_base_url').value = data.embedding_base_url;

  setProviderState('openai_state', data.has_openai_key);
  setProviderState('anthropic_state', data.has_anthropic_key);
  setProviderState('gemini_state', data.has_gemini_key);
  setProviderState('compat_state', data.has_compat_key);
  setProviderState('pubmed_state', data.has_pubmed_key);
  setProviderState('embedding_state', data.has_embedding_key);

  byId('sync_backend').textContent = data.ironclaw_sync.llm_backend;
  byId('sync_base_url').textContent = data.ironclaw_sync.llm_base_url || 'n/a';
  byId('sync_model').textContent = data.ironclaw_sync.llm_model || 'n/a';
  setSyncState('sync_llm_api_key', data.ironclaw_sync.has_llm_api_key);
  setSyncState('sync_openai', data.ironclaw_sync.has_openai_key);
  setSyncState('sync_anthropic', data.ironclaw_sync.has_anthropic_key);
  setSyncState('sync_gemini', data.ironclaw_sync.has_gemini_key);
  setSyncState('sync_cached_chat', data.ironclaw_sync.compat_cached_chat_enabled);
}

async function saveSettings() {
  const btn = byId('master-save-btn');
  const originalText = btn.innerHTML;
  btn.disabled = true;
  btn.innerHTML = 'Saving...';

  const payload = {
    llm_mode: byId('llm_mode').value,
    llm_default_backend: byId('llm_default_backend').value,
    llm_local_backend: byId('llm_local_backend').value,
    ollama_base_url: byId('ollama_base_url').value,
    ollama_model: byId('ollama_model').value,
    openai_model: byId('openai_model').value,
    anthropic_model: byId('anthropic_model').value,
    gemini_model: byId('gemini_model').value,
    compat_base_url: byId('compat_base_url').value,
    compat_model: byId('compat_model').value,
    compat_cached_chat: byId('compat_cached_chat').checked,
    embedding_backend: byId('embedding_backend').value,
    embedding_model: byId('embedding_model').value,
    embedding_base_url: byId('embedding_base_url').value,
    openai_api_key: byId('openai_api_key').value || null,
    anthropic_api_key: byId('anthropic_api_key').value || null,
    gemini_api_key: byId('gemini_api_key').value || null,
    compat_api_key: byId('compat_api_key').value || null,
    pubmed_api_key: byId('pubmed_api_key').value || null,
    embedding_api_key: byId('embedding_api_key').value || null,
  };

  try {
    const res = await fetch('/api/settings', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    });
    const data = await res.json();
    if (!res.ok || !data.ok) throw new Error(data.message || 'save failed');

    btn.innerHTML = 'Saved';
    btn.style.backgroundColor = 'var(--success)';

    ['openai_api_key','anthropic_api_key','gemini_api_key','compat_api_key','pubmed_api_key','embedding_api_key']
      .forEach((id) => { byId(id).value = ''; });

    await loadSettings();
  } catch (_) {
    btn.innerHTML = 'Save Failed';
    btn.style.backgroundColor = 'var(--danger)';
  } finally {
    setTimeout(() => {
      btn.disabled = false;
      btn.innerHTML = originalText;
      btn.style.backgroundColor = '';
    }, 1400);
  }
}

document.addEventListener('DOMContentLoaded', () => {
  tabInit();
  loadSettings();
});
"#;

const SETTINGS_PAGE_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Settings - Ferrumyx</title>
  <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@400;500;600;700&family=Inter:wght@400;500;600&display=swap" rel="stylesheet">
  <link rel="stylesheet" href="/static/css/main.css?v=1.0.3" />
  <style>
    .settings-grid { display:grid; grid-template-columns: 260px 1fr; gap:1rem; align-items:start; }
    .tabs { background:var(--bg-card); border:1px solid var(--border-glass); border-radius:12px; padding:0.6rem; display:flex; flex-direction:column; gap:0.4rem; position:sticky; top:1rem; }
    .tab-btn { width:100%; background:transparent; border:1px solid transparent; color:var(--text-muted); text-align:left; border-radius:10px; padding:0.65rem 0.8rem; font-weight:600; cursor:pointer; }
    .tab-btn.active { background:rgba(59,130,246,0.12); border-color:rgba(59,130,246,0.25); color:var(--text-main); }
    .tab-panel { display:none; }
    .tab-panel.active { display:block; }
    .settings-section-title { font-family:'Outfit',sans-serif; font-size:1.15rem; color:var(--text-main); margin-bottom:1rem; }
    .form-grid { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:0.9rem; }
    .form-group { margin-bottom:0.9rem; }
    .form-group label { display:block; margin-bottom:0.35rem; color:var(--text-muted); font-weight:500; font-size:0.92rem; }
    .form-control { width:100%; background:var(--bg-surface); border:1px solid var(--border-glass); color:var(--text-main); border-radius:8px; font-family:'Inter',sans-serif; transition:var(--transition-fast); padding:0.62rem 0.7rem; }
    .form-control:focus { border-color:var(--brand-blue); box-shadow:0 0 0 2px rgba(59,130,246,0.2); outline:none; }
    .help-text { font-size:0.82rem; color:rgba(156,163,175,0.75); margin-top:0.28rem; }
    .security-note { margin-top:1rem; padding:0.85rem; border-radius:10px; border:1px solid var(--border-glass); background:rgba(15,23,42,0.45); color:var(--text-muted); font-size:0.9rem; }
    .state-pill { display:inline-block; margin-left:0.4rem; font-size:0.75rem; border-radius:999px; border:1px solid var(--border-glass); padding:0.15rem 0.45rem; color:var(--text-muted); }
    @media (max-width:1100px) {
      .settings-grid { grid-template-columns:1fr; }
      .tabs { position:static; flex-direction:row; overflow:auto; }
      .tab-btn { min-width:180px; }
      .form-grid { grid-template-columns:1fr; }
    }
  </style>
</head>
<body>
__NAV__
<main class="main-content">
  <div class="page-header">
    <div>
      <h1 class="page-title">Global Settings</h1>
      <p class="text-muted">Provider configuration, API credentials, and runtime defaults.</p>
    </div>
    <button class="btn btn-primary" id="master-save-btn" onclick="saveSettings()">Commit Configuration</button>
  </div>

  <div class="settings-grid">
    <nav class="tabs">
      <button class="tab-btn active" data-tab="tab-llm">LLM Providers</button>
      <button class="tab-btn" data-tab="tab-ingestion">Ingestion APIs</button>
      <button class="tab-btn" data-tab="tab-embeddings">Embeddings</button>
      <button class="tab-btn" data-tab="tab-runtime">Runtime</button>
    </nav>

    <div>
      <section id="tab-llm" class="tab-panel active card p-4">
        <h3 class="settings-section-title">Language Model Providers</h3>
        <div class="form-grid">
          <div class="form-group"><label for="llm_mode">Mode</label><select id="llm_mode" class="form-control"><option value="local_only">local_only</option><option value="prefer_local">prefer_local</option><option value="any">any</option></select></div>
          <div class="form-group"><label for="llm_default_backend">Default Backend</label><select id="llm_default_backend" class="form-control"><option value="openai">openai</option><option value="anthropic">anthropic</option><option value="gemini">gemini</option><option value="openai_compatible">openai_compatible</option><option value="ollama">ollama</option></select></div>
          <div class="form-group"><label for="llm_local_backend">Local Backend</label><select id="llm_local_backend" class="form-control"><option value="ollama">ollama</option><option value="openai_compatible">openai_compatible</option></select></div>
          <div class="form-group"><label for="ollama_base_url">Ollama Base URL</label><input id="ollama_base_url" class="form-control" placeholder="http://localhost:11434" /></div>
          <div class="form-group"><label for="ollama_model">Ollama Model</label><input id="ollama_model" class="form-control" placeholder="llama3.1:8b" /></div>
        </div>

        <h4 class="settings-section-title" style="margin-top:1rem;">API Providers</h4>
        <div class="form-grid">
          <div class="form-group"><label for="openai_model">OpenAI Model <span id="openai_state" class="state-pill">Not Set</span></label><input id="openai_model" class="form-control" placeholder="gpt-4o-mini" /></div>
          <div class="form-group"><label for="openai_api_key">OpenAI API Key</label><input id="openai_api_key" type="password" class="form-control" placeholder="Leave blank to keep existing" /></div>
          <div class="form-group"><label for="anthropic_model">Anthropic Model <span id="anthropic_state" class="state-pill">Not Set</span></label><input id="anthropic_model" class="form-control" placeholder="claude-sonnet-4-6" /></div>
          <div class="form-group"><label for="anthropic_api_key">Anthropic API Key</label><input id="anthropic_api_key" type="password" class="form-control" placeholder="Leave blank to keep existing" /></div>
          <div class="form-group"><label for="gemini_model">Gemini Model <span id="gemini_state" class="state-pill">Not Set</span></label><input id="gemini_model" class="form-control" placeholder="gemini-1.5-flash" /></div>
          <div class="form-group"><label for="gemini_api_key">Gemini API Key</label><input id="gemini_api_key" type="password" class="form-control" placeholder="Leave blank to keep existing" /></div>
        </div>

        <h4 class="settings-section-title" style="margin-top:1rem;">OpenAI-Compatible</h4>
        <div class="form-grid">
          <div class="form-group"><label for="compat_base_url">Compatible Base URL</label><input id="compat_base_url" class="form-control" placeholder="https://api.groq.com/openai" /></div>
          <div class="form-group"><label for="compat_model">Compatible Model <span id="compat_state" class="state-pill">Not Set</span></label><input id="compat_model" class="form-control" placeholder="llama-3.3-70b-versatile" /></div>
          <div class="form-group"><label for="compat_api_key">Compatible API Key</label><input id="compat_api_key" type="password" class="form-control" placeholder="Leave blank to keep existing" /></div>
          <div class="form-group">
            <label for="compat_cached_chat">Cached Chat (Provider Support Required)</label>
            <input id="compat_cached_chat" type="checkbox" checked />
            <div class="help-text">Enabled by default. Unsupported models/providers ignore caching hints safely.</div>
          </div>
        </div>
      </section>

      <section id="tab-ingestion" class="tab-panel card p-4">
        <h3 class="settings-section-title">Ingestion API Keys</h3>
        <div class="form-grid">
          <div class="form-group">
            <label for="pubmed_api_key">PubMed / Entrez API Key <span id="pubmed_state" class="state-pill">Not Set</span></label>
            <input id="pubmed_api_key" type="password" class="form-control" placeholder="Leave blank to keep existing" />
            <div class="help-text">Used by ingestion pipeline for higher NCBI throughput.</div>
          </div>
        </div>
      </section>

      <section id="tab-embeddings" class="tab-panel card p-4">
        <h3 class="settings-section-title">Embedding Backend</h3>
        <div class="form-grid">
          <div class="form-group"><label for="embedding_backend">Embedding Backend</label><select id="embedding_backend" class="form-control"><option value="rust_native">rust_native</option><option value="openai">openai</option><option value="gemini">gemini</option><option value="openai_compatible">openai_compatible</option><option value="ollama">ollama</option></select></div>
          <div class="form-group"><label for="embedding_model">Embedding Model</label><input id="embedding_model" class="form-control" placeholder="text-embedding-3-small" /></div>
          <div class="form-group"><label for="embedding_base_url">Embedding Base URL (compat/ollama)</label><input id="embedding_base_url" class="form-control" placeholder="http://localhost:11434" /></div>
          <div class="form-group"><label for="embedding_api_key">Embedding API Key <span id="embedding_state" class="state-pill">Not Set</span></label><input id="embedding_api_key" type="password" class="form-control" placeholder="Leave blank to keep existing" /></div>
        </div>
      </section>

      <section id="tab-runtime" class="tab-panel card p-4">
        <h3 class="settings-section-title">Runtime Notes</h3>
        <div class="security-note">API keys are never returned to the browser once saved. Empty password fields keep existing keys unchanged. Settings are persisted to your Ferrumyx config file and applied on next agent restart.</div>
        <div class="security-note" style="margin-top:0.8rem;">
          <strong style="color:var(--text-main);">IronClaw Sync Status</strong>
          <div style="margin-top:0.55rem; display:grid; grid-template-columns: 230px 1fr; row-gap:0.35rem; column-gap:0.9rem;">
            <div>LLM Backend</div><div id="sync_backend">n/a</div>
            <div>LLM Base URL</div><div id="sync_base_url">n/a</div>
            <div>LLM Model</div><div id="sync_model">n/a</div>
            <div>LLM API Key</div><div id="sync_llm_api_key">Missing</div>
            <div>OPENAI_API_KEY</div><div id="sync_openai">Missing</div>
            <div>ANTHROPIC_API_KEY</div><div id="sync_anthropic">Missing</div>
            <div>GEMINI_API_KEY</div><div id="sync_gemini">Missing</div>
            <div>Compat Cached Chat</div><div id="sync_cached_chat">Missing</div>
          </div>
        </div>
      </section>
    </div>
  </div>
</main>
<script>__SCRIPT__</script>
<script src="/static/js/main.js"></script>
</body>
</html>
"#;

#[derive(Debug, Serialize)]
pub struct SettingsView {
    llm_mode: String,
    llm_default_backend: String,
    llm_local_backend: String,
    ollama_base_url: String,
    ollama_model: String,
    openai_model: String,
    anthropic_model: String,
    gemini_model: String,
    compat_base_url: String,
    compat_model: String,
    compat_cached_chat: bool,
    embedding_backend: String,
    embedding_model: String,
    embedding_base_url: String,
    has_openai_key: bool,
    has_anthropic_key: bool,
    has_gemini_key: bool,
    has_compat_key: bool,
    has_pubmed_key: bool,
    has_embedding_key: bool,
    ironclaw_sync: IronclawSyncView,
}

#[derive(Debug, Serialize)]
pub struct IronclawSyncView {
    llm_backend: String,
    llm_base_url: String,
    llm_model: String,
    has_llm_api_key: bool,
    has_openai_key: bool,
    has_anthropic_key: bool,
    has_gemini_key: bool,
    compat_cached_chat_enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct SettingsSaveRequest {
    llm_mode: String,
    llm_default_backend: String,
    llm_local_backend: String,
    ollama_base_url: String,
    ollama_model: String,
    openai_model: String,
    anthropic_model: String,
    gemini_model: String,
    compat_base_url: String,
    compat_model: String,
    #[serde(default = "default_true")]
    compat_cached_chat: bool,
    embedding_backend: String,
    embedding_model: String,
    embedding_base_url: String,
    openai_api_key: Option<String>,
    anthropic_api_key: Option<String>,
    gemini_api_key: Option<String>,
    compat_api_key: Option<String>,
    pubmed_api_key: Option<String>,
    embedding_api_key: Option<String>,
}

fn default_true() -> bool { true }

#[derive(Debug, Serialize)]
pub struct SaveResponse {
    ok: bool,
    message: String,
}

pub async fn settings_page(State(_state): State<SharedState>) -> Html<String> {
    let page = SETTINGS_PAGE_HTML
        .replace("__NAV__", NAV_HTML)
        .replace("__SCRIPT__", SETTINGS_SCRIPT);
    Html(page)
}

pub async fn settings_get(
    State(_state): State<SharedState>,
) -> Result<Json<SettingsView>, (StatusCode, Json<SaveResponse>)> {
    let view = load_settings_view().map_err(internal_err)?;
    Ok(Json(view))
}

pub async fn settings_save(
    State(_state): State<SharedState>,
    Json(payload): Json<SettingsSaveRequest>,
) -> Result<Json<SaveResponse>, (StatusCode, Json<SaveResponse>)> {
    save_settings(payload).map_err(internal_err)?;
    Ok(Json(SaveResponse {
        ok: true,
        message: "Settings saved".to_string(),
    }))
}

fn internal_err(e: anyhow::Error) -> (StatusCode, Json<SaveResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(SaveResponse {
            ok: false,
            message: format!("settings error: {e}"),
        }),
    )
}

fn config_path() -> PathBuf {
    std::env::var("FERRUMYX_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("ferrumyx.toml"))
}

fn load_toml() -> anyhow::Result<toml::Value> {
    let path = config_path();
    if !path.exists() {
        return Ok(toml::Value::Table(toml::map::Map::new()));
    }
    let content = fs::read_to_string(&path)?;
    Ok(toml::from_str::<toml::Value>(&content)?)
}

fn save_toml(v: &toml::Value) -> anyhow::Result<()> {
    let path = config_path();
    fs::write(&path, toml::to_string_pretty(v)?)?;
    Ok(())
}

fn table_mut<'a>(root: &'a mut toml::Value, key: &str) -> &'a mut toml::map::Map<String, toml::Value> {
    let root_tbl = root.as_table_mut().expect("root TOML table");
    if !root_tbl.contains_key(key) {
        root_tbl.insert(key.to_string(), toml::Value::Table(toml::map::Map::new()));
    }
    root_tbl.get_mut(key).and_then(|v| v.as_table_mut()).expect("child TOML table")
}

fn nested_table_mut<'a>(
    parent: &'a mut toml::map::Map<String, toml::Value>,
    key: &str,
) -> &'a mut toml::map::Map<String, toml::Value> {
    if !parent.contains_key(key) {
        parent.insert(key.to_string(), toml::Value::Table(toml::map::Map::new()));
    }
    parent.get_mut(key).and_then(|v| v.as_table_mut()).expect("nested table")
}

fn str_at(root: &toml::Value, path: &[&str], default: &str) -> String {
    let mut cur = root;
    for p in path {
        match cur.get(*p) {
            Some(next) => cur = next,
            None => return default.to_string(),
        }
    }
    cur.as_str().unwrap_or(default).to_string()
}

fn has_nonempty(root: &toml::Value, path: &[&str]) -> bool {
    let mut cur = root;
    for p in path {
        match cur.get(*p) {
            Some(next) => cur = next,
            None => return false,
        }
    }
    cur.as_str().map(|s| !s.trim().is_empty()).unwrap_or(false)
}

fn bool_at(root: &toml::Value, path: &[&str], default: bool) -> bool {
    let mut cur = root;
    for p in path {
        match cur.get(*p) {
            Some(next) => cur = next,
            None => return default,
        }
    }
    cur.as_bool().unwrap_or(default)
}

fn set_str(map: &mut toml::map::Map<String, toml::Value>, key: &str, value: String) {
    map.insert(key.to_string(), toml::Value::String(value));
}

fn maybe_set_secret(
    map: &mut toml::map::Map<String, toml::Value>,
    key: &str,
    value: &Option<String>,
) {
    if let Some(v) = value {
        let t = v.trim();
        if !t.is_empty() && t != "********" {
            set_str(map, key, t.to_string());
        }
    }
}

fn load_settings_view() -> anyhow::Result<SettingsView> {
    let root = load_toml()?;
    Ok(SettingsView {
        llm_mode: str_at(&root, &["llm", "mode"], "any"),
        llm_default_backend: str_at(&root, &["llm", "default_backend"], "openai"),
        llm_local_backend: str_at(&root, &["llm", "local_backend"], "ollama"),
        ollama_base_url: str_at(&root, &["llm", "ollama", "base_url"], "http://localhost:11434"),
        ollama_model: str_at(&root, &["llm", "ollama", "model"], "llama3.1:8b"),
        openai_model: str_at(&root, &["llm", "openai", "model"], "gpt-4o-mini"),
        anthropic_model: str_at(&root, &["llm", "anthropic", "model"], "claude-haiku-4-5"),
        gemini_model: str_at(&root, &["llm", "gemini", "model"], "gemini-1.5-flash"),
        compat_base_url: str_at(&root, &["llm", "openai_compatible", "base_url"], "https://api.groq.com/openai"),
        compat_model: str_at(&root, &["llm", "openai_compatible", "model"], "llama-3.3-70b-versatile"),
        compat_cached_chat: bool_at(&root, &["llm", "openai_compatible", "cached_chat"], true),
        embedding_backend: str_at(&root, &["embedding", "backend"], "rust_native"),
        embedding_model: str_at(&root, &["embedding", "embedding_model"], "microsoft/BiomedNLP-BiomedBERT-base-uncased-abstract-fulltext"),
        embedding_base_url: str_at(&root, &["embedding", "base_url"], ""),
        has_openai_key: has_nonempty(&root, &["llm", "openai", "api_key"]) || std::env::var("FERRUMYX_OPENAI_API_KEY").is_ok(),
        has_anthropic_key: has_nonempty(&root, &["llm", "anthropic", "api_key"]) || std::env::var("FERRUMYX_ANTHROPIC_API_KEY").is_ok(),
        has_gemini_key: has_nonempty(&root, &["llm", "gemini", "api_key"]) || std::env::var("FERRUMYX_GEMINI_API_KEY").is_ok(),
        has_compat_key: has_nonempty(&root, &["llm", "openai_compatible", "api_key"]) || std::env::var("FERRUMYX_COMPAT_API_KEY").is_ok(),
        has_pubmed_key: has_nonempty(&root, &["ingestion", "pubmed", "api_key"]) || std::env::var("FERRUMYX_PUBMED_API_KEY").is_ok(),
        has_embedding_key: has_nonempty(&root, &["embedding", "api_key"]),
        ironclaw_sync: IronclawSyncView {
            llm_backend: std::env::var("LLM_BACKEND").unwrap_or_else(|_| "unset".to_string()),
            llm_base_url: std::env::var("LLM_BASE_URL").unwrap_or_default(),
            llm_model: std::env::var("LLM_MODEL").unwrap_or_default(),
            has_llm_api_key: std::env::var("LLM_API_KEY").is_ok_and(|v| !v.trim().is_empty()),
            has_openai_key: std::env::var("OPENAI_API_KEY").is_ok_and(|v| !v.trim().is_empty()),
            has_anthropic_key: std::env::var("ANTHROPIC_API_KEY").is_ok_and(|v| !v.trim().is_empty()),
            has_gemini_key: std::env::var("GEMINI_API_KEY").is_ok_and(|v| !v.trim().is_empty()),
            compat_cached_chat_enabled: std::env::var("LLM_COMPAT_CACHED_CHAT")
                .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true")),
        },
    })
}

fn save_settings(payload: SettingsSaveRequest) -> anyhow::Result<()> {
    let mut root = load_toml()?;

    let llm = table_mut(&mut root, "llm");
    set_str(llm, "mode", payload.llm_mode);
    set_str(llm, "default_backend", payload.llm_default_backend);
    set_str(llm, "local_backend", payload.llm_local_backend);

    let ollama = nested_table_mut(llm, "ollama");
    set_str(ollama, "base_url", payload.ollama_base_url);
    set_str(ollama, "model", payload.ollama_model);

    let openai = nested_table_mut(llm, "openai");
    set_str(openai, "model", payload.openai_model);
    maybe_set_secret(openai, "api_key", &payload.openai_api_key);

    let anthropic = nested_table_mut(llm, "anthropic");
    set_str(anthropic, "model", payload.anthropic_model);
    maybe_set_secret(anthropic, "api_key", &payload.anthropic_api_key);

    let gemini = nested_table_mut(llm, "gemini");
    set_str(gemini, "model", payload.gemini_model);
    maybe_set_secret(gemini, "api_key", &payload.gemini_api_key);

    let compat = nested_table_mut(llm, "openai_compatible");
    set_str(compat, "base_url", payload.compat_base_url);
    set_str(compat, "model", payload.compat_model);
    compat.insert(
        "cached_chat".to_string(),
        toml::Value::Boolean(payload.compat_cached_chat),
    );
    maybe_set_secret(compat, "api_key", &payload.compat_api_key);

    let ingestion = table_mut(&mut root, "ingestion");
    let pubmed = nested_table_mut(ingestion, "pubmed");
    maybe_set_secret(pubmed, "api_key", &payload.pubmed_api_key);

    let embedding = table_mut(&mut root, "embedding");
    set_str(embedding, "backend", payload.embedding_backend);
    set_str(embedding, "embedding_model", payload.embedding_model);
    if !payload.embedding_base_url.trim().is_empty() {
        set_str(embedding, "base_url", payload.embedding_base_url);
    }
    maybe_set_secret(embedding, "api_key", &payload.embedding_api_key);

    save_toml(&root)?;
    apply_runtime_env_from_saved_toml(&root);
    Ok(())
}

fn apply_runtime_env_from_saved_toml(root: &toml::Value) {
    let default_backend = str_at(root, &["llm", "default_backend"], "openai");
    std::env::set_var("LLM_BACKEND", default_backend);

    let ollama_base = str_at(root, &["llm", "ollama", "base_url"], "");
    if !ollama_base.is_empty() {
        std::env::set_var("OLLAMA_BASE_URL", ollama_base);
    }
    let ollama_model = str_at(root, &["llm", "ollama", "model"], "");
    if !ollama_model.is_empty() {
        std::env::set_var("OLLAMA_MODEL", ollama_model);
    }

    let openai_key = str_at(root, &["llm", "openai", "api_key"], "");
    if !openai_key.is_empty() {
        std::env::set_var("FERRUMYX_OPENAI_API_KEY", &openai_key);
        std::env::set_var("OPENAI_API_KEY", &openai_key);
    }

    let anthropic_key = str_at(root, &["llm", "anthropic", "api_key"], "");
    if !anthropic_key.is_empty() {
        std::env::set_var("FERRUMYX_ANTHROPIC_API_KEY", &anthropic_key);
        std::env::set_var("ANTHROPIC_API_KEY", &anthropic_key);
    }

    let gemini_key = str_at(root, &["llm", "gemini", "api_key"], "");
    if !gemini_key.is_empty() {
        std::env::set_var("FERRUMYX_GEMINI_API_KEY", &gemini_key);
        std::env::set_var("GEMINI_API_KEY", &gemini_key);
    }

    let compat_key = str_at(root, &["llm", "openai_compatible", "api_key"], "");
    if !compat_key.is_empty() {
        std::env::set_var("FERRUMYX_COMPAT_API_KEY", &compat_key);
        std::env::set_var("LLM_API_KEY", &compat_key);
    }
    let compat_url = str_at(root, &["llm", "openai_compatible", "base_url"], "");
    if !compat_url.is_empty() {
        std::env::set_var("LLM_BASE_URL", compat_url);
    }
    let compat_model = str_at(root, &["llm", "openai_compatible", "model"], "");
    if !compat_model.is_empty() {
        std::env::set_var("LLM_MODEL", compat_model);
    }
    let compat_cached_chat = bool_at(root, &["llm", "openai_compatible", "cached_chat"], true);
    std::env::set_var(
        "FERRUMYX_COMPAT_CACHED_CHAT",
        if compat_cached_chat { "1" } else { "0" },
    );
    std::env::set_var(
        "LLM_COMPAT_CACHED_CHAT",
        if compat_cached_chat { "1" } else { "0" },
    );
}
