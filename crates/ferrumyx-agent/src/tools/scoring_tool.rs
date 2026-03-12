use async_trait::async_trait;
use ironclaw::context::JobContext;
use ironclaw::tools::{Tool, ToolError, ToolOutput};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

use ferrumyx_db::Database;

/// Tool to force a target-score recomputation from current KG facts.
pub struct RecomputeTargetScoresTool {
    db: Arc<Database>,
}

impl RecomputeTargetScoresTool {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl Tool for RecomputeTargetScoresTool {
    fn name(&self) -> &str {
        "recompute_target_scores"
    }

    fn description(&self) -> &str {
        "Recomputes persisted target scores using current knowledge graph evidence."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    fn execution_timeout(&self) -> Duration {
        // Score recomputation can exceed default chat/tool limits on larger KG states.
        Duration::from_secs(20 * 60)
    }

    async fn execute(
        &self,
        _params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let started = std::time::Instant::now();
        let upserted = ferrumyx_kg::compute_target_scores(self.db.clone())
            .await
            .map_err(|e| {
                ToolError::ExecutionFailed(format!("target score recompute failed: {e}"))
            })?;

        Ok(ToolOutput::success(
            json!({
                "status": "ok",
                "target_scores_upserted": upserted
            }),
            started.elapsed(),
        ))
    }
}
