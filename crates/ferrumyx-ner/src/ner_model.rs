//! Simplified NER using a known-working model structure.

use std::collections::HashMap;
use std::time::Instant;

use candle_core::{Device, Tensor, DType};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use hf_hub::api::sync::Api;
use tokenizers::Tokenizer;
use tracing::{debug, info};

use crate::{NerError, Result, entity_types::{EntityType, normalize_entity_label}};

/// NER configuration.
#[derive(Debug, Clone)]
pub struct NerConfig {
    pub model_id: String,
    pub max_length: usize,
    pub use_gpu: bool,
}

impl Default for NerConfig {
    fn default() -> Self {
        Self {
            model_id: "d4data/biomedical-ner-all".to_string(),
            max_length: 512,
            use_gpu: true,
        }
    }
}

impl NerConfig {
    pub fn biomedical() -> Self {
        Self::default()
    }
}

/// Extracted entity.
#[derive(Debug, Clone)]
pub struct NerEntity {
    pub text: String,
    pub label: String,
    pub entity_type: EntityType,
    pub start: usize,
    pub end: usize,
    pub score: f32,
    pub normalized_id: Option<String>,
}

/// NER model wrapper.
pub struct NerModel {
    model: BertModel,
    tokenizer: Tokenizer,
    classifier: Tensor,
    label_map: HashMap<i64, String>,
    config: NerConfig,
    device: Device,
}

