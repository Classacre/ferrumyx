//! Ferrumyx — Autonomous Oncology Drug Discovery Engine
//! Entry point for the agent binary.

use std::collections::{HashMap, HashSet};
use std::io::IsTerminal;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

mod config;
mod tools;
use rig::client::CompletionClient;
use rig::providers::anthropic::Client as AnthropicClient;
use rig::providers::gemini::Client as GeminiClient;
use rig::providers::openai::Client as OpenAiClient;
use rig::providers::openai::CompletionsClient as OpenAiCompletionsClient;
use serde::Deserialize;
use tokio::process::Command;

/// Returns a Boxed CompletionModel to inject into the Agent.
/// It natively maps the Ferrumyx config directly to `rig-core` LLM clients.
async fn build_completion_model(
    config: &config::Config,
) -> anyhow::Result<Arc<dyn ironclaw::llm::LlmProvider>> {
    let mode = config.llm.mode.to_lowercase();
    let default_backend = config.llm.default_backend.to_lowercase();

    if mode == "local_only" || mode == "prefer_local" || default_backend == "ollama" {
        if let Some(provider) = try_build_ollama(config).await? {
            return Ok(provider);
        }
    }

    if default_backend == "openai" {
        if let Some(provider) = try_build_openai(config)? {
            return Ok(provider);
        }
    }
    if default_backend == "anthropic" {
        if let Some(provider) = try_build_anthropic(config)? {
            return Ok(provider);
        }
    }
    if default_backend == "gemini" {
        if let Some(provider) = try_build_gemini(config)? {
            return Ok(provider);
        }
    }
    if default_backend == "openai_compatible" {
        if let Some(provider) = try_build_openai_compatible(config)? {
            return Ok(provider);
        }
    }
    if default_backend == "ollama" {
        if let Some(provider) = try_build_ollama(config).await? {
            return Ok(provider);
        }
    }

    // Fallback order when selected backend is unavailable.
    if let Some(provider) = try_build_openai(config)? {
        return Ok(provider);
    }
    if let Some(provider) = try_build_anthropic(config)? {
        return Ok(provider);
    }
    if let Some(provider) = try_build_gemini(config)? {
        return Ok(provider);
    }
    if let Some(provider) = try_build_openai_compatible(config)? {
        return Ok(provider);
    }
    if let Some(provider) = try_build_ollama(config).await? {
        return Ok(provider);
    }

    anyhow::bail!("No LLM providers were successfully configured in ferrumyx.toml")
}

fn try_build_openai(
    config: &config::Config,
) -> anyhow::Result<Option<Arc<dyn ironclaw::llm::LlmProvider>>> {
    if let Some(ref openai) = config.llm.openai {
        let key = if openai.api_key.is_empty() {
            std::env::var("FERRUMYX_OPENAI_API_KEY").unwrap_or_default()
        } else {
            openai.api_key.clone()
        };
        if !key.is_empty() {
            tracing::info!("Using OpenAI: {}", openai.model);
            let client: OpenAiClient = OpenAiClient::new(&key)?;
            return Ok(Some(Arc::new(ironclaw::llm::RigAdapter::new(
                client.completion_model(&openai.model),
                &openai.model,
            ))));
        }
    }
    Ok(None)
}

fn try_build_anthropic(
    config: &config::Config,
) -> anyhow::Result<Option<Arc<dyn ironclaw::llm::LlmProvider>>> {
    if let Some(ref anthropic) = config.llm.anthropic {
        let key = if anthropic.api_key.is_empty() {
            std::env::var("FERRUMYX_ANTHROPIC_API_KEY").unwrap_or_default()
        } else {
            anthropic.api_key.clone()
        };
        if !key.is_empty() {
            tracing::info!("Using Anthropic: {}", anthropic.model);
            let client: AnthropicClient = AnthropicClient::new(&key)?;
            return Ok(Some(Arc::new(ironclaw::llm::RigAdapter::new(
                client.completion_model(&anthropic.model),
                &anthropic.model,
            ))));
        }
    }
    Ok(None)
}

