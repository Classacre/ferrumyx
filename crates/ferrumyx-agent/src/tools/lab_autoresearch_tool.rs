use async_trait::async_trait;
use ironclaw::context::JobContext;
use ironclaw::tools::{Tool, ToolError, ToolOutput};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ferrumyx_db::Database;

use super::lab_planner_tool::LabPlannerTool;
use super::lab_retriever_tool::LabRetrieverTool;
use super::lab_state;
use super::lab_validator_tool::LabValidatorTool;

/// Coordinator role for the lab team loop (planner -> retriever -> validator).
/// This tool keeps dynamic adaptation in one place while preserving role boundaries.
pub struct LabAutoresearchTool {
    db: Arc<Database>,
}

impl LabAutoresearchTool {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StopReason {
    RuntimeBudgetReached,
    PlateauDetected,
    SafetyCapReached,
}

impl StopReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::RuntimeBudgetReached => "runtime_budget_reached",
            Self::PlateauDetected => "dynamic_plateau",
            Self::SafetyCapReached => "safety_cycle_cap_reached",
        }
    }
}

#[async_trait]
impl Tool for LabAutoresearchTool {
    fn name(&self) -> &str {
        "run_lab_autoresearch"
    }

    fn description(&self) -> &str {
        "Runs a dynamic lab-team loop (planner -> retriever -> validator) with adaptive retrieval and plateau-aware stopping."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "objective": { "type": "string", "description": "High-level discovery objective." },
                "research_question": { "type": "string", "description": "Optional explicit question." },
                "cancer_type": { "type": "string", "description": "Optional cancer context." },
                "target_gene": { "type": "string", "description": "Optional initial target gene/entity." },
                "mutation": { "type": "string", "description": "Optional mutation context." },
                "max_runtime_minutes": { "type": "integer", "description": "Wall-clock budget. Dynamic loop runs until meaningful plateau or this budget." },
                "max_cycles_safety": { "type": "integer", "description": "Safety cap only; primary stopping remains dynamic." },
                "retrieval_max_results_start": { "type": "integer", "description": "Initial retrieval size before adaptive tuning." },
                "validation_top_n": { "type": "integer", "description": "How many ranked targets validator should inspect." },
                "plateau_patience": { "type": "integer", "description": "Consecutive stagnating cycles required before dynamic stop." },
                "dynamic_novelty_mode": { "type": "boolean", "description": "Enable duplicate-pressure adaptation and novelty-aware retrieval tuning." }
            },
            "required": ["objective"]
        })
    }

    fn execution_timeout(&self) -> Duration {
        Duration::from_secs(4 * 60 * 60)
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let started = Instant::now();
        let objective = require_nonempty_str(&params, "objective")?.to_string();
        let research_question = optional_str(&params, "research_question");
        let cancer_type = optional_str(&params, "cancer_type");
        let target_gene = optional_str(&params, "target_gene").map(|g| g.to_uppercase());
        let mutation = optional_str(&params, "mutation");

        let max_runtime_minutes = params
            .get("max_runtime_minutes")
            .and_then(|v| v.as_u64())
            .unwrap_or(60)
            .clamp(5, 720);
        let max_cycles_safety = params
            .get("max_cycles_safety")
            .and_then(|v| v.as_u64())
            .unwrap_or(18)
            .clamp(1, 64) as usize;
        let validation_top_n = params
            .get("validation_top_n")
            .and_then(|v| v.as_u64())
            .unwrap_or(12)
            .clamp(3, 40) as usize;
        let plateau_patience = params
            .get("plateau_patience")
            .and_then(|v| v.as_u64())
            .unwrap_or(2)
            .clamp(1, 8) as usize;
        let dynamic_novelty_mode = params
            .get("dynamic_novelty_mode")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let mut retrieval_max_results = params
            .get("retrieval_max_results_start")
            .and_then(|v| v.as_u64())
            .unwrap_or(120)
            .clamp(20, 900) as usize;

        let planner = LabPlannerTool::new();
        let retriever = LabRetrieverTool::new(self.db.clone());
        let validator = LabValidatorTool::new(self.db.clone());

        let planner_output = planner
            .execute(
                json!({
                    "objective": objective,
                    "research_question": research_question,
                    "cancer_type": cancer_type,
                    "target_gene": target_gene,
                    "max_cycles_hint": max_cycles_safety
                }),
                ctx,
            )
            .await?;
        let planner_payload = planner_output.result;
        let run_id = planner_payload
            .get("run_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("planner did not return run_id".to_string()))?
            .to_string();

        let runtime_budget = Duration::from_secs(max_runtime_minutes * 60);
        let loop_started = Instant::now();
        let mut cycles = Vec::new();
        let mut max_score_gain = 0.0_f64;
        let mut max_evidence_gain = 0.0_f64;
        let mut stagnation_streak = 0usize;
        let mut last_top_score = None::<f64>;
        let mut total_papers_inserted = 0u64;
        let mut total_chunks_inserted = 0u64;
        let mut total_duplicates = 0u64;
        let mut stop_reason = StopReason::SafetyCapReached;

        for cycle in 1..=max_cycles_safety {
            if loop_started.elapsed() >= runtime_budget {
                stop_reason = StopReason::RuntimeBudgetReached;
                break;
            }

            let mut retriever_params = json!({
                "run_id": run_id,
                "max_results": retrieval_max_results
            });
            if let Some(ref ct) = cancer_type {
                retriever_params["cancer_type"] = json!(ct);
            }
            if let Some(ref gene) = target_gene {
                retriever_params["gene"] = json!(gene);
            }
            if let Some(ref mutn) = mutation {
                retriever_params["mutation"] = json!(mutn);
            }

            let retriever_output = retriever.execute(retriever_params, ctx).await?;
            let retriever_payload = retriever_output.result;
            let parsed_metrics = retriever_payload
                .get("parsed_metrics")
                .cloned()
                .unwrap_or_else(|| json!({}));

            let papers_inserted = value_u64(parsed_metrics.get("papers_inserted")).unwrap_or(0);
            let chunks_inserted = value_u64(parsed_metrics.get("chunks_inserted")).unwrap_or(0);
            let papers_duplicate = value_u64(parsed_metrics.get("papers_duplicate")).unwrap_or(0);
            let novelty_ratio = value_f64(parsed_metrics.get("novelty_ratio"));
            let duplicate_pressure = value_f64(parsed_metrics.get("duplicate_pressure"));

            total_papers_inserted += papers_inserted;
            total_chunks_inserted += chunks_inserted;
            total_duplicates += papers_duplicate;

            let validator_output = validator
                .execute(
                    json!({
                        "run_id": run_id,
                        "max_results": validation_top_n
                    }),
                    ctx,
                )
                .await?;
            let validator_payload = validator_output.result;
            let top_score = value_f64(validator_payload.get("top_score"));
            let score_delta = value_f64(validator_payload.get("score_delta")).unwrap_or(0.0);
            let recommendation = validator_payload
                .get("recommendation")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let top_target = validator_payload
                .get("top_target")
                .and_then(|v| v.as_str())
                .map(ToString::to_string);

            let score_gain = score_delta.max(0.0);
            let evidence_gain = papers_inserted as f64 + (chunks_inserted as f64 * 0.20);
            if score_gain > max_score_gain {
                max_score_gain = score_gain;
            }
            if evidence_gain > max_evidence_gain {
                max_evidence_gain = evidence_gain;
            }
            let score_floor = if max_score_gain > 0.0 {
                max_score_gain * 0.15
            } else {
                0.0
            };
            let evidence_floor = if max_evidence_gain > 0.0 {
                max_evidence_gain * 0.20
            } else {
                0.0
            };
            let stagnating =
                cycle > 1 && score_gain <= score_floor && evidence_gain <= evidence_floor;
            if cycle > 1 {
                if stagnating {
                    stagnation_streak += 1;
                } else {
                    stagnation_streak = 0;
                }
            }

            let previous_max_results = retrieval_max_results;
            retrieval_max_results = tune_next_max_results(
                retrieval_max_results,
                papers_inserted,
                papers_duplicate,
                novelty_ratio,
                duplicate_pressure,
                dynamic_novelty_mode,
            );

            cycles.push(json!({
                "cycle": cycle,
                "retrieval": {
                    "max_results_requested": previous_max_results,
                    "papers_inserted": papers_inserted,
                    "papers_duplicate": papers_duplicate,
                    "chunks_inserted": chunks_inserted,
                    "novelty_ratio": novelty_ratio,
                    "duplicate_pressure": duplicate_pressure
                },
                "validation": {
                    "top_target": top_target,
                    "top_score": top_score,
                    "score_delta": score_delta,
                    "recommendation": recommendation
                },
                "adaptive": {
                    "stagnating": stagnating,
                    "stagnation_streak": stagnation_streak,
                    "plateau_patience": plateau_patience,
                    "next_max_results": retrieval_max_results
                },
                "live_links": [
                    "/chat",
                    "/ingestion",
                    "/kg",
                    "/targets",
                    "/metrics"
                ]
            }));

            if stagnation_streak >= plateau_patience {
                stop_reason = StopReason::PlateauDetected;
                break;
            }
            last_top_score = top_score.or(last_top_score);
        }

        let _ = lab_state::set_stage(&run_id, "completed", None);

        Ok(ToolOutput::success(
            json!({
                "status": "ok",
                "role": "coordinator",
                "run_id": run_id,
                "team_roles": ["planner", "retriever", "validator"],
                "objective": objective,
                "research_question": research_question,
                "runtime_budget_minutes": max_runtime_minutes,
                "max_cycles_safety": max_cycles_safety,
                "termination_reason": stop_reason.as_str(),
                "summary": {
                    "cycles_executed": cycles.len(),
                    "papers_inserted_total": total_papers_inserted,
                    "chunks_inserted_total": total_chunks_inserted,
                    "duplicates_total": total_duplicates,
                    "latest_top_score": last_top_score,
                    "peak_score_gain": max_score_gain,
                    "peak_evidence_gain": max_evidence_gain
                },
                "cycles": cycles,
                "next": {
                    "tool": "lab_run_status",
                    "required": ["run_id"],
                    "params_template": {
                        "run_id": run_id
                    }
                },
                "live_links": [
                    "/chat",
                    "/ingestion",
                    "/kg",
                    "/targets",
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

fn optional_str(params: &serde_json::Value, name: &str) -> Option<String> {
    params
        .get(name)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

fn value_u64(value: Option<&Value>) -> Option<u64> {
    let raw = value?;
    if let Some(v) = raw.as_u64() {
        return Some(v);
    }
    raw.as_str()
        .map(str::trim)
        .and_then(|s| s.parse::<u64>().ok())
}

fn value_f64(value: Option<&Value>) -> Option<f64> {
    let raw = value?;
    if let Some(v) = raw.as_f64() {
        return Some(v);
    }
    raw.as_str()
        .map(str::trim)
        .and_then(|s| s.parse::<f64>().ok())
}

fn tune_next_max_results(
    current: usize,
    papers_inserted: u64,
    papers_duplicate: u64,
    novelty_ratio: Option<f64>,
    duplicate_pressure: Option<f64>,
    dynamic_novelty_mode: bool,
) -> usize {
    let mut next = current;
    if papers_inserted <= 1 {
        next = ((next as f64) * 1.35).ceil() as usize;
    }
    if dynamic_novelty_mode {
        let denom = (papers_inserted + papers_duplicate).max(1) as f64;
        let dup = duplicate_pressure.unwrap_or(papers_duplicate as f64 / denom);
        let nov = novelty_ratio.unwrap_or(papers_inserted as f64 / denom);
        if dup >= 0.70 && nov <= 0.22 {
            next = ((next as f64) * 1.75).ceil() as usize;
        } else if nov >= 0.50 && papers_inserted >= 12 {
            next = ((next as f64) * 0.85).floor() as usize;
        }
    }
    next.clamp(40, 900)
}

#[cfg(test)]
mod tests {
    use super::tune_next_max_results;

    #[test]
    fn raises_retrieval_budget_when_duplicates_dominate() {
        let next = tune_next_max_results(120, 1, 88, Some(0.01), Some(0.88), true);
        assert!(next > 180);
    }

    #[test]
    fn lowers_retrieval_budget_when_novelty_is_high() {
        let next = tune_next_max_results(200, 18, 5, Some(0.78), Some(0.12), true);
        assert!(next < 200);
    }
}
