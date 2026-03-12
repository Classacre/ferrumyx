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
  byId('ingestion_default_max_results').value = data.ingestion_default_max_results;
  byId('ingestion_idle_timeout_secs').value = data.ingestion_idle_timeout_secs;
  byId('ingestion_max_runtime_secs').value = data.ingestion_max_runtime_secs;
  byId('ingestion_enable_embeddings').checked = data.ingestion_enable_embeddings;
  byId('ingestion_source_profile').value = data.ingestion_source_profile;
  byId('ingestion_source_timeout_secs').value = data.ingestion_source_timeout_secs;
  byId('ingestion_full_text_step_timeout_secs').value = data.ingestion_full_text_step_timeout_secs;
  byId('ingestion_full_text_total_timeout_secs').value = data.ingestion_full_text_total_timeout_secs;
  byId('ingestion_full_text_prefetch_workers').value = data.ingestion_full_text_prefetch_workers;
  byId('ingestion_paper_process_workers').value = data.ingestion_paper_process_workers;
  byId('ingestion_perf_mode').value = data.ingestion_perf_mode;
  byId('ingestion_source_cache_enabled').checked = data.ingestion_source_cache_enabled;
  byId('ingestion_source_cache_ttl_secs').value = data.ingestion_source_cache_ttl_secs;
  byId('ingestion_entity_batch_size').value = data.ingestion_entity_batch_size;
  byId('ingestion_fact_batch_size').value = data.ingestion_fact_batch_size;
  byId('ingestion_strict_fuzzy_dedup').checked = data.ingestion_strict_fuzzy_dedup;
  byId('ingestion_source_max_inflight').value = data.ingestion_source_max_inflight;
  byId('ingestion_source_retries').value = data.ingestion_source_retries;
  byId('ingestion_pdf_host_concurrency').value = data.ingestion_pdf_host_concurrency;
  byId('ingestion_pdf_parse_cache_enabled').checked = data.ingestion_pdf_parse_cache_enabled;
  byId('ingestion_full_text_negative_cache_enabled').checked = data.ingestion_full_text_negative_cache_enabled;
  byId('ingestion_full_text_negative_cache_ttl_secs').value = data.ingestion_full_text_negative_cache_ttl_secs;
  byId('ingestion_chunk_fingerprint_cache_enabled').checked = data.ingestion_chunk_fingerprint_cache_enabled;
  byId('ingestion_chunk_fingerprint_cache_ttl_secs').value = data.ingestion_chunk_fingerprint_cache_ttl_secs;
  byId('ingestion_heavy_lane_async_enabled').checked = data.ingestion_heavy_lane_async_enabled;
  byId('ingestion_min_ner_chars').value = data.ingestion_min_ner_chars;
  byId('ingestion_max_relation_genes_per_chunk').value = data.ingestion_max_relation_genes_per_chunk;
  byId('ingestion_async_post_ingest_scoring').checked = data.ingestion_async_post_ingest_scoring;
  byId('unpaywall_email').value = data.unpaywall_email;
  byId('scihub_domains').value = data.scihub_domains;
  byId('scihub_request_timeout_secs').value = data.scihub_request_timeout_secs;

  setProviderState('openai_state', data.has_openai_key);
  setProviderState('anthropic_state', data.has_anthropic_key);
  setProviderState('gemini_state', data.has_gemini_key);
  setProviderState('compat_state', data.has_compat_key);
  setProviderState('pubmed_state', data.has_pubmed_key);
  setProviderState('semanticscholar_state', data.has_semanticscholar_key);
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
    ingestion_default_max_results: Number(byId('ingestion_default_max_results').value || 50),
    ingestion_idle_timeout_secs: Number(byId('ingestion_idle_timeout_secs').value || 600),
    ingestion_max_runtime_secs: Number(byId('ingestion_max_runtime_secs').value || 14400),
    ingestion_enable_embeddings: byId('ingestion_enable_embeddings').checked,
    ingestion_source_profile: byId('ingestion_source_profile').value,
    ingestion_source_timeout_secs: Number(byId('ingestion_source_timeout_secs').value || 18),
    ingestion_full_text_step_timeout_secs: Number(byId('ingestion_full_text_step_timeout_secs').value || 15),
    ingestion_full_text_total_timeout_secs: Number(byId('ingestion_full_text_total_timeout_secs').value || 28),
    ingestion_full_text_prefetch_workers: Number(byId('ingestion_full_text_prefetch_workers').value || 4),
    ingestion_paper_process_workers: Number(byId('ingestion_paper_process_workers').value || 4),
    ingestion_perf_mode: byId('ingestion_perf_mode').value,
    ingestion_source_cache_enabled: byId('ingestion_source_cache_enabled').checked,
    ingestion_source_cache_ttl_secs: Number(byId('ingestion_source_cache_ttl_secs').value || 1800),
    ingestion_entity_batch_size: Number(byId('ingestion_entity_batch_size').value || 256),
    ingestion_fact_batch_size: Number(byId('ingestion_fact_batch_size').value || 512),
    ingestion_strict_fuzzy_dedup: byId('ingestion_strict_fuzzy_dedup').checked,
    ingestion_source_max_inflight: Number(byId('ingestion_source_max_inflight').value || 4),
    ingestion_source_retries: Number(byId('ingestion_source_retries').value || 2),
    ingestion_pdf_host_concurrency: Number(byId('ingestion_pdf_host_concurrency').value || 4),
    ingestion_pdf_parse_cache_enabled: byId('ingestion_pdf_parse_cache_enabled').checked,
    ingestion_full_text_negative_cache_enabled: byId('ingestion_full_text_negative_cache_enabled').checked,
    ingestion_full_text_negative_cache_ttl_secs: Number(byId('ingestion_full_text_negative_cache_ttl_secs').value || 21600),
    ingestion_chunk_fingerprint_cache_enabled: byId('ingestion_chunk_fingerprint_cache_enabled').checked,
    ingestion_chunk_fingerprint_cache_ttl_secs: Number(byId('ingestion_chunk_fingerprint_cache_ttl_secs').value || 172800),
    ingestion_heavy_lane_async_enabled: byId('ingestion_heavy_lane_async_enabled').checked,
    ingestion_min_ner_chars: Number(byId('ingestion_min_ner_chars').value || 500),
    ingestion_max_relation_genes_per_chunk: Number(byId('ingestion_max_relation_genes_per_chunk').value || 4),
    ingestion_async_post_ingest_scoring: byId('ingestion_async_post_ingest_scoring').checked,
    unpaywall_email: byId('unpaywall_email').value,
    scihub_domains: byId('scihub_domains').value,
    scihub_request_timeout_secs: Number(byId('scihub_request_timeout_secs').value || 10),
    openai_api_key: byId('openai_api_key').value || null,
    anthropic_api_key: byId('anthropic_api_key').value || null,
    gemini_api_key: byId('gemini_api_key').value || null,
    compat_api_key: byId('compat_api_key').value || null,
    pubmed_api_key: byId('pubmed_api_key').value || null,
    semanticscholar_api_key: byId('semanticscholar_api_key').value || null,
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

    ['openai_api_key','anthropic_api_key','gemini_api_key','compat_api_key','pubmed_api_key','semanticscholar_api_key','embedding_api_key']
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
          <div class="form-group">
            <label for="semanticscholar_api_key">Semantic Scholar API Key <span id="semanticscholar_state" class="state-pill">Not Set</span></label>
            <input id="semanticscholar_api_key" type="password" class="form-control" placeholder="Leave blank to keep existing" />
            <div class="help-text">Used by Semantic Scholar Graph API source for higher throughput and quota.</div>
          </div>
          <div class="form-group">
            <label for="unpaywall_email">Unpaywall Contact Email</label>
            <input id="unpaywall_email" class="form-control" placeholder="you@domain.com" />
            <div class="help-text">Optional but recommended. Enables Unpaywall DOI->PDF OA resolution tier.</div>
          </div>
        </div>
        <h4 class="settings-section-title" style="margin-top:1rem;">Sci-Hub Full-Text Fallback</h4>
        <div class="form-grid">
          <div class="form-group">
            <label for="scihub_domains">Sci-Hub Mirror List (comma-separated)</label>
            <textarea id="scihub_domains" class="form-control" rows="3" placeholder="https://sci-hub.al,https://sci-hub.mk,https://sci-hub.ee"></textarea>
            <div class="help-text">Tried in order when OA routes fail. Uses first mirror returning a valid PDF.</div>
          </div>
          <div class="form-group">
            <label for="scihub_request_timeout_secs">Sci-Hub Request Timeout (seconds)</label>
            <input id="scihub_request_timeout_secs" type="number" min="4" max="45" class="form-control" />
            <div class="help-text">Per-request timeout for mirror and PDF fetches. Lower is faster failover, higher tolerates slow mirrors.</div>
          </div>
        </div>
        <h4 class="settings-section-title" style="margin-top:1rem;">Ingestion Runtime Policy</h4>
        <div class="form-grid">
          <div class="form-group">
            <label for="ingestion_default_max_results">Default Max Papers Per Run</label>
            <input id="ingestion_default_max_results" type="number" min="1" max="5000" class="form-control" />
            <div class="help-text">Fallback paper cap when the tool call does not set <code>max_results</code>.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_idle_timeout_secs">Idle Timeout (seconds)</label>
            <input id="ingestion_idle_timeout_secs" type="number" min="60" max="3600" class="form-control" />
            <div class="help-text">Abort only if no progress heartbeat arrives within this window.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_max_runtime_secs">Max Runtime Safety Cap (seconds)</label>
            <input id="ingestion_max_runtime_secs" type="number" min="600" max="86400" class="form-control" />
            <div class="help-text">Hard stop to prevent runaway jobs. Keep high for large corpora.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_enable_embeddings">Enable Embeddings During Ingestion</label>
            <input id="ingestion_enable_embeddings" type="checkbox" />
            <div class="help-text">When enabled, chunk embeddings run during ingestion using the Embeddings tab provider.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_source_profile">Default Source Profile</label>
            <select id="ingestion_source_profile" class="form-control">
              <option value="fast">fast (PubMed + EuropePMC)</option>
              <option value="full">full (all configured sources)</option>
            </select>
            <div class="help-text">Controls default source mix for agent ingestion runs.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_source_timeout_secs">Per-Source Timeout (seconds)</label>
            <input id="ingestion_source_timeout_secs" type="number" min="5" max="300" class="form-control" />
            <div class="help-text">Stops slow upstream APIs from stalling ingestion.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_full_text_step_timeout_secs">Full-Text Step Timeout (seconds)</label>
            <input id="ingestion_full_text_step_timeout_secs" type="number" min="5" max="120" class="form-control" />
            <div class="help-text">Timeout budget per full-text strategy step (PMC XML/PDF, Unpaywall, etc.).</div>
          </div>
          <div class="form-group">
            <label for="ingestion_full_text_total_timeout_secs">Full-Text Total Timeout (seconds)</label>
            <input id="ingestion_full_text_total_timeout_secs" type="number" min="8" max="180" class="form-control" />
            <div class="help-text">Overall per-paper full-text budget across all strategy attempts.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_full_text_prefetch_workers">Full-Text Prefetch Workers</label>
            <input id="ingestion_full_text_prefetch_workers" type="number" min="1" max="32" class="form-control" />
            <div class="help-text">Parallel full-text fetch workers. Higher values improve throughput on strong hardware.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_paper_process_workers">Paper Processing Workers</label>
            <input id="ingestion_paper_process_workers" type="number" min="1" max="16" class="form-control" />
            <div class="help-text">Parallel post-upsert workers for chunking, NER, KG facts, and parse status updates.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_perf_mode">Performance Mode</label>
            <select id="ingestion_perf_mode" class="form-control">
              <option value="auto">auto (hardware-aware)</option>
              <option value="throughput">throughput (aggressive)</option>
              <option value="balanced">balanced</option>
              <option value="safe">safe (stability-first)</option>
            </select>
            <div class="help-text">Controls default runtime tuning profile for ingestion execution.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_source_cache_enabled">Persistent Source Cache</label>
            <input id="ingestion_source_cache_enabled" type="checkbox" />
            <div class="help-text">Reuses recent source search responses to avoid repeated network fetches for identical queries.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_source_cache_ttl_secs">Source Cache TTL (seconds)</label>
            <input id="ingestion_source_cache_ttl_secs" type="number" min="60" max="86400" class="form-control" />
            <div class="help-text">How long cached source search payloads remain valid.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_entity_batch_size">Entity Insert Batch Size</label>
            <input id="ingestion_entity_batch_size" type="number" min="16" max="2048" class="form-control" />
            <div class="help-text">Batch size for new entity writes during NER/KG extraction.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_fact_batch_size">KG Fact Insert Batch Size</label>
            <input id="ingestion_fact_batch_size" type="number" min="16" max="4096" class="form-control" />
            <div class="help-text">Batch size for knowledge-graph fact inserts per paper.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_strict_fuzzy_dedup">Strict Fuzzy Dedup</label>
            <input id="ingestion_strict_fuzzy_dedup" type="checkbox" />
            <div class="help-text">When enabled, lexical fuzzy dedup is applied in addition to DOI/PMID checks (can reduce recall).</div>
          </div>
          <div class="form-group">
            <label for="ingestion_source_max_inflight">Source Max Inflight</label>
            <input id="ingestion_source_max_inflight" type="number" min="1" max="16" class="form-control" />
            <div class="help-text">Caps concurrent source API searches to reduce throttling and improve stability.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_source_retries">Source Retries</label>
            <input id="ingestion_source_retries" type="number" min="0" max="5" class="form-control" />
            <div class="help-text">Automatic retries with backoff for transient source/network failures.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_pdf_host_concurrency">PDF Host Concurrency</label>
            <input id="ingestion_pdf_host_concurrency" type="number" min="1" max="16" class="form-control" />
            <div class="help-text">Per-host cap for parallel PDF downloads (prevents mirror/API overload).</div>
          </div>
          <div class="form-group">
            <label for="ingestion_pdf_parse_cache_enabled">PDF Parse Cache Enabled</label>
            <input id="ingestion_pdf_parse_cache_enabled" type="checkbox" />
            <div class="help-text">Caches parsed PDF sections by content hash to skip repeated parse work.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_full_text_negative_cache_enabled">Full-Text Negative Cache Enabled</label>
            <input id="ingestion_full_text_negative_cache_enabled" type="checkbox" />
            <div class="help-text">Skips repeated failed DOI/PMCID/PDF lookups for a short TTL to reduce network waste.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_full_text_negative_cache_ttl_secs">Full-Text Negative Cache TTL (seconds)</label>
            <input id="ingestion_full_text_negative_cache_ttl_secs" type="number" min="60" max="604800" class="form-control" />
            <div class="help-text">How long failed full-text attempts stay suppressed before retrying.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_chunk_fingerprint_cache_enabled">Chunk Fingerprint Cache Enabled</label>
            <input id="ingestion_chunk_fingerprint_cache_enabled" type="checkbox" />
            <div class="help-text">Skips repeated NER/KG extraction for duplicate chunk text across papers.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_chunk_fingerprint_cache_ttl_secs">Chunk Fingerprint Cache TTL (seconds)</label>
            <input id="ingestion_chunk_fingerprint_cache_ttl_secs" type="number" min="300" max="1209600" class="form-control" />
            <div class="help-text">Retention window for duplicate-chunk skip fingerprints.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_heavy_lane_async_enabled">Heavy Lane Async Enrichment</label>
            <input id="ingestion_heavy_lane_async_enabled" type="checkbox" />
            <div class="help-text">Runs expensive NER/KG enrichment in background after fast chunk insertion.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_min_ner_chars">Quality Gate Minimum Chars</label>
            <input id="ingestion_min_ner_chars" type="number" min="120" max="5000" class="form-control" />
            <div class="help-text">Documents below this total chunk text size skip deep NER/KG for speed.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_max_relation_genes_per_chunk">Max Relation Genes Per Chunk</label>
            <input id="ingestion_max_relation_genes_per_chunk" type="number" min="1" max="16" class="form-control" />
            <div class="help-text">Caps per-chunk gene relation expansion to avoid combinatorial slowdowns.</div>
          </div>
          <div class="form-group">
            <label for="ingestion_async_post_ingest_scoring">Async Post-Ingestion Scoring</label>
            <input id="ingestion_async_post_ingest_scoring" type="checkbox" />
            <div class="help-text">Queues target score recompute/provider refresh in background so ingestion returns faster.</div>
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
    ingestion_default_max_results: u64,
    ingestion_idle_timeout_secs: u64,
    ingestion_max_runtime_secs: u64,
    ingestion_enable_embeddings: bool,
    #[serde(default = "default_source_profile")]
    ingestion_source_profile: String,
    #[serde(default = "default_source_timeout_secs")]
    ingestion_source_timeout_secs: u64,
    #[serde(default = "default_full_text_step_timeout_secs")]
    ingestion_full_text_step_timeout_secs: u64,
    #[serde(default = "default_full_text_total_timeout_secs")]
    ingestion_full_text_total_timeout_secs: u64,
    #[serde(default = "default_full_text_prefetch_workers")]
    ingestion_full_text_prefetch_workers: u64,
    #[serde(default = "default_paper_process_workers")]
    ingestion_paper_process_workers: u64,
    #[serde(default = "default_perf_mode")]
    ingestion_perf_mode: String,
    #[serde(default = "default_true")]
    ingestion_source_cache_enabled: bool,
    #[serde(default = "default_source_cache_ttl_secs")]
    ingestion_source_cache_ttl_secs: u64,
    #[serde(default = "default_entity_batch_size")]
    ingestion_entity_batch_size: u64,
    #[serde(default = "default_fact_batch_size")]
    ingestion_fact_batch_size: u64,
    #[serde(default)]
    ingestion_strict_fuzzy_dedup: bool,
    #[serde(default = "default_source_max_inflight")]
    ingestion_source_max_inflight: u64,
    #[serde(default = "default_source_retries")]
    ingestion_source_retries: u64,
    #[serde(default = "default_pdf_host_concurrency")]
    ingestion_pdf_host_concurrency: u64,
    #[serde(default = "default_true")]
    ingestion_pdf_parse_cache_enabled: bool,
    #[serde(default = "default_true")]
    ingestion_full_text_negative_cache_enabled: bool,
    #[serde(default = "default_full_text_negative_cache_ttl_secs")]
    ingestion_full_text_negative_cache_ttl_secs: u64,
    #[serde(default = "default_true")]
    ingestion_chunk_fingerprint_cache_enabled: bool,
    #[serde(default = "default_chunk_fingerprint_cache_ttl_secs")]
    ingestion_chunk_fingerprint_cache_ttl_secs: u64,
    #[serde(default = "default_true")]
    ingestion_heavy_lane_async_enabled: bool,
    #[serde(default = "default_min_ner_chars")]
    ingestion_min_ner_chars: u64,
    #[serde(default = "default_max_relation_genes_per_chunk")]
    ingestion_max_relation_genes_per_chunk: u64,
    #[serde(default = "default_true")]
    ingestion_async_post_ingest_scoring: bool,
    unpaywall_email: String,
    #[serde(default = "default_scihub_domains")]
    scihub_domains: String,
    #[serde(default = "default_scihub_request_timeout_secs")]
    scihub_request_timeout_secs: u64,
    has_openai_key: bool,
    has_anthropic_key: bool,
    has_gemini_key: bool,
    has_compat_key: bool,
    has_pubmed_key: bool,
    has_semanticscholar_key: bool,
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
    ingestion_default_max_results: u64,
    ingestion_idle_timeout_secs: u64,
    ingestion_max_runtime_secs: u64,
    ingestion_enable_embeddings: bool,
    ingestion_source_profile: String,
    ingestion_source_timeout_secs: u64,
    ingestion_full_text_step_timeout_secs: u64,
    #[serde(default = "default_full_text_total_timeout_secs")]
    ingestion_full_text_total_timeout_secs: u64,
    ingestion_full_text_prefetch_workers: u64,
    ingestion_paper_process_workers: u64,
    ingestion_perf_mode: String,
    ingestion_source_cache_enabled: bool,
    ingestion_source_cache_ttl_secs: u64,
    ingestion_entity_batch_size: u64,
    ingestion_fact_batch_size: u64,
    ingestion_strict_fuzzy_dedup: bool,
    #[serde(default = "default_source_max_inflight")]
    ingestion_source_max_inflight: u64,
    #[serde(default = "default_source_retries")]
    ingestion_source_retries: u64,
    #[serde(default = "default_pdf_host_concurrency")]
    ingestion_pdf_host_concurrency: u64,
    #[serde(default = "default_true")]
    ingestion_pdf_parse_cache_enabled: bool,
    #[serde(default = "default_true")]
    ingestion_full_text_negative_cache_enabled: bool,
    #[serde(default = "default_full_text_negative_cache_ttl_secs")]
    ingestion_full_text_negative_cache_ttl_secs: u64,
    #[serde(default = "default_true")]
    ingestion_chunk_fingerprint_cache_enabled: bool,
    #[serde(default = "default_chunk_fingerprint_cache_ttl_secs")]
    ingestion_chunk_fingerprint_cache_ttl_secs: u64,
    #[serde(default = "default_true")]
    ingestion_heavy_lane_async_enabled: bool,
    #[serde(default = "default_min_ner_chars")]
    ingestion_min_ner_chars: u64,
    #[serde(default = "default_max_relation_genes_per_chunk")]
    ingestion_max_relation_genes_per_chunk: u64,
    #[serde(default = "default_true")]
    ingestion_async_post_ingest_scoring: bool,
    unpaywall_email: String,
    #[serde(default = "default_scihub_domains")]
    scihub_domains: String,
    #[serde(default = "default_scihub_request_timeout_secs")]
    scihub_request_timeout_secs: u64,
    openai_api_key: Option<String>,
    anthropic_api_key: Option<String>,
    gemini_api_key: Option<String>,
    compat_api_key: Option<String>,
    pubmed_api_key: Option<String>,
    semanticscholar_api_key: Option<String>,
    embedding_api_key: Option<String>,
}