fn try_build_gemini(
    config: &config::Config,
) -> anyhow::Result<Option<Arc<dyn ironclaw::llm::LlmProvider>>> {
    if let Some(ref gemini) = config.llm.gemini {
        let key = if gemini.api_key.is_empty() {
            std::env::var("FERRUMYX_GEMINI_API_KEY").unwrap_or_default()
        } else {
            gemini.api_key.clone()
        };
        if !key.is_empty() {
            tracing::info!("Using Gemini: {}", gemini.model);
            let client: GeminiClient = GeminiClient::new(&key)?;
            return Ok(Some(Arc::new(ironclaw::llm::RigAdapter::new(
                client.completion_model(&gemini.model),
                &gemini.model,
            ))));
        }
    }
    Ok(None)
}

fn try_build_openai_compatible(
    config: &config::Config,
) -> anyhow::Result<Option<Arc<dyn ironclaw::llm::LlmProvider>>> {
    if let Some(ref compat) = config.llm.openai_compatible {
        let key = if compat.api_key.is_empty() {
            std::env::var("FERRUMYX_COMPAT_API_KEY").unwrap_or_default()
        } else {
            compat.api_key.clone()
        };
        // Do not use remote OpenAI-compatible providers without a key.
        if key.is_empty() && !is_local_base_url(&compat.base_url) {
            tracing::warn!(
                "Skipping OpenAI-compatible backend {} because no API key is configured",
                compat.base_url
            );
            return Ok(None);
        }
        let api_key = if key.is_empty() {
            "none".to_string()
        } else {
            key
        };
        tracing::info!(
            "Using OpenAI-compatible backend: {} ({}) [cached_chat={}]",
            compat.model,
            compat.base_url,
            compat.cached_chat
        );
        // OpenAI-compatible backends (e.g. Poe, many proxies) may not support
        // the newer Responses API. Force Chat Completions for compatibility.
        let client: OpenAiCompletionsClient = OpenAiCompletionsClient::builder()
            .base_url(&compat.base_url)
            .api_key(&api_key)
            .build()?;
        return Ok(Some(Arc::new(ironclaw::llm::RigAdapter::new(
            client.completion_model(&compat.model),
            &compat.model,
        ))));
    }
    Ok(None)
}

async fn try_build_ollama(
    config: &config::Config,
) -> anyhow::Result<Option<Arc<dyn ironclaw::llm::LlmProvider>>> {
    if let Some(ref ollama) = config.llm.ollama {
        let model = ensure_ollama_ready(&ollama.base_url, &ollama.model).await;
        let tags_url = format!("{}/api/tags", ollama.base_url.trim_end_matches('/'));
        let client = reqwest::Client::new();
        if !ollama_healthy(&client, &tags_url).await {
            tracing::warn!(
                "Ollama is unavailable at {} after startup check. Falling back to other providers.",
                ollama.base_url
            );
            return Ok(None);
        }
        // Fallback to local Ollama (OpenAI compatible API)
        tracing::info!("Using Local Ollama: {}", model);
        let client: OpenAiCompletionsClient = OpenAiCompletionsClient::builder()
            .base_url(&format!("{}/v1", ollama.base_url))
            .api_key("ollama")
            .build()?;
        return Ok(Some(Arc::new(ironclaw::llm::RigAdapter::new(
            client.completion_model(&model),
            &model,
        ))));
    }
    Ok(None)
}

fn is_local_base_url(base_url: &str) -> bool {
    let b = base_url.to_lowercase();
    b.contains("localhost") || b.contains("127.0.0.1")
}

#[derive(Debug, Deserialize)]
struct OllamaTags {
    models: Option<Vec<OllamaTagModel>>,
}

#[derive(Debug, Deserialize)]
struct OllamaTagModel {
    name: String,
}

