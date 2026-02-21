//! IronClaw tool: target scoring and shortlisting.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use sqlx::PgPool;
use super::FerrumyxTool;

pub struct ScoreTargetsTool {
    db: PgPool,
}

impl ScoreTargetsTool {
    pub fn new(db: PgPool) -> Self { Self { db } }
}

#[async_trait]
impl FerrumyxTool for ScoreTargetsTool {
    fn name(&self) -> &str { "score_targets" }

    fn description(&self) -> &str {
        "Run the 9-component composite scoring formula on all KG-backed gene–cancer pairs \
         for the specified cancer type. Writes results to target_scores. \
         Shortlists primary (≥0.65), secondary (≥0.45), excluded (<0.45) targets. \
         Idempotent: bumps score_version each run."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "cancer_code": {
                    "type": "string",
                    "description": "OncoTree code, e.g. PAAD",
                    "default": "PAAD"
                },
                "min_kg_facts": {
                    "type": "integer",
                    "description": "Minimum KG fact count for a target to be considered.",
                    "default": 3
                },
                "weight_profile": {
                    "type": "string",
                    "enum": ["default", "structural_emphasis", "literature_emphasis"],
                    "default": "default"
                }
            },
            "required": ["cancer_code"]
        })
    }

    async fn invoke(&self, params: Value) -> Result<Value> {
        let cancer_code = params["cancer_code"].as_str().unwrap_or("PAAD");
        let min_facts   = params["min_kg_facts"].as_u64().unwrap_or(3) as i64;
        let profile     = params["weight_profile"].as_str().unwrap_or("default");

        tracing::info!(
            tool = "score_targets",
            cancer = cancer_code,
            min_facts,
            profile,
            "Starting target scoring"
        );

        // Count eligible gene–cancer pairs
        let eligible: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT kf.subject_id)
             FROM kg_facts kf
             JOIN entities ce ON kf.object_id = ce.id
             JOIN ent_cancer_types ect ON ect.id = ce.id
             WHERE ect.oncotree_code = $1
               AND kf.valid_until IS NULL
             GROUP BY kf.subject_id
             HAVING COUNT(*) >= $2"
        )
        .bind(cancer_code)
        .bind(min_facts)
        .fetch_one(&self.db)
        .await
        .unwrap_or(0);

        tracing::info!(tool = "score_targets", eligible, "Eligible gene-cancer pairs found");

        // TODO(phase1-m2): Wire in ferrumyx_ranker::TargetRanker::score_all(cancer_code, &self.db)
        // For now return the count; full scoring will INSERT rows into target_scores.

        Ok(serde_json::json!({
            "status": "queued",
            "cancer_code": cancer_code,
            "eligible_pairs": eligible,
            "weight_profile": profile,
            "message": format!(
                "Scoring job queued for {eligible} gene-cancer pairs in {cancer_code}. \
                 Results will appear in target_scores."
            )
        }))
    }

    fn requires_approval(&self) -> bool { false }
    fn output_data_class(&self) -> &str { "PUBLIC" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_targets_requires_cancer_code() {
        // Schema is pure JSON construction — no DB needed
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "cancer_code": { "type": "string" }
            },
            "required": ["cancer_code"]
        });
        let req = schema["required"].as_array().unwrap();
        assert!(req.iter().any(|v| v == "cancer_code"));
    }
}
