use async_trait::async_trait;
use ironclaw::context::JobContext;
use ironclaw::tools::{Tool, ToolError, ToolOutput};
use serde_json::json;
use std::sync::Arc;

use ferrumyx_db::Database;

use super::lab_state;
use super::query_tool::TargetQueryTool;
use super::scoring_tool::RecomputeTargetScoresTool;
use super::workflow_status_tool::WorkflowStatusTool;

/// Validator role for the multi-agent autonomous research loop.
pub struct LabValidatorTool {
    db: Arc<Database>,
}

impl LabValidatorTool {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl Tool for LabValidatorTool {
    fn name(&self) -> &str {
        "lab_validator"
    }

    fn description(&self) -> &str {
        "Validator agent role: evaluates retrieved evidence quality, checks ranking progress, and recommends the next autonomous step."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "run_id": { "type": "string", "description": "Planner-generated run id." },
                "max_results": { "type": "integer", "description": "How many targets to inspect in validation query." }
            },
            "required": ["run_id"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let started = std::time::Instant::now();
        let run_id = require_nonempty_str(&params, "run_id")?;
        let state = lab_state::get_run(run_id).ok_or_else(|| {
            ToolError::InvalidParameters(format!(
                "unknown run_id: {run_id}. Call lab_planner first."
            ))
        })?;
        let _ = lab_state::set_stage(
            run_id,
            "validating",
            Some("Checking whether retrieval materially improved target evidence.".to_string()),
        );

        let max_results = params
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|v| v.clamp(3, 50) as usize)
            .unwrap_or(10);

        // Keep rankings fresh before validation.
        let scoring_tool = RecomputeTargetScoresTool::new(self.db.clone());
        let scoring_result = scoring_tool.execute(json!({}), ctx).await.ok();

        let status_tool = WorkflowStatusTool::new(self.db.clone());
        let status_result = status_tool
            .execute(json!({ "top_n": max_results }), ctx)
            .await?;

        let query_tool = TargetQueryTool::new(self.db.clone());
        let query_result = query_tool
            .execute(
                json!({
                    "query_text": state.research_question,
                    "cancer_code": state.cancer_type,
                    "gene_symbol": state.target_gene,
                    "max_results": max_results
                }),
                ctx,
            )
            .await?;

        let query_payload = query_result.result;
        let ranked_results = query_payload
            .get("results")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let top = ranked_results.first().cloned().unwrap_or_else(|| json!({}));
        let top_target = pick_string_field(&top, &["gene", "gene_symbol", "target", "gene_target"]);
        let top_score = pick_f64_field(
            &top,
            &[
                "confidence_adjusted_score",
                "priority_score",
                "score",
                "composite_score",
            ],
        );
        let previous_score = state.validation_history.last().and_then(|v| v.top_score);
        let score_delta = match (top_score, previous_score) {
            (Some(curr), Some(prev)) => Some(curr - prev),
            _ => None,
        };
        let latest_retrieval = state.retrieval_history.last().cloned();
        let retrieval_novelty_ratio = latest_retrieval.as_ref().and_then(|r| r.novelty_ratio);
        let retrieval_duplicate_pressure =
            latest_retrieval.as_ref().and_then(|r| r.duplicate_pressure);
        let retrieval_inserted = latest_retrieval.as_ref().and_then(|r| r.papers_inserted);

        let recommendation = recommend_next_step(
            top_score,
            score_delta,
            ranked_results.len(),
            retrieval_novelty_ratio,
            retrieval_duplicate_pressure,
            retrieval_inserted,
        );
        let next_tool = if recommendation.contains("retrieve") {
            "lab_retriever"
        } else {
            "lab_planner"
        };

        let updated = lab_state::append_validation(
            run_id,
            top_target.clone(),
            top_score,
            recommendation.clone(),
            Some(next_tool.to_string()),
        )
        .ok_or_else(|| ToolError::ExecutionFailed("failed to update run-state".to_string()))?;

