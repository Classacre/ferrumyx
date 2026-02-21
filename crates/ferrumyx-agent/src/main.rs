//! Ferrumyx — Autonomous Oncology Drug Discovery Engine
//! Entry point for the agent binary.

mod config;

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
            tracing::warn!("Running with defaults. Copy ferrumyx.example.toml to ferrumyx.toml to configure.");
            // Continue with placeholder — DB connection etc. will fail later
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
        .map_err(|e| anyhow::anyhow!("DB connection failed: {e}\nIs PostgreSQL running? Try: docker compose up -d (in ./docker/)"))?;

    info!("✅ PostgreSQL connected.");

    // Run pending migrations
    // Path is relative to workspace root (where Cargo.toml is), not the crate
    info!("Running schema migrations...");
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("Migration failed: {e}"))?;

    info!("✅ Migrations complete.");
    info!("🔬 Ferrumyx ready. IronClaw tool registration coming in next phase.");

    Ok(())
}
