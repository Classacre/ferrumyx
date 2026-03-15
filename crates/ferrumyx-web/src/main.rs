//! Ferrumyx Web Server
//!
//! Run with: cargo run -p ferrumyx-web

use std::net::SocketAddr;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Ferrumyx Web Server...");

    // Create app state
    let state = ferrumyx_web::state::AppState::new_without_db().await?;

    // Build router
    let app = ferrumyx_web::router::build_router(state);

    // Bind to port (override with FERRUMYX_WEB_ADDR, e.g. 127.0.0.1:3005)
    let addr: SocketAddr = std::env::var("FERRUMYX_WEB_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3001".to_string())
        .parse()?;
    info!("🚀 Server listening on http://{}", addr);
    info!("📱 Open your browser and navigate to http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