impl NerModel {
    /// Load a NER model from Hugging Face Hub.
    pub async fn new(config: NerConfig) -> Result<Self> {
        let start = Instant::now();
        info!("Loading NER model: {}", config.model_id);

        let device = if config.use_gpu {
            Device::cuda_if_available(0).unwrap_or(Device::Cpu)
        } else {
            Device::Cpu
        };
        debug!("Using device: {:?}", device);

        let model_id = config.model_id.clone();
        let (bert_config, tokenizer, weights_path, label_map) = 
            tokio::task::spawn_blocking(move || {
                Self::download_model(&model_id)
            })
            .await
            .map_err(|e| NerError::Download(e.to_string()))??;

        info!("Loading model weights from {:?}", weights_path);
        
        // Load weights
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path.clone()], DType::F32, &device)
                .map_err(|e| NerError::ModelLoad(e.to_string()))?
        };

        // Try to load BertModel with different prefixes
        let model = BertModel::load(vb.clone(), &bert_config)
            .or_else(|_| BertModel::load(vb.pp("bert"), &bert_config))
            .or_else(|_| BertModel::load(vb.pp("roberta"), &bert_config))
            .map_err(|e| NerError::ModelLoad(format!("BertModel: {}", e)))?;

        // Load classification head
        let num_labels = label_map.len().max(1);
        let hidden_size = bert_config.hidden_size;
        
        // For BERT NER, the classifier weight shape should be [num_labels, hidden_size]
        // and we apply it as: hidden @ weight.T -> [batch, seq, hidden] @ [hidden, num_labels]
        let classifier = vb.pp("classifier")
            .get((num_labels, hidden_size), "weight")
            .or_else(|_| vb.get((num_labels, hidden_size), "classifier.weight"))
            .or_else(|_| vb.pp("bert").pp("classifier").get((num_labels, hidden_size), "weight"))
            .or_else(|_| vb.pp("qa_outputs").get((num_labels, hidden_size), "weight"))
            .map_err(|e| NerError::ModelLoad(format!("Classifier: {}", e)))?;

        info!("Classifier shape: {:?}", classifier.shape());

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

    fn download_model(model_id: &str) -> Result<(Config, Tokenizer, std::path::PathBuf, HashMap<i64, String>)> {
        use hf_hub::{Repo, RepoType};
        
        let api = Api::new()
            .map_err(|e| NerError::Download(format!("API init: {}", e)))?;
        
        let repo = Repo::new(model_id.to_string(), RepoType::Model);
        let api_repo = api.repo(repo);

        // Download config
        let config_path = api_repo.get("config.json")
            .map_err(|e| NerError::Download(format!("config.json: {}", e)))?;
        let config_content = std::fs::read_to_string(&config_path)?;
        
        let config_json: serde_json::Value = serde_json::from_str(&config_content)
            .map_err(|e| NerError::Download(format!("Parse config: {}", e)))?;
        
        // Parse label map
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

        // Download tokenizer - prefer tokenizer.json which has proper configuration
        let tokenizer = if let Ok(tok_path) = api_repo.get("tokenizer.json") {
            info!("Loading tokenizer from: {:?}", tok_path);
            Tokenizer::from_file(&tok_path)
                .map_err(|e| NerError::Tokenization(e.to_string()))?
        } else if let Ok(vocab_path) = api_repo.get("vocab.txt") {
            info!("Building tokenizer from vocab.txt");
            let vocab_content = std::fs::read_to_string(&vocab_path)?;
            let vocab: std::collections::HashMap<String, u32> = vocab_content
                .lines()
                .enumerate()
                .map(|(i, line)| (line.to_string(), i as u32))
                .collect();
            
            let ahash_vocab: ahash::AHashMap<String, u32> = vocab.into_iter().collect();
            
            use tokenizers::models::wordpiece::WordPieceBuilder;
            let wordpiece = WordPieceBuilder::new()
                .vocab(ahash_vocab)
                .continuing_subword_prefix("##".to_string())
                .max_input_chars_per_word(100)
                .unk_token("[UNK]".to_string())
                .build()
                .map_err(|e| NerError::Tokenization(format!("WordPiece: {}", e)))?;
            
            let mut tokenizer = Tokenizer::new(wordpiece);
            // Add normalizer for BERT (lowercase + NFD normalization)
            use tokenizers::normalizers::bert::BertNormalizer;
            let normalizer = BertNormalizer::new(true, true, Some(false), false);
            tokenizer.with_normalizer(normalizer.into());
            
            // Add pre-tokenizer for word splitting
            use tokenizers::pre_tokenizers::whitespace::Whitespace;
            tokenizer.with_pre_tokenizer(Whitespace.into());
            tokenizer
        } else {
            return Err(NerError::Tokenization("No tokenizer found".to_string()));
        };
        
        eprintln!("Tokenizer vocab size: {:?}", tokenizer.get_vocab_size(false));
        eprintln!("Sample tokens: John={:?}, Smith={:?}", 
            tokenizer.encode("John", false).ok().map(|e| e.get_ids().to_vec()),
            tokenizer.encode("Smith", false).ok().map(|e| e.get_ids().to_vec()));
        eprintln!("Full text test: {:?}", tokenizer.encode("John Smith works at Google", false).ok().map(|e| e.get_tokens().to_vec()));

        // Download model weights
        let weights_path = api_repo.get("model.safetensors")
            .or_else(|_| api_repo.get("pytorch_model.bin"))
            .map_err(|e| NerError::Download(format!("Model weights: {}", e)))?;

        Ok((bert_config, tokenizer, weights_path, label_map))
    }

    /// Extract entities from text.
    pub fn extract(&self, text: &str) -> Result<Vec<NerEntity>> {
        let start = Instant::now();

        // Tokenize - encode returns a new Encoding, not modifying self
        let encoding = self.tokenizer
            .encode(text, false)
            .map_err(|e| NerError::Tokenization(e.to_string()))?;

        let tokens = encoding.get_tokens();
        let input_ids = encoding.get_ids();
        let offsets = encoding.get_offsets();
        
        debug!("Tokens: {:?}", tokens);
        debug!("Offsets: {:?}", offsets);
        
        if tokens.is_empty() || input_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Truncate to max length
        let input_ids: Vec<i64> = if input_ids.len() > self.config.max_length {
            input_ids[..self.config.max_length].iter().map(|&id| id as i64).collect()
        } else {
            input_ids.iter().map(|&id| id as i64).collect()
        };

        let seq_len = input_ids.len();
        
        // Create input tensors [batch=1, seq_len]
        let input_ids_tensor = Tensor::new(&input_ids[..], &self.device)?
            .unsqueeze(0)?
            .to_dtype(DType::I64)?;
        
        let token_type_ids = Tensor::zeros((1, seq_len), DType::I64, &self.device)?;
        let attention_mask = Tensor::ones((1, seq_len), DType::F32, &self.device)?;

        // Run model - returns [batch, seq, hidden]
        let hidden_states = self.model.forward(&input_ids_tensor, &token_type_ids, Some(&attention_mask))?;
        
        // Reshape hidden_states to [batch * seq, hidden] for matmul
        let (batch, seq, hidden) = hidden_states.dims3()
            .map_err(|e| NerError::Inference(e.to_string()))?;
        let hidden_states_2d = hidden_states.reshape((batch * seq, hidden))?;
        
        // Apply classifier: [batch*seq, hidden] @ [hidden, num_labels] = [batch*seq, num_labels]
        // classifier is [num_labels, hidden], so transpose it
        let logits = hidden_states_2d.matmul(&self.classifier.t()?)?;
        
        // Reshape back to [batch, seq, num_labels] then squeeze batch
        let num_labels = self.classifier.dim(0)?;
        let logits = logits.reshape((batch, seq, num_labels))?;
        let logits = logits.squeeze(0)?;  // [seq, num_labels]
        
        // Get predictions per token
        let probs = candle_nn::ops::softmax(&logits, 1)?;
        let preds = probs.argmax(1)?
            .to_dtype(DType::I64)?
            .to_vec1::<i64>()
            .map_err(|e| NerError::Inference(e.to_string()))?;

        // Extract entities using BIO tagging
        let tokens: Vec<String> = encoding.get_tokens().iter().map(|s| s.to_string()).collect();
        let entities = self.extract_bio_entities(&tokens, &preds, &probs, text, &encoding);

        debug!("Extracted {} entities in {:?}", entities.len(), start.elapsed());
        Ok(entities)
    }

    fn extract_bio_entities(
        &self,
        tokens: &[String],
        preds: &[i64],
        probs: &Tensor,
        text: &str,
        encoding: &tokenizers::Encoding,
    ) -> Vec<NerEntity> {
        let mut entities = Vec::new();
        // (label, start, end, score) - we reconstruct text from offsets
        let mut current_entity: Option<(String, usize, usize, f32)> = None;

        let offsets = encoding.get_offsets();

        for (i, &pred) in preds.iter().enumerate() {
            let label = self.label_map.get(&pred)
                .map(|s| s.as_str())
                .unwrap_or("O");
            
            let score = probs.get(i)
                .ok()
                .and_then(|t| t.get(pred as usize).ok())
                .and_then(|t| t.to_scalar::<f32>().ok())
                .unwrap_or(0.0);

            // Skip special tokens
            if tokens[i].starts_with('[') && tokens[i].ends_with(']') {
                continue;
            }
            
            // Skip subword tokens (## prefix) for entity detection
            // They will be handled as part of the parent token
            let is_subword = tokens[i].starts_with("##");

            let is_begin = label.starts_with("B-");
            let is_inside = label.starts_with("I-");
            let entity_type = if is_begin || is_inside {
                label.trim_start_matches("B-").trim_start_matches("I-")
            } else {
                ""
            };

            debug!("Token {} '{}' -> {} (score: {:.3})", i, tokens[i], label, score);

            // Handle subword tokens - extend current entity if we have one
            if is_subword {
                if let Some((_, _, ref mut end, ref mut avg_score)) = current_entity {
                    if i < offsets.len() {
                        let (_, char_end) = offsets[i];
                        *end = char_end;
                        *avg_score = (*avg_score + score) / 2.0;
                    }
                }
                continue;
            }

            if is_begin || (is_inside && current_entity.is_none()) {
                // Save previous entity
                if let Some((label, start, end, score)) = current_entity.take() {
                    let entity_text = if end <= text.len() {
                        text[start..end].to_string()
                    } else {
                        String::new()
                    };
                    entities.push(NerEntity {
                        text: entity_text,
                        label: label.clone(),
                        entity_type: normalize_entity_label(&label),
                        start,
                        end,
                        score,
                        normalized_id: None,
                    });
                }

                // Start new entity
                if i < offsets.len() {
                    let (char_start, char_end) = offsets[i];
                    current_entity = Some((
                        entity_type.to_string(),
                        char_start,
                        char_end,
                        score,
                    ));
                }
            } else if is_inside {
                // Check if this continues the current entity (same type)
                if let Some((ref curr_label, ref mut start, ref mut end, ref mut avg_score)) = current_entity {
                    // Continue if same entity type OR if transitioning from ORG to LOC (common for landmarks)
                    let can_continue = curr_label == entity_type || 
                        (curr_label == "ORG" && entity_type == "LOC"); // Eiffel Tower case
                    
                    if can_continue {
                        if i < offsets.len() {
                            let (_, char_end) = offsets[i];
                            *end = char_end;
                            *avg_score = (*avg_score + score) / 2.0;
                        }
                    } else {
                        // Different entity type - save current and start new
                        let (old_label, old_start, old_end, old_score) = 
                            current_entity.take().unwrap();
                        let entity_text = if old_end <= text.len() {
                            text[old_start..old_end].to_string()
                        } else {
                            String::new()
                        };
                        entities.push(NerEntity {
                            text: entity_text,
                            label: old_label.clone(),
                            entity_type: normalize_entity_label(&old_label),
                            start: old_start,
                            end: old_end,
                            score: old_score,
                            normalized_id: None,
                        });
                        // Start new entity
                        let (char_start, char_end) = offsets[i];
                        current_entity = Some((
                            entity_type.to_string(),
                            char_start,
                            char_end,
                            score,
                        ));
                    }
                }
            } else if label == "O" {
                // Save current entity
                if let Some((label, start, end, score)) = current_entity.take() {
                    let entity_text = if end <= text.len() {
                        text[start..end].to_string()
                    } else {
                        String::new()
                    };
                    entities.push(NerEntity {
                        text: entity_text,
                        label: label.clone(),
                        entity_type: normalize_entity_label(&label),
                        start,
                        end,
                        score,
                        normalized_id: None,
                    });
                }
            }
        }

        // Save final entity
        if let Some((label, start, end, score)) = current_entity {
            let entity_text = if end <= text.len() {
                text[start..end].to_string()
            } else {
                String::new()
            };
            entities.push(NerEntity {
                text: entity_text,
                label: label.clone(),
                entity_type: normalize_entity_label(&label),
                start,
                end,
                score,
                normalized_id: None,
            });
        }

        entities
    }
}
