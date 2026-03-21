//! Ingestion pipeline monitor and trigger — wired to real pipeline.

use axum::{extract::State, response::Html, Form};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;

use ferrumyx_ingestion::embedding::{
    EmbeddingBackend as IngestionEmbeddingBackend, EmbeddingConfig as IngestionEmbeddingConfig,
};
use ferrumyx_ingestion::pipeline::{run_ingestion, IngestionJob, IngestionSourceSpec};
use ferrumyx_ingestion::repository::IngestionRepository;

use crate::handlers::dashboard::NAV_HTML;
use crate::state::{AppEvent, SharedState};

// ── Form input ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct IngestionForm {
    pub gene: String,
    pub mutation: Option<String>,
    pub cancer: String,
    pub max_results: Option<usize>,
    /// Sources checked in form
    pub src_pubmed: Option<String>,
    pub src_europepmc: Option<String>,
    pub src_biorxiv: Option<String>,
    pub src_medrxiv: Option<String>,
    pub src_arxiv: Option<String>,
    pub src_clinicaltrials: Option<String>,
    pub src_crossref: Option<String>,
    pub src_semanticscholar: Option<String>,
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
    let mut sources = Vec::new();
    if form.src_pubmed.is_some() {
        sources.push(IngestionSourceSpec::PubMed);
    }
    if form.src_europepmc.is_some() {
        sources.push(IngestionSourceSpec::EuropePmc);
    }
    if form.src_biorxiv.is_some() {
        sources.push(IngestionSourceSpec::BioRxiv);
    }
    if form.src_medrxiv.is_some() {
        sources.push(IngestionSourceSpec::MedRxiv);
    }
    if form.src_arxiv.is_some() {
        sources.push(IngestionSourceSpec::Arxiv);
    }
    if form.src_clinicaltrials.is_some() {
        sources.push(IngestionSourceSpec::ClinicalTrials);
    }
    if form.src_crossref.is_some() {
        sources.push(IngestionSourceSpec::CrossRef);
    }
    if form.src_semanticscholar.is_some() {
        sources.push(IngestionSourceSpec::SemanticScholar);
    }

    if sources.is_empty() {
        sources.push(IngestionSourceSpec::PubMed);
    }

    let job = IngestionJob {
        gene: form.gene.clone(),
        mutation: form.mutation.clone().filter(|m| !m.is_empty()),
        cancer_type: form.cancer.clone(),
        max_results: form.max_results.unwrap_or(100),
        sources,
        pubmed_api_key: resolve_pubmed_api_key(),
        semantic_scholar_api_key: resolve_semantic_scholar_api_key(),
        unpaywall_email: resolve_unpaywall_email(),
        embedding_cfg: resolve_embedding_cfg_for_form(&form),
        enable_scihub_fallback: form.enable_scihub.is_some()
            && form.enable_scihub.as_deref() == Some("on"),
        full_text_enabled: true,
        source_timeout_secs: Some(45),
        full_text_step_timeout_secs: Some(15),
        full_text_prefetch_workers: None,
        source_cache_enabled: true,
        source_cache_ttl_secs: Some(30 * 60),
    };

    // Emit SSE start event immediately
    let _ = state.event_tx.send(AppEvent::PipelineStatus {
        stage: "search".to_string(),
        message: format!(
            "Starting ingestion: {} {} in {}",
            job.gene,
            job.mutation.as_deref().unwrap_or(""),
            job.cancer_type
        ),
        count: 0,
    });

    // Spawn ingestion in background task so we can return immediately
    let event_tx = state.event_tx.clone();
    let db = state.db.clone();

    tokio::spawn(async move {
        let repo = Arc::new(IngestionRepository::new(db));

        // Update progress before starting
        let _ = event_tx.send(AppEvent::PipelineStatus {
            stage: "searching".to_string(),
            message: "Searching PubMed and Europe PMC...".to_string(),
            count: 0,
        });

        let result = run_ingestion(job, repo, None).await;

        // Emit SSE completion events
        let _ = event_tx.send(AppEvent::PipelineStatus {
            stage: "complete".to_string(),
            message: format!(
                "Ingestion complete — {} papers found, {} inserted, {} chunks",
                result.papers_found, result.papers_inserted, result.chunks_inserted
            ),
            count: result.papers_inserted as u64,
        });

        // Also emit individual PaperIngested events
        for i in 0..result.papers_inserted.min(10) {
            let _ = event_tx.send(AppEvent::PaperIngested {
                paper_id: format!("{}-{}", result.job_id, i),
                title: format!("Paper #{} ingested from {}", i + 1, result.query),
                source: "ingestion".to_string(),
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

    Html(render_page_with_progress(
        stats,
        &summary,
        form.max_results.unwrap_or(100) as i64,
    ))
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
    let repo = IngestionRepository::new(state.db.clone());
    let total = repo.paper_count().await.unwrap_or(0);
    let parsed = repo.paper_count_by_status("parsed").await.unwrap_or(0)
        + repo.paper_count_by_status("parsed_fast").await.unwrap_or(0)
        + repo
            .paper_count_by_status("parsed_light")
            .await
            .unwrap_or(0);
    let pending = repo.paper_count_by_status("pending").await.unwrap_or(0)
        + repo.paper_count_by_status("processing").await.unwrap_or(0);
    let failed = repo.paper_count_by_status("failed").await.unwrap_or(0);

    PageStats {
        total,
        parsed,
        pending,
        failed,
        recent_audit: vec![],
    }
}

fn resolve_pubmed_api_key() -> Option<String> {
    if let Ok(v) = std::env::var("FERRUMYX_PUBMED_API_KEY") {
        let t = v.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    let path = std::env::var("FERRUMYX_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("ferrumyx.toml"));
    let content = std::fs::read_to_string(path).ok()?;
    let root = toml::from_str::<toml::Value>(&content).ok()?;
    root.get("ingestion")
        .and_then(|v| v.get("pubmed"))
        .and_then(|v| v.get("api_key").or_else(|| v.get("api_key_secret")))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

fn resolve_semantic_scholar_api_key() -> Option<String> {
    if let Ok(v) = std::env::var("FERRUMYX_SEMANTIC_SCHOLAR_API_KEY") {
        let t = v.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    if let Ok(v) = std::env::var("SEMANTIC_SCHOLAR_API_KEY") {
        let t = v.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    let path = std::env::var("FERRUMYX_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("ferrumyx.toml"));
    let content = std::fs::read_to_string(path).ok()?;
    let root = toml::from_str::<toml::Value>(&content).ok()?;
    root.get("ingestion")
        .and_then(|v| v.get("semanticscholar"))
        .and_then(|v| v.get("api_key").or_else(|| v.get("api_key_secret")))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

fn resolve_unpaywall_email() -> Option<String> {
    if let Ok(v) = std::env::var("FERRUMYX_UNPAYWALL_EMAIL") {
        let t = v.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    let path = std::env::var("FERRUMYX_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("ferrumyx.toml"));
    let content = std::fs::read_to_string(path).ok()?;
    let root = toml::from_str::<toml::Value>(&content).ok()?;
    root.get("ingestion")
        .and_then(|v| v.get("unpaywall"))
        .and_then(|v| v.get("email"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

fn toml_string(root: &toml::Value, path: &[&str]) -> Option<String> {
    let mut cur = root;
    for p in path {
        cur = cur.get(*p)?;
    }
    cur.as_str()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn toml_u64(root: &toml::Value, path: &[&str], default: u64) -> u64 {
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

fn toml_bool(root: &toml::Value, path: &[&str], default: bool) -> bool {
    let mut cur = root;
    for p in path {
        match cur.get(*p) {
            Some(next) => cur = next,
            None => return default,
        }
    }
    cur.as_bool().unwrap_or(default)
}

fn parse_embedding_backend(s: &str) -> IngestionEmbeddingBackend {
    match s.trim().to_lowercase().as_str() {
        "openai" => IngestionEmbeddingBackend::OpenAi,
        "gemini" => IngestionEmbeddingBackend::Gemini,
        "openai_compatible" | "compat" => IngestionEmbeddingBackend::OpenAiCompatible,
        "ollama" => IngestionEmbeddingBackend::Ollama,
        "biomedbert" => IngestionEmbeddingBackend::BiomedBert,
        _ => IngestionEmbeddingBackend::RustNative,
    }
}

fn resolve_embedding_cfg_for_form(form: &IngestionForm) -> Option<IngestionEmbeddingConfig> {
    let path = std::env::var("FERRUMYX_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("ferrumyx.toml"));
    let root = std::fs::read_to_string(path)
        .ok()
        .and_then(|content| toml::from_str::<toml::Value>(&content).ok());

    let default_enabled = root
        .as_ref()
        .map(|r| toml_bool(r, &["ingestion", "enable_embeddings"], false))
        .unwrap_or(false);

    let explicit_backend = form
        .embed_backend
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    if explicit_backend.is_none() && !default_enabled {
        return None;
    }

    let backend_str = explicit_backend
        .map(ToString::to_string)
        .or_else(|| {
            root.as_ref()
                .and_then(|r| toml_string(r, &["embedding", "backend"]))
        })
        .unwrap_or_else(|| "rust_native".to_string());
    let backend = parse_embedding_backend(&backend_str);

    let model = form
        .embed_model
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            root.as_ref()
                .and_then(|r| toml_string(r, &["embedding", "embedding_model"]))
        })
        .unwrap_or_else(|| {
            "microsoft/BiomedNLP-BiomedBERT-base-uncased-abstract-fulltext".to_string()
        });

    let base_url = root
        .as_ref()
        .and_then(|r| toml_string(r, &["embedding", "base_url"]));
    let batch_size = root
        .as_ref()
        .map(|r| toml_u64(r, &["embedding", "batch_size"], 32))
        .unwrap_or(32)
        .clamp(1, 256) as usize;
    let dim = root
        .as_ref()
        .map(|r| {
            toml_u64(
                r,
                &["embedding", "embedding_dim"],
                if matches!(
                    backend,
                    IngestionEmbeddingBackend::RustNative | IngestionEmbeddingBackend::BiomedBert
                ) {
                    768
                } else {
                    1536
                },
            )
        })
        .unwrap_or(
            if matches!(
                backend,
                IngestionEmbeddingBackend::RustNative | IngestionEmbeddingBackend::BiomedBert
            ) {
                768
            } else {
                1536
            },
        )
        .clamp(64, 8192) as usize;

    let api_key = form
        .embed_api_key
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            root.as_ref()
                .and_then(|r| toml_string(r, &["embedding", "api_key"]))
        })
        .or_else(|| match backend {
            IngestionEmbeddingBackend::OpenAi => std::env::var("FERRUMYX_OPENAI_API_KEY")
                .ok()
                .or_else(|| std::env::var("OPENAI_API_KEY").ok()),
            IngestionEmbeddingBackend::Gemini => std::env::var("FERRUMYX_GEMINI_API_KEY")
                .ok()
                .or_else(|| std::env::var("GEMINI_API_KEY").ok()),
            IngestionEmbeddingBackend::OpenAiCompatible => std::env::var("FERRUMYX_COMPAT_API_KEY")
                .ok()
                .or_else(|| std::env::var("LLM_API_KEY").ok()),
            _ => None,
        })
        .filter(|v| !v.trim().is_empty());

    Some(IngestionEmbeddingConfig {
        backend,
        api_key,
        model,
        dim,
        batch_size,
        base_url,
    })
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
        let alert_class = if is_running {
            "alert-info"
        } else {
            "alert-success"
        };
        format!(
            r#"
        <div class="alert {} alert-dismissible mt-3">
            {}
            <button type="button" class="btn-close" data-bs-dismiss="alert"></button>
        </div>"#,
            alert_class, summary
        )
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

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Ingestion — Ferrumyx</title>
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@400;500;600;700&family=Inter:wght@400;500;600&display=swap" rel="stylesheet">
    <link rel="stylesheet" href="/static/css/main.css?v=1.0.3">
    <style>
        .sse-live {{ display: inline-flex; align-items: center; gap: 6px; }}
        .sse-dot {{ width: 8px; height: 8px; border-radius: 50%; background: var(--success); animation: pulse 2s infinite; }}
        .source-grid {{ display:grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap:10px; }}
        .source-chip {{ display:flex; align-items:center; gap:0.5rem; background:var(--bg-hover); padding:0.6rem 0.75rem; border-radius:8px; border:1px solid var(--border-glass); cursor:pointer; }}
        .pipeline-card-body {{ padding: 0.85rem 1rem 1rem; }}
        .pipeline-meta {{ display:grid; grid-template-columns:repeat(3, minmax(0,1fr)); gap:0.75rem; margin-top: 0.95rem; }}
        .pipeline-meta-item {{ background: rgba(15, 23, 42, 0.5); border: 1px solid var(--border-glass); border-radius: 8px; padding: 0.55rem 0.7rem; }}
        .pipeline-meta-label {{ font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.04em; color: var(--text-muted); }}
        .pipeline-meta-value {{ font-size: 0.92rem; color: var(--text-main); margin-top: 0.2rem; }}
        .run-grid {{ display:grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 0.8rem 0.9rem; }}
        .advanced-block {{ border-top:1px solid var(--border-glass); margin-top:12px; padding-top:12px; }}
        .advanced-block summary {{ cursor:pointer; color:var(--text-muted); font-weight:600; }}
        .status-line {{ margin-top: 0.75rem; margin-bottom: 0; font-family: monospace; font-size: 0.9rem; color:var(--brand-cyan); line-height: 1.45; }}
        @media (max-width: 1200px) {{
            .pipeline-meta {{ grid-template-columns: 1fr; }}
            .source-grid {{ grid-template-columns: repeat(2, minmax(0, 1fr)); }}
            .run-grid {{ grid-template-columns: 1fr; }}
        }}
        @keyframes pulse {{ 0% {{ opacity: 1; }} 50% {{ opacity: 0.4; }} 100% {{ opacity: 1; }} }}
    </style>
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">
                <svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" viewBox="0 0 24 24"><path d="M19 9h-4V3H9v6H5l7 7 7-7zM5 18v2h14v-2H5z"/></svg>
                Ingestion Pipeline
            </h1>
            <p class="text-muted">Run literature ingestion jobs and monitor progress in real time.</p>
        </div>
    </div>

    {}

    <div class="stats-grid mt-4">
        <div class="stat-card card-hover">
            <div class="stat-icon"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M14 2H6c-1.1 0-1.99.9-1.99 2L4 20c0 1.1.89 2 1.99 2H18c1.1 0 2-.9 2-2V8l-6-6zm2 16H8v-2h8v2zm0-4H8v-2h8v2zm-3-5V3.5L18.5 9H13z"/></svg></div>
            <div class="stat-value text-gradient" id="papers-count">{}</div><div class="stat-label">Total Literature</div></div>
        <div class="stat-card card-hover" style="border-bottom: 2px solid var(--success);">
            <div class="stat-icon"><svg stroke="var(--success)" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/></svg></div>
            <div class="stat-value" style="color:var(--success)">{}</div><div class="stat-label">Parsed Successfully (full+fast+light)</div></div>
        <div class="stat-card card-hover" style="border-bottom: 2px solid var(--warning);">
            <div class="stat-icon"><svg stroke="var(--warning)" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M11.99 2C6.47 2 2 6.48 2 12s4.47 10 9.99 10C17.52 22 22 17.52 22 12S17.52 2 11.99 2zM12 20c-4.42 0-8-3.58-8-8s3.58-8 8-8 8 3.58 8 8-3.58 8-8 8zm.5-13H11v6l5.25 3.15.75-1.23-4.5-2.67z"/></svg></div>
            <div class="stat-value" style="color:var(--warning)">{}</div><div class="stat-label">Pending Queues (pending+processing)</div></div>
        <div class="stat-card card-hover" style="border-bottom: 2px solid var(--danger);">
            <div class="stat-icon"><svg stroke="var(--danger)" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/></svg></div>
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
        <div class="pipeline-card-body">
            <div class="progress-track" style="height: 12px; margin-bottom: 1rem;">
                <div id="pipeline-progress" class="progress-bar brand" style="width: 5%">
                </div>
            </div>
            <div class="pipeline-meta">
                <div class="pipeline-meta-item">
                    <div class="pipeline-meta-label">Vectors Decoded</div>
                    <div class="pipeline-meta-value" id="papers-found">0</div>
                </div>
                <div class="pipeline-meta-item">
                    <div class="pipeline-meta-label">Inserted</div>
                    <div class="pipeline-meta-value" id="papers-inserted">0 / <span id="progress-text">5%</span></div>
                </div>
                <div class="pipeline-meta-item">
                    <div class="pipeline-meta-label">Remaining</div>
                    <div class="pipeline-meta-value" id="papers-remaining">{}</div>
                </div>
            </div>
            <p id="pipeline-status-text" class="status-line">> Initiating search sequence for PubMed and Europe PMC interfaces...</p>
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
                    document.getElementById('papers-inserted').innerHTML = papersInserted + ' / <span id="progress-text"></span>';
                    const percent = Math.min(100, Math.round((papersInserted / totalExpected) * 100));
                    document.getElementById('pipeline-progress').style.width = percent + '%';
                    document.getElementById('progress-text').textContent = percent + '%';
                    document.getElementById('papers-remaining').textContent = Math.max(0, totalExpected - papersInserted);
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
            <div class="card-header">Start Ingestion Job</div>
            <form method="POST" action="/ingestion/run" class="d-flex flex-column gap-3">
                <div class="run-grid">
                    <div>
                        <label class="form-label">Target Gene</label>
                        <input type="text" name="gene" class="form-control" placeholder="e.g. EGFR, TP53, BRAF" required>
                    </div>
                    <div>
                        <label class="form-label">Mutation Filter</label>
                        <input type="text" name="mutation" class="form-control" placeholder="G12D (optional)">
                    </div>
                    <div>
                        <label class="form-label">Cancer Context</label>
                        <input type="text" name="cancer" class="form-control" placeholder="e.g. NSCLC, colorectal cancer" required>
                    </div>
                    <div>
                        <label class="form-label">Max Papers</label>
                        <input type="number" name="max_results" class="form-control" value="100" min="10" max="1000">
                    </div>
                </div>
                <div class="mt-2">
                    <label class="form-label">Sources</label>
                    <div class="source-grid mt-1">
                        <label class="source-chip">
                            <input type="checkbox" name="src_pubmed" id="src_pubmed" checked> <span style="font-weight:500">PubMed</span>
                        </label>
                        <label class="source-chip">
                            <input type="checkbox" name="src_europepmc" id="src_europepmc" checked> <span style="font-weight:500">Europe PMC</span>
                        </label>
                        <label class="source-chip">
                            <input type="checkbox" name="src_biorxiv" id="src_biorxiv"> <span style="font-weight:500">bioRxiv</span>
                        </label>
                        <label class="source-chip">
                            <input type="checkbox" name="src_medrxiv" id="src_medrxiv"> <span style="font-weight:500">medRxiv</span>
                        </label>
                        <label class="source-chip">
                            <input type="checkbox" name="src_arxiv" id="src_arxiv"> <span style="font-weight:500">arXiv</span>
                        </label>
                        <label class="source-chip">
                            <input type="checkbox" name="src_clinicaltrials" id="src_clinicaltrials"> <span style="font-weight:500">ClinicalTrials</span>
                        </label>
                        <label class="source-chip">
                            <input type="checkbox" name="src_crossref" id="src_crossref"> <span style="font-weight:500">CrossRef</span>
                        </label>
                        <label class="source-chip">
                            <input type="checkbox" name="src_semanticscholar" id="src_semanticscholar"> <span style="font-weight:500">Semantic Scholar</span>
                        </label>
                    </div>
                </div>
                <details class="advanced-block">
                    <summary>Advanced Options</summary>
                    <div class="mt-2">
                        <label style="display:flex; align-items:center; gap:0.5rem; cursor:pointer;">
                            <input type="checkbox" name="enable_scihub" id="enable_scihub"> <span style="font-weight:500; color: var(--brand-purple);">Enable Sci-Hub fallback for full-text retrieval</span>
                        </label>
                    </div>
                </details>
                <div class="mt-4 pt-4" style="border-top:1px solid var(--border-glass)">
                    <button type="submit" class="btn btn-primary w-100" style="padding: 1rem; font-size: 1.1rem; justify-content:center;">
                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M8 5v14l11-7z"/></svg>
                        Run Ingestion
                    </button>
                    <div class="text-muted text-center mt-2 small">Live status updates stream automatically while the job runs.</div>
                </div>
            </form>
        </div>

        <div class="card">
            <div class="card-header d-flex justify-between">
                <div>Recent Jobs</div>
                <a href="/audit" class="btn btn-sm btn-outline">View Logs</a>
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
<script src="/static/js/main.js"></script>
</body>
</html>"#,
        NAV_HTML,
        banner,
        stats.total,
        stats.parsed,
        stats.pending,
        stats.failed,
        progress_display,
        total_expected,
        total_expected,
        audit_rows
    )
}
