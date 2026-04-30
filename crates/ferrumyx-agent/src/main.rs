//! Ferrumyx — Autonomous Oncology Drug Discovery Engine
//! Entry point for the agent binary.

// Standard library
use std::collections::{HashMap, HashSet};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

// Third-party
use ironclaw::prelude::*;
use rig::providers::{anthropic::Client as AnthropicClient, gemini::Client as GeminiClient, openai::{Client as OpenAiClient, CompletionsClient as OpenAiCompletionsClient}};
use serde::Deserialize;
use tokio::process::Command;
use tracing::info;
use tracing_subscriber::EnvFilter;

// Ferrumyx internal crates
use ferrumyx_runtime::llm::{CooldownConfig, FailoverProvider};
use ferrumyx_runtime::channels::{ChannelManager, web::GatewayChannel, wasm::{WasmChannelLoader, WasmChannelRuntime, WasmChannelRuntimeConfig}};
use ferrumyx_runtime::config::GatewayConfig;
use ferrumyx_runtime::tools::wasm::{WasmToolLoader, load_dev_tools};
use ferrumyx_runtime::extensions::manager::ExtensionManager;
use ferrumyx_runtime::pairing::PairingStore;
use ferrumyx_common::memory;

mod config;
mod ironclaw_config;
mod llm_routing;
mod tools;
mod channels;
mod container_orchestrator;

use container_orchestrator::BioContainerOrchestrator;

/// Returns a Boxed CompletionModel to inject into the Agent.
/// It natively maps the Ferrumyx config directly to `rig-core` LLM clients.
async fn build_completion_model(
    config: &config::Config,
) -> anyhow::Result<Arc<llm_routing::IronClawLlmRouter>> {
    let mode = config.llm.mode.to_lowercase();
    let local_only = mode == "local_only";
    let default_backend = normalize_backend_name(&config.llm.default_backend);
    let local_backend = normalize_backend_name(&config.llm.local_backend);
    let failover_order = resolve_failover_backend_order(&default_backend, &local_backend, &mode);

    let mut local_providers: Vec<Arc<dyn ferrumyx_runtime::llm::LlmProvider>> = Vec::new();
    let mut remote_providers: Vec<Arc<dyn ferrumyx_runtime::llm::LlmProvider>> = Vec::new();
    let mut local_provider_names: Vec<String> = Vec::new();
    let mut remote_provider_names: Vec<String> = Vec::new();
    let mut seen = HashSet::new();

    for backend in failover_order {
        let backend = normalize_backend_name(&backend);
        if !seen.insert(backend.clone()) {
            continue;
        }
        if local_only && !matches!(backend.as_str(), "ollama" | "openai_compatible") {
            continue;
        }

        let maybe_provider = match backend.as_str() {
            "openai" => try_build_openai(config)?,
            "anthropic" => try_build_anthropic(config)?,
            "gemini" => try_build_gemini(config)?,
            "openai_compatible" => try_build_openai_compatible(config, local_only)?,
            "ollama" => try_build_ollama(config).await?,
            other => {
                tracing::warn!("Unknown LLM backend in failover order: {}", other);
                None
            }
        };

        if let Some(provider) = maybe_provider {
            if matches!(backend.as_str(), "ollama" | "openai_compatible") && is_local_backend(&backend, config) {
                local_provider_names.push(backend);
                local_providers.push(provider);
            } else {
                remote_provider_names.push(backend);
                remote_providers.push(provider);
            }
        }
    }

    if local_providers.is_empty() && remote_providers.is_empty() {
        anyhow::bail!("No LLM providers were successfully configured in ferrumyx.toml");
    }

    // Build local failover provider
    let local_provider = if local_providers.len() == 1 {
        local_providers.remove(0)
    } else if !local_providers.is_empty() {
        let cooldown_secs = env_u64("FERRUMYX_LLM_FAILOVER_COOLDOWN_SECS", 120).clamp(15, 3600);
        let failure_threshold =
            env_u64("FERRUMYX_LLM_FAILOVER_FAILURE_THRESHOLD", 2).clamp(1, 10) as u32;
        FailoverProvider::with_cooldown(
            local_providers,
            CooldownConfig {
                cooldown_duration: Duration::from_secs(cooldown_secs),
                failure_threshold,
            },
        )
        .map_err(|e| anyhow::anyhow!("failed to build local LLM failover chain: {e}"))?
    } else {
        anyhow::bail!("No local LLM providers configured, but required for IronClaw routing");
    };

    // Build remote failover provider (optional)
    let remote_provider = if remote_providers.len() == 1 {
        Some(remote_providers.remove(0))
    } else if !remote_providers.is_empty() {
        let cooldown_secs = env_u64("FERRUMYX_LLM_FAILOVER_COOLDOWN_SECS", 120).clamp(15, 3600);
        let failure_threshold =
            env_u64("FERRUMYX_LLM_FAILOVER_FAILURE_THRESHOLD", 2).clamp(1, 10) as u32;
        Some(Arc::new(FailoverProvider::with_cooldown(
            remote_providers,
            CooldownConfig {
                cooldown_duration: Duration::from_secs(cooldown_secs),
                failure_threshold,
            },
        )
        .map_err(|e| anyhow::anyhow!("failed to build remote LLM failover chain: {e}"))?))
    } else {
        None
    };

    // Create IronClaw router
    let router = llm_routing::IronClawLlmRouter::new(local_provider, remote_provider);

    tracing::info!(
        "IronClaw LLM routing active: local={} remote={}",
        local_provider_names.join(","),
        remote_provider_names.join(",")
    );

    Ok(Arc::new(router))
}