async fn ensure_ollama_ready(base_url: &str, configured_model: &str) -> String {
    let selected =
        if configured_model.trim().is_empty() || configured_model.eq_ignore_ascii_case("auto") {
            pick_ollama_model_for_hardware()
        } else {
            configured_model.to_string()
        };
    tracing::info!("Ollama selected model: {}", selected);

    let tags_url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    if !ollama_healthy(&client, &tags_url).await {
        tracing::warn!("Ollama is offline. Attempting to launch `ollama serve`.");
        let _ = Command::new("ollama")
            .arg("serve")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        tokio::time::sleep(Duration::from_secs(3)).await;
    }

    if let Ok(resp) = client.get(&tags_url).send().await {
        if let Ok(tags) = resp.json::<OllamaTags>().await {
            let has_model = tags
                .models
                .unwrap_or_default()
                .iter()
                .any(|m| m.name.eq_ignore_ascii_case(&selected));
            if !has_model {
                tracing::info!(
                    "Ollama model '{}' not found locally. Starting background pull...",
                    selected
                );
                let _ = Command::new("ollama")
                    .arg("pull")
                    .arg(&selected)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();
            }
        }
    }

    selected
}

async fn ollama_healthy(client: &reqwest::Client, tags_url: &str) -> bool {
    client
        .get(tags_url)
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

fn pick_ollama_model_for_hardware() -> String {
    let mut system = sysinfo::System::new_all();
    system.refresh_memory();
    let total_mem = system.total_memory() as f64;
    let ram_gb = if total_mem > 1_000_000_000_000.0 {
        total_mem / (1024.0 * 1024.0 * 1024.0)
    } else {
        total_mem / (1024.0 * 1024.0)
    };

    let has_nvidia = std::process::Command::new("nvidia-smi")
        .arg("--query-gpu=name")
        .arg("--format=csv,noheader")
        .output()
        .map(|out| out.status.success() && !String::from_utf8_lossy(&out.stdout).trim().is_empty())
        .unwrap_or(false);

    let model = if has_nvidia || ram_gb >= 24.0 {
        "llama3.1:8b"
    } else if ram_gb >= 12.0 {
        "llama3.2:3b"
    } else {
        "llama3.2:1b"
    };

    tracing::info!(
        "Hardware probe: ram_gb={:.1}, has_nvidia={} -> model={}",
        ram_gb,
        has_nvidia,
        model
    );
    model.to_string()
}

fn sync_ironclaw_env_from_config(config: &config::Config) {
    std::env::set_var("LLM_BACKEND", config.llm.default_backend.clone());

    if let Some(ref ollama) = config.llm.ollama {
        std::env::set_var("OLLAMA_BASE_URL", ollama.base_url.clone());
        std::env::set_var("OLLAMA_MODEL", ollama.model.clone());
    }

    if let Some(ref openai) = config.llm.openai {
        let key = if openai.api_key.is_empty() {
            std::env::var("FERRUMYX_OPENAI_API_KEY").unwrap_or_default()
        } else {
            openai.api_key.clone()
        };
        if !key.is_empty() {
            std::env::set_var("OPENAI_API_KEY", key);
        }
        std::env::set_var("OPENAI_MODEL", openai.model.clone());
    }

    if let Some(ref anthropic) = config.llm.anthropic {
        let key = if anthropic.api_key.is_empty() {
            std::env::var("FERRUMYX_ANTHROPIC_API_KEY").unwrap_or_default()
        } else {
            anthropic.api_key.clone()
        };
        if !key.is_empty() {
            std::env::set_var("ANTHROPIC_API_KEY", key);
        }
        std::env::set_var("ANTHROPIC_MODEL", anthropic.model.clone());
    }

    if let Some(ref gemini) = config.llm.gemini {
        let key = if gemini.api_key.is_empty() {
            std::env::var("FERRUMYX_GEMINI_API_KEY").unwrap_or_default()
        } else {
            gemini.api_key.clone()
        };
        if !key.is_empty() {
            std::env::set_var("GEMINI_API_KEY", key);
        }
    }

    if let Some(ref compat) = config.llm.openai_compatible {
        std::env::set_var("LLM_BASE_URL", compat.base_url.clone());
        std::env::set_var("LLM_MODEL", compat.model.clone());
        std::env::set_var(
            "FERRUMYX_COMPAT_CACHED_CHAT",
            if compat.cached_chat { "1" } else { "0" },
        );
        std::env::set_var(
            "LLM_COMPAT_CACHED_CHAT",
            if compat.cached_chat { "1" } else { "0" },
        );
        let key = if compat.api_key.is_empty() {
            std::env::var("FERRUMYX_COMPAT_API_KEY").unwrap_or_default()
        } else {
            compat.api_key.clone()
        };
        if !key.is_empty() {
            std::env::set_var("LLM_API_KEY", key);
        }
    }
}

#[derive(Debug, Clone)]
struct BackgroundProviderRefreshConfig {
    enabled: bool,
    interval_secs: u64,
    max_genes: usize,
    batch_size: usize,
    retries: u8,
    fact_scan_limit: usize,
    min_gene_mentions: usize,
    alert_error_rate: f64,
    alert_streak_runs: u64,
    cancer_code: Option<String>,
    seed_genes: Vec<String>,
}

impl BackgroundProviderRefreshConfig {
    fn from_env() -> Self {
        let enabled = env_bool("FERRUMYX_PHASE4_BG_REFRESH_ENABLED", true);
        let interval_secs =
            env_u64("FERRUMYX_PHASE4_BG_REFRESH_INTERVAL_SECS", 900).clamp(60, 86_400);
        let max_genes = env_u64("FERRUMYX_PHASE4_BG_REFRESH_MAX_GENES", 24).clamp(1, 200) as usize;
        let batch_size = env_u64("FERRUMYX_PHASE4_BG_REFRESH_BATCH_SIZE", 6).clamp(1, 32) as usize;
        let retries = env_u64("FERRUMYX_PHASE4_BG_REFRESH_RETRIES", 1).clamp(0, 3) as u8;
        let fact_scan_limit =
            env_u64("FERRUMYX_PHASE4_BG_REFRESH_FACT_SCAN_LIMIT", 4000).clamp(250, 20_000) as usize;
        let min_gene_mentions =
            env_u64("FERRUMYX_PHASE4_BG_REFRESH_MIN_GENE_MENTIONS", 2).clamp(1, 50) as usize;
        let alert_error_rate =
            env_f64("FERRUMYX_PHASE4_BG_REFRESH_ALERT_ERROR_RATE", 0.55).clamp(0.05, 1.0);
        let alert_streak_runs =
            env_u64("FERRUMYX_PHASE4_BG_REFRESH_ALERT_STREAK_RUNS", 3).clamp(1, 100);
        let cancer_code = std::env::var("FERRUMYX_PHASE4_BG_REFRESH_CANCER_CODE")
            .ok()
            .map(|v| v.trim().to_uppercase())
            .filter(|v| !v.is_empty());
        let seed_genes = parse_seed_genes(
            &std::env::var("FERRUMYX_PHASE4_BG_REFRESH_GENES").unwrap_or_default(),
        );

        Self {
            enabled,
            interval_secs,
            max_genes,
            batch_size,
            retries,
            fact_scan_limit,
            min_gene_mentions,
            alert_error_rate,
            alert_streak_runs,
            cancer_code,
            seed_genes,
        }
    }
}

fn env_bool(name: &str, default_value: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(default_value)
}

fn env_u64(name: &str, default_value: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default_value)
}

