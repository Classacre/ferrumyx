//! NER model implementation using Candle token classification.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use candle_core::{Device, Tensor, DType};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use hf_hub::api::sync::Api;
use tokenizers::Tokenizer;
use tracing::{debug, info, warn};

use crate::{NerError, Result, EntityType, normalize_entity_label};

/// A single extracted entity.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NerEntity {
    pub text: String,
    pub label: String,
    pub entity_type: EntityType,
    pub start: usize,
    pub end: usize,
    pub score: f32,
    pub normalized_id: Option<String>,
}

/// Configuration for the NER model.
#[derive(Debug, Clone)]
pub struct NerConfig {
    /// Hugging Face model ID
    pub model_id: String,
    /// Maximum sequence length (default: 512)
    pub max_length: usize,
    /// Minimum confidence threshold (default: 0.5)
    pub min_confidence: f32,
    /// Use GPU if available
    pub use_gpu: bool,
}

impl Default for NerConfig {
    fn default() -> Self {
        Self {
            model_id: "d4data/biomedical-ner-all".to_string(),
            max_length: 512,
            min_confidence: 0.5,
            use_gpu: false,
        }
    }
}

impl NerConfig {
    /// Create config for biomedical NER model.
    pub fn biomedical() -> Self {
        Self {
            model_id: "d4data/biomedical-ner-all".to_string(),
            ..Default::default()
        }
    }
    
    /// Create config for BC5CDR (chemical/disease) model.
    pub fn bc5cdr() -> Self {
        Self {
            model_id: "alvaroalon2/biobert_diseases_ner".to_string(),
            ..Default::default()
        }
    }
}

/// Biomedical NER model wrapper.
pub struct NerModel {
    model: BertModel,
    tokenizer: Tokenizer,
    classifier: Tensor,  // Classification head weights
    label_map: HashMap<i64, String>,
    config: NerConfig,
    device: Device,
}