fn normalize_backend_name(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "openai-compatible" | "compat" => "openai_compatible".to_string(),
        "local" => "ollama".to_string(),
        other => other.to_string(),
    }
}

fn is_local_backend(backend: &str, config: &config::Config) -> bool {
    matches!(backend, "ollama") ||
    (backend == "openai_compatible" && config.llm.openai_compatible.as_ref()
        .map(|c| is_local_base_url(&c.base_url)).unwrap_or(false))
}

fn resolve_failover_backend_order(
    default_backend: &str,
    local_backend: &str,
    mode: &str,
) -> Vec<String> {
    if let Ok(raw) = std::env::var("FERRUMYX_LLM_FAILOVER_ORDER") {
        let mut parsed = Vec::new();
        let mut seen = HashSet::new();
        for token in raw.split(',').map(normalize_backend_name) {
            if token.trim().is_empty() {
                continue;
            }
            if seen.insert(token.clone()) {
                parsed.push(token);
            }
        }
        if !parsed.is_empty() {
            return parsed;
        }
    }

    let local_primary = match local_backend {
        "openai_compatible" => "openai_compatible",
        _ => "ollama",
    };
    let local_secondary = if local_primary == "ollama" {
        "openai_compatible"
    } else {
        "ollama"
    };

    let mut order = Vec::new();
    if mode == "local_only" || mode == "prefer_local" || default_backend == "ollama" {
        order.push(local_primary.to_string());
        order.push(local_secondary.to_string());
        if mode != "local_only" {
            order.extend(
                ["openai", "anthropic", "gemini"]
                    .iter()
                    .map(|v| v.to_string()),
            );
        }
    } else {
        order.push(default_backend.to_string());
        order.extend(
            [
                "openai_compatible",
                "openai",
                "anthropic",
                "gemini",
                "ollama",
            ]
            .iter()
            .map(|v| v.to_string()),
        );
    }
    if !order.iter().any(|b| b == default_backend) {
        order.insert(0, default_backend.to_string());
    }

    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for backend in order {
        let name = normalize_backend_name(&backend);
        if seen.insert(name.clone()) {
            out.push(name);
        }
    }
    out
}

