//! Shared application state for the web server.

use std::sync::Arc;
use sqlx::PgPool;
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
    pub db: PgPool,
    /// Broadcast channel for SSE push events
    pub event_tx: broadcast::Sender<AppEvent>,
}

impl AppState {
    pub fn new(db: PgPool) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self { db, event_tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.event_tx.subscribe()
    }
}

pub type SharedState = Arc<AppState>;
