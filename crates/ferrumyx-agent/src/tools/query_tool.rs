use async_trait::async_trait;
use ironclaw::context::JobContext;
use ironclaw::tools::{require_str, Tool, ToolError, ToolOutput};
use serde_json::json;
use std::sync::Arc;

use ferrumyx_common::query::QueryRequest;
use ferrumyx_db::Database;
use ferrumyx_ranker::TargetQueryEngine;

/// Tool to run target prioritization queries from REPL/Gateway.
pub struct TargetQueryTool {
    db: Arc<Database>,
}

impl TargetQueryTool {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl Tool for TargetQueryTool {
    fn name(&self) -> &str {
        "query_targets"
    }

    fn description(&self) -> &str {
        "Executes a Ferrumyx target query and returns ranked targets with score/tier evidence."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query_text": {
                    "type": "string",
                    "description": "Natural-language research question"
                },
                "cancer_code": {
                    "type": "string",
                    "description": "Cancer code (e.g. PAAD, LUAD)"
                },
                "gene_symbol": {
                    "type": "string",
                    "description": "Optional gene filter (e.g. KRAS)"
                },
                "mutation": {
                    "type": "string",
                    "description": "Optional mutation filter (e.g. G12D)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Max ranked rows (default: 20)"
                }
            },
            "required": ["query_text"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let req = QueryRequest {
            query_text: require_str(&params, "query_text")?.to_string(),
            cancer_code: params
                .get("cancer_code")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            gene_symbol: params
                .get("gene_symbol")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            mutation: params
                .get("mutation")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            max_results: params
                .get("max_results")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize)
                .unwrap_or(20),
        };

        let started = std::time::Instant::now();
        let engine = TargetQueryEngine::new(self.db.clone());
        let results = engine
            .execute_query(req.clone())
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("query execution failed: {e}")))?;

        let payload = json!({
            "query_text": req.query_text,
            "result_count": results.len(),
            "results": results,
        });

        Ok(ToolOutput::success(payload, started.elapsed()))
    }
}