fn try_build_openai(
    config: &config::Config,
) -> anyhow::Result<Option<Arc<dyn ferrumyx_runtime::llm::LlmProvider>>> {
    if let Some(ref openai) = config.llm.openai {
        let key = if openai.api_key.is_empty() {
            std::env::var("FERRUMYX_OPENAI_API_KEY").unwrap_or_default()
        } else {
            openai.api_key.clone()
        };
        if !key.is_empty() {
            tracing::info!("Using OpenAI: {}", openai.model);
            let client: OpenAiClient = OpenAiClient::new(&key)?;
            return Ok(Some(Arc::new(ferrumyx_runtime::llm::RigAdapter::new(
                client.completion_model(&openai.model),
                &openai.model,
            ))));
        }
    }
    Ok(None)
}

fn try_build_anthropic(
    config: &config::Config,
) -> anyhow::Result<Option<Arc<dyn ferrumyx_runtime::llm::LlmProvider>>> {
    if let Some(ref anthropic) = config.llm.anthropic {
        let key = if anthropic.api_key.is_empty() {
            std::env::var("FERRUMYX_ANTHROPIC_API_KEY").unwrap_or_default()
        } else {
            anthropic.api_key.clone()
        };
        if !key.is_empty() {
            tracing::info!("Using Anthropic: {}", anthropic.model);
            let client: AnthropicClient = AnthropicClient::new(&key)?;
            return Ok(Some(Arc::new(ferrumyx_runtime::llm::RigAdapter::new(
                client.completion_model(&anthropic.model),
                &anthropic.model,
            ))));
        }
    }
    Ok(None)
}

fn try_build_gemini(
    config: &config::Config,
) -> anyhow::Result<Option<Arc<dyn ferrumyx_runtime::llm::LlmProvider>>> {
    if let Some(ref gemini) = config.llm.gemini {
        let key = if gemini.api_key.is_empty() {
            std::env::var("FERRUMYX_GEMINI_API_KEY").unwrap_or_default()
        } else {
            gemini.api_key.clone()
        };
        if !key.is_empty() {
            tracing::info!("Using Gemini: {}", gemini.model);
            let client: GeminiClient = GeminiClient::new(&key)?;
            return Ok(Some(Arc::new(ferrumyx_runtime::llm::RigAdapter::new(
                client.completion_model(&gemini.model),
                &gemini.model,
            ))));
        }
    }
    Ok(None)
}

fn try_build_openai_compatible(
    config: &config::Config,
    local_only: bool,
) -> anyhow::Result<Option<Arc<dyn ferrumyx_runtime::llm::LlmProvider>>> {
    if let Some(ref compat) = config.llm.openai_compatible {
        let key = if compat.api_key.is_empty() {
            std::env::var("FERRUMYX_COMPAT_API_KEY").unwrap_or_default()
        } else {
            compat.api_key.clone()
        };
        if local_only && !is_local_base_url(&compat.base_url) {
            tracing::warn!(
                "Skipping OpenAI-compatible backend {} in local_only mode (non-local base URL)",
                compat.base_url
            );
            return Ok(None);
        }
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
        return Ok(Some(Arc::new(ferrumyx_runtime::llm::RigAdapter::new(
            client.completion_model(&compat.model),
            &compat.model,
        ))));
    }
    Ok(None)
}