        Ok(ToolOutput::success(
            json!({
                "status": "ok",
                "role": "validator",
                "run_id": run_id,
                "ranked_result_count": ranked_results.len(),
                "top_target": top_target,
                "top_score": top_score,
                "previous_top_score": previous_score,
                "score_delta": score_delta,
                "recommendation": recommendation,
                "retrieval_quality": {
                    "papers_inserted": retrieval_inserted,
                    "novelty_ratio": retrieval_novelty_ratio,
                    "duplicate_pressure": retrieval_duplicate_pressure
                },
                "next": {
                    "tool": next_tool,
                    "required": ["run_id"],
                    "params_template": {
                        "run_id": run_id
                    }
                },
                "validation_sources": {
                    "workflow_status": status_result.result,
                    "query_targets": query_payload,
                    "scoring_refresh": scoring_result.map(|r| r.result)
                },
                "run_state": updated,
                "live_links": [
                    "/targets",
                    "/kg",
                    "/metrics"
                ]
            }),
            started.elapsed(),
        ))
    }
}

fn require_nonempty_str<'a>(
    params: &'a serde_json::Value,
    name: &str,
) -> Result<&'a str, ToolError> {
    params
        .get(name)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            ToolError::InvalidParameters(format!("missing required string parameter: {name}"))
        })
}

fn pick_string_field(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(v) = value.get(*key).and_then(|x| x.as_str()) {
            let trimmed = v.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn pick_f64_field(value: &serde_json::Value, keys: &[&str]) -> Option<f64> {
    for key in keys {
        if let Some(raw) = value.get(*key) {
            if let Some(f) = raw.as_f64() {
                return Some(f);
            }
            if let Some(s) = raw.as_str() {
                if let Ok(f) = s.trim().parse::<f64>() {
                    return Some(f);
                }
            }
        }
    }
    None
}

fn recommend_next_step(
    top_score: Option<f64>,
    score_delta: Option<f64>,
    ranked_count: usize,
    novelty_ratio: Option<f64>,
    duplicate_pressure: Option<f64>,
    papers_inserted: Option<u64>,
) -> String {
    if ranked_count == 0 {
        return "No ranked targets were produced; retrieve broader evidence (lab_retriever)."
            .to_string();
    }
    if let Some(dup) = duplicate_pressure {
        if dup >= 0.75 && papers_inserted.unwrap_or(0) <= 2 {
            return "Duplicate pressure is high with low new yield; re-plan search scope and seed hypotheses (lab_planner).".to_string();
        }
    }
    if let Some(novelty) = novelty_ratio {
        if novelty <= 0.08 && papers_inserted.unwrap_or(0) <= 2 {
            return "Novelty ratio is very low; broaden retrieval strategy (lab_retriever)."
                .to_string();
        }
    }
    if let Some(score) = top_score {
        if score < 0.45 {
            return "Top score remains low; retrieve additional literature and provider signals (lab_retriever).".to_string();
        }
    }
    if let Some(delta) = score_delta {
        if delta < 0.02 {
            return "Ranking improvement plateau detected; re-plan hypothesis scope (lab_planner) before next retrieval.".to_string();
        }
    }
    "Validation passed with measurable progress; continue retrieval-validation loop (lab_retriever).".to_string()
}

#[cfg(test)]
mod tests {
    use super::recommend_next_step;

    #[test]
    fn recommends_retrieval_on_no_results() {
        let rec = recommend_next_step(None, None, 0, None, None, None);
        assert!(rec.contains("lab_retriever"));
    }

    #[test]
    fn recommends_replan_on_plateau() {
        let rec = recommend_next_step(Some(0.71), Some(0.005), 8, None, None, None);
        assert!(rec.contains("lab_planner"));
    }

    #[test]
    fn recommends_replan_on_duplicate_pressure() {
        let rec = recommend_next_step(Some(0.74), Some(0.05), 10, Some(0.02), Some(0.88), Some(1));
        assert!(rec.contains("lab_planner"));
    }
}
