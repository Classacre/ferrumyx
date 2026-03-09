use async_trait::async_trait;
use ironclaw::context::JobContext;
use ironclaw::tools::{Tool, ToolError, ToolOutput};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

use ferrumyx_db::Database;
use ferrumyx_ingestion::pipeline::{run_ingestion, IngestionJob, IngestionSourceSpec};
use ferrumyx_ingestion::repository::IngestionRepository;

/// Tool to run the Ferrumyx end-to-end knowledge ingestion pipeline natively.
pub struct IngestionTool {
    db: Arc<Database>,
}

impl IngestionTool {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

fn require_str<'a>(params: &'a serde_json::Value, name: &str) -> Result<&'a str, ToolError> {
    params
        .get(name)
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::InvalidParameters(format!("missing '{}' parameter", name)))
}

#[async_trait]
impl Tool for IngestionTool {
    fn name(&self) -> &str {
        "ingest_literature"
    }

    fn description(&self) -> &str {
        "Ingests scientific literature for a given gene, mutation, and cancer type. Extracts text chunks and builds the knowledge graph."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "gene": {
                    "type": "string",
                    "description": "The gene symbol to search for (e.g., KRAS)"
                },
                "cancer_type": {
                    "type": "string",
                    "description": "The type of cancer (e.g., pancreatic cancer)"
                },
                "mutation": {
                    "type": "string",
                    "description": "Optional specific mutation (e.g., G12D)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of papers to fetch (default: 50)"
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

        let mutation = params
            .get("mutation")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let max_results = params
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(50);

        let job = IngestionJob {
            gene,
            mutation,
            cancer_type,
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
        };

        let repo = Arc::new(IngestionRepository::new(self.db.clone()));

        // Execute the ingestion pipeline without relying on external channel communication
        let result = run_ingestion(job, repo, None).await;

        let output_text = format!(
            "Ingestion completed in {}ms. Found {} papers across sources. Inserted {} new papers and {} knowledge chunks into LanceDB. Skipped {} duplicates.",
            result.duration_ms, result.papers_found, result.papers_inserted, result.chunks_inserted, result.papers_duplicate
        );

        Ok(ToolOutput::text(
            output_text,
            Duration::from_millis(result.duration_ms),
        ))
    }
}