async fn try_build_ollama(
    config: &config::Config,
) -> anyhow::Result<Option<Arc<dyn ferrumyx_runtime::llm::LlmProvider>>> {
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
        return Ok(Some(Arc::new(ferrumyx_runtime::llm::RigAdapter::new(
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

fn sync_runtime_env_from_config(config: &config::Config) {
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

    // Bridge Ferrumyx settings into runtime core env-style configuration.
    sync_runtime_env_from_config(&config);

    // Connect to LanceDB
    info!("Connecting to LanceDB...");
    let db = ferrumyx_db::Database::open(&config.database.url).await?;
    db.initialize().await?;
    let db = std::sync::Arc::new(db);
    info!("✅ LanceDB connected and initialized.");

    // Start memory monitoring
    memory::start_memory_monitoring(1000, Duration::from_secs(30), Duration::from_secs(3600), 10).await;

    // Start Phase 3: Knowledge Graph Event Queue
    let _kg_event_tx = ferrumyx_kg::update::start_scoring_event_queue(db.clone());
    info!("✅ KG event-driven scoring queue initialized.");
    spawn_background_provider_refresh_scheduler(db.clone());

    // Initialize Docker container orchestrator for bioinformatics tools
    info!("Initializing Docker container orchestrator...");
    let container_orchestrator = match BioContainerOrchestrator::new().await {
        Ok(orc) => {
            info!("✅ Docker container orchestrator initialized.");
            Arc::new(orc)
        }
        Err(e) => {
            tracing::warn!("Could not initialize Docker container orchestrator: {e}");
            tracing::warn!("Bioinformatics tools will run in placeholder mode.");
            return Ok(());
        }
    };

    // Build LLM client
    let ironclaw_router = build_completion_model(&config).await?;
    let runtime_llm: Arc<dyn ferrumyx_runtime::llm::LlmProvider> = ironclaw_router.clone();
    let runtime_core_llm = ferrumyx_runtime::llm::to_core_provider(runtime_llm.clone());

    // Build Tool Registry
    let runtime_tool_registry = Arc::new(ferrumyx_runtime::tools::ToolRegistry::new());

    // Load WASM bioinformatics tools for secure sandboxed execution
    info!("Loading WASM bioinformatics tools...");
    let wasm_config = ferrumyx_runtime::config::WasmConfig::resolve().unwrap_or_default();
    if wasm_config.enabled {
        let wasm_loader = WasmToolLoader::new(
            None, // WASM runtime will be initialized if needed
            runtime_tool_registry.clone(),
            None, // No custom secrets store
        );

        match load_dev_tools(&wasm_loader, &wasm_config.tools_dir).await {
            Ok(count) => {
                if count > 0 {
                    info!("✅ Loaded {} WASM bioinformatics tools: BLAST, FastQC, PyMOL", count);
                } else {
                    info!("ℹ️  No WASM tools found (expected in development mode)");
                }
            }
            Err(e) => {
                tracing::warn!("Could not load WASM tools: {e}");
                tracing::warn!("Bioinformatics tools will use container-based execution.");
            }
        }
    } else {
        info!("ℹ️  WASM sandboxing disabled, using container-based execution for bioinformatics tools");
    }
    runtime_tool_registry.register_sync(Arc::new(tools::ingestion_tool::IngestionTool::new(
        db.clone(),
    )));
    runtime_tool_registry.register_sync(Arc::new(
        tools::embedding_backfill_tool::BackfillEmbeddingsTool::new(db.clone()),
    ));
    runtime_tool_registry.register_sync(Arc::new(tools::query_tool::TargetQueryTool::new(
        db.clone(),
    )));
    runtime_tool_registry.register_sync(Arc::new(
        tools::workflow_status_tool::WorkflowStatusTool::new(db.clone()),
    ));
    runtime_tool_registry.register_sync(Arc::new(
        tools::scoring_tool::RecomputeTargetScoresTool::new(db.clone()),
    ));
    runtime_tool_registry.register_sync(Arc::new(
        tools::provider_refresh_tool::RefreshProviderSignalsTool::new(db.clone()),
    ));
    runtime_tool_registry.register_sync(Arc::new(
        tools::molecule_tool::RunMoleculePipelineTool::new(),
    ));
    runtime_tool_registry.register_sync(Arc::new(
        tools::autonomous_cycle_tool::AutonomousCycleTool::new(db.clone()),
    ));
    runtime_tool_registry.register_sync(Arc::new(tools::lab_planner_tool::LabPlannerTool::new()));
    runtime_tool_registry.register_sync(Arc::new(
        tools::lab_retriever_tool::LabRetrieverTool::new(db.clone()),
    ));
    runtime_tool_registry.register_sync(Arc::new(
        tools::lab_validator_tool::LabValidatorTool::new(db.clone()),
    ));
    runtime_tool_registry.register_sync(Arc::new(
        tools::lab_autoresearch_tool::LabAutoresearchTool::new(db.clone()),
    ));
    runtime_tool_registry
        .register_sync(Arc::new(tools::lab_run_status_tool::LabRunStatusTool::new()));
    runtime_tool_registry.register_sync(Arc::new(
        tools::system_command_tool::SystemCommandTool::new(),
    ));

    // Register bioinformatics tools
    runtime_tool_registry.register_sync(Arc::new(tools::pubmed_search_tool::PubMedSearchTool::new(db.clone())));
    runtime_tool_registry.register_sync(Arc::new(tools::bio_tools::FastQCTool::new(container_orchestrator.clone())));
    runtime_tool_registry.register_sync(Arc::new(tools::bio_tools::BlastTool::new(container_orchestrator.clone())));
    runtime_tool_registry.register_sync(Arc::new(tools::bio_tools::PyMOLTool::new(container_orchestrator.clone())));
    runtime_tool_registry.register_sync(Arc::new(tools::bio_tools::ExpressionAnalysisTool::new()));
    runtime_tool_registry.register_sync(Arc::new(tools::bio_tools::PathwayEnrichmentTool::new()));
    runtime_tool_registry.register_sync(Arc::new(tools::bio_tools::VariantCallingTool::new()));
    runtime_tool_registry.register_sync(Arc::new(tools::bio_tools::TargetIdentificationTool::new()));

    runtime_tool_registry.register_sync(Arc::new(
        tools::llm_audit_tool::LlmAuditTool::new(ironclaw_router.clone()),
    ));
    let tool_registry = runtime_tool_registry.to_core_registry();

    // Build Skill Registry
    let skill_registry = std::sync::Arc::new(std::sync::RwLock::new(
        ferrumyx_runtime::skills::registry::SkillRegistry::new(std::path::PathBuf::from(
            "./data/skills",
        )),
    ));
    let skill_catalog = std::sync::Arc::new(ferrumyx_runtime::skills::catalog::SkillCatalog::new());

    // Enhanced IronClaw agent with full orchestration for oncology discovery
    let ironclaw_config = ironclaw_config::load_config();
    let agent = ironclaw::Agent::builder()
        .name("Ferrumyx Drug Discovery Agent")
        .model(runtime_llm.clone())
        .tools(tool_registry.clone())
        .config(ironclaw_config)
        .with_max_parallel_jobs(4) // Allow parallel oncology discovery jobs
        .with_job_timeout(std::time::Duration::from_secs(3600))
        .with_stuck_threshold(std::time::Duration::from_secs(300))
        .with_repair_check_interval(std::time::Duration::from_secs(60))
        .with_max_repair_attempts(3)
        .with_session_idle_timeout(std::time::Duration::from_secs(86400))
        .with_max_tool_iterations(50)
        .with_auto_approve_tools(true)
        // Configure autonomous oncology discovery routines
        .with_routine_config(ironclaw::config::RoutineConfig {
            enabled: true,
            cron_schedule: "0 */4 * * *".to_string(), // Every 4 hours for discovery cycles
            event_triggers: vec![
                "on_new_pubmed_data".to_string(),
                "on_gene_target_update".to_string(),
                "on_molecule_pipeline_complete".to_string(),
            ],
            webhook_endpoints: vec![],
        })
        // Configure heartbeat for proactive monitoring
        .with_heartbeat_config(ironclaw::config::HeartbeatConfig {
            enabled: true,
            interval_secs: 300, // 5 minutes
            tasks: vec![
                "check_pubmed_updates".to_string(),
                "validate_lab_status".to_string(),
                "monitor_provider_signals".to_string(),
                "audit_discovery_progress".to_string(),
            ],
        })
        .build()
        .expect("Failed to build IronClaw agent");

    // Set up multi-channel support
    let channels = ferrumyx_runtime::channels::ChannelManager::new();
    let channel_router = channels::ChannelRouter::new();

    // Add GatewayChannel for web/SSE communication with oncology formatting
    let gw_config = ferrumyx_runtime::config::GatewayConfig {
        host: "127.0.0.1".to_string(),
        port: 3002,
        user_id: "User".to_string(),
        auth_token: Some("ferrumyx-local-dev-token".to_string()),
    };
    let gateway = ferrumyx_runtime::channels::web::GatewayChannel::new(gw_config.clone());
    let oncology_gateway = channels::OncologyChannelWrapper::new(gateway);
    channels.add(Box::new(oncology_gateway)).await;

    // Load WASM channels for WhatsApp, Slack, Discord
    info!("Loading WASM channels for multi-channel support...");
    let wasm_runtime_config = WasmChannelRuntimeConfig::for_testing();
    let wasm_runtime = Arc::new(WasmChannelRuntime::new(wasm_runtime_config).unwrap());
    let pairing_store = Arc::new(PairingStore::new());
    let wasm_loader = WasmChannelLoader::new(wasm_runtime.clone(), pairing_store.clone(), None);

    let channels_dir = ferrumyx_runtime::channels::wasm::loader::default_channels_dir();
    match wasm_loader.load_from_dir(&channels_dir).await {
        Ok(results) => {
            if results.success_count() > 0 {
                info!("✅ Loaded {} WASM channels: {}", results.success_count(),
                    results.loaded.iter().map(|c| c.name()).collect::<Vec<_>>().join(", "));
            } else {
                info!("ℹ️  No WASM channels found in {}", channels_dir.display());
            }

            if !results.errors.is_empty() {
                for (path, error) in results.errors {
                    tracing::warn!("Failed to load WASM channel {}: {}", path.display(), error);
                }
            }

            // Add loaded WASM channels to the channel manager with oncology formatting
            for loaded_channel in results.take_channels() {
                let oncology_channel = channels::OncologyChannelWrapper::new(loaded_channel);
                channels.add(Box::new(oncology_channel)).await;
            }
        }
        Err(e) => {
            tracing::warn!("Could not load WASM channels from {}: {}", channels_dir.display(), e);
        }
    }

    // Start channel message processing
    let channel_stream = channels.start().await.expect("Failed to start channels");
    let agent_clone = Arc::clone(&agent);

    // Bridge channel messages to IronClaw agent
    let agent_handle = tokio::spawn(async move {
        info!("Starting IronClaw agent orchestration loop...");
        if let Err(e) = agent.run().await {
            tracing::error!("IronClaw agent orchestration exited with error: {}", e);
        }
    });

    // Start channel processing
    let channel_handle = tokio::spawn(async move {
        info!("Starting channel message processing...");
        while let Some(message) = channel_stream.recv().await {
            tracing::info!(
                "Channel message from {}: {} (user: {}, thread: {:?})",
                message.channel, message.content, message.user_id, message.thread_id
            );

            // Forward message to IronClaw agent
            let agent = Arc::clone(&agent_clone);
            tokio::spawn(async move {
                match agent.handle_message(&message).await {
                    Ok(Some(response)) => {
                        tracing::info!("Agent responded: {}", response);
                        // Send response back through the channel
                        if let Err(e) = channels.respond(&message, ferrumyx_runtime::channels::OutgoingResponse::text(response)).await {
                            tracing::error!("Failed to send response: {}", e);
                        }
                    }
                    Ok(None) => {
                        // No response needed (e.g., system command)
                    }
                    Err(e) => {
                        tracing::error!("Agent failed to handle message: {}", e);
                        // Send error response
                        if let Err(send_err) = channels.respond(&message, ferrumyx_runtime::channels::OutgoingResponse::text(format!("Error: {}", e))).await {
                            tracing::error!("Failed to send error response: {}", send_err);
                        }
                    }
                }
            });
        }
    });

    // Start container cleanup task
    let cleanup_orchestrator = container_orchestrator.clone();
    let cleanup_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300)); // Every 5 minutes
        loop {
            interval.tick().await;
            if let Err(e) = cleanup_orchestrator.health_check().await {
                tracing::warn!("Container health check failed: {}", e);
            }
            // Note: Automatic cleanup is handled by container auto_remove: true
        }
    });

    // Wait for either task to complete
    tokio::select! {
        result = agent_handle => {
            if let Err(e) = result {
                tracing::error!("Agent task failed: {}", e);
            }
        }
        result = channel_handle => {
            if let Err(e) = result {
                tracing::error!("Channel task failed: {}", e);
            }
        }
        result = cleanup_handle => {
            if let Err(e) = result {
                tracing::error!("Cleanup task failed: {}", e);
            }
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use ferrumyx_test_utils::{mocks::MockHttpClient, fixtures::TestFixtureManager};
    use tokio::test;

    #[test]
    fn test_normalize_backend_name() {
        assert_eq!(normalize_backend_name("openai"), "openai");
        assert_eq!(normalize_backend_name("openai-compatible"), "openai_compatible");
        assert_eq!(normalize_backend_name("compat"), "openai_compatible");
        assert_eq!(normalize_backend_name("local"), "ollama");
        assert_eq!(normalize_backend_name("ANTHROPIC"), "anthropic");
    }

    #[test]
    fn test_is_local_backend() {
        let config = config::Config::default();

        assert!(is_local_backend("ollama", &config));
        assert!(!is_local_backend("openai", &config));
        assert!(!is_local_backend("anthropic", &config));
    }

    #[test]
    fn test_resolve_failover_backend_order() {
        let default_backend = "openai".to_string();
        let local_backend = "ollama".to_string();

        // Test local_only mode
        let order = resolve_failover_backend_order(&default_backend, &local_backend, "local_only");
        assert!(order.contains(&"ollama".to_string()));
        assert!(!order.contains(&"openai".to_string()));

        // Test prefer_local mode
        let order = resolve_failover_backend_order(&default_backend, &local_backend, "prefer_local");
        assert_eq!(order[0], "ollama");
        assert!(order.contains(&"openai".to_string()));

        // Test any mode
        let order = resolve_failover_backend_order(&default_backend, &local_backend, "any");
        assert!(order.contains(&"openai".to_string()));
        assert!(order.contains(&"ollama".to_string()));
    }

    #[test]
    fn test_env_bool() {
        std::env::set_var("TEST_BOOL_TRUE", "true");
        std::env::set_var("TEST_BOOL_FALSE", "false");
        std::env::set_var("TEST_BOOL_ONE", "1");
        std::env::set_var("TEST_BOOL_ZERO", "0");

        assert!(env_bool("TEST_BOOL_TRUE", false));
        assert!(!env_bool("TEST_BOOL_FALSE", true));
        assert!(env_bool("TEST_BOOL_ONE", false));
        assert!(!env_bool("TEST_BOOL_ZERO", true));
        assert_eq!(env_bool("NON_EXISTENT", true), true);
        assert_eq!(env_bool("NON_EXISTENT", false), false);
    }

    #[test]
    fn test_env_u64() {
        std::env::set_var("TEST_U64", "12345");
        std::env::set_var("TEST_U64_INVALID", "not_a_number");

        assert_eq!(env_u64("TEST_U64", 0), 12345);
        assert_eq!(env_u64("TEST_U64_INVALID", 999), 999);
        assert_eq!(env_u64("NON_EXISTENT", 42), 42);
    }

    #[test]
    fn test_env_f64() {
        std::env::set_var("TEST_F64", "3.14");
        std::env::set_var("TEST_F64_INVALID", "not_a_number");

        assert_eq!(env_f64("TEST_F64", 0.0), 3.14);
        assert_eq!(env_f64("TEST_F64_INVALID", 2.71), 2.71);
        assert_eq!(env_f64("NON_EXISTENT", 1.0), 1.0);
    }

    #[test]
    fn test_parse_seed_genes() {
        let raw = "KRAS, TP53, EGFR,invalid, BRCA1,, EGFR";
        let genes = parse_seed_genes(raw);

        assert_eq!(genes.len(), 4);
        assert!(genes.contains(&"KRAS".to_string()));
        assert!(genes.contains(&"TP53".to_string()));
        assert!(genes.contains(&"EGFR".to_string()));
        assert!(genes.contains(&"BRCA1".to_string()));
        // Duplicates should be removed
        assert_eq!(genes.iter().filter(|&g| g == "EGFR").count(), 1);
    }

    #[test]
    fn test_looks_like_gene_symbol() {
        assert!(looks_like_gene_symbol("KRAS"));
        assert!(looks_like_gene_symbol("TP53"));
        assert!(looks_like_gene_symbol("EGFR"));
        assert!(looks_like_gene_symbol("BRCA1"));
        assert!(looks_like_gene_symbol("MYC-1"));

        assert!(!looks_like_gene_symbol("kras")); // lowercase
        assert!(!looks_like_gene_symbol("A")); // too short
        assert!(!looks_like_gene_symbol("VERYVERYLONGGENENAME")); // too long
        assert!(!looks_like_gene_symbol("GENE@SYMBOL")); // invalid char
        assert!(!looks_like_gene_symbol("")); // empty
    }

    #[test]
    fn test_background_provider_refresh_config_from_env() {
        std::env::set_var("FERRUMYX_PHASE4_BG_REFRESH_ENABLED", "false");
        std::env::set_var("FERRUMYX_PHASE4_BG_REFRESH_INTERVAL_SECS", "1800");
        std::env::set_var("FERRUMYX_PHASE4_BG_REFRESH_MAX_GENES", "50");
        std::env::set_var("FERRUMYX_PHASE4_BG_REFRESH_BATCH_SIZE", "10");
        std::env::set_var("FERRUMYX_PHASE4_BG_REFRESH_RETRIES", "2");
        std::env::set_var("FERRUMYX_PHASE4_BG_REFRESH_CANCER_CODE", "LUAD");
        std::env::set_var("FERRUMYX_PHASE4_BG_REFRESH_GENES", "KRAS,TP53");

        let config = BackgroundProviderRefreshConfig::from_env();

        assert!(!config.enabled);
        assert_eq!(config.interval_secs, 1800);
        assert_eq!(config.max_genes, 50);
        assert_eq!(config.batch_size, 10);
        assert_eq!(config.retries, 2);
        assert_eq!(config.cancer_code, Some("LUAD".to_string()));
        assert_eq!(config.seed_genes, vec!["KRAS".to_string(), "TP53".to_string()]);

        // Clean up
        std::env::remove_var("FERRUMYX_PHASE4_BG_REFRESH_ENABLED");
        std::env::remove_var("FERRUMYX_PHASE4_BG_REFRESH_INTERVAL_SECS");
        std::env::remove_var("FERRUMYX_PHASE4_BG_REFRESH_MAX_GENES");
        std::env::remove_var("FERRUMYX_PHASE4_BG_REFRESH_BATCH_SIZE");
        std::env::remove_var("FERRUMYX_PHASE4_BG_REFRESH_RETRIES");
        std::env::remove_var("FERRUMYX_PHASE4_BG_REFRESH_CANCER_CODE");
        std::env::remove_var("FERRUMYX_PHASE4_BG_REFRESH_GENES");
    }

    #[test]
    fn test_is_local_base_url() {
        assert!(is_local_base_url("http://localhost:11434"));
        assert!(is_local_base_url("https://127.0.0.1:8080"));
        assert!(!is_local_base_url("https://api.openai.com"));
        assert!(!is_local_base_url("https://api.anthropic.com"));
    }

    #[test]
    fn test_pick_ollama_model_for_hardware() {
        // This is a basic test - in a real scenario we'd mock the system info
        let model = pick_ollama_model_for_hardware();
        assert!(!model.is_empty());
        assert!(model.starts_with("llama"));
    }

    #[tokio::test]
    async fn test_ollama_healthy() {
        let client = reqwest::Client::new();

        // Test with invalid URL (should return false)
        let result = ollama_healthy(&client, "http://invalid-url-that-does-not-exist").await;
        assert!(!result);
    }

    #[test]
    fn test_sync_runtime_env_from_config() {
        let mut config = config::Config::default();
        config.llm.default_backend = "anthropic".to_string();
        config.llm.anthropic = Some(config::AnthropicBackendConfig {
            api_key: "test-key".to_string(),
            model: "claude-3".to_string(),
        });

        sync_runtime_env_from_config(&config);

        assert_eq!(std::env::var("LLM_BACKEND").unwrap(), "anthropic");
        assert_eq!(std::env::var("ANTHROPIC_API_KEY").unwrap(), "test-key");
        assert_eq!(std::env::var("ANTHROPIC_MODEL").unwrap(), "claude-3");
    }
}

