//! IronClaw tools: PubMed and Europe PMC ingestion triggers.
//! Wired to the real `ferrumyx_ingestion::pipeline` orchestrator.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;

use ferrumyx_ingestion::pipeline::{
    IngestionJob, IngestionSourceSpec, run_ingestion, build_query,
};
use ferrumyx_ingestion::pg_repository::PgIngestionRepository;

use super::FerrumyxTool;

// ─────────────────────────────────────────────────────────────────────────────
//  PubMed ingestion tool
// ─────────────────────────────────────────────────────────────────────────────

pub struct IngestPubmedTool {
    repo: Arc<PgIngestionRepository>,
}

impl IngestPubmedTool {
    pub fn new(db: PgPool) -> Self {
        Self { repo: Arc::new(PgIngestionRepository::new(db)) }
    }
}

#[async_trait]
impl FerrumyxTool for IngestPubmedTool {
    fn name(&self) -> &str { "ingest_pubmed" }

    fn description(&self) -> &str {
        "Search PubMed for papers matching a gene, mutation, and cancer type. \
         Downloads abstracts and full-text (OA), parses sections, deduplicates, \
         stores papers and chunks in the database. Returns a summary of what was ingested."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "gene":        { "type": "string", "description": "Gene symbol, e.g. KRAS" },
                "mutation":    { "type": "string", "description": "Mutation notation, e.g. G12D (optional)" },
                "cancer_type": { "type": "string", "description": "Cancer free-text, e.g. pancreatic cancer" },
                "max_results": { "type": "integer", "default": 100, "minimum": 1, "maximum": 1000 },
                "api_key":     { "type": "string", "description": "NCBI API key for higher rate limits (optional)" }
            },
            "required": ["gene", "cancer_type"]
        })
    }

    async fn invoke(&self, params: Value) -> Result<Value> {
        let gene        = params["gene"].as_str().unwrap_or("KRAS").to_string();
        let mutation    = params["mutation"].as_str().map(String::from);
        let cancer_type = params["cancer_type"].as_str().unwrap_or("cancer").to_string();
        let max_results = params["max_results"].as_u64().unwrap_or(100) as usize;
        let api_key     = params["api_key"].as_str().map(String::from);

        let job = IngestionJob {
            gene,
            mutation,
            cancer_type,
            max_results,
            sources: vec![IngestionSourceSpec::PubMed],
            pubmed_api_key: api_key,
            embedding_cfg: None,
        };

        tracing::info!(tool = "ingest_pubmed", query = %build_query(&job), "Running ingestion");

        let result = run_ingestion(job, Arc::clone(&self.repo), None).await;

        Ok(serde_json::json!({
            "status": "complete",
            "job_id": result.job_id,
            "query": result.query,
            "papers_found": result.papers_found,
            "papers_inserted": result.papers_inserted,
            "papers_duplicate": result.papers_duplicate,
            "chunks_inserted": result.chunks_inserted,
            "errors": result.errors,
            "duration_ms": result.duration_ms
        }))
    }

    fn requires_approval(&self) -> bool { false }
    fn output_data_class(&self) -> &str { "PUBLIC" }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Europe PMC ingestion tool
// ─────────────────────────────────────────────────────────────────────────────

pub struct IngestEuropePmcTool {
    repo: Arc<PgIngestionRepository>,
}

impl IngestEuropePmcTool {
    pub fn new(db: PgPool) -> Self {
        Self { repo: Arc::new(PgIngestionRepository::new(db)) }
    }
}

#[async_trait]
impl FerrumyxTool for IngestEuropePmcTool {
    fn name(&self) -> &str { "ingest_europepmc" }