fn default_true() -> bool {
    true
}
fn default_source_profile() -> String {
    "fast".to_string()
}
fn default_source_timeout_secs() -> u64 {
    18
}
fn default_full_text_step_timeout_secs() -> u64 {
    15
}
fn default_full_text_total_timeout_secs() -> u64 {
    28
}
fn default_full_text_prefetch_workers() -> u64 {
    4
}
fn default_paper_process_workers() -> u64 {
    4
}
fn default_perf_mode() -> String {
    "auto".to_string()
}
fn default_source_cache_ttl_secs() -> u64 {
    1800
}
fn default_entity_batch_size() -> u64 {
    256
}
fn default_fact_batch_size() -> u64 {
    512
}
fn default_source_max_inflight() -> u64 {
    4
}
fn default_source_retries() -> u64 {
    2
}
fn default_pdf_host_concurrency() -> u64 {
    4
}
fn default_full_text_negative_cache_ttl_secs() -> u64 {
    6 * 60 * 60
}
fn default_chunk_fingerprint_cache_ttl_secs() -> u64 {
    2 * 24 * 60 * 60
}
fn default_min_ner_chars() -> u64 {
    500
}
fn default_max_relation_genes_per_chunk() -> u64 {
    4
}
fn default_scihub_request_timeout_secs() -> u64 {
    10
}
fn default_scihub_domains() -> String {
    "https://sci-hub.al,https://sci-hub.mk,https://sci-hub.ee,https://sci-hub.vg,https://sci-hub.st,http://sci-hub.al,http://sci-hub.mk,http://sci-hub.ee,http://sci-hub.vg,http://sci-hub.st".to_string()
}

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