impl NerModel {
    /// Load a NER model from Hugging Face Hub.
    pub async fn new(config: NerConfig) -> Result<Self> {
        let start = Instant::now();
        info!("Loading NER model: {}", config.model_id);

        let device = Self::select_device(&config)?;
        debug!("Using device: {:?}", device);

        let model_id = config.model_id.clone();
        let (bert_config, tokenizer, weights_path, label_map) = 
            tokio::task::spawn_blocking(move || {
                Self::download_model(&model_id)
            })
            .await
            .map_err(|e| NerError::Download(e.to_string()))??;

        info!("Loading model weights...");
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, &device)
                .map_err(|e| NerError::ModelLoad(e.to_string()))?
        };

        let model = BertModel::load(vb.clone(), &bert_config)
            .map_err(|e| NerError::ModelLoad(e.to_string()))?;

        // Load classification head (ner_head.weight)
        let classifier = vb.pp("classifier")
            .get((label_map.len(), bert_config.hidden_size), "weight")
            .map_err(|e| NerError::ModelLoad(format!("Classifier head: {}", e)))?;

        info!("NER model loaded in {:?}", start.elapsed());

        Ok(Self {
            model,
            tokenizer,
            classifier,
            label_map,
            config,
            device,
        })
    }

    fn select_device(config: &NerConfig) -> Result<Device> {
        if config.use_gpu {
            if let Ok(device) = Device::new_cuda(0) {
                return Ok(device);
            }
        }
        Ok(Device::Cpu)
    }

    fn download_model(model_id: &str) -> Result<(Config, Tokenizer, std::path::PathBuf, HashMap<i64, String>)> {
        use hf_hub::{Repo, RepoType};
        
        let api = Api::new()
            .map_err(|e| NerError::Download(format!("API init: {}", e)))?;
        
        let repo = Repo::new(model_id.to_string(), RepoType::Model);
        let api_repo = api.repo(repo);

        // Download config
        let config_path = api_repo.get("config.json")
            .map_err(|e| NerError::Download(format!("config.json: {}", e)))?;
        let config_content = std::fs::read_to_string(&config_path)
            .map_err(|e| NerError::Download(format!("Read config: {}", e)))?;
        
        // Parse config to get label map
        let config_json: serde_json::Value = serde_json::from_str(&config_content)
            .map_err(|e| NerError::Download(format!("Parse config: {}", e)))?;
        
        let label_map: HashMap<i64, String> = config_json["id2label"]
            .as_object()
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| {
                        let id: i64 = k.parse().ok()?;
                        let label = v.as_str()?.to_string();
                        Some((id, label))
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Build BertConfig
        let bert_config = Config {
            vocab_size: config_json["vocab_size"].as_u64().unwrap_or(30522) as usize,
            hidden_size: config_json["hidden_size"].as_u64().unwrap_or(768) as usize,
            num_hidden_layers: config_json["num_hidden_layers"].as_u64().unwrap_or(12) as usize,
            num_attention_heads: config_json["num_attention_heads"].as_u64().unwrap_or(12) as usize,
            intermediate_size: config_json["intermediate_size"].as_u64().unwrap_or(3072) as usize,
            hidden_act: candle_transformers::models::bert::HiddenAct::Gelu,
            hidden_dropout_prob: 0.1,
            max_position_embeddings: config_json["max_position_embeddings"].as_u64().unwrap_or(512) as usize,
            type_vocab_size: 2,
            initializer_range: 0.02,
            layer_norm_eps: 1e-12,
            pad_token_id: 0,
            position_embedding_type: candle_transformers::models::bert::PositionEmbeddingType::Absolute,
            use_cache: true,
            classifier_dropout: None,
            model_type: Some("bert".to_string()),
        };

        // Download tokenizer
        let tokenizer = if let Ok(tok_path) = api_repo.get("tokenizer.json") {
            Tokenizer::from_file(&tok_path)
                .map_err(|e| NerError::Tokenization(e.to_string()))?
        } else if let Ok(vocab_path) = api_repo.get("vocab.txt") {
            // Build from vocab.txt using AHashMap for tokenizers
            use std::collections::HashMap as StdHashMap;
            let vocab_content = std::fs::read_to_string(&vocab_path)?;
            let vocab: StdHashMap<String, u32> = vocab_content
                .lines()
                .enumerate()
                .map(|(i, line)| (line.to_string(), i as u32))
                .collect();
            
            // Convert to AHashMap for tokenizers crate
            let ahash_vocab: ahash::AHashMap<String, u32> = vocab.into_iter().collect();
            
            use tokenizers::models::wordpiece::WordPieceBuilder;
            let wordpiece = WordPieceBuilder::new()
                .vocab(ahash_vocab)
                .continuing_subword_prefix("##".to_string())
                .max_input_chars_per_word(100)
                .unk_token("[UNK]".to_string())
                .build()
                .map_err(|e| NerError::Tokenization(format!("WordPiece: {}", e)))?;
            Tokenizer::new(wordpiece)
        } else {
            return Err(NerError::Tokenization("No tokenizer found".to_string()));
        };

        // Download model weights
        let weights_path = api_repo.get("model.safetensors")
            .or_else(|_| api_repo.get("pytorch_model.bin"))
            .map_err(|e| NerError::Download(format!("Model weights: {}", e)))?;

        Ok((bert_config, tokenizer, weights_path, label_map))
    }

    /// Extract entities from text.
    pub fn extract(&self, text: &str) -> Result<Vec<NerEntity>> {
        let start = Instant::now();

        // Tokenize
        let encoding = self.tokenizer
            .encode(text, true)
            .map_err(|e| NerError::Tokenization(e.to_string()))?;

        let tokens = encoding.get_tokens();
        let input_ids = encoding.get_ids();
        
        if tokens.is_empty() {
            return Ok(Vec::new());
        }

        // Truncate to max length
        let input_ids: Vec<i64> = if input_ids.len() > self.config.max_length {
            input_ids[..self.config.max_length].iter().map(|&id| id as i64).collect()
        } else {
            input_ids.iter().map(|&id| id as i64).collect()
        };

        let seq_len = input_ids.len();
        
        // Create input tensors
        let input_ids_tensor = Tensor::new(&input_ids[..], &self.device)?
            .unsqueeze(0)?;
        
        let attention_mask = Tensor::ones((1, seq_len), DType::F32, &self.device)?;
        let token_type_ids = Tensor::zeros((1, seq_len), DType::I64, &self.device)?;

        // Run model
        let hidden_states = self.model.forward(&input_ids_tensor, &token_type_ids, None)?;
        
        // Apply classifier head: [batch, seq, hidden] @ [num_labels, hidden].T
        let logits = hidden_states.matmul(&self.classifier.t()?)?;
        
        // Get predictions
        let logits_vec: Vec<Vec<f32>> = logits
            .squeeze(0)?
            .to_vec2()
            .map_err(|e| NerError::Inference(e.to_string()))?;

        // Decode entities using BIO tagging
        let entities = self.decode_bio(&logits_vec, tokens, text, &encoding);

        debug!(
            n_entities = entities.len(),
            elapsed_ms = start.elapsed().as_millis(),
            "NER extraction complete"
        );

        Ok(entities)
    }

    fn decode_bio(
        &self,
        logits: &[Vec<f32>],
        tokens: &[String],
        text: &str,
        encoding: &tokenizers::Encoding,
    ) -> Vec<NerEntity> {
        let mut entities = Vec::new();
        let mut current_entity: Option<(String, EntityType, usize, usize, f32)> = None;

        for (i, token_logits) in logits.iter().enumerate() {
            // Skip special tokens
            if tokens[i].starts_with('[') && tokens[i].ends_with(']') {
                continue;
            }

            // Find best label
            let (label_id, score) = token_logits
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(id, &s)| (id as i64, s))
                .unwrap_or((0, 0.0));

            let label = self.label_map.get(&label_id)
                .map(|s| s.as_str())
                .unwrap_or("O");

            if score < self.config.min_confidence {
                continue;
            }

            let is_begin = label.starts_with("B-");
            let is_inside = label.starts_with("I-");
            let entity_label = label.trim_start_matches("B-").trim_start_matches("I-");

            if is_begin || (is_inside && current_entity.is_none()) {
                // Save previous entity
                if let Some((text, etype, start, end, score)) = current_entity.take() {
                    entities.push(NerEntity {
                        text,
                        label: etype.as_str().to_string(),
                        entity_type: etype,
                        start,
                        end,
                        score,
                        normalized_id: None,
                    });
                }

                // Start new entity - use token offsets
                let offsets = encoding.get_offsets();
                if i < offsets.len() {
                    let (char_start, char_end) = offsets[i];
                    current_entity = Some((
                        text[char_start..char_end].to_string(),
                        normalize_entity_label(entity_label),
                        char_start,
                        char_end,
                        score,
                    ));
                }
            } else if is_inside {
                if let Some((_, _, _, ref mut end, ref mut avg_score)) = current_entity {
                    let offsets = encoding.get_offsets();
                    if i < offsets.len() {
                        let (_, char_end) = offsets[i];
                        *end = char_end;
                        // Average score
                        *avg_score = (*avg_score + score) / 2.0;
                    }
                }
            } else if label == "O" {
                // End current entity
                if let Some((text, etype, start, end, score)) = current_entity.take() {
                    entities.push(NerEntity {
                        text,
                        label: etype.as_str().to_string(),
                        entity_type: etype,
                        start,
                        end,
                        score,
                        normalized_id: None,
                    });
                }
            }
        }

        // Don't forget last entity
        if let Some((text, etype, start, end, score)) = current_entity {
            entities.push(NerEntity {
                text,
                label: etype.as_str().to_string(),
                entity_type: etype,
                start,
                end,
                score,
                normalized_id: None,
            });
        }

        entities
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ner_config_default() {
        let config = NerConfig::default();
        assert!(config.model_id.contains("biomedical"));
    }
}
