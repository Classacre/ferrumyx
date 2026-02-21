//! Ferrumyx — Autonomous Oncology Drug Discovery Engine
//! Entry point for the agent binary.

mod config;
mod tools;

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

    // Connect to PostgreSQL
    info!("Connecting to PostgreSQL...");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .min_connections(config.database.min_connections)
        .connect(&config.database.url)
        .await
        .map_err(|e| anyhow::anyhow!(
            "DB connection failed: {e}\nIs PostgreSQL running?\nTry: cd docker && docker compose up -d"
        ))?;

    info!("✅ PostgreSQL connected.");

    // Run pending migrations
    info!("Running schema migrations...");
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("Migration failed: {e}"))?;
    info!("✅ Migrations complete.");

    // Register Ferrumyx tools
    let tool_registry = tools::build_default_registry(pool.clone());
    info!("✅ Tool registry ready: {} tools registered.", tool_registry.len());
    let _ = tool_registry; // Will be stored in AppState in a later phase

    // Build app state and router
    let state = ferrumyx_web::state::AppState::new(pool);
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
