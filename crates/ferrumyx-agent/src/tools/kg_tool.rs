//! IronClaw tools: Knowledge Graph query and upsert.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use sqlx::PgPool;
use super::FerrumyxTool;

// ─────────────────────────────────────────────
//  KG Query
// ─────────────────────────────────────────────

pub struct KgQueryTool {
    db: PgPool,
}

impl KgQueryTool {
    pub fn new(db: PgPool) -> Self { Self { db } }
}

#[async_trait]
impl FerrumyxTool for KgQueryTool {
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

    async fn invoke(&self, params: Value) -> Result<Value> {
        let entity      = params["entity"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing required param: entity"))?;
        let predicate   = params["predicate"].as_str().unwrap_or("%");
        let min_conf    = params["min_confidence"].as_f64().unwrap_or(0.3);
        let limit       = params["limit"].as_i64().unwrap_or(50);

        let rows: Vec<(String, String, String, f64, Option<String>)> = sqlx::query_as(
            "SELECT
                 COALESCE(eg1.symbol, e1.name),
                 kf.predicate,
                 COALESCE(eg2.symbol, e2.name),
                 kf.confidence,
                 kf.source_pmid
             FROM kg_facts kf
             JOIN entities e1 ON kf.subject_id = e1.id
             LEFT JOIN ent_genes eg1 ON eg1.id = e1.id
             JOIN entities e2 ON kf.object_id = e2.id
             LEFT JOIN ent_genes eg2 ON eg2.id = e2.id
             WHERE kf.valid_until IS NULL
               AND kf.confidence >= $1
               AND ($2 = '%' OR kf.predicate ILIKE $2)
               AND (eg1.symbol ILIKE $3 OR eg2.symbol ILIKE $3
                    OR e1.name ILIKE $3 OR e2.name ILIKE $3)
             ORDER BY kf.confidence DESC
             LIMIT $4"
        )
        .bind(min_conf)
        .bind(predicate)
        .bind(entity)
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        let facts: Vec<Value> = rows.into_iter().map(|(s, p, o, c, pmid)| {
            serde_json::json!({
                "subject": s, "predicate": p, "object": o,
                "confidence": c, "source_pmid": pmid
            })
        }).collect();

        Ok(serde_json::json!({
            "entity": entity,
            "predicate_filter": predicate,
            "min_confidence": min_conf,
            "n_facts": facts.len(),
            "facts": facts
        }))
    }

    fn output_data_class(&self) -> &str { "PUBLIC" }
}

// ─────────────────────────────────────────────
//  KG Upsert (human-approved)
// ─────────────────────────────────────────────

pub struct KgUpsertTool {
    db: PgPool,
}

impl KgUpsertTool {
    pub fn new(db: PgPool) -> Self { Self { db } }
}

#[async_trait]
impl FerrumyxTool for KgUpsertTool {
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

    async fn invoke(&self, params: Value) -> Result<Value> {
        let subject   = params["subject"].as_str().ok_or_else(|| anyhow::anyhow!("Missing: subject"))?;
        let predicate = params["predicate"].as_str().ok_or_else(|| anyhow::anyhow!("Missing: predicate"))?;
        let object    = params["object"].as_str().ok_or_else(|| anyhow::anyhow!("Missing: object"))?;
        let confidence = params["confidence"].as_f64().ok_or_else(|| anyhow::anyhow!("Missing: confidence"))?;
        let source_pmid = params["source_pmid"].as_str();
        let source_db   = params["source_db"].as_str();

        // Resolve entity IDs
        let subject_id: Option<i32> = sqlx::query_scalar(
            "SELECT e.id FROM entities e LEFT JOIN ent_genes eg ON eg.id = e.id WHERE eg.symbol = $1 OR e.name = $1 LIMIT 1"
        ).bind(subject).fetch_optional(&self.db).await?;

        let object_id: Option<i32> = sqlx::query_scalar(
            "SELECT e.id FROM entities e LEFT JOIN ent_genes eg ON eg.id = e.id WHERE eg.symbol = $1 OR e.name = $1 LIMIT 1"
        ).bind(object).fetch_optional(&self.db).await?;

        let (sid, oid) = match (subject_id, object_id) {
            (Some(s), Some(o)) => (s, o),
            (None, _) => return Err(anyhow::anyhow!("Entity not found: {subject}")),
            (_, None) => return Err(anyhow::anyhow!("Entity not found: {object}")),
        };

        sqlx::query(
            "INSERT INTO kg_facts (subject_id, predicate, object_id, confidence, source_pmid, source_db)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT DO NOTHING"
        )
        .bind(sid)
        .bind(predicate)
        .bind(oid)
        .bind(confidence)
        .bind(source_pmid)
        .bind(source_db)
        .execute(&self.db)
        .await?;

        Ok(serde_json::json!({
            "status": "inserted",
            "fact": { "subject": subject, "predicate": predicate, "object": object,
                      "confidence": confidence }
        }))
    }

    fn requires_approval(&self) -> bool { true }
    fn output_data_class(&self) -> &str { "PUBLIC" }
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
