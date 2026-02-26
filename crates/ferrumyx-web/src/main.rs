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

    // Bind to port
    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    info!("🚀 Server listening on http://{}", addr);
    info!("📱 Open your browser and navigate to http://localhost:3001");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
