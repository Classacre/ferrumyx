//! Shared application state for the web server.

use std::sync::Arc;
use ferrumyx_db::Database;
use tokio::sync::broadcast;
use serde::{Deserialize, Serialize};

/// Events pushed to connected clients via SSE.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppEvent {
    /// A paper was ingested
    PaperIngested { paper_id: String, title: String, source: String },
    /// A target score was computed
    TargetScored { gene: String, cancer: String, score: f64 },
    /// A docking job completed
    DockingComplete { molecule_id: String, gene: String, vina_score: f64 },
    /// Ingestion pipeline status update
    PipelineStatus { stage: String, message: String, count: u64 },
    /// Feedback metric computed
    FeedbackMetric { metric: String, value: f64 },
    /// General system notification
    Notification { level: String, message: String },
}

/// Shared state injected into every Axum handler.
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    /// Broadcast channel for SSE push events
    pub event_tx: broadcast::Sender<AppEvent>,
}

impl AppState {
    pub fn new(db: Arc<Database>) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self { db, event_tx }
    }

    /// Create state with embedded database (LanceDB)
    pub async fn new_with_db() -> anyhow::Result<Self> {
        // Get data directory from environment or use default
        let data_dir = std::env::var("FERRUMYX_DATA_DIR")
            .unwrap_or_else(|_| "./data".to_string());
        
        // Create data directory if it doesn't exist
        std::fs::create_dir_all(&data_dir)?;
        
        // Connect to LanceDB (embedded, no external server needed)
        let db = Database::open(&data_dir).await?;
        
        tracing::info!("Connected to LanceDB at: {}", data_dir);
        
        let (event_tx, _) = broadcast::channel(256);
        Ok(Self { db: Arc::new(db), event_tx })
    }

    /// Create state without database (for testing/demo)
    pub async fn new_without_db() -> anyhow::Result<Self> {
        Self::new_with_db().await
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.event_tx.subscribe()
    }
}

pub type SharedState = Arc<AppState>;
