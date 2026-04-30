//! Ferrumyx Web Server
//!
//! Run with: cargo run -p ferrumyx-web

use std::net::SocketAddr;
use std::time::Duration;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;
use tokio::task;

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
        .unwrap_or_else(|_| "127.0.0.1:3000".to_string())
        .parse()?;
    info!("🚀 Server listening on http://{}", addr);
    info!("📱 Open your browser and navigate to http://{}", addr);

    // Start memory monitoring task
    let _monitor_handle = task::spawn(async {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Ok(usage) = sys_info::mem_info() {
                let used_mb = (usage.total - usage.free) / 1024;
                if used_mb > 1000 { // Warn if > 1GB used
                    warn!("High memory usage detected: {} MB", used_mb);
                }
            }
        }
    });

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