fn env_f64(name: &str, default_value: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(default_value)
}

fn parse_seed_genes(raw: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    raw.split(',')
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty())
        .filter(|v| looks_like_gene_symbol(v))
        .filter(|v| seen.insert(v.clone()))
        .collect()
}

fn looks_like_gene_symbol(name: &str) -> bool {
    let n = name.trim();
    if n.len() < 2 || n.len() > 12 {
        return false;
    }
    n.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        && n.chars().any(|c| c.is_ascii_uppercase())
}

async fn discover_background_refresh_genes(
    db: Arc<ferrumyx_db::Database>,
    cfg: &BackgroundProviderRefreshConfig,
) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for gene in &cfg.seed_genes {
        if seen.insert(gene.clone()) {
            out.push(gene.clone());
        }
    }
    if out.len() >= cfg.max_genes {
        out.truncate(cfg.max_genes);
        return out;
    }

    let repo = ferrumyx_db::kg_facts::KgFactRepository::new(db);
    let facts = match repo.list(0, cfg.fact_scan_limit).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(
                "background provider refresh: failed to read kg_facts for seed discovery: {}",
                e
            );
            out.truncate(cfg.max_genes);
            return out;
        }
    };

    let mut counts: HashMap<String, usize> = HashMap::new();
    for fact in facts {
        let gene = fact.subject_name.trim().to_uppercase();
        if looks_like_gene_symbol(&gene) {
            *counts.entry(gene).or_insert(0) += 1;
        }
    }

    let mut ranked: Vec<(String, usize)> = counts
        .into_iter()
        .filter(|(_, c)| *c >= cfg.min_gene_mentions)
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    for (gene, _) in ranked {
        if out.len() >= cfg.max_genes {
            break;
        }
        if seen.insert(gene.clone()) {
            out.push(gene);
        }
    }
    out
}

