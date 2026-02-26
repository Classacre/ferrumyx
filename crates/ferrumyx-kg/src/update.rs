//! Event-driven KG update rules.
//! See ARCHITECTURE.md §3.4

use ferrumyx_common::confidence::aggregate_confidence;
use tokio::sync::mpsc;
use std::sync::Arc;
use ferrumyx_db::Database;
use tracing::{info, warn};

/// Represents a trigger event from the routines engine.
#[derive(Debug, Clone)]
pub enum KgUpdateTrigger {
    /// A new fact was inserted for (subject, predicate, object).
    NewFact {
        subject_id: uuid::Uuid,
        predicate: String,
        object_id: uuid::Uuid,
        new_confidence: f64,
    },
    /// An existing fact's confidence changed significantly.
    FactConfidenceChanged {
        fact_id: uuid::Uuid,
        old_confidence: f64,
        new_confidence: f64,
        subject_id: uuid::Uuid,
    },
}

/// Determines whether a target re-scoring should be queued.
/// See ARCHITECTURE.md §3.4: re-score if confidence delta > 0.05
pub fn should_requeue_scoring(old_confidence: f64, new_confidence: f64) -> bool {
    (new_confidence - old_confidence).abs() > 0.05
}

/// Start the background event-driven scoring queue constraint.
/// Listens to new kg_facts insertion events, and if they breach the confidence threshold,
/// triggers the Target Prioritisation engine natively.
pub fn start_scoring_event_queue(db: Arc<Database>) -> mpsc::UnboundedSender<KgUpdateTrigger> {
    let (tx, mut rx) = mpsc::unbounded_channel::<KgUpdateTrigger>();
    
    tokio::spawn(async move {
        info!("Started event-driven KG scoring queue worker");
        
        while let Some(event) = rx.recv().await {
            match event {
                KgUpdateTrigger::FactConfidenceChanged { old_confidence, new_confidence, subject_id, .. } => {
                    if should_requeue_scoring(old_confidence, new_confidence) {
                        info!("Confidence delta > 0.05 for target {:?}. Queuing re-score...", subject_id);
                        // Trigger async recompute
                        if let Err(e) = crate::scoring::compute_target_scores(db.clone()).await {
                            warn!("Failed to re-score targets: {}", e);
                        }
                    }
                },
                KgUpdateTrigger::NewFact { subject_id, new_confidence, .. } => {
                    if should_requeue_scoring(0.0, new_confidence) {
                        info!("New strong fact added for target {:?}. Queuing re-score...", subject_id);
                        if let Err(e) = crate::scoring::compute_target_scores(db.clone()).await {
                            warn!("Failed to re-score targets: {}", e);
                        }
                    }
                }
            }
        }
    });

    tx
}

/// Recompute aggregate confidence after adding a new evidence item.
/// Uses noisy-OR model (see ARCHITECTURE.md §3.3).
pub fn recompute_aggregate(existing_confidences: &[f64], new_confidence: f64) -> f64 {
    let mut all = existing_confidences.to_vec();
    all.push(new_confidence);
    aggregate_confidence(&all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_requeue_on_large_delta() {
        assert!(should_requeue_scoring(0.5, 0.56));
        assert!(!should_requeue_scoring(0.5, 0.53));
    }

    #[test]
    fn test_aggregate_increases_with_new_evidence() {
        let existing = vec![0.7, 0.6];
        let old_agg = aggregate_confidence(&existing);
        let new_agg = recompute_aggregate(&existing, 0.8);
        assert!(new_agg > old_agg);
    }
}
