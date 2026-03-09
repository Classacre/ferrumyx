//! Ferrumyx — Autonomous Oncology Drug Discovery Engine
//! Entry point for the agent binary.

use std::sync::Arc;
use std::process::Stdio;
use std::time::Duration;
use std::io::IsTerminal;

mod config;
mod tools;
use rig::providers::openai::Client as OpenAiClient;
use rig::providers::anthropic::Client as AnthropicClient;
use rig::providers::gemini::Client as GeminiClient;
use rig::client::CompletionClient;
use serde::Deserialize;
use tokio::process::Command;

/// Returns a Boxed CompletionModel to inject into the Agent.
/// It natively maps the Ferrumyx config directly to `rig-core` LLM clients.
async fn build_completion_model(config: &config::Config) -> anyhow::Result<Arc<dyn ironclaw::llm::LlmProvider>> {
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

fn try_build_openai(config: &config::Config) -> anyhow::Result<Option<Arc<dyn ironclaw::llm::LlmProvider>>> {
    if let Some(ref openai) = config.llm.openai {
        let key = if openai.api_key.is_empty() {
            std::env::var("FERRUMYX_OPENAI_API_KEY").unwrap_or_default()
        } else {
            openai.api_key.clone()
        };
        if !key.is_empty() {
            tracing::info!("Using OpenAI: {}", openai.model);
            let client: OpenAiClient = OpenAiClient::new(&key)?;
            return Ok(Some(Arc::new(ironclaw::llm::RigAdapter::new(client.completion_model(&openai.model), &openai.model))));
        }
    }
    Ok(None)
}

fn try_build_anthropic(config: &config::Config) -> anyhow::Result<Option<Arc<dyn ironclaw::llm::LlmProvider>>> {
    if let Some(ref anthropic) = config.llm.anthropic {
        let key = if anthropic.api_key.is_empty() {
            std::env::var("FERRUMYX_ANTHROPIC_API_KEY").unwrap_or_default()
        } else {
            anthropic.api_key.clone()
        };
        if !key.is_empty() {
            tracing::info!("Using Anthropic: {}", anthropic.model);
            let client: AnthropicClient = AnthropicClient::new(&key)?;
            return Ok(Some(Arc::new(ironclaw::llm::RigAdapter::new(client.completion_model(&anthropic.model), &anthropic.model))));
        }
    }
    Ok(None)
}

fn try_build_gemini(config: &config::Config) -> anyhow::Result<Option<Arc<dyn ironclaw::llm::LlmProvider>>> {
    if let Some(ref gemini) = config.llm.gemini {
        let key = if gemini.api_key.is_empty() {
            std::env::var("FERRUMYX_GEMINI_API_KEY").unwrap_or_default()
        } else {
            gemini.api_key.clone()
        };
        if !key.is_empty() {
            tracing::info!("Using Gemini: {}", gemini.model);
            let client: GeminiClient = GeminiClient::new(&key)?;
            return Ok(Some(Arc::new(ironclaw::llm::RigAdapter::new(client.completion_model(&gemini.model), &gemini.model))));
        }
    }
    Ok(None)
}

fn try_build_openai_compatible(config: &config::Config) -> anyhow::Result<Option<Arc<dyn ironclaw::llm::LlmProvider>>> {
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
        let api_key = if key.is_empty() { "none".to_string() } else { key };
        tracing::info!(
            "Using OpenAI-compatible backend: {} ({}) [cached_chat={}]",
            compat.model,
            compat.base_url,
            compat.cached_chat
        );
        let client: OpenAiClient = OpenAiClient::builder()
            .base_url(&compat.base_url)
            .api_key(&api_key)
            .build()?;
        return Ok(Some(Arc::new(ironclaw::llm::RigAdapter::new(client.completion_model(&compat.model), &compat.model))));
    }
    Ok(None)
}

