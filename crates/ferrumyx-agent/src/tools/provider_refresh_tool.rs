use async_trait::async_trait;
use ironclaw::context::JobContext;
use ironclaw::tools::{Tool, ToolError, ToolOutput};
use serde_json::json;
use std::sync::Arc;

use ferrumyx_db::Database;
use ferrumyx_ranker::{ProviderRefreshRequest, TargetQueryEngine};

/// Tool to proactively refresh provider-backed Phase 4 signal tables.
pub struct RefreshProviderSignalsTool {
    db: Arc<Database>,
}

impl RefreshProviderSignalsTool {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl Tool for RefreshProviderSignalsTool {
    fn name(&self) -> &str {
        "refresh_provider_signals"
    }

    fn description(&self) -> &str {
        "Refreshes cBioPortal/TCGA/GTEx/ChEMBL/Reactome provider signal cache tables in staged batches with retries."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "genes": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Gene symbols to refresh (e.g. [\"KRAS\",\"EGFR\"])."
                },
                "gene": {
                    "type": "string",
                    "description": "Single-gene convenience alias if genes[] is omitted."
                },
                "cancer_code": {
                    "type": "string",
                    "description": "Optional cancer code for TCGA refresh (e.g. PAAD)."
                },
                "max_genes": {
                    "type": "integer",
                    "description": "Maximum unique genes to process (default: 24)."
                },
                "batch_size": {
                    "type": "integer",
                    "description": "Batch size for staged refresh (default: 6)."
                },
                "retries": {
                    "type": "integer",
                    "description": "Retry count per provider fetch (default: 1, max: 3)."
                }
            }
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let mut genes: Vec<String> = params
            .get("genes")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        if genes.is_empty() {
            if let Some(g) = params.get("gene").and_then(|v| v.as_str()) {
                genes.push(g.to_string());
            }
        }

        let request = ProviderRefreshRequest {
            genes,
            cancer_code: params
                .get("cancer_code")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            max_genes: params
                .get("max_genes")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(24),
            batch_size: params
                .get("batch_size")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(6),
            retries: params
                .get("retries")
                .and_then(|v| v.as_u64())
                .map(|v| v as u8)
                .unwrap_or(1),
        };

        let started = std::time::Instant::now();
        let engine = TargetQueryEngine::new(self.db.clone());
        let report = engine
            .refresh_provider_signals(request)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("provider refresh failed: {e}")))?;

        Ok(ToolOutput::success(
            json!({
                "status": "ok",
                "report": report
            }),
            started.elapsed(),
        ))
    }
}
