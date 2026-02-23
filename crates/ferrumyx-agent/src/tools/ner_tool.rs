//! IronClaw tool: NER extraction via Rust-native Candle model.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use ferrumyx_ner::{NerModel, NerConfig, NerEntity};
use super::FerrumyxTool;
use std::sync::OnceLock;

static NER_MODEL: OnceLock<NerModel> = OnceLock::new();

pub struct NerExtractTool {
    config: NerConfig,
}

impl NerExtractTool {
    pub fn new() -> Self {
        Self {
            config: NerConfig::biomedical(),
        }
    }
    
    async fn get_model(&self) -> Result<&'static NerModel> {
        if let Some(model) = NER_MODEL.get() {
            return Ok(model);
        }
        
        let model = NerModel::new(self.config.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load NER model: {}", e))?;
        
        // Race condition is fine - both threads would load the same model
        let _ = NER_MODEL.set(model);
        Ok(NER_MODEL.get().unwrap())
    }
}

#[async_trait]
impl FerrumyxTool for NerExtractTool {
    fn name(&self) -> &str { "ner_extract" }

    fn description(&self) -> &str {
        "Extract biomedical named entities from text using Rust-native NER. \
         Identifies genes, mutations, diseases, chemicals, and species."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "The text to extract entities from."
                },
                "chunk_id": {
                    "type": "string",
                    "description": "Optional chunk UUID for provenance tracking."
                }
            },
            "required": ["text"]
        })
    }

    async fn invoke(&self, params: Value) -> Result<Value> {
        let text = params["text"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing required param: text"))?;
        let chunk_id = params["chunk_id"].as_str();

        tracing::debug!(
            tool = "ner_extract",
            text_len = text.len(),
            chunk_id = ?chunk_id,
            "Running NER extraction"
        );

        let model = self.get_model().await?;
        let entities = model.extract(text)
            .map_err(|e| anyhow::anyhow!("NER extraction failed: {}", e))?;

        tracing::info!(
            tool = "ner_extract",
            n_entities = entities.len(),
            "NER complete"
        );

        Ok(serde_json::json!({
            "entities": entities,
            "n_entities": entities.len(),
            "chunk_id": chunk_id
        }))
    }

    fn output_data_class(&self) -> &str { "PUBLIC" }
}

impl Default for NerExtractTool {
    fn default() -> Self { Self::new() }
}
