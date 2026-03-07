//! Ferrumyx — Autonomous Oncology Drug Discovery Engine
//! Entry point for the agent binary.

use std::sync::Arc;
use ironclaw::llm;

mod config;
mod tools;
use rig::providers::openai::Client as OpenAiClient;
use rig::providers::anthropic::Client as AnthropicClient;
use rig::providers::gemini::Client as GeminiClient;
use rig::client::CompletionClient;

/// Returns a Boxed CompletionModel to inject into the Agent.
/// It natively maps the Ferrumyx config directly to `rig-core` LLM clients.
fn build_completion_model(config: &config::Config) -> anyhow::Result<Arc<dyn ironclaw::llm::LlmProvider>> {
    if let Some(ref openai) = config.llm.openai {
        let key = if openai.api_key.is_empty() {
            std::env::var("FERRUMYX_OPENAI_API_KEY").unwrap_or_default()
        } else {
            openai.api_key.clone()
        };
        if !key.is_empty() {
            tracing::info!("Using OpenAI: {}", openai.model);
            let client: OpenAiClient = OpenAiClient::new(&key)?;
            return Ok(Arc::new(ironclaw::llm::RigAdapter::new(client.completion_model(&openai.model), &openai.model)));
        }
    }

    if let Some(ref anthropic) = config.llm.anthropic {
        let key = if anthropic.api_key.is_empty() {
            std::env::var("FERRUMYX_ANTHROPIC_API_KEY").unwrap_or_default()
        } else {
            anthropic.api_key.clone()
        };
        if !key.is_empty() {
            tracing::info!("Using Anthropic: {}", anthropic.model);
            let client: AnthropicClient = AnthropicClient::new(&key)?;
            return Ok(Arc::new(ironclaw::llm::RigAdapter::new(client.completion_model(&anthropic.model), &anthropic.model)));
        }
    }

    if let Some(ref gemini) = config.llm.gemini {
        let key = if gemini.api_key.is_empty() {
            std::env::var("FERRUMYX_GEMINI_API_KEY").unwrap_or_default()
        } else {
            gemini.api_key.clone()
        };
        if !key.is_empty() {
            tracing::info!("Using Gemini: {}", gemini.model);
            let client: GeminiClient = GeminiClient::new(&key)?;
            return Ok(Arc::new(ironclaw::llm::RigAdapter::new(client.completion_model(&gemini.model), &gemini.model)));
        }
    }

    if let Some(ref ollama) = config.llm.ollama {
        // Fallback to local Ollama (OpenAI compatible API)
        tracing::info!("Using Local Ollama: {}", ollama.model);
        let client: OpenAiClient = OpenAiClient::builder()
            .base_url(&format!("{}/v1", ollama.base_url))
            .api_key("ollama")
            .build()?;
        return Ok(Arc::new(ironclaw::llm::RigAdapter::new(client.completion_model(&ollama.model), &ollama.model)));
    }

    anyhow::bail!("No LLM providers were successfully configured in ferrumyx.toml")
}

use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    // Connect to LanceDB
    info!("Connecting to LanceDB...");
    let db = ferrumyx_db::Database::open(&config.database.url).await?;
    let db = std::sync::Arc::new(db);
    info!("✅ LanceDB connected.");

    // Start Phase 3: Knowledge Graph Event Queue
    let kg_event_tx = ferrumyx_kg::update::start_scoring_event_queue(db.clone());
    info!("✅ KG event-driven scoring queue initialized.");

    // Build LLM client
    let ironclaw_llm = build_completion_model(&config)?;

    // Build Tool Registry
    let tool_registry = Arc::new(ironclaw::tools::ToolRegistry::new());
    tool_registry.register_sync(Arc::new(tools::ingestion_tool::IngestionTool::new(db.clone())));

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
    };

    let mut channels = ironclaw::channels::ChannelManager::new();
    channels.add(Box::new(ironclaw::channels::ReplChannel::new())).await;

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
