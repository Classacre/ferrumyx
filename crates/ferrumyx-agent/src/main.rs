//! Ferrumyx — Autonomous Oncology Drug Discovery Engine
//! Entry point for the agent binary.

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

    // TODO Phase 1 Month 1:
    //   1. Load configuration (ferrumyx.toml)
    //   2. Connect to PostgreSQL + pgvector
    //   3. Register IronClaw tools
    //   4. Start agent loop / REPL

    info!("Ferrumyx agent placeholder — implementation begins Phase 1 Month 1.");
    Ok(())
}
