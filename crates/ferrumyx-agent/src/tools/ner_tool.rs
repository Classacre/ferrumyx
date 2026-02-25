//! IronClaw tool: NER extraction via Rust-native Trie-based matching.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use ferrumyx_ner::{TrieNer, ExtractedEntity};
use super::FerrumyxTool;
use std::sync::OnceLock;

static NER_MODEL: OnceLock<TrieNer> = OnceLock::new();

pub struct NerExtractTool;

impl NerExtractTool {
    pub fn new() -> Self {
        Self
    }
    
    fn get_model() -> &'static TrieNer {
        NER_MODEL.get_or_init(|| {
            match TrieNer::with_complete_databases() {
                Ok(ner) => {
                    tracing::info!("NER model loaded: {} patterns", ner.stats().total_patterns);
                    ner
                }
                Err(e) => {
                    tracing::warn!("Failed to load complete databases, using embedded subset: {}", e);
                    TrieNer::with_embedded_subset()
                }
            }
        })
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

        let model = Self::get_model();
        let entities = model.extract(text);

        tracing::info!(
            tool = "ner_extract",
            n_entities = entities.len(),
            "NER complete"
        );

        // Convert entities to JSON-serializable format
        let entities_json: Vec<Value> = entities.iter().map(|e| {
            serde_json::json!({
                "text": e.text,
                "label": format!("{:?}", e.label),
                "start": e.start,
                "end": e.end,
                "confidence": e.confidence
            })
        }).collect();

        Ok(serde_json::json!({
            "entities": entities_json,
            "n_entities": entities.len(),
            "chunk_id": chunk_id
        }))
    }

    fn output_data_class(&self) -> &str { "PUBLIC" }
}

impl Default for NerExtractTool {
    fn default() -> Self { Self::new() }
}
