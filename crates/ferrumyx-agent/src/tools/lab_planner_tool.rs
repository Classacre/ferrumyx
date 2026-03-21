use async_trait::async_trait;
use ferrumyx_runtime::context::JobContext;
use ferrumyx_runtime::tools::{Tool, ToolError, ToolOutput};
use serde_json::json;

use super::lab_state;

/// Planner role for the multi-agent autonomous research loop.
pub struct LabPlannerTool;

impl LabPlannerTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for LabPlannerTool {
    fn name(&self) -> &str {
        "lab_planner"
    }

    fn description(&self) -> &str {
        "Planner agent role: creates a run_id, writes hypotheses, and defines the next retrieval/validation steps."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "objective": { "type": "string", "description": "High-level discovery goal." },
                "research_question": { "type": "string", "description": "Optional explicit research question." },
                "cancer_type": { "type": "string", "description": "Optional cancer context." },
                "target_gene": { "type": "string", "description": "Optional initial gene/entity of interest." },
                "max_cycles_hint": { "type": "integer", "description": "Optional planning hint for iteration budget." }
            },
            "required": ["objective"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let started = std::time::Instant::now();
        let objective = require_nonempty_str(&params, "objective")?.to_string();
        let research_question = params
            .get("research_question")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| objective.clone());
        let cancer_type = params
            .get("cancer_type")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string);
        let target_gene = params
            .get("target_gene")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_uppercase());
        let max_cycles_hint = params
            .get("max_cycles_hint")
            .and_then(|v| v.as_u64())
            .map(|v| v.clamp(1, 30) as usize)
            .unwrap_or(10);

        let hypotheses =
            build_hypotheses(&objective, cancer_type.as_deref(), target_gene.as_deref());
        let planner_notes = vec![
            format!("Objective: {objective}"),
            format!("Question: {research_question}"),
            format!("Cycle strategy: adaptive with novelty pressure, max hint={max_cycles_hint}"),
            "Role handoff: planner -> retriever -> validator".to_string(),
        ];

        let state = lab_state::create_run(
            objective.clone(),
            research_question.clone(),
            cancer_type.clone(),
            target_gene.clone(),
            planner_notes.clone(),
        );

        Ok(ToolOutput::success(
            json!({
                "status": "ok",
                "role": "planner",
                "run_id": state.run_id,
                "objective": objective,
                "research_question": research_question,
                "cancer_type": cancer_type,
                "target_gene": target_gene,
                "max_cycles_hint": max_cycles_hint,
                "hypotheses": hypotheses,
                "planner_notes": planner_notes,
                "next": {
                    "tool": "lab_retriever",
                    "required": ["run_id"],
                    "params_template": {
                        "run_id": state.run_id,
                        "max_results": 150
                    }
                },
                "workflow_links": [
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

fn build_hypotheses(
    objective: &str,
    cancer_type: Option<&str>,
    target_gene: Option<&str>,
) -> Vec<String> {
    let cancer = cancer_type.unwrap_or("the target cancer context");
    let gene = target_gene.unwrap_or("the initial seed target");
    vec![
        format!("Novel targets in {cancer} may emerge from non-generic mechanistic predicates rather than mention-level edges."),
        format!("Provider-backed evidence can re-rank candidates relative to {gene} when novelty pressure penalizes saturated hubs."),
        format!("Expanded literature retrieval around {gene} should increase entity diversity and reduce duplicate-heavy ingestion."),
        format!("Objective alignment check: {objective}"),
    ]
}