fn spawn_background_provider_refresh_scheduler(db: Arc<ferrumyx_db::Database>) {
    let bootstrap_cfg = BackgroundProviderRefreshConfig::from_env();
    if !bootstrap_cfg.enabled {
        tracing::info!("Phase 4 background provider refresh scheduler disabled.");
        return;
    }
    tracing::info!(
        "Phase 4 background provider refresh scheduler enabled (interval={}s, max_genes={})",
        bootstrap_cfg.interval_secs,
        bootstrap_cfg.max_genes
    );

    tokio::spawn(async move {
        let mut current_interval_secs = bootstrap_cfg.interval_secs.max(60);
        let mut interval = tokio::time::interval(Duration::from_secs(current_interval_secs));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut alert_streak = 0u64;

        loop {
            interval.tick().await;
            let cfg = BackgroundProviderRefreshConfig::from_env();
            if !cfg.enabled {
                continue;
            }

            let next_interval_secs = cfg.interval_secs.max(60);
            if next_interval_secs != current_interval_secs {
                current_interval_secs = next_interval_secs;
                interval = tokio::time::interval(Duration::from_secs(current_interval_secs));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            }

            let genes = discover_background_refresh_genes(db.clone(), &cfg).await;
            if genes.is_empty() {
                tracing::debug!(
                    "background provider refresh skipped: no candidate genes discovered"
                );
                continue;
            }

            let request = ferrumyx_ranker::ProviderRefreshRequest {
                genes,
                cancer_code: cfg.cancer_code.clone(),
                max_genes: cfg.max_genes,
                batch_size: cfg.batch_size,
                retries: cfg.retries,
                offline_strict: false,
            };
            let engine = ferrumyx_ranker::TargetQueryEngine::new(db.clone());
            match engine.refresh_provider_signals(request).await {
                Ok(report) => {
                    let attempted = report.cbio_attempted
                        + report.cosmic_attempted
                        + report.gtex_attempted
                        + report.tcga_attempted
                        + report.chembl_attempted
                        + report.reactome_attempted;
                    let failed = report.cbio_failed
                        + report.cosmic_failed
                        + report.gtex_failed
                        + report.tcga_failed
                        + report.chembl_failed
                        + report.reactome_failed;
                    let error_rate = if attempted > 0 {
                        failed as f64 / attempted as f64
                    } else {
                        0.0
                    };

                    if attempted > 0 && error_rate >= cfg.alert_error_rate {
                        alert_streak += 1;
                        tracing::warn!(
                            target: "ferrumyx_provider_refresh_bg",
                            attempted = attempted,
                            failed = failed,
                            error_rate = error_rate,
                            streak = alert_streak,
                            "background provider refresh elevated provider error rate"
                        );
                        if alert_streak >= cfg.alert_streak_runs {
                            tracing::error!(
                                target: "ferrumyx_provider_refresh_bg",
                                attempted = attempted,
                                failed = failed,
                                error_rate = error_rate,
                                streak = alert_streak,
                                "background provider refresh alert threshold exceeded"
                            );
                        }
                    } else {
                        alert_streak = 0;
                        tracing::info!(
                            target: "ferrumyx_provider_refresh_bg",
                            genes_processed = report.genes_processed,
                            attempted = attempted,
                            failed = failed,
                            duration_ms = report.duration_ms,
                            "background provider refresh completed"
                        );
                    }
                }
                Err(e) => {
                    alert_streak += 1;
                    tracing::warn!(
                        target: "ferrumyx_provider_refresh_bg",
                        streak = alert_streak,
                        error = %e,
                        "background provider refresh failed"
                    );
                    if alert_streak >= cfg.alert_streak_runs {
                        tracing::error!(
                            target: "ferrumyx_provider_refresh_bg",
                            streak = alert_streak,
                            error = %e,
                            "background provider refresh repeated failure alert"
                        );
                    }
                }
            }
        }
    });
}

