use async_trait::async_trait;
use ironclaw::context::JobContext;
use ironclaw::tools::{Tool, ToolError, ToolOutput};
use serde_json::json;
use std::sync::Arc;

use ferrumyx_common::query::QueryRequest;
use ferrumyx_db::Database;
use ferrumyx_ingestion::pipeline::{run_ingestion, IngestionJob, IngestionSourceSpec};
use ferrumyx_ingestion::repository::IngestionRepository;
use ferrumyx_ranker::{ProviderRefreshRequest, TargetQueryEngine};

/// Tool to run a bounded autonomous loop over ingestion -> scoring -> ranking.
pub struct AutonomousCycleTool {
    db: Arc<Database>,
}

impl AutonomousCycleTool {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl Tool for AutonomousCycleTool {
    fn name(&self) -> &str {
        "run_autonomous_cycle"
    }

    fn description(&self) -> &str {
        "Runs iterative autonomous discovery cycles and stops when ranking score gain plateaus."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "gene": { "type": "string", "description": "Gene symbol (for example KRAS)" },
                "cancer_type": { "type": "string", "description": "Cancer type text (for example pancreatic cancer)" },
                "query_text": { "type": "string", "description": "Research question used for ranking output" },
                "cancer_code": { "type": "string", "description": "OncoTree-like cancer code (for example PAAD)" },
                "mutation": { "type": "string", "description": "Optional mutation (for example G12D)" },
                "max_results": { "type": "integer", "description": "Per-cycle ingestion paper cap (default: 40)" },
                "max_cycles": { "type": "integer", "description": "Maximum autonomous loops (default: 3, max: 6)" },
                "improvement_threshold": {
                    "type": "number",
                    "description": "Minimum top-score increase required to continue (default: 0.02)"
                }
            },
            "required": ["gene", "cancer_type"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let gene = require_str(&params, "gene")?.to_string();
        let cancer_type = require_str(&params, "cancer_type")?.to_string();
        let query_text = params
            .get("query_text")
            .and_then(|v| v.as_str())
            .unwrap_or("Prioritize actionable cancer targets using current evidence.")
            .to_string();
        let cancer_code = params
            .get("cancer_code")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let mutation = params
            .get("mutation")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let max_results = params
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(40)
            .clamp(10, 200);
        let max_cycles = params
            .get("max_cycles")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(3)
            .clamp(1, 6);
        let improvement_threshold = params
            .get("improvement_threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.02)
            .clamp(0.0, 0.5);

        let started = std::time::Instant::now();
        let repo = Arc::new(IngestionRepository::new(self.db.clone()));
        let ranker = TargetQueryEngine::new(self.db.clone());

        let mut cycles = Vec::new();
        let mut previous_top_score = 0.0_f64;

        for cycle in 1..=max_cycles {
            let ingest = run_ingestion(
                IngestionJob {
                    gene: gene.clone(),
                    mutation: mutation.clone(),
                    cancer_type: cancer_type.clone(),
                    max_results,
                    sources: vec![
                        IngestionSourceSpec::PubMed,
                        IngestionSourceSpec::EuropePmc,
                        IngestionSourceSpec::BioRxiv,
                        IngestionSourceSpec::MedRxiv,
                        IngestionSourceSpec::ClinicalTrials,
                    ],
                    pubmed_api_key: None,
                    embedding_cfg: None,
                    enable_scihub_fallback: false,
                },
                repo.clone(),
                None,
            )
            .await;

            let recomputed = ferrumyx_kg::compute_target_scores(self.db.clone())
                .await
                .map_err(|e| ToolError::ExecutionFailed(format!("cycle {cycle} scoring failed: {e}")))?;

            let refresh = ranker
                .refresh_provider_signals(ProviderRefreshRequest {
                    genes: vec![gene.clone()],
                    cancer_code: cancer_code.clone(),
                    max_genes: 8,
                    batch_size: 4,
                    retries: 1,
                })
                .await
                .map_err(|e| ToolError::ExecutionFailed(format!("cycle {cycle} provider refresh failed: {e}")))?;

            let query = QueryRequest {
                query_text: query_text.clone(),
                cancer_code: cancer_code.clone(),
                gene_symbol: Some(gene.clone()),
                mutation: mutation.clone(),
                max_results: 10,
            };
            let ranked = ranker
                .execute_query(query)
                .await
                .map_err(|e| ToolError::ExecutionFailed(format!("cycle {cycle} ranking failed: {e}")))?;
            let top_score = ranked.first().map(|r| r.composite_score).unwrap_or(0.0);
            let improvement = top_score - previous_top_score;

            cycles.push(json!({
                "cycle": cycle,
                "ingestion": {
                    "papers_found": ingest.papers_found,
                    "papers_inserted": ingest.papers_inserted,
                    "papers_duplicate": ingest.papers_duplicate,
                    "chunks_inserted": ingest.chunks_inserted,
                    "duration_ms": ingest.duration_ms
                },
                "scoring": {
                    "target_scores_upserted": recomputed
                },
                "provider_refresh": refresh,
                "ranking": {
                    "top_score": top_score,
                    "top_gene": ranked.first().map(|r| r.gene_symbol.clone()),
                    "result_count": ranked.len()
                },
                "improvement": improvement
            }));

            if cycle > 1 && improvement < improvement_threshold {
                break;
            }
            previous_top_score = top_score;
        }

        Ok(ToolOutput::success(
            json!({
                "status": "ok",
                "gene": gene,
                "cancer_type": cancer_type,
                "cycles": cycles
            }),
            started.elapsed(),
        ))
    }
}

fn require_str<'a>(params: &'a serde_json::Value, name: &str) -> Result<&'a str, ToolError> {
    params
        .get(name)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| ToolError::InvalidParameters(format!("missing required string parameter: {name}")))
}
