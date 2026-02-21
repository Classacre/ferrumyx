//! IronClaw tool: NER extraction via the SciSpacy Docker service.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use super::FerrumyxTool;

/// A single extracted entity from NER.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NerEntity {
    pub text: String,
    pub label: String,  // e.g. GENE, DISEASE, MUTATION, CHEMICAL, SPECIES
    pub start: usize,
    pub end: usize,
    pub normalized_id: Option<String>, // e.g. HGNC:6407, MESH:D010190
}

/// Response from the SciSpacy NER FastAPI service.
#[derive(Debug, Deserialize)]
struct NerResponse {
    entities: Vec<NerEntity>,
    model: String,
    elapsed_ms: f64,
}

pub struct NerExtractTool {
    /// URL of the SciSpacy NER FastAPI service. Default: http://localhost:8001
    service_url: String,
}

impl NerExtractTool {
    pub fn new() -> Self {
        Self {
            service_url: std::env::var("FERRUMYX_NER_URL")
                .unwrap_or_else(|_| "http://localhost:8001".to_string()),
        }
    }

    pub fn with_url(url: impl Into<String>) -> Self {
        Self { service_url: url.into() }
    }
}

#[async_trait]
impl FerrumyxTool for NerExtractTool {
    fn name(&self) -> &str { "ner_extract" }

    fn description(&self) -> &str {
        "Extract biomedical named entities from text using SciSpacy. \
         Identifies genes, mutations, diseases, chemicals, and species. \
         Uses en_core_sci_lg for general biomedical NER and en_ner_bc5cdr_md \
         for chemical/disease NER. Returns normalised entity IDs where available."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "The text to extract entities from (up to 10,000 chars)."
                },
                "chunk_id": {
                    "type": "string",
                    "description": "Optional chunk UUID for provenance tracking."
                },
                "model": {
                    "type": "string",
                    "enum": ["sci_lg", "bc5cdr", "craft"],
                    "default": "sci_lg",
                    "description": "Which SciSpacy model to use."
                }
            },
            "required": ["text"]
        })
    }

    async fn invoke(&self, params: Value) -> Result<Value> {
        let text = params["text"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing required param: text"))?;
        let model = params["model"].as_str().unwrap_or("sci_lg");
        let chunk_id = params["chunk_id"].as_str();

        tracing::debug!(
            tool = "ner_extract",
            text_len = text.len(),
            model = model,
            chunk_id = ?chunk_id,
            "Calling NER service"
        );

        // Call the SciSpacy FastAPI service
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let resp = client
            .post(format!("{}/ner", self.service_url))
            .json(&serde_json::json!({
                "text": text,
                "model": model
            }))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(
                "NER service unreachable at {}: {e}\nIs the SciSpacy Docker container running?\n  cd docker && docker compose up -d scispacy",
                self.service_url
            ))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("NER service error {status}: {body}"));
        }

        let ner: NerResponse = resp.json().await
            .map_err(|e| anyhow::anyhow!("Failed to parse NER response: {e}"))?;

        tracing::info!(
            tool = "ner_extract",
            model = %ner.model,
            n_entities = ner.entities.len(),
            elapsed_ms = ner.elapsed_ms,
            "NER complete"
        );

        Ok(serde_json::json!({
            "entities": ner.entities,
            "model": ner.model,
            "elapsed_ms": ner.elapsed_ms,
            "chunk_id": chunk_id,
            "n_entities": ner.entities.len()
        }))
    }

    fn output_data_class(&self) -> &str { "PUBLIC" }
}

impl Default for NerExtractTool {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ner_schema_requires_text() {
        let tool = NerExtractTool::new();
        let schema = tool.parameters_schema();
        let required = schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "text"));
    }

    #[test]
    fn test_ner_tool_name() {
        let tool = NerExtractTool::new();
        assert_eq!(tool.name(), "ner_extract");
    }

    #[test]
    fn test_ner_output_is_public() {
        assert_eq!(NerExtractTool::new().output_data_class(), "PUBLIC");
    }
}