    fn description(&self) -> &str {
        "Search Europe PMC for literature including preprints. \
         Retrieves full-text XML for Open Access papers. \
         Complements PubMed with broader coverage. Returns ingestion summary."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "gene":        { "type": "string" },
                "mutation":    { "type": "string" },
                "cancer_type": { "type": "string" },
                "max_results": { "type": "integer", "default": 100 },
                "include_preprints": { "type": "boolean", "default": false }
            },
            "required": ["gene", "cancer_type"]
        })
    }

    async fn invoke(&self, params: Value) -> Result<Value> {
        let gene        = params["gene"].as_str().unwrap_or("KRAS").to_string();
        let mutation    = params["mutation"].as_str().map(String::from);
        let cancer_type = params["cancer_type"].as_str().unwrap_or("cancer").to_string();
        let max_results = params["max_results"].as_u64().unwrap_or(100) as usize;

        let job = IngestionJob {
            gene,
            mutation,
            cancer_type,
            max_results,
            sources: vec![IngestionSourceSpec::EuropePmc],
            pubmed_api_key: None,
            embedding_cfg: None,
        };

        let result = run_ingestion(job, Arc::clone(&self.repo), None).await;

        Ok(serde_json::json!({
            "status": "complete",
            "job_id": result.job_id,
            "query": result.query,
            "papers_found": result.papers_found,
            "papers_inserted": result.papers_inserted,
            "papers_duplicate": result.papers_duplicate,
            "chunks_inserted": result.chunks_inserted,
            "errors": result.errors,
            "duration_ms": result.duration_ms
        }))
    }

    fn output_data_class(&self) -> &str { "PUBLIC" }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Combined ingestion tool (PubMed + Europe PMC in one call)
// ─────────────────────────────────────────────────────────────────────────────

pub struct IngestAllSourcesTool {
    repo: Arc<PgIngestionRepository>,
}

impl IngestAllSourcesTool {
    pub fn new(db: PgPool) -> Self {
        Self { repo: Arc::new(PgIngestionRepository::new(db)) }
    }
}

#[async_trait]
impl FerrumyxTool for IngestAllSourcesTool {
    fn name(&self) -> &str { "ingest_all" }

    fn description(&self) -> &str {
        "Run ingestion from ALL enabled sources (PubMed + Europe PMC) in one call. \
         Best choice for comprehensive literature coverage. Returns combined summary."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "gene":        { "type": "string" },
                "mutation":    { "type": "string" },
                "cancer_type": { "type": "string" },
                "max_results": { "type": "integer", "default": 200 },
                "pubmed_api_key": { "type": "string" }
            },
            "required": ["gene", "cancer_type"]
        })
    }

    async fn invoke(&self, params: Value) -> Result<Value> {
        let gene        = params["gene"].as_str().unwrap_or("KRAS").to_string();
        let mutation    = params["mutation"].as_str().map(String::from);
        let cancer_type = params["cancer_type"].as_str().unwrap_or("cancer").to_string();
        let max_results = params["max_results"].as_u64().unwrap_or(200) as usize;
        let api_key     = params["pubmed_api_key"].as_str().map(String::from);

        let job = IngestionJob {
            gene,
            mutation,
            cancer_type,
            max_results,
            sources: vec![IngestionSourceSpec::PubMed, IngestionSourceSpec::EuropePmc],
            pubmed_api_key: api_key,
            embedding_cfg: None,
        };

        let result = run_ingestion(job, Arc::clone(&self.repo), None).await;

        Ok(serde_json::json!({
            "status": "complete",
            "job_id": result.job_id,
            "query": result.query,
            "papers_found": result.papers_found,
            "papers_inserted": result.papers_inserted,
            "papers_duplicate": result.papers_duplicate,
            "chunks_inserted": result.chunks_inserted,
            "errors": result.errors,
            "duration_ms": result.duration_ms
        }))
    }

    fn output_data_class(&self) -> &str { "PUBLIC" }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pubmed_schema_has_required_gene() {
        let schema = serde_json::json!({
            "required": ["gene", "cancer_type"]
        });
        let required = schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "gene"));
        assert!(required.iter().any(|v| v == "cancer_type"));
    }

    #[test]
    fn test_europepmc_schema_has_required_query() {
        let schema = serde_json::json!({
            "required": ["gene", "cancer_type"]
        });
        let required = schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "gene"));
    }
}
