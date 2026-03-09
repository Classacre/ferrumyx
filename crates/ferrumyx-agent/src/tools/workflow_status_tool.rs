use async_trait::async_trait;
use ironclaw::context::JobContext;
use ironclaw::tools::{Tool, ToolError, ToolOutput};
use serde_json::json;
use std::sync::Arc;

use ferrumyx_db::target_scores::TargetScoreRepository;
use ferrumyx_db::Database;

/// Tool to inspect high-level workflow progress and top-ranked targets.
pub struct WorkflowStatusTool {
    db: Arc<Database>,
}

impl WorkflowStatusTool {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl Tool for WorkflowStatusTool {
    fn name(&self) -> &str {
        "workflow_status"
    }

    fn description(&self) -> &str {
        "Returns Ferrumyx pipeline status: DB counts and current top target scores."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "top_n": {
                    "type": "integer",
                    "description": "How many top target rows to include (default: 10, max: 50)"
                }
            }
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let top_n = params
            .get("top_n")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(10)
            .clamp(1, 50);

        let started = std::time::Instant::now();
        let stats = self
            .db
            .stats()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("stats failed: {e}")))?;

        let score_repo = TargetScoreRepository::new(self.db.clone());
        let mut scores = score_repo
            .list(0, 20_000)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("target score fetch failed: {e}")))?;
        scores.sort_by(|a, b| {
            b.confidence_adjusted_score
                .partial_cmp(&a.confidence_adjusted_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scores.truncate(top_n);

        let top_targets: Vec<serde_json::Value> = scores
            .into_iter()
            .map(|s| {
                let mut gene = s.gene_id.to_string();
                let mut cancer = s.cancer_id.to_string();
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s.components_raw) {
                    if let Some(g) = v.get("gene").and_then(|x| x.as_str()) {
                        gene = g.to_string();
                    }
                    if let Some(c) = v.get("cancer_code").and_then(|x| x.as_str()) {
                        cancer = c.to_string();
                    }
                }
                json!({
                    "gene": gene,
                    "cancer": cancer,
                    "score": s.composite_score,
                    "confidence_adjusted_score": s.confidence_adjusted_score,
                    "tier": s.shortlist_tier,
                })
            })
            .collect();

        Ok(ToolOutput::success(
            json!({
                "db": {
                    "papers": stats.papers,
                    "chunks": stats.chunks,
                    "entities": stats.entities,
                    "entity_mentions": stats.entity_mentions,
                    "kg_facts": stats.kg_facts,
                    "target_scores": stats.target_scores,
                    "ingestion_audit": stats.ingestion_audit
                },
                "top_targets": top_targets
            }),
            started.elapsed(),
        ))
    }
}