async fn try_build_ollama(config: &config::Config) -> anyhow::Result<Option<Arc<dyn ironclaw::llm::LlmProvider>>> {
    if let Some(ref ollama) = config.llm.ollama {
        let model = ensure_ollama_ready(&ollama.base_url, &ollama.model).await;
        // Fallback to local Ollama (OpenAI compatible API)
        tracing::info!("Using Local Ollama: {}", model);
        let client: OpenAiClient = OpenAiClient::builder()
            .base_url(&format!("{}/v1", ollama.base_url))
            .api_key("ollama")
            .build()?;
        return Ok(Some(Arc::new(ironclaw::llm::RigAdapter::new(client.completion_model(&model), &model))));
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
    let selected = if configured_model.trim().is_empty() || configured_model.eq_ignore_ascii_case("auto") {
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
                tracing::info!("Ollama model '{}' not found locally. Starting background pull...", selected);
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
            info!("Configuration loaded. LLM mode: {}, Focus: {} {}",
                c.llm.mode, c.scoring.focus_cancer, c.scoring.focus_mutation);
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
    let kg_event_tx = ferrumyx_kg::update::start_scoring_event_queue(db.clone());
    info!("✅ KG event-driven scoring queue initialized.");

    // Build LLM client
    let ironclaw_llm = build_completion_model(&config).await?;

    // Build Tool Registry
    let tool_registry = Arc::new(ironclaw::tools::ToolRegistry::new());
    tool_registry.register_sync(Arc::new(tools::ingestion_tool::IngestionTool::new(db.clone())));
    tool_registry.register_sync(Arc::new(tools::query_tool::TargetQueryTool::new(db.clone())));
    tool_registry.register_sync(Arc::new(tools::workflow_status_tool::WorkflowStatusTool::new(db.clone())));
    tool_registry.register_sync(Arc::new(tools::scoring_tool::RecomputeTargetScoresTool::new(db.clone())));
    tool_registry.register_sync(Arc::new(tools::provider_refresh_tool::RefreshProviderSignalsTool::new(db.clone())));
    tool_registry.register_sync(Arc::new(tools::molecule_tool::RunMoleculePipelineTool::new()));
    tool_registry.register_sync(Arc::new(tools::autonomous_cycle_tool::AutonomousCycleTool::new(db.clone())));

    // Build Skill Registry
    let skill_registry = std::sync::Arc::new(std::sync::RwLock::new(
        ironclaw::skills::registry::SkillRegistry::new(std::path::PathBuf::from("./data/skills"))
    ));
    let skill_catalog = std::sync::Arc::new(ironclaw::skills::catalog::SkillCatalog::new());

    let deps = ironclaw::agent::AgentDeps {
        store: None,
        llm: ironclaw_llm.clone(),
        cheap_llm: None,
        safety: std::sync::Arc::new(ironclaw::safety::SafetyLayer::new(&ironclaw::config::SafetyConfig {
            max_output_length: 100_000,
            injection_check_enabled: true,
        })),
        tools: tool_registry.clone(),
        workspace: None,
        extension_manager: None,
        skill_registry: Some(skill_registry.clone()),
        skill_catalog: Some(skill_catalog.clone()),
        skills_config: ironclaw::config::SkillsConfig::default(),
        hooks: std::sync::Arc::new(ironclaw::hooks::HookRegistry::new()),
        cost_guard: std::sync::Arc::new(ironclaw::agent::cost_guard::CostGuard::new(ironclaw::agent::cost_guard::CostGuardConfig::default())),
        sse_tx: None,
        http_interceptor: None,
        transcription: None,
        document_extraction: None,
    };

    let mut channels = ironclaw::channels::ChannelManager::new();
    let disable_repl = std::env::var("FERRUMYX_DISABLE_REPL")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !disable_repl && std::io::stdin().is_terminal() {
        channels.add(Box::new(ironclaw::channels::ReplChannel::new())).await;
    } else {
        tracing::info!("Non-interactive runtime detected; skipping REPL channel.");
    }

    let gw_config = ironclaw::config::GatewayConfig {
        host: "127.0.0.1".to_string(),
        port: 3002,
        user_id: "User".to_string(),
        auth_token: Some("ferrumyx-local-dev-token".to_string()),
    };
    channels.add(Box::new(ironclaw::channels::GatewayChannel::new(gw_config))).await;

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
        None,
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
    let bind_addr = std::env::var("FERRUMYX_BIND")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string());

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
