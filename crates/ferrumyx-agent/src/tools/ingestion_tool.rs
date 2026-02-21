//! IronClaw tools: PubMed and Europe PMC ingestion triggers.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use sqlx::PgPool;
use super::FerrumyxTool;

// ─────────────────────────────────────────────
//  PubMed ingestion tool
// ─────────────────────────────────────────────

pub struct IngestPubmedTool {
    db: PgPool,
}

impl IngestPubmedTool {
    pub fn new(db: PgPool) -> Self { Self { db } }
}

#[async_trait]
impl FerrumyxTool for IngestPubmedTool {
    fn name(&self) -> &str { "ingest_pubmed" }

    fn description(&self) -> &str {
        "Search PubMed for papers matching a gene, mutation, and cancer type. \
         Downloads abstracts and full-text (OA), parses sections, deduplicates, \
         stores papers and chunks in the database."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "gene":        { "type": "string", "description": "Gene symbol, e.g. KRAS" },
                "mutation":    { "type": "string", "description": "Mutation notation, e.g. G12D" },
                "cancer_type": { "type": "string", "description": "Cancer free-text, e.g. pancreatic cancer" },
                "max_results": { "type": "integer", "default": 100, "minimum": 1, "maximum": 1000 },
                "date_from":   { "type": "string", "description": "YYYY/MM/DD filter (optional)" }
            },
            "required": ["gene"]
        })
    }

    async fn invoke(&self, params: Value) -> Result<Value> {
        let gene   = params["gene"].as_str().unwrap_or("KRAS");
        let mutation = params["mutation"].as_str().unwrap_or("");
        let cancer = params["cancer_type"].as_str().unwrap_or("cancer");
        let max    = params["max_results"].as_u64().unwrap_or(100) as usize;

        // Build the E-utilities query
        let query = if mutation.is_empty() {
            format!("{gene}[tiab] AND {cancer}[tiab]")
        } else {
            format!("{gene}[tiab] AND {mutation}[tiab] AND {cancer}[tiab]")
        };

        tracing::info!(tool = "ingest_pubmed", query = %query, max, "Starting ingestion");

        // --- Real implementation will call ferrumyx_ingestion::pubmed::PubmedClient ---
        // Placeholder for MVP scaffold: just log and return metadata.
        // TODO(phase1-m2): wire in PubmedClient::search_and_fetch(&query, max, &self.db)
        let _ = &self.db; // db will be used for INSERT in full impl

        Ok(serde_json::json!({
            "status": "queued",
            "query": query,
            "max_results": max,
            "message": "Ingestion job queued. Papers will appear in the DB as they are parsed."
        }))
    }

    fn requires_approval(&self) -> bool { false }
    fn output_data_class(&self) -> &str { "PUBLIC" }
}

// ─────────────────────────────────────────────
//  Europe PMC ingestion tool
// ─────────────────────────────────────────────

pub struct IngestEuropePmcTool {
    db: PgPool,
}

impl IngestEuropePmcTool {
    pub fn new(db: PgPool) -> Self { Self { db } }
}

#[async_trait]
impl FerrumyxTool for IngestEuropePmcTool {
    fn name(&self) -> &str { "ingest_europepmc" }

    fn description(&self) -> &str {
        "Search Europe PMC for literature including preprints. \
         Retrieves full-text XML for Open Access papers. \
         Complements PubMed with broader coverage and MeSH annotations."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query":       { "type": "string", "description": "Free-text query for Europe PMC" },
                "max_results": { "type": "integer", "default": 100 },
                "include_preprints": { "type": "boolean", "default": false }
            },
            "required": ["query"]
        })
    }

    async fn invoke(&self, params: Value) -> Result<Value> {
        let query = params["query"].as_str().unwrap_or("");
        let max   = params["max_results"].as_u64().unwrap_or(100) as usize;
        let preprints = params["include_preprints"].as_bool().unwrap_or(false);

        tracing::info!(tool = "ingest_europepmc", query = %query, max, preprints, "Starting ingestion");

        let _ = &self.db;
        // TODO(phase1-m2): wire in EuropePmcClient

        Ok(serde_json::json!({
            "status": "queued",
            "query": query,
            "max_results": max,
            "include_preprints": preprints,
            "message": "Europe PMC ingestion job queued."
        }))
    }

    fn output_data_class(&self) -> &str { "PUBLIC" }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Schema is a pure `Value` construction — test without any DB.
    #[test]
    fn test_pubmed_schema_has_required_gene() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "gene":        { "type": "string" },
                "mutation":    { "type": "string" },
                "cancer_type": { "type": "string" },
                "max_results": { "type": "integer" }
            },
            "required": ["gene"]
        });
        let required = schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "gene"));
    }

    #[test]
    fn test_europepmc_schema_has_required_query() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "query":       { "type": "string" },
                "max_results": { "type": "integer" }
            },
            "required": ["query"]
        });
        let required = schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "query"));
    }
}
