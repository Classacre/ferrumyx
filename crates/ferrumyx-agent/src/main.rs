//! Ferrumyx — Autonomous Oncology Drug Discovery Engine
//! Entry point for the agent binary.

mod config;
mod tools;

use ferrumyx_llm::router::{build_router, BackendConfig, BackendKind, RoutingPolicy};

fn build_llm_backends(config: &config::Config) -> ferrumyx_llm::router::LlmRouter {
    let policy = RoutingPolicy {
        local_only_mode:       config.llm.mode == "local_only",
        allow_internal_remote: false,
        default_backend:       config.llm.default_backend.clone(),
        local_backend:         config.llm.local_backend.clone(),
    };

    let mut backends: Vec<BackendConfig> = Vec::new();

    if let Some(ref ollama) = config.llm.ollama {
        backends.push(BackendConfig {
            name:            "ollama".to_string(),
            kind:            BackendKind::Ollama,
            model:           ollama.model.clone(),
            api_key:         None,
            base_url:        Some(ollama.base_url.clone()),
            embedding_model: None,
        });
    }

    if let Some(ref openai) = config.llm.openai {
        let key = if openai.api_key.is_empty() {
            std::env::var("FERRUMYX_OPENAI_API_KEY").unwrap_or_default()
        } else {
            openai.api_key.clone()
        };
        if !key.is_empty() {
            backends.push(BackendConfig {
                name:            "openai".to_string(),
                kind:            BackendKind::OpenAi,
                model:           openai.model.clone(),
                api_key:         Some(key),
                base_url:        None,
                embedding_model: openai.embedding_model.clone(),
            });
        } else {
            tracing::warn!("OpenAI configured but no API key found (set llm.openai.api_key or FERRUMYX_OPENAI_API_KEY)");
        }
    }

    if let Some(ref anthropic) = config.llm.anthropic {
        let key = if anthropic.api_key.is_empty() {
            std::env::var("FERRUMYX_ANTHROPIC_API_KEY").unwrap_or_default()
        } else {
            anthropic.api_key.clone()
        };
        if !key.is_empty() {
            backends.push(BackendConfig {
                name:            "anthropic".to_string(),
                kind:            BackendKind::Anthropic,
                model:           anthropic.model.clone(),
                api_key:         Some(key),
                base_url:        None,
                embedding_model: None,
            });
        } else {
            tracing::warn!("Anthropic configured but no API key found (set llm.anthropic.api_key or FERRUMYX_ANTHROPIC_API_KEY)");
        }
    }

    if let Some(ref gemini) = config.llm.gemini {
        let key = if gemini.api_key.is_empty() {
            std::env::var("FERRUMYX_GEMINI_API_KEY").unwrap_or_default()
        } else {
            gemini.api_key.clone()
        };
        if !key.is_empty() {
            backends.push(BackendConfig {
                name:            "gemini".to_string(),
                kind:            BackendKind::Gemini,
                model:           gemini.model.clone(),
                api_key:         Some(key),
                base_url:        None,
                embedding_model: gemini.embedding_model.clone(),
            });
        } else {
            tracing::warn!("Gemini configured but no API key found (set llm.gemini.api_key or FERRUMYX_GEMINI_API_KEY)");
        }
    }

    if let Some(ref compat) = config.llm.openai_compatible {
        let key = if compat.api_key.is_empty() {
            std::env::var("FERRUMYX_COMPAT_API_KEY").ok()
        } else {
            Some(compat.api_key.clone())
        };
        backends.push(BackendConfig {
            name:            "openai_compatible".to_string(),
            kind:            BackendKind::OpenAiCompatible,
            model:           compat.model.clone(),
            api_key:         key,
            base_url:        Some(compat.base_url.clone()),
            embedding_model: compat.embedding_model.clone(),
        });
    }

    if backends.is_empty() {
        tracing::warn!(
            "No LLM backends configured! Add at least one provider to ferrumyx.toml. \
             NER extraction and KG summarisation will be unavailable."
        );
    }

    build_router(backends, policy)
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

    // Build LLM router from config
    let llm_backends = build_llm_backends(&config);
    let n_backends = llm_backends.registered_backends().len();
    info!("✅ LLM router ready: {} backends registered.", n_backends);

    // Register Ferrumyx tools
    let tool_registry = tools::build_default_registry(db.clone());
    info!("✅ Tool registry ready");
    // Initialize IronClaw Agent with Ollama (if configured)
    if let Some(ref ollama_cfg) = config.llm.ollama {
        info!("🤖 Initializing IronClaw Agent with Ollama model: {}", ollama_cfg.model);
        
        let client: rig::providers::ollama::Client = rig::providers::ollama::Client::builder()
            .base_url(&ollama_cfg.base_url)
            .api_key(rig::client::Nothing)
            .build()
            .expect("Failed to build Ollama client");
            
        use rig::client::CompletionClient;
        let model = client.completion_model(&ollama_cfg.model);
        let ironclaw_llm = std::sync::Arc::new(ironclaw::llm::RigAdapter::new(model, &ollama_cfg.model));
        
        let deps = ironclaw::agent::AgentDeps {
            store: None,
            llm: ironclaw_llm,
            cheap_llm: None,
            safety: std::sync::Arc::new(ironclaw::safety::SafetyLayer::new(&ironclaw::config::SafetyConfig {
                max_output_length: 100_000,
                injection_check_enabled: true,
            })),
            tools: tool_registry.clone(),
            workspace: None,
            extension_manager: None,
            skill_registry: None,
            skill_catalog: None,
            skills_config: ironclaw::config::SkillsConfig::default(),
            hooks: std::sync::Arc::new(ironclaw::hooks::HookRegistry::new()),
            cost_guard: std::sync::Arc::new(ironclaw::agent::cost_guard::CostGuard::new(ironclaw::agent::cost_guard::CostGuardConfig::default())),
        };

        let channels = std::sync::Arc::new(ironclaw::channels::ChannelManager::new());
        let repl = ironclaw::channels::ReplChannel::new();
        channels.add(Box::new(repl)).await;

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
            channels,
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
    } else {
        tracing::warn!("Ollama is not configured in ferrumyx.toml, IronClaw agent loop will not start.");
    }

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