fn table_mut<'a>(
    root: &'a mut toml::Value,
    key: &str,
) -> &'a mut toml::map::Map<String, toml::Value> {
    let root_tbl = root.as_table_mut().expect("root TOML table");
    if !root_tbl.contains_key(key) {
        root_tbl.insert(key.to_string(), toml::Value::Table(toml::map::Map::new()));
    }
    root_tbl
        .get_mut(key)
        .and_then(|v| v.as_table_mut())
        .expect("child TOML table")
}

fn nested_table_mut<'a>(
    parent: &'a mut toml::map::Map<String, toml::Value>,
    key: &str,
) -> &'a mut toml::map::Map<String, toml::Value> {
    if !parent.contains_key(key) {
        parent.insert(key.to_string(), toml::Value::Table(toml::map::Map::new()));
    }
    parent
        .get_mut(key)
        .and_then(|v| v.as_table_mut())
        .expect("nested table")
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

fn int_at(root: &toml::Value, path: &[&str], default: u64) -> u64 {
    let mut cur = root;
    for p in path {
        match cur.get(*p) {
            Some(next) => cur = next,
            None => return default,
        }
    }
    cur.as_integer()
        .and_then(|v| if v >= 0 { Some(v as u64) } else { None })
        .unwrap_or(default)
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
        ollama_base_url: str_at(
            &root,
            &["llm", "ollama", "base_url"],
            "http://localhost:11434",
        ),
        ollama_model: str_at(&root, &["llm", "ollama", "model"], "llama3.1:8b"),
        openai_model: str_at(&root, &["llm", "openai", "model"], "gpt-4o-mini"),
        anthropic_model: str_at(&root, &["llm", "anthropic", "model"], "claude-haiku-4-5"),
        gemini_model: str_at(&root, &["llm", "gemini", "model"], "gemini-1.5-flash"),
        compat_base_url: str_at(
            &root,
            &["llm", "openai_compatible", "base_url"],
            "https://api.groq.com/openai",
        ),
        compat_model: str_at(
            &root,
            &["llm", "openai_compatible", "model"],
            "llama-3.3-70b-versatile",
        ),
        compat_cached_chat: bool_at(&root, &["llm", "openai_compatible", "cached_chat"], true),
        embedding_backend: str_at(&root, &["embedding", "backend"], "rust_native"),
        embedding_model: str_at(
            &root,
            &["embedding", "embedding_model"],
            "microsoft/BiomedNLP-BiomedBERT-base-uncased-abstract-fulltext",
        ),
        embedding_base_url: str_at(&root, &["embedding", "base_url"], ""),
        ingestion_default_max_results: int_at(&root, &["ingestion", "default_max_results"], 50),
        ingestion_idle_timeout_secs: int_at(
            &root,
            &["ingestion", "watchdog", "idle_timeout_secs"],
            600,
        ),
        ingestion_max_runtime_secs: int_at(
            &root,
            &["ingestion", "watchdog", "max_runtime_secs"],
            14_400,
        ),
        ingestion_enable_embeddings: bool_at(&root, &["ingestion", "enable_embeddings"], false),
        ingestion_source_profile: str_at(
            &root,
            &["ingestion", "performance", "source_profile"],
            "fast",
        ),
        ingestion_source_timeout_secs: int_at(
            &root,
            &["ingestion", "performance", "source_timeout_secs"],
            18,
        ),
        ingestion_full_text_step_timeout_secs: int_at(
            &root,
            &["ingestion", "performance", "full_text_step_timeout_secs"],
            15,
        ),
        ingestion_full_text_total_timeout_secs: int_at(
            &root,
            &["ingestion", "performance", "full_text_total_timeout_secs"],
            default_full_text_total_timeout_secs(),
        ),
        ingestion_full_text_prefetch_workers: int_at(
            &root,
            &["ingestion", "performance", "full_text_prefetch_workers"],
            4,
        ),
        ingestion_paper_process_workers: int_at(
            &root,
            &["ingestion", "performance", "paper_process_workers"],
            4,
        ),
        ingestion_perf_mode: str_at(&root, &["ingestion", "performance", "perf_mode"], "auto"),
        ingestion_source_cache_enabled: bool_at(
            &root,
            &["ingestion", "performance", "source_cache_enabled"],
            true,
        ),
        ingestion_source_cache_ttl_secs: int_at(
            &root,
            &["ingestion", "performance", "source_cache_ttl_secs"],
            1800,
        ),
        ingestion_entity_batch_size: int_at(
            &root,
            &["ingestion", "performance", "entity_batch_size"],
            256,
        ),
        ingestion_fact_batch_size: int_at(
            &root,
            &["ingestion", "performance", "fact_batch_size"],
            512,
        ),
        ingestion_strict_fuzzy_dedup: bool_at(
            &root,
            &["ingestion", "performance", "strict_fuzzy_dedup"],
            false,
        ),
        ingestion_source_max_inflight: int_at(
            &root,
            &["ingestion", "performance", "source_max_inflight"],
            default_source_max_inflight(),
        ),
        ingestion_source_retries: int_at(
            &root,
            &["ingestion", "performance", "source_retries"],
            default_source_retries(),
        ),
        ingestion_pdf_host_concurrency: int_at(
            &root,
            &["ingestion", "performance", "pdf_host_concurrency"],
            default_pdf_host_concurrency(),
        ),
        ingestion_pdf_parse_cache_enabled: bool_at(
            &root,
            &["ingestion", "performance", "pdf_parse_cache_enabled"],
            true,
        ),
        ingestion_full_text_negative_cache_enabled: bool_at(
            &root,
            &[
                "ingestion",
                "performance",
                "full_text_negative_cache_enabled",
            ],
            true,
        ),
        ingestion_full_text_negative_cache_ttl_secs: int_at(
            &root,
            &[
                "ingestion",
                "performance",
                "full_text_negative_cache_ttl_secs",
            ],
            default_full_text_negative_cache_ttl_secs(),
        ),
        ingestion_chunk_fingerprint_cache_enabled: bool_at(
            &root,
            &[
                "ingestion",
                "performance",
                "chunk_fingerprint_cache_enabled",
            ],
            true,
        ),
        ingestion_chunk_fingerprint_cache_ttl_secs: int_at(
            &root,
            &[
                "ingestion",
                "performance",
                "chunk_fingerprint_cache_ttl_secs",
            ],
            default_chunk_fingerprint_cache_ttl_secs(),
        ),
        ingestion_heavy_lane_async_enabled: bool_at(
            &root,
            &["ingestion", "performance", "heavy_lane_async_enabled"],
            true,
        ),
        ingestion_min_ner_chars: int_at(
            &root,
            &["ingestion", "performance", "min_ner_chars"],
            default_min_ner_chars(),
        ),
        ingestion_max_relation_genes_per_chunk: int_at(
            &root,
            &["ingestion", "performance", "max_relation_genes_per_chunk"],
            default_max_relation_genes_per_chunk(),
        ),
        ingestion_async_post_ingest_scoring: bool_at(
            &root,
            &["ingestion", "performance", "async_post_ingest_scoring"],
            true,
        ),
        unpaywall_email: {
            let toml_value = str_at(&root, &["ingestion", "unpaywall", "email"], "");
            if toml_value.trim().is_empty() {
                std::env::var("FERRUMYX_UNPAYWALL_EMAIL").unwrap_or_default()
            } else {
                toml_value
            }
        },
        scihub_domains: {
            let toml_value = str_at(&root, &["ingestion", "scihub", "domains"], "");
            if !toml_value.trim().is_empty() {
                toml_value
            } else {
                std::env::var("FERRUMYX_SCIHUB_DOMAINS")
                    .ok()
                    .filter(|v| !v.trim().is_empty())
                    .unwrap_or_else(default_scihub_domains)
            }
        },
        scihub_request_timeout_secs: {
            let toml_value = int_at(&root, &["ingestion", "scihub", "request_timeout_secs"], 0);
            if toml_value >= 4 {
                toml_value.clamp(4, 45)
            } else {
                std::env::var("FERRUMYX_SCIHUB_REQUEST_TIMEOUT_SECS")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(default_scihub_request_timeout_secs())
                    .clamp(4, 45)
            }
        },
        has_openai_key: has_nonempty(&root, &["llm", "openai", "api_key"])
            || std::env::var("FERRUMYX_OPENAI_API_KEY").is_ok(),
        has_anthropic_key: has_nonempty(&root, &["llm", "anthropic", "api_key"])
            || std::env::var("FERRUMYX_ANTHROPIC_API_KEY").is_ok(),
        has_gemini_key: has_nonempty(&root, &["llm", "gemini", "api_key"])
            || std::env::var("FERRUMYX_GEMINI_API_KEY").is_ok(),
        has_compat_key: has_nonempty(&root, &["llm", "openai_compatible", "api_key"])
            || std::env::var("FERRUMYX_COMPAT_API_KEY").is_ok(),
        has_pubmed_key: has_nonempty(&root, &["ingestion", "pubmed", "api_key"])
            || has_nonempty(&root, &["ingestion", "pubmed", "api_key_secret"])
            || std::env::var("FERRUMYX_PUBMED_API_KEY").is_ok(),
        has_semanticscholar_key: has_nonempty(&root, &["ingestion", "semanticscholar", "api_key"])
            || has_nonempty(&root, &["ingestion", "semanticscholar", "api_key_secret"])
            || std::env::var("FERRUMYX_SEMANTIC_SCHOLAR_API_KEY")
                .is_ok_and(|v| !v.trim().is_empty())
            || std::env::var("SEMANTIC_SCHOLAR_API_KEY").is_ok_and(|v| !v.trim().is_empty()),
        has_embedding_key: has_nonempty(&root, &["embedding", "api_key"]),
        ironclaw_sync: IronclawSyncView {
            llm_backend: std::env::var("LLM_BACKEND").unwrap_or_else(|_| "unset".to_string()),
            llm_base_url: std::env::var("LLM_BASE_URL").unwrap_or_default(),
            llm_model: std::env::var("LLM_MODEL").unwrap_or_default(),
            has_llm_api_key: std::env::var("LLM_API_KEY").is_ok_and(|v| !v.trim().is_empty()),
            has_openai_key: std::env::var("OPENAI_API_KEY").is_ok_and(|v| !v.trim().is_empty()),
            has_anthropic_key: std::env::var("ANTHROPIC_API_KEY")
                .is_ok_and(|v| !v.trim().is_empty()),
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
    ingestion.insert(
        "enable_embeddings".to_string(),
        toml::Value::Boolean(payload.ingestion_enable_embeddings),
    );
    ingestion.insert(
        "default_max_results".to_string(),
        toml::Value::Integer(payload.ingestion_default_max_results.clamp(1, 5000) as i64),
    );
    let watchdog = nested_table_mut(ingestion, "watchdog");
    watchdog.insert(
        "idle_timeout_secs".to_string(),
        toml::Value::Integer(payload.ingestion_idle_timeout_secs.clamp(60, 3600) as i64),
    );
    watchdog.insert(
        "max_runtime_secs".to_string(),
        toml::Value::Integer(payload.ingestion_max_runtime_secs.clamp(600, 86_400) as i64),
    );
    let performance = nested_table_mut(ingestion, "performance");
    set_str(
        performance,
        "source_profile",
        if payload.ingestion_source_profile.to_lowercase() == "full" {
            "full".to_string()
        } else {
            "fast".to_string()
        },
    );
    performance.insert(
        "source_timeout_secs".to_string(),
        toml::Value::Integer(payload.ingestion_source_timeout_secs.clamp(5, 300) as i64),
    );
    performance.insert(
        "full_text_step_timeout_secs".to_string(),
        toml::Value::Integer(payload.ingestion_full_text_step_timeout_secs.clamp(5, 120) as i64),
    );
    performance.insert(
        "full_text_total_timeout_secs".to_string(),
        toml::Value::Integer(payload.ingestion_full_text_total_timeout_secs.clamp(8, 180) as i64),
    );
    performance.insert(
        "full_text_prefetch_workers".to_string(),
        toml::Value::Integer(payload.ingestion_full_text_prefetch_workers.clamp(1, 32) as i64),
    );
    performance.insert(
        "paper_process_workers".to_string(),
        toml::Value::Integer(payload.ingestion_paper_process_workers.clamp(1, 16) as i64),
    );
    set_str(
        performance,
        "perf_mode",
        match payload.ingestion_perf_mode.to_lowercase().as_str() {
            "throughput" => "throughput".to_string(),
            "balanced" => "balanced".to_string(),
            "safe" => "safe".to_string(),
            _ => "auto".to_string(),
        },
    );
    performance.insert(
        "source_cache_enabled".to_string(),
        toml::Value::Boolean(payload.ingestion_source_cache_enabled),
    );
    performance.insert(
        "source_cache_ttl_secs".to_string(),
        toml::Value::Integer(payload.ingestion_source_cache_ttl_secs.clamp(60, 86_400) as i64),
    );
    performance.insert(
        "entity_batch_size".to_string(),
        toml::Value::Integer(payload.ingestion_entity_batch_size.clamp(16, 2048) as i64),
    );
    performance.insert(
        "fact_batch_size".to_string(),
        toml::Value::Integer(payload.ingestion_fact_batch_size.clamp(16, 4096) as i64),
    );
    performance.insert(
        "strict_fuzzy_dedup".to_string(),
        toml::Value::Boolean(payload.ingestion_strict_fuzzy_dedup),
    );
    performance.insert(
        "source_max_inflight".to_string(),
        toml::Value::Integer(payload.ingestion_source_max_inflight.clamp(1, 16) as i64),
    );
    performance.insert(
        "source_retries".to_string(),
        toml::Value::Integer(payload.ingestion_source_retries.clamp(0, 5) as i64),
    );
    performance.insert(
        "pdf_host_concurrency".to_string(),
        toml::Value::Integer(payload.ingestion_pdf_host_concurrency.clamp(1, 16) as i64),
    );
    performance.insert(
        "pdf_parse_cache_enabled".to_string(),
        toml::Value::Boolean(payload.ingestion_pdf_parse_cache_enabled),
    );
    performance.insert(
        "full_text_negative_cache_enabled".to_string(),
        toml::Value::Boolean(payload.ingestion_full_text_negative_cache_enabled),
    );
    performance.insert(
        "full_text_negative_cache_ttl_secs".to_string(),
        toml::Value::Integer(
            payload
                .ingestion_full_text_negative_cache_ttl_secs
                .clamp(60, 604_800) as i64,
        ),
    );
    performance.insert(
        "chunk_fingerprint_cache_enabled".to_string(),
        toml::Value::Boolean(payload.ingestion_chunk_fingerprint_cache_enabled),
    );
    performance.insert(
        "chunk_fingerprint_cache_ttl_secs".to_string(),
        toml::Value::Integer(
            payload
                .ingestion_chunk_fingerprint_cache_ttl_secs
                .clamp(300, 1_209_600) as i64,
        ),
    );
    performance.insert(
        "heavy_lane_async_enabled".to_string(),
        toml::Value::Boolean(payload.ingestion_heavy_lane_async_enabled),
    );
    performance.insert(
        "min_ner_chars".to_string(),
        toml::Value::Integer(payload.ingestion_min_ner_chars.clamp(120, 5000) as i64),
    );
    performance.insert(
        "max_relation_genes_per_chunk".to_string(),
        toml::Value::Integer(payload.ingestion_max_relation_genes_per_chunk.clamp(1, 16) as i64),
    );
    performance.insert(
        "async_post_ingest_scoring".to_string(),
        toml::Value::Boolean(payload.ingestion_async_post_ingest_scoring),
    );
    let pubmed = nested_table_mut(ingestion, "pubmed");
    maybe_set_secret(pubmed, "api_key", &payload.pubmed_api_key);
    maybe_set_secret(pubmed, "api_key_secret", &payload.pubmed_api_key);
    let unpaywall = nested_table_mut(ingestion, "unpaywall");
    set_str(
        unpaywall,
        "email",
        payload.unpaywall_email.trim().to_string(),
    );
    let scihub = nested_table_mut(ingestion, "scihub");
    let domains = payload
        .scihub_domains
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(",");
    set_str(
        scihub,
        "domains",
        if domains.is_empty() {
            default_scihub_domains()
        } else {
            domains
        },
    );
    scihub.insert(
        "request_timeout_secs".to_string(),
        toml::Value::Integer(payload.scihub_request_timeout_secs.clamp(4, 45) as i64),
    );
    let semanticscholar = nested_table_mut(ingestion, "semanticscholar");
    maybe_set_secret(semanticscholar, "api_key", &payload.semanticscholar_api_key);
    maybe_set_secret(
        semanticscholar,
        "api_key_secret",
        &payload.semanticscholar_api_key,
    );

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

    let ingestion_default_max_results = int_at(root, &["ingestion", "default_max_results"], 50);
    let ingestion_idle_timeout_secs =
        int_at(root, &["ingestion", "watchdog", "idle_timeout_secs"], 600);
    let ingestion_max_runtime_secs =
        int_at(root, &["ingestion", "watchdog", "max_runtime_secs"], 14_400);
    std::env::set_var(
        "FERRUMYX_INGESTION_DEFAULT_MAX_RESULTS",
        ingestion_default_max_results.to_string(),
    );
    std::env::set_var(
        "FERRUMYX_INGESTION_IDLE_TIMEOUT_SECS",
        ingestion_idle_timeout_secs.to_string(),
    );
    std::env::set_var(
        "FERRUMYX_INGESTION_MAX_RUNTIME_SECS",
        ingestion_max_runtime_secs.to_string(),
    );
    let ingestion_enable_embeddings = bool_at(root, &["ingestion", "enable_embeddings"], false);
    std::env::set_var(
        "FERRUMYX_INGESTION_ENABLE_EMBEDDINGS",
        if ingestion_enable_embeddings {
            "1"
        } else {
            "0"
        },
    );
    let ingestion_source_profile = str_at(
        root,
        &["ingestion", "performance", "source_profile"],
        "fast",
    );
    std::env::set_var(
        "FERRUMYX_INGESTION_SOURCE_PROFILE",
        ingestion_source_profile,
    );
    let ingestion_source_timeout_secs = int_at(
        root,
        &["ingestion", "performance", "source_timeout_secs"],
        18,
    );
    std::env::set_var(
        "FERRUMYX_INGESTION_SOURCE_TIMEOUT_SECS",
        ingestion_source_timeout_secs.to_string(),
    );
    let ingestion_full_text_step_timeout_secs = int_at(
        root,
        &["ingestion", "performance", "full_text_step_timeout_secs"],
        15,
    );
    std::env::set_var(
        "FERRUMYX_INGESTION_FULLTEXT_STEP_TIMEOUT_SECS",
        ingestion_full_text_step_timeout_secs.to_string(),
    );
    let ingestion_full_text_total_timeout_secs = int_at(
        root,
        &["ingestion", "performance", "full_text_total_timeout_secs"],
        default_full_text_total_timeout_secs(),
    );
    std::env::set_var(
        "FERRUMYX_INGESTION_FULLTEXT_TOTAL_TIMEOUT_SECS",
        ingestion_full_text_total_timeout_secs
            .clamp(8, 180)
            .to_string(),
    );
    let ingestion_full_text_prefetch_workers = int_at(
        root,
        &["ingestion", "performance", "full_text_prefetch_workers"],
        4,
    );
    std::env::set_var(
        "FERRUMYX_INGESTION_FULLTEXT_PREFETCH_WORKERS",
        ingestion_full_text_prefetch_workers.to_string(),
    );
    let ingestion_paper_process_workers = int_at(
        root,
        &["ingestion", "performance", "paper_process_workers"],
        4,
    );
    std::env::set_var(
        "FERRUMYX_PAPER_PROCESS_WORKERS",
        ingestion_paper_process_workers.to_string(),
    );
    let ingestion_perf_mode = str_at(root, &["ingestion", "performance", "perf_mode"], "auto");
    std::env::set_var("FERRUMYX_INGESTION_PERF_MODE", ingestion_perf_mode);
    let ingestion_source_cache_enabled = bool_at(
        root,
        &["ingestion", "performance", "source_cache_enabled"],
        true,
    );
    std::env::set_var(
        "FERRUMYX_INGESTION_SOURCE_CACHE_ENABLED",
        if ingestion_source_cache_enabled {
            "1"
        } else {
            "0"
        },
    );
    let ingestion_source_cache_ttl_secs = int_at(
        root,
        &["ingestion", "performance", "source_cache_ttl_secs"],
        1800,
    );
    std::env::set_var(
        "FERRUMYX_INGESTION_SOURCE_CACHE_TTL_SECS",
        ingestion_source_cache_ttl_secs.to_string(),
    );
    let ingestion_entity_batch_size = int_at(
        root,
        &["ingestion", "performance", "entity_batch_size"],
        256,
    );
    std::env::set_var(
        "FERRUMYX_INGESTION_ENTITY_BATCH_SIZE",
        ingestion_entity_batch_size.to_string(),
    );
    let ingestion_fact_batch_size =
        int_at(root, &["ingestion", "performance", "fact_batch_size"], 512);
    std::env::set_var(
        "FERRUMYX_INGESTION_FACT_BATCH_SIZE",
        ingestion_fact_batch_size.to_string(),
    );
    let ingestion_strict_fuzzy_dedup = bool_at(
        root,
        &["ingestion", "performance", "strict_fuzzy_dedup"],
        false,
    );
    std::env::set_var(
        "FERRUMYX_STRICT_FUZZY_DEDUP",
        if ingestion_strict_fuzzy_dedup {
            "1"
        } else {
            "0"
        },
    );
    let ingestion_source_max_inflight = int_at(
        root,
        &["ingestion", "performance", "source_max_inflight"],
        default_source_max_inflight(),
    );
    std::env::set_var(
        "FERRUMYX_INGESTION_SOURCE_MAX_INFLIGHT",
        ingestion_source_max_inflight.clamp(1, 16).to_string(),
    );
    let ingestion_source_retries = int_at(
        root,
        &["ingestion", "performance", "source_retries"],
        default_source_retries(),
    );
    std::env::set_var(
        "FERRUMYX_INGESTION_SOURCE_RETRIES",
        ingestion_source_retries.clamp(0, 5).to_string(),
    );
    let ingestion_pdf_host_concurrency = int_at(
        root,
        &["ingestion", "performance", "pdf_host_concurrency"],
        default_pdf_host_concurrency(),
    );
    std::env::set_var(
        "FERRUMYX_PDF_HOST_CONCURRENCY",
        ingestion_pdf_host_concurrency.clamp(1, 16).to_string(),
    );
    let ingestion_pdf_parse_cache_enabled = bool_at(
        root,
        &["ingestion", "performance", "pdf_parse_cache_enabled"],
        true,
    );
    std::env::set_var(
        "FERRUMYX_PDF_PARSE_CACHE_ENABLED",
        if ingestion_pdf_parse_cache_enabled {
            "1"
        } else {
            "0"
        },
    );
    let ingestion_full_text_negative_cache_enabled = bool_at(
        root,
        &[
            "ingestion",
            "performance",
            "full_text_negative_cache_enabled",
        ],
        true,
    );
    std::env::set_var(
        "FERRUMYX_FULLTEXT_NEGATIVE_CACHE_ENABLED",
        if ingestion_full_text_negative_cache_enabled {
            "1"
        } else {
            "0"
        },
    );
    let ingestion_full_text_negative_cache_ttl_secs = int_at(
        root,
        &[
            "ingestion",
            "performance",
            "full_text_negative_cache_ttl_secs",
        ],
        default_full_text_negative_cache_ttl_secs(),
    );
    std::env::set_var(
        "FERRUMYX_FULLTEXT_NEGATIVE_CACHE_TTL_SECS",
        ingestion_full_text_negative_cache_ttl_secs
            .clamp(60, 604_800)
            .to_string(),
    );
    let ingestion_chunk_fingerprint_cache_enabled = bool_at(
        root,
        &["ingestion", "performance", "chunk_fingerprint_cache_enabled"],
        true,
    );
    std::env::set_var(
        "FERRUMYX_CHUNK_FINGERPRINT_CACHE_ENABLED",
        if ingestion_chunk_fingerprint_cache_enabled {
            "1"
        } else {
            "0"
        },
    );
    let ingestion_chunk_fingerprint_cache_ttl_secs = int_at(
        root,
        &["ingestion", "performance", "chunk_fingerprint_cache_ttl_secs"],
        default_chunk_fingerprint_cache_ttl_secs(),
    );
    std::env::set_var(
        "FERRUMYX_CHUNK_FINGERPRINT_CACHE_TTL_SECS",
        ingestion_chunk_fingerprint_cache_ttl_secs
            .clamp(300, 1_209_600)
            .to_string(),
    );
    let ingestion_heavy_lane_async_enabled = bool_at(
        root,
        &["ingestion", "performance", "heavy_lane_async_enabled"],
        true,
    );
    std::env::set_var(
        "FERRUMYX_INGESTION_HEAVY_LANE_ASYNC",
        if ingestion_heavy_lane_async_enabled {
            "1"
        } else {
            "0"
        },
    );
    let ingestion_min_ner_chars = int_at(
        root,
        &["ingestion", "performance", "min_ner_chars"],
        default_min_ner_chars(),
    );
    std::env::set_var(
        "FERRUMYX_INGESTION_MIN_NER_CHARS",
        ingestion_min_ner_chars.clamp(120, 5000).to_string(),
    );
    let ingestion_max_relation_genes_per_chunk = int_at(
        root,
        &["ingestion", "performance", "max_relation_genes_per_chunk"],
        default_max_relation_genes_per_chunk(),
    );
    std::env::set_var(
        "FERRUMYX_INGESTION_MAX_RELATION_GENES_PER_CHUNK",
        ingestion_max_relation_genes_per_chunk
            .clamp(1, 16)
            .to_string(),
    );
    let ingestion_async_post_ingest_scoring = bool_at(
        root,
        &["ingestion", "performance", "async_post_ingest_scoring"],
        true,
    );
    std::env::set_var(
        "FERRUMYX_INGESTION_ASYNC_POST_SCORE",
        if ingestion_async_post_ingest_scoring {
            "1"
        } else {
            "0"
        },
    );

    let pubmed_key = str_at(root, &["ingestion", "pubmed", "api_key"], "");
    if !pubmed_key.is_empty() {
        std::env::set_var("FERRUMYX_PUBMED_API_KEY", &pubmed_key);
    }
    let unpaywall_email = str_at(root, &["ingestion", "unpaywall", "email"], "");
    if !unpaywall_email.is_empty() {
        std::env::set_var("FERRUMYX_UNPAYWALL_EMAIL", &unpaywall_email);
    }
    let scihub_domains = str_at(root, &["ingestion", "scihub", "domains"], "");
    if !scihub_domains.trim().is_empty() {
        std::env::set_var("FERRUMYX_SCIHUB_DOMAINS", scihub_domains);
    } else {
        std::env::set_var("FERRUMYX_SCIHUB_DOMAINS", default_scihub_domains());
    }
    let scihub_timeout_secs = int_at(
        root,
        &["ingestion", "scihub", "request_timeout_secs"],
        default_scihub_request_timeout_secs(),
    );
    std::env::set_var(
        "FERRUMYX_SCIHUB_REQUEST_TIMEOUT_SECS",
        scihub_timeout_secs.clamp(4, 45).to_string(),
    );

    let semanticscholar_key = {
        let k = str_at(root, &["ingestion", "semanticscholar", "api_key"], "");
        if !k.is_empty() {
            k
        } else {
            str_at(
                root,
                &["ingestion", "semanticscholar", "api_key_secret"],
                "",
            )
        }
    };
    if !semanticscholar_key.is_empty() {
        std::env::set_var("FERRUMYX_SEMANTIC_SCHOLAR_API_KEY", &semanticscholar_key);
        std::env::set_var("SEMANTIC_SCHOLAR_API_KEY", &semanticscholar_key);
    }
}
