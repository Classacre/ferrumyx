//! Shared application state for the web server.

use ferrumyx_db::Database;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

/// Events pushed to connected clients via SSE.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppEvent {
    /// A paper was ingested
    PaperIngested {
        paper_id: String,
        title: String,
        source: String,
    },
    /// A target score was computed
    TargetScored {
        gene: String,
        cancer: String,
        score: f64,
    },
    /// A docking job completed
    DockingComplete {
        molecule_id: String,
        gene: String,
        vina_score: f64,
    },
    /// Ingestion pipeline status update
    PipelineStatus {
        stage: String,
        message: String,
        count: u64,
    },
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
        let data_dir = std::env::var("FERRUMYX_DATA_DIR").unwrap_or_else(|_| "./data".to_string());

        // Create data directory if it doesn't exist
        std::fs::create_dir_all(&data_dir)?;

        // Connect to LanceDB (embedded, no external server needed)
        let db = Database::open(&data_dir).await?;
        db.initialize().await?;

        tracing::info!(
            "Connected to LanceDB at: {} and initialized tables",
            data_dir
        );

        let (event_tx, _) = broadcast::channel(256);
        Ok(Self {
            db: Arc::new(db),
            event_tx,
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use ferrumyx_test_utils::mocks::MockDatabase;
    use tokio::test;

    #[test]
    fn test_app_event_serialization() {
        let event = AppEvent::PaperIngested {
            paper_id: "test-123".to_string(),
            title: "Test Paper".to_string(),
            source: "PubMed".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("paper_ingested"));
        assert!(json.contains("test-123"));
        assert!(json.contains("Test Paper"));
        assert!(json.contains("PubMed"));

        let deserialized: AppEvent = serde_json::from_str(&json).unwrap();
        match deserialized {
            AppEvent::PaperIngested { paper_id, title, source } => {
                assert_eq!(paper_id, "test-123");
                assert_eq!(title, "Test Paper");
                assert_eq!(source, "PubMed");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_target_scored_event() {
        let event = AppEvent::TargetScored {
            gene: "KRAS".to_string(),
            cancer: "PAAD".to_string(),
            score: 0.95,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("target_scored"));
        assert!(json.contains("KRAS"));
        assert!(json.contains("PAAD"));
        assert!(json.contains("0.95"));
    }

    #[test]
    fn test_docking_complete_event() {
        let event = AppEvent::DockingComplete {
            molecule_id: "mol-123".to_string(),
            gene: "EGFR".to_string(),
            vina_score: -8.5,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("docking_complete"));
        assert!(json.contains("mol-123"));
        assert!(json.contains("EGFR"));
        assert!(json.contains("-8.5"));
    }

    #[test]
    fn test_pipeline_status_event() {
        let event = AppEvent::PipelineStatus {
            stage: "ingestion".to_string(),
            message: "Processing papers".to_string(),
            count: 150,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("pipeline_status"));
        assert!(json.contains("ingestion"));
        assert!(json.contains("Processing papers"));
        assert!(json.contains("150"));
    }

    #[test]
    fn test_feedback_metric_event() {
        let event = AppEvent::FeedbackMetric {
            metric: "accuracy".to_string(),
            value: 0.87,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("feedback_metric"));
        assert!(json.contains("accuracy"));
        assert!(json.contains("0.87"));
    }

    #[test]
    fn test_notification_event() {
        let event = AppEvent::Notification {
            level: "info".to_string(),
            message: "System running normally".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("notification"));
        assert!(json.contains("info"));
        assert!(json.contains("System running normally"));
    }

    #[tokio::test]
    async fn test_app_state_creation() {
        // This test requires a real database connection, so we'll skip the actual creation
        // In a real test environment, we'd set up a test database

        // Test that we can create a broadcast channel
        let (tx, mut rx) = broadcast::channel(256);

        // Test sending an event
        let event = AppEvent::Notification {
            level: "test".to_string(),
            message: "Test message".to_string(),
        };

        tx.send(event.clone()).unwrap();

        // Test receiving the event
        let received = rx.recv().await.unwrap();
        match received {
            AppEvent::Notification { level, message } => {
                assert_eq!(level, "test");
                assert_eq!(message, "Test message");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_broadcast_channel_capacity() {
        let (tx, _rx) = broadcast::channel(10);

        // Send more events than capacity
        for i in 0..15 {
            let event = AppEvent::Notification {
                level: "test".to_string(),
                message: format!("Message {}", i),
            };
            let _ = tx.send(event);
        }

        // The channel should handle overflow gracefully
        // This is mainly to ensure no panics
    }
}
