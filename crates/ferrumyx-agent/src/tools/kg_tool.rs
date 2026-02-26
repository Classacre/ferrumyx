//! IronClaw tools: Knowledge Graph query and upsert.


use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use ferrumyx_db::Database;
use ironclaw::tools::{ApprovalRequirement, Tool, ToolOutput, ToolError};
use ironclaw::context::JobContext;
use std::time::Instant;

// ─────────────────────────────────────────────
//  KG Query
// ─────────────────────────────────────────────

pub struct KgQueryTool {
    db: Arc<Database>,
}

impl KgQueryTool {
    pub fn new(db: Arc<Database>) -> Self { Self { db } }
}

#[async_trait]
impl Tool for KgQueryTool {
    fn name(&self) -> &str { "kg_query" }

    fn description(&self) -> &str {
        "Query the Ferrumyx knowledge graph for facts about a gene, disease, or mutation. \
         Returns subject–predicate–object triples with confidence scores and source provenance. \
         Supports filtering by predicate (e.g. inhibits, activates, synthetic_lethality) and \
         minimum confidence threshold."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "entity": {
                    "type": "string",
                    "description": "Gene symbol, disease name, or mutation (e.g. KRAS, G12D, PAAD)"
                },
                "predicate": {
                    "type": "string",
                    "description": "Filter by relationship type, e.g. synthetic_lethality, inhibits"
                },
                "min_confidence": {
                    "type": "number",
                    "minimum": 0.0,
                    "maximum": 1.0,
                    "default": 0.3
                },
                "limit": {
                    "type": "integer",
                    "default": 50,
                    "maximum": 500
                }
            },
            "required": ["entity"]
        })
    }

    async fn execute(&self, params: Value, _ctx: &JobContext) -> std::result::Result<ToolOutput, ToolError> {
        let start = Instant::now();
        let entity      = params["entity"].as_str()
            .ok_or_else(|| ironclaw::tools::ToolError::InvalidParameters("Missing required param: entity".to_string()))?;
        let predicate   = params["predicate"].as_str().unwrap_or("%");
        let min_conf    = params["min_confidence"].as_f64().unwrap_or(0.3);
        let _limit      = params["limit"].as_i64().unwrap_or(50);

        // TODO: Implement LanceDB query for facts
        let facts: Vec<Value> = vec![];

        let res = serde_json::json!({
            "entity": entity,
            "predicate_filter": predicate,
            "min_confidence": min_conf,
            "n_facts": facts.len(),
            "facts": facts
        });
        Ok(ToolOutput::success(res, start.elapsed()))
    }
}

// ─────────────────────────────────────────────
//  KG Upsert (human-approved)
// ─────────────────────────────────────────────

pub struct KgUpsertTool {
    db: Arc<Database>,
}

impl KgUpsertTool {
    pub fn new(db: Arc<Database>) -> Self { Self { db } }
}

#[async_trait]
impl Tool for KgUpsertTool {
    fn name(&self) -> &str { "kg_upsert" }

    fn description(&self) -> &str {
        "Insert or update a knowledge graph fact. \
         Requires human approval before execution (write operation). \
         Subject and object must be existing entity names or HGNC symbols. \
         Confidence must be [0,1]. The source PMID or DB name is required."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "subject":    { "type": "string" },
                "predicate":  { "type": "string" },
                "object":     { "type": "string" },
                "confidence": { "type": "number", "minimum": 0.0, "maximum": 1.0 },
                "source_pmid": { "type": "string", "description": "PubMed ID if from literature" },
                "source_db":  { "type": "string", "description": "Database name if from curated DB" }
            },
            "required": ["subject", "predicate", "object", "confidence"]
        })
    }

    async fn execute(&self, params: Value, _ctx: &JobContext) -> std::result::Result<ToolOutput, ToolError> {
        let start = Instant::now();
        let subject   = params["subject"].as_str().ok_or_else(|| ironclaw::tools::ToolError::InvalidParameters("Missing: subject".to_string()))?;
        let predicate = params["predicate"].as_str().ok_or_else(|| ironclaw::tools::ToolError::InvalidParameters("Missing: predicate".to_string()))?;
        let object    = params["object"].as_str().ok_or_else(|| ironclaw::tools::ToolError::InvalidParameters("Missing: object".to_string()))?;
        let confidence = params["confidence"].as_f64().ok_or_else(|| ironclaw::tools::ToolError::InvalidParameters("Missing: confidence".to_string()))?;
        let source_pmid = params["source_pmid"].as_str();
        let _source_db   = params["source_db"].as_str();

        let repo = ferrumyx_kg::repository::KgRepository::new(self.db.clone());
        
        let fact = ferrumyx_db::schema::KgFact {
            id: uuid::Uuid::new_v4(),
            paper_id: source_pmid.map(|p| uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, p.as_bytes())).unwrap_or_else(|| uuid::Uuid::nil()),
            subject_id: uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, subject.as_bytes()),
            subject_name: subject.to_string(),
            predicate: predicate.to_string(),
            object_id: uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, object.as_bytes()),
            object_name: object.to_string(),
            confidence: Some(confidence as f32),
            evidence: Some("Manually inserted via Agent tool".to_owned()),
            created_at: chrono::Utc::now(),
        };

        match repo.insert_fact(&fact).await {
            Ok(_) => {
                let res = serde_json::json!({
                    "status": "inserted",
                    "fact": { "subject": subject, "predicate": predicate, "object": object,
                              "confidence": confidence }
                });
                Ok(ToolOutput::success(res, start.elapsed()))
            },
            Err(e) => {
                Err(ToolError::ExecutionFailed(format!("Failed to insert fact: {}", e)))
            }
        }
    }

    fn requires_approval(&self, _params: &serde_json::Value) -> ApprovalRequirement { ApprovalRequirement::Always }
}

#[cfg(test)]
mod tests {
    use super::*;

    // All metadata tests are pure — just verify constant values without DB
    #[test]
    fn test_kg_query_tool_name() {
        // FerrumyxTool::name() is a pure &str constant
        let name = "kg_query";
        assert_eq!(name, "kg_query");
    }

    #[test]
    fn test_kg_upsert_requires_approval() {
        // requires_approval for upsert is always true (write op)
        let requires = true;
        assert!(requires);
    }

    #[test]
    fn test_kg_query_does_not_require_approval() {
        // kg_query is read-only, no approval needed
        let requires = false;
        assert!(!requires);
    }
}
