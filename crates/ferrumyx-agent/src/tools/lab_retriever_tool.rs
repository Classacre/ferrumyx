use async_trait::async_trait;
use ironclaw::context::JobContext;
use ironclaw::tools::{Tool, ToolError, ToolOutput};
use serde_json::json;
use std::sync::Arc;

use ferrumyx_db::Database;

use super::ingestion_tool::IngestionTool;
use super::lab_state;

/// Retriever role for the multi-agent autonomous research loop.
pub struct LabRetrieverTool {
    db: Arc<Database>,
}

impl LabRetrieverTool {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl Tool for LabRetrieverTool {
    fn name(&self) -> &str {
        "lab_retriever"
    }

    fn description(&self) -> &str {
        "Retriever agent role: ingests literature for a planned run_id and updates shared run-state with retrieval evidence."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "run_id": { "type": "string", "description": "Planner-generated run id." },
                "max_results": { "type": "integer", "description": "Optional retrieval volume target." },
                "gene": { "type": "string", "description": "Optional override gene." },
                "cancer_type": { "type": "string", "description": "Optional override cancer type." },
                "mutation": { "type": "string", "description": "Optional mutation filter." }
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

        let selected_gene = params
            .get("gene")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_uppercase())
            .or_else(|| state.target_gene.clone())
            .or_else(|| guess_gene_from_text(&state.objective))
            .unwrap_or_else(|| "TP53".to_string());
        let selected_cancer = params
            .get("cancer_type")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .or_else(|| state.cancer_type.clone())
            .unwrap_or_else(|| "pan-cancer".to_string());
        let selected_mutation = params
            .get("mutation")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string);
        let user_max_results = params
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|v| v.clamp(10, 1_000) as usize);
        let max_results = choose_dynamic_max_results(user_max_results, &state);

        let _ = lab_state::set_stage(
            run_id,
            "retrieving",
            Some("Running ingestion for planned hypothesis.".to_string()),
        );

        let ingestion_tool = IngestionTool::new(self.db.clone());
        let mut ingestion_params = json!({
            "gene": selected_gene,
            "cancer_type": selected_cancer,
            "max_results": max_results,
            "idle_timeout_secs": 1200,
            "max_runtime_secs": 21600
        });
        if let Some(mutation) = selected_mutation.clone() {
            ingestion_params["mutation"] = json!(mutation);
        }

        let ingestion_output = ingestion_tool.execute(ingestion_params, ctx).await?;
        let ingestion_summary = ingestion_output
            .result
            .as_str()
            .map(ToString::to_string)
            .unwrap_or_else(|| ingestion_output.result.to_string());
        let papers_found_raw = extract_number_after(&ingestion_summary, "Source fetch returned ");
        let papers_found_unique = extract_number_after(&ingestion_summary, " papers, ");
        let papers_inserted = extract_number_after(&ingestion_summary, "Inserted ");
        let chunks_inserted = extract_number_after(&ingestion_summary, "new papers and ");
        let papers_duplicate = extract_number_after(&ingestion_summary, "Skipped ");
        let novelty_ratio = ratio(papers_inserted, papers_found_unique.or(papers_found_raw));
        let duplicate_pressure = ratio(papers_duplicate, papers_found_unique.or(papers_found_raw));

        let updated = lab_state::append_retrieval(
            run_id,
            ingestion_summary.clone(),
            selected_gene.clone(),
            selected_cancer.clone(),
            papers_found_raw,
            papers_found_unique,
            papers_inserted,
            papers_duplicate,
            chunks_inserted,
        )
        .ok_or_else(|| ToolError::ExecutionFailed("failed to update run-state".to_string()))?;

        Ok(ToolOutput::success(
            json!({
                "status": "ok",
                "role": "retriever",
                "run_id": run_id,
                "selected_gene": selected_gene,
                "selected_cancer": selected_cancer,
                "selected_mutation": selected_mutation,
                "max_results": max_results,
                "adaptive_retrieval_profile": {
                    "user_override": user_max_results,
                    "history_cycles": state.retrieval_history.len(),
                    "novelty_ratio_prev": state.retrieval_history.last().and_then(|r| r.novelty_ratio),
                    "duplicate_pressure_prev": state.retrieval_history.last().and_then(|r| r.duplicate_pressure)
                },
                "ingestion_summary": ingestion_summary,
                "parsed_metrics": {
                    "papers_found_raw": papers_found_raw,
                    "papers_found_unique": papers_found_unique,
                    "papers_inserted": papers_inserted,
                    "papers_duplicate": papers_duplicate,
                    "chunks_inserted": chunks_inserted,
                    "novelty_ratio": novelty_ratio,
                    "duplicate_pressure": duplicate_pressure
                },
                "run_state": updated,
                "next": {
                    "tool": "lab_validator",
                    "required": ["run_id"],
                    "params_template": {
                        "run_id": run_id
                    }
                },
                "live_links": [
                    "/ingestion",
                    "/kg",
                    "/targets"
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

fn guess_gene_from_text(text: &str) -> Option<String> {
    let mut seen = std::collections::HashSet::new();
    for token in text
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .map(str::trim)
    {
        if token.len() < 2 || token.len() > 12 {
            continue;
        }
        if !token.chars().any(|c| c.is_ascii_uppercase()) {
            continue;
        }
        if !token.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            continue;
        }
        let candidate = token.to_uppercase();
        if seen.insert(candidate.clone()) {
            return Some(candidate);
        }
    }
    None
}

fn extract_number_after(text: &str, marker: &str) -> Option<u64> {
    let start = text.find(marker)?;
    let mut digits = String::new();
    for ch in text[start + marker.len()..].chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else if ch == ',' {
            continue;
        } else if !digits.is_empty() {
            break;
        } else if ch.is_ascii_whitespace() {
            continue;
        } else {
            return None;
        }
    }
    if digits.is_empty() {
        None
    } else {
        digits.parse::<u64>().ok()
    }
}

fn ratio(num: Option<u64>, denom: Option<u64>) -> Option<f64> {
    let n = num?;
    let d = denom?;
    if d == 0 {
        return None;
    }
    Some(n as f64 / d as f64)
}

fn choose_dynamic_max_results(
    user_override: Option<usize>,
    state: &lab_state::LabRunState,
) -> usize {
    if let Some(override_max) = user_override {
        return override_max.clamp(10, 1_000);
    }

    let mut dynamic = 120usize;
    if let Some(last) = state.retrieval_history.last() {
        let inserted = last.papers_inserted.unwrap_or(0);
        let novelty = last.novelty_ratio.unwrap_or(0.0);
        let duplicate_pressure = last.duplicate_pressure.unwrap_or(0.0);

        if duplicate_pressure >= 0.70 && novelty <= 0.22 {
            dynamic = ((dynamic as f64) * 1.8).ceil() as usize;
        } else if duplicate_pressure >= 0.55 && novelty <= 0.30 {
            dynamic = ((dynamic as f64) * 1.35).ceil() as usize;
        } else if novelty >= 0.45 && inserted >= 10 {
            dynamic = ((dynamic as f64) * 0.85).floor() as usize;
        }

        if inserted <= 1 {
            dynamic = ((dynamic as f64) * 1.4).ceil() as usize;
        }
    }

    if state.retrieval_history.len() >= 3 {
        dynamic = ((dynamic as f64) * 1.1).ceil() as usize;
    }

    dynamic.clamp(40, 700)
}

#[cfg(test)]
mod tests {
    use super::{choose_dynamic_max_results, extract_number_after};
    use crate::tools::lab_state::{LabRunState, RetrievalCheckpoint};
    use chrono::Utc;

    #[test]
    fn parses_inserted_number_from_summary() {
        let text = "Inserted 494 new papers and 7620 knowledge chunks into LanceDB.";
        assert_eq!(extract_number_after(text, "Inserted "), Some(494));
        assert_eq!(extract_number_after(text, "new papers and "), Some(7620));
    }

    #[test]
    fn increases_dynamic_retrieval_for_duplicate_pressure() {
        let now = Utc::now();
        let state = LabRunState {
            run_id: "lab-x".to_string(),
            objective: "obj".to_string(),
            research_question: "q".to_string(),
            cancer_type: None,
            target_gene: None,
            stage: "retrieved".to_string(),
            next_action: Some("lab_validator".to_string()),
            planner_notes: Vec::new(),
            retrieval_history: vec![RetrievalCheckpoint {
                at: now,
                summary: String::new(),
                selected_gene: "KRAS".to_string(),
                selected_cancer: "PAAD".to_string(),
                papers_found_raw: Some(100),
                papers_found_unique: Some(100),
                papers_inserted: Some(1),
                papers_duplicate: Some(88),
                chunks_inserted: Some(3),
                novelty_ratio: Some(0.01),
                duplicate_pressure: Some(0.88),
            }],
            validation_history: Vec::new(),
            created_at: now,
            updated_at: now,
        };
        assert!(choose_dynamic_max_results(None, &state) > 120);
    }
}
