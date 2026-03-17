use async_trait::async_trait;
use ironclaw::context::JobContext;
use ironclaw::tools::{Tool, ToolError, ToolOutput};
use serde_json::json;

use super::lab_state;

/// Returns status for one run or a compact list of most recent lab runs.
pub struct LabRunStatusTool;

impl LabRunStatusTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for LabRunStatusTool {
    fn name(&self) -> &str {
        "lab_run_status"
    }

    fn description(&self) -> &str {
        "Shows current state, evidence progression, and recommendations for lab autonomous runs."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "run_id": { "type": "string", "description": "Optional run id. If omitted, returns recent runs." },
                "limit": { "type": "integer", "description": "When run_id is omitted, number of recent runs to return." }
            }
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let started = std::time::Instant::now();
        let run_id = params
            .get("run_id")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());

        if let Some(run_id) = run_id {
            let state = lab_state::get_run(run_id)
                .ok_or_else(|| ToolError::InvalidParameters(format!("unknown run_id: {run_id}")))?;
            return Ok(ToolOutput::success(
                json!({
                    "status": "ok",
                    "mode": "single",
                    "run": summarize_run(&state),
                    "timeline": {
                        "retrieval_history": state.retrieval_history,
                        "validation_history": state.validation_history
                    },
                    "live_links": ["/chat", "/ingestion", "/kg", "/targets", "/metrics"]
                }),
                started.elapsed(),
            ));
        }

        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(8)
            .clamp(1, 25) as usize;
        let runs = lab_state::list_runs(limit);
        let run_summaries = runs.iter().map(summarize_run).collect::<Vec<_>>();

        Ok(ToolOutput::success(
            json!({
                "status": "ok",
                "mode": "list",
                "count": run_summaries.len(),
                "runs": run_summaries,
                "hint": "Call lab_run_status with a specific run_id for full retrieval/validation timeline."
            }),
            started.elapsed(),
        ))
    }
}

fn summarize_run(state: &lab_state::LabRunState) -> serde_json::Value {
    let last_retrieval = state.retrieval_history.last();
    let last_validation = state.validation_history.last();
    json!({
        "run_id": state.run_id,
        "stage": state.stage,
        "next_action": state.next_action,
        "objective": state.objective,
        "research_question": state.research_question,
        "cancer_type": state.cancer_type,
        "target_gene": state.target_gene,
        "planner_notes_count": state.planner_notes.len(),
        "retrieval_cycles": state.retrieval_history.len(),
        "validation_cycles": state.validation_history.len(),
        "last_retrieval": {
            "at": last_retrieval.map(|r| r.at),
            "papers_inserted": last_retrieval.and_then(|r| r.papers_inserted),
            "papers_duplicate": last_retrieval.and_then(|r| r.papers_duplicate),
            "chunks_inserted": last_retrieval.and_then(|r| r.chunks_inserted),
            "novelty_ratio": last_retrieval.and_then(|r| r.novelty_ratio),
            "duplicate_pressure": last_retrieval.and_then(|r| r.duplicate_pressure)
        },
        "last_validation": {
            "at": last_validation.map(|v| v.at),
            "top_target": last_validation.and_then(|v| v.top_target.clone()),
            "top_score": last_validation.and_then(|v| v.top_score),
            "recommendation": last_validation.map(|v| v.recommendation.clone())
        },
        "created_at": state.created_at,
        "updated_at": state.updated_at
    })
}