use ironclaw::agent::SessionManager;
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() -> anyhow::Result<()> {
    // Slightly larger per-thread stacks reduce risk of overflow under
    // deep parser/expression workloads in dependent crates.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(8 * 1024 * 1024)
        .build()?;
    runtime.block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    // Initialise structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("ferrumyx=debug,info")),
        )
        .init();

    info!("🔬 Ferrumyx starting up...");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = match config::Config::load() {
        Ok(c) => {
            info!(
                "Configuration loaded. LLM mode: {}, Focus: {} {}",
                c.llm.mode, c.scoring.focus_cancer, c.scoring.focus_mutation
            );
            c
        }
        Err(e) => {
            tracing::warn!("Could not load ferrumyx.toml: {e}");
            tracing::warn!("Copy ferrumyx.example.toml to ferrumyx.toml and edit it.");
            return Ok(());
        }
    };

    // Bridge Ferrumyx settings into IronClaw env-style configuration.
    sync_ironclaw_env_from_config(&config);

    // Connect to LanceDB
    info!("Connecting to LanceDB...");
    let db = ferrumyx_db::Database::open(&config.database.url).await?;
    db.initialize().await?;
    let db = std::sync::Arc::new(db);
    info!("✅ LanceDB connected and initialized.");

    // Start Phase 3: Knowledge Graph Event Queue
    let _kg_event_tx = ferrumyx_kg::update::start_scoring_event_queue(db.clone());
    info!("✅ KG event-driven scoring queue initialized.");
    spawn_background_provider_refresh_scheduler(db.clone());

    // Build LLM client
    let ironclaw_llm = build_completion_model(&config).await?;

    // Build Tool Registry
    let tool_registry = Arc::new(ironclaw::tools::ToolRegistry::new());
    tool_registry.register_sync(Arc::new(tools::ingestion_tool::IngestionTool::new(
        db.clone(),
    )));
    tool_registry.register_sync(Arc::new(tools::query_tool::TargetQueryTool::new(
        db.clone(),
    )));
    tool_registry.register_sync(Arc::new(
        tools::workflow_status_tool::WorkflowStatusTool::new(db.clone()),
    ));
    tool_registry.register_sync(Arc::new(
        tools::scoring_tool::RecomputeTargetScoresTool::new(db.clone()),
    ));
    tool_registry.register_sync(Arc::new(
        tools::provider_refresh_tool::RefreshProviderSignalsTool::new(db.clone()),
    ));
    tool_registry.register_sync(Arc::new(
        tools::molecule_tool::RunMoleculePipelineTool::new(),
    ));
    tool_registry.register_sync(Arc::new(
        tools::autonomous_cycle_tool::AutonomousCycleTool::new(db.clone()),
    ));
    tool_registry.register_sync(Arc::new(
        tools::system_command_tool::SystemCommandTool::new(),
    ));

    // Build Skill Registry
    let skill_registry = std::sync::Arc::new(std::sync::RwLock::new(
        ironclaw::skills::registry::SkillRegistry::new(std::path::PathBuf::from("./data/skills")),
    ));
    let skill_catalog = std::sync::Arc::new(ironclaw::skills::catalog::SkillCatalog::new());

    let deps = ironclaw::agent::AgentDeps {
        store: None,
        llm: ironclaw_llm.clone(),
        cheap_llm: None,
        safety: std::sync::Arc::new(ironclaw::safety::SafetyLayer::new(
            &ironclaw::config::SafetyConfig {
                max_output_length: 100_000,
                injection_check_enabled: true,
            },
        )),
        tools: tool_registry.clone(),
        workspace: None,
        extension_manager: None,
        skill_registry: Some(skill_registry.clone()),
        skill_catalog: Some(skill_catalog.clone()),
        skills_config: ironclaw::config::SkillsConfig::default(),
        hooks: std::sync::Arc::new(ironclaw::hooks::HookRegistry::new()),
        cost_guard: std::sync::Arc::new(ironclaw::agent::cost_guard::CostGuard::new(
            ironclaw::agent::cost_guard::CostGuardConfig::default(),
        )),
        sse_tx: None,
        http_interceptor: None,
        transcription: None,
        document_extraction: None,
    };

    let session_manager = Arc::new(SessionManager::new());
    let channels = ironclaw::channels::ChannelManager::new();
    let disable_repl = std::env::var("FERRUMYX_DISABLE_REPL")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !disable_repl && std::io::stdin().is_terminal() {
        channels
            .add(Box::new(ironclaw::channels::ReplChannel::new()))
            .await;
    } else {
        tracing::info!("Non-interactive runtime detected; skipping REPL channel.");
    }

    let gw_config = ironclaw::config::GatewayConfig {
        host: "127.0.0.1".to_string(),
        port: 3002,
        user_id: "User".to_string(),
        auth_token: Some("ferrumyx-local-dev-token".to_string()),
    };
    let gateway = ironclaw::channels::GatewayChannel::new(gw_config)
        .with_session_manager(session_manager.clone())
        .with_llm_provider(ironclaw_llm.clone());
    channels.add(Box::new(gateway)).await;

    let agent_config = ironclaw::config::AgentConfig {
        name: "Ferrumyx Drug Discovery Agent".to_string(),
        max_parallel_jobs: 1,
        job_timeout: std::time::Duration::from_secs(3600),
        stuck_threshold: std::time::Duration::from_secs(300),
        repair_check_interval: std::time::Duration::from_secs(60),
        max_repair_attempts: 3,
        use_planning: true,
        session_idle_timeout: std::time::Duration::from_secs(86400),
        allow_local_tools: true,
        max_cost_per_day_cents: None,
        max_actions_per_hour: None,
        max_tool_iterations: 50,
        auto_approve_tools: true,
        default_timezone: "UTC".to_string(),
    };

    let agent = ironclaw::agent::Agent::new(
        agent_config,
        deps,
        Arc::new(channels),
        None,
        None,
        None,
        None,
        Some(session_manager),
    );

    tokio::spawn(async move {
        if let Err(e) = agent.run().await {
            tracing::error!("Agent loop exited with error: {}", e);
        }
    });

    // Build app state and router
    let state = ferrumyx_web::state::AppState::new(db);
    let router = ferrumyx_web::router::build_router(state);

    // Start web server
    let bind_addr = std::env::var("FERRUMYX_BIND").unwrap_or_else(|_| "0.0.0.0:3000".to_string());

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    info!("🌐 Web GUI listening on http://{}", bind_addr);
    info!("   Dashboard:    http://localhost:3000/");
    info!("   Target Query: http://localhost:3000/query");
    info!("   KG Explorer:  http://localhost:3000/kg");
    info!("   Ingestion:    http://localhost:3000/ingestion");
    info!("   Metrics:      http://localhost:3000/metrics");
    info!("   System:       http://localhost:3000/system");
    info!("");
    info!("🔬 Ferrumyx ready. Press Ctrl+C to stop.");

    axum::serve(listener, router).await?;

    Ok(())
}
