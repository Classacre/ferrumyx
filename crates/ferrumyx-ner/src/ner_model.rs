//! Biomedical NER using Candle with OpenMed models.
//!
//! Supports BERT, RoBERTa, XLM-RoBERTa, and DeBERTa-v2 architectures from OpenMed.
//! OpenMed provides 380+ specialized biomedical NER models.
//! See: https://huggingface.co/OpenMed

use std::collections::HashMap;
use std::time::Instant;

use candle_core::{Device, Tensor, DType, Module};
use candle_nn::VarBuilder;
use candle_transformers::models::{bert, debertav2};
use hf_hub::api::sync::Api;
use tokenizers::Tokenizer;
use tracing::{debug, info, warn};

use crate::{NerError, Result, entity_types::{EntityType, normalize_entity_label}};

/// NER configuration for OpenMed biomedical models.
#[derive(Debug, Clone)]
pub struct NerConfig {
    pub model_id: String,
    pub max_length: usize,
    pub use_gpu: bool,
}

impl Default for NerConfig {
    fn default() -> Self {
        Self::diseases()
    }
}

impl NerConfig {
    // ============================================
    // DISEASE DETECTION
    // ============================================
    
    /// Biomedical NER - Combined disease and chemical detection (alias for diseases)
    pub fn biomedical() -> Self {
        Self::diseases()
    }

    /// Disease NER - OpenMed BERT (F1: 0.900 on BC5CDR-Disease)
    pub fn diseases() -> Self {
        Self {
            model_id: "OpenMed/OpenMed-NER-DiseaseDetect-BioMed-335M".to_string(),
            max_length: 512,
            use_gpu: true,
        }
    }
    
    /// Disease NER - Larger model (F1: 0.912, 434M params)
    pub fn diseases_large() -> Self {
        Self {
            model_id: "OpenMed/OpenMed-NER-DiseaseDetect-SuperClinical-434M".to_string(),
            max_length: 512,
            use_gpu: true,
        }
    }
    
    // ============================================
    // PHARMACOLOGY / CHEMICALS
    // ============================================
    
    /// Pharmaceutical/Chemical NER (F1: 0.961 on BC5CDR-Chem)
    pub fn pharmaceuticals() -> Self {
        Self {
            model_id: "OpenMed/OpenMed-NER-PharmaDetect-SuperClinical-434M".to_string(),
            max_length: 512,
            use_gpu: true,
        }
    }
    
    /// Chemical NER (F1: 0.954 on BC4CHEMD)
    pub fn chemicals() -> Self {
        Self {
            model_id: "OpenMed/OpenMed-NER-ChemicalDetect-PubMed-335M".to_string(),
            max_length: 512,
            use_gpu: true,
        }
    }
    
    // ============================================
    // GENOMICS / GENETICS
    // ============================================
    
    /// Genomic NER - Cell lines (F1: 0.998 on Gellus) - XLM-RoBERTa
    pub fn genomic() -> Self {
        Self {
            model_id: "OpenMed/OpenMed-NER-GenomicDetect-SnowMed-568M".to_string(),
            max_length: 512,
            use_gpu: true,
        }
    }
    
    /// DNA/Protein NER (F1: 0.819 on JNLPBA)
    pub fn dna_proteins() -> Self {
        Self {
            model_id: "OpenMed/OpenMed-NER-DNADetect-SuperClinical-434M".to_string(),
            max_length: 512,
            use_gpu: true,
        }
    }
    
    /// Genome NER - Gene mentions (F1: 0.901 on BC2GM)
    pub fn genome() -> Self {
        Self {
            model_id: "OpenMed/OpenMed-NER-GenomeDetect-SuperClinical-434M".to_string(),
            max_length: 512,
            use_gpu: true,
        }
    }
    
    /// Protein NER (F1: 0.961 on FSU)
    pub fn proteins() -> Self {
        Self {
            model_id: "OpenMed/OpenMed-NER-ProteinDetect-SnowMed-568M".to_string(),
            max_length: 512,
            use_gpu: true,
        }
    }
    
    // ============================================
    // ONCOLOGY
    // ============================================
    
    /// Oncology NER - Cancer entities (F1: 0.899 on BioNLP 2013 CG) - RoBERTa
    pub fn oncology() -> Self {
        Self {
            model_id: "OpenMed/OpenMed-NER-OncologyDetect-SuperMedical-355M".to_string(),
            max_length: 512,
            use_gpu: true,
        }
    }
    
    // ============================================
    // ANATOMY
    // ============================================
    
    /// Anatomy NER (F1: 0.906 on AnatEM)
    pub fn anatomy() -> Self {
        Self {
            model_id: "OpenMed/OpenMed-NER-AnatomyDetect-ElectraMed-560M".to_string(),
            max_length: 512,
            use_gpu: true,
        }
    }
    
    // ============================================
    // SPECIES / ORGANISMS
    // ============================================
    
    /// Species NER (F1: 0.965 on Linnaeus)
    pub fn species() -> Self {
        Self {
            model_id: "OpenMed/OpenMed-NER-SpeciesDetect-PubMed-335M".to_string(),
            max_length: 512,
            use_gpu: true,
        }
    }
    
    // ============================================
    // MUTATIONS
    // ============================================
    
    /// Mutation NER - Protein changes and variants (F1: 0.892 on tmVar)
    pub fn mutations() -> Self {
        Self {
            model_id: "OpenMed/OpenMed-NER-MutationDetect-SuperClinical-434M".to_string(),
            max_length: 512,
            use_gpu: true,
        }
    }
    
    // ============================================
    // GENERAL NER
    // ============================================
    
    /// General NER - BERT-based model for PER, ORG, LOC, MISC (CoNLL-2003)
    pub fn general() -> Self {
        Self {
            model_id: "dslim/bert-base-NER".to_string(),
            max_length: 512,
            use_gpu: true,
        }
    }
    
    // ============================================
    // CUSTOM
    // ============================================
    
    /// Custom OpenMed model from Hugging Face Hub
    pub fn custom(model_id: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            max_length: 512,
            use_gpu: true,
        }
    }
}

/// Extracted entity.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NerEntity {
    pub text: String,
    pub label: String,
    #[serde(rename = "type")]
    pub entity_type: EntityType,
    pub start: usize,
    pub end: usize,
    pub score: f32,
    pub normalized_id: Option<String>,
}

/// Model architecture type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelArch {
    Bert,
    Roberta,  // Uses BERT model with different tokenizer
    XlmRoberta,
    DebertaV2,
}

/// Inner model container supporting multiple architectures
enum ModelInner {
    Bert {
        model: bert::BertModel,
        classifier: candle_nn::Linear,
    },
    DebertaV2 {
        model: debertav2::DebertaV2NERModel,
    },
}

/// Model config extracted from HuggingFace config.json
struct ModelConfigInfo {
    arch: ModelArch,
    id2label: HashMap<u32, String>,
    hidden_size: usize,
    num_labels: usize,
}

/// NER model wrapper supporting multiple architectures.
pub struct NerModel {
    model: ModelInner,
    tokenizer: Tokenizer,
    id2label: HashMap<u32, String>,
    config: NerConfig,
    device: Device,
    arch: ModelArch,
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
        let device_clone = device.clone();
        let (tokenizer, weights_path, config_info) = 
            tokio::task::spawn_blocking(move || {
                Self::download_model(&model_id, &device_clone)
            })
            .await
            .map_err(|e| NerError::Download(e.to_string()))??;

        info!("Model architecture: {:?}", config_info.arch);
        info!("Labels: {:?}", config_info.id2label);
        info!("Loading model weights from {:?}", weights_path);
        
        // Load weights - use F32 for CPU inference (BF16 not supported on CPU for matmul)
        // Models saved as BF16 will be converted to F32 during loading
        let dtype = DType::F32;
        
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path.clone()], dtype, &device)
                .map_err(|e| NerError::ModelLoad(e.to_string()))?
        };

        // Load model based on architecture
        let model = match config_info.arch {
            ModelArch::Bert | ModelArch::Roberta | ModelArch::XlmRoberta => {
                Self::load_bert_model(vb, &config_info)?
            }
            ModelArch::DebertaV2 => {
                Self::load_deberta_model(vb, &config_info)?
            }
        };

        info!("NER model loaded in {:?}", start.elapsed());

        Ok(Self {
            model,
            tokenizer,
            id2label: config_info.id2label,
            config,
            device,
            arch: config_info.arch,
        })
    }

    fn load_bert_model(vb: VarBuilder, config_info: &ModelConfigInfo) -> Result<ModelInner> {
        // Architecture-specific defaults
        let (vocab_size, type_vocab_size, position_embedding_type, model_prefix) = match config_info.arch {
            ModelArch::XlmRoberta => {
                // XLM-RoBERTa: 250k vocab, no type embeddings
                (250002, 1, bert::PositionEmbeddingType::Absolute, "roberta")
            }
            ModelArch::Roberta => {
                // RoBERTa: 50k vocab, no type embeddings
                (50265, 1, bert::PositionEmbeddingType::Absolute, "roberta")
            }
            ModelArch::Bert | _ => {
                // Standard BERT
                (30522, 2, bert::PositionEmbeddingType::Absolute, "bert")
            }
        };
        
        // Get max position embeddings from config - models have different values
        let max_position_embeddings = match config_info.arch {
            ModelArch::XlmRoberta => 8194,  // XLM-RoBERTa uses 8194
            ModelArch::Roberta => 514,      // RoBERTa uses 514
            _ => 512,                       // Standard BERT uses 512
        };
        
        // Build BERT config
        let bert_config = bert::Config {
            vocab_size,
            hidden_size: config_info.hidden_size,
            num_hidden_layers: 24,
            num_attention_heads: 16,
            intermediate_size: 4096,
            hidden_act: bert::HiddenAct::Gelu,
            hidden_dropout_prob: 0.2,
            max_position_embeddings,
            type_vocab_size,
            initializer_range: 0.02,
            layer_norm_eps: 1e-7,
            pad_token_id: 1,  // RoBERTa/XLM-R use 1 for padding
            position_embedding_type,
            use_cache: true,
            classifier_dropout: Some(0.2),
            model_type: Some("bert".to_string()),
        };
        
        // For RoBERTa/XLM-RoBERTa, we need to remap tensor names from "roberta.*" to "bert.*"
        // because candle's BertModel expects "bert.*" prefix
        let (vb, prefix) = if config_info.arch == ModelArch::Roberta || config_info.arch == ModelArch::XlmRoberta {
            // Create a renaming VarBuilder that maps "bert.*" -> "roberta.*"
            // This way when BertModel asks for "bert.embeddings...", we look for "roberta.embeddings..."
            let renamer: Box<dyn Fn(&str) -> String + Send + Sync> = Box::new(move |name: &str| {
                if name.starts_with("bert.") {
                    name.replacen("bert.", "roberta.", 1)
                } else {
                    name.to_string()
                }
            });
            (vb.clone().rename(renamer), "bert")
        } else {
            (vb.clone(), "bert")
        };
        
        // Load BERT model
        let bert_model = bert::BertModel::load(vb.pp(prefix), &bert_config)
            .map_err(|e| NerError::ModelLoad(format!("BertModel: {}", e)))?;
        
        // Load classifier layer - try "classifier" first (it's at root level in safetensors)
        let classifier = candle_nn::linear(config_info.hidden_size, config_info.num_labels, vb.pp("classifier"))
            .map_err(|e| NerError::ModelLoad(format!("Classifier: {}", e)))?;
        
        Ok(ModelInner::Bert {
            model: bert_model,
            classifier,
        })
    }
    
    fn load_deberta_model(vb: VarBuilder, config_info: &ModelConfigInfo) -> Result<ModelInner> {
        // Convert id2label to the format expected by DeBERTa (u32 keys)
        let id2label: std::collections::HashMap<u32, String> = config_info.id2label.clone();
        
        // Build DeBERTa config with id2label
        let deberta_config = debertav2::Config {
            vocab_size: 128100,
            hidden_size: config_info.hidden_size,
            num_hidden_layers: 24,
            num_attention_heads: 16,
            intermediate_size: 4096,
            hidden_act: debertav2::HiddenAct::Gelu,
            hidden_dropout_prob: 0.1,
            attention_probs_dropout_prob: 0.1,
            max_position_embeddings: 512,
            type_vocab_size: 0,
            initializer_range: 0.02,
            layer_norm_eps: 1e-7,
            relative_attention: true,
            max_relative_positions: -1,
            pad_token_id: Some(0),
            position_biased_input: false,
            pos_att_type: vec!["p2c".to_string(), "c2p".to_string()],
            position_buckets: None,
            share_att_key: None,
            attention_head_size: None,
            embedding_size: None,
            norm_rel_ebd: None,
            conv_kernel_size: None,
            conv_groups: None,
            conv_act: None,
            id2label: Some(id2label),
            label2id: None,
            pooler_dropout: None,
            pooler_hidden_act: None,
            pooler_hidden_size: None,
            cls_dropout: None,
        };
        
        // DeBERTa models use "deberta" prefix in safetensors
        let vb = vb.set_prefix("deberta");
        
        // Try loading with position_buckets disabled if it fails
        let model = match debertav2::DebertaV2NERModel::load(vb.clone(), &deberta_config, None) {
            Ok(m) => m,
            Err(e) => {
                warn!("Standard DeBERTa load failed, trying with position_buckets=0: {}", e);
                let mut config = deberta_config;
                config.position_buckets = Some(0);
                debertav2::DebertaV2NERModel::load(vb, &config, None)
                    .map_err(|e| NerError::ModelLoad(format!("DebertaV2: {}", e)))?
            }
        };
        
        Ok(ModelInner::DebertaV2 { model })
    }

    fn download_model(model_id: &str, _device: &Device) -> Result<(Tokenizer, std::path::PathBuf, ModelConfigInfo)> {
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
        
        // Detect architecture
        let arch = Self::detect_architecture(&config_json);
        info!("Detected architecture: {:?}", arch);
        
        // Parse id2label
        let id2label: HashMap<u32, String> = config_json["id2label"]
            .as_object()
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| {
                        let id: u32 = k.parse().ok()?;
                        let label = v.as_str()?.to_string();
                        Some((id, label))
                    })
                    .collect()
            })
            .ok_or_else(|| NerError::Download("No id2label in config".to_string()))?;
        
        let num_labels = id2label.len();
        
        // Get hidden_size from config
        let hidden_size = config_json["hidden_size"].as_u64().unwrap_or(1024) as usize;
        
        info!("Model labels: {:?}", id2label);
        info!("Hidden size: {}, Num labels: {}", hidden_size, num_labels);

        // Download tokenizer - try architecture-specific tokenizers first
        let tokenizer = if let Ok(tok_path) = api_repo.get("tokenizer.json") {
            info!("Loading tokenizer from: {:?}", tok_path);
            Tokenizer::from_file(&tok_path)
                .map_err(|e| NerError::Tokenization(e.to_string()))?
        } else if arch == ModelArch::Roberta || arch == ModelArch::XlmRoberta {
            // RoBERTa/XLM-RoBERTa use ByteLevel BPE tokenizer from roberta-base
            info!("Loading RoBERTa/XLM-R ByteLevel BPE tokenizer...");
            
            // Try to download tokenizer from the base model
            let base_model = if arch == ModelArch::XlmRoberta {
                "xlm-roberta-base"
            } else {
                "roberta-base"
            };
            
            let base_repo = Repo::new(base_model.to_string(), RepoType::Model);
            let base_api_repo = api.repo(base_repo);
            
            if let Ok(tok_path) = base_api_repo.get("tokenizer.json") {
                info!("Loading tokenizer from base model: {:?}", tok_path);
                Tokenizer::from_file(&tok_path)
                    .map_err(|e| NerError::Tokenization(format!("Failed to load {} tokenizer: {}", base_model, e)))?
            } else {
                return Err(NerError::Tokenization(
                    format!("Could not download {} tokenizer. Please ensure internet connectivity.", base_model)
                ));
            }
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
            use tokenizers::normalizers::bert::BertNormalizer;
            let normalizer = BertNormalizer::new(true, true, Some(false), false);
            tokenizer.with_normalizer(normalizer.into());
            
            use tokenizers::pre_tokenizers::whitespace::Whitespace;
            tokenizer.with_pre_tokenizer(Whitespace.into());
            tokenizer
        } else {
            return Err(NerError::Tokenization("No tokenizer found".to_string()));
        };
        
        info!("Tokenizer vocab size: {:?}", tokenizer.get_vocab_size(false));

        // Download model weights - safetensors only
        let weights_path = api_repo.get("model.safetensors")
            .map_err(|e| NerError::Download(format!("Safetensors not available: {}. Note: This model requires safetensors format for Candle.", e)))?;
        
        let metadata = std::fs::metadata(&weights_path)
            .map_err(|e| NerError::Download(format!("Cannot read weights: {}", e)))?;
        info!("Model weights size: {} MB", metadata.len() / 1_000_000);

        let config_info = ModelConfigInfo {
            arch,
            id2label,
            hidden_size,
            num_labels,
        };

        Ok((tokenizer, weights_path, config_info))
    }
    
    fn detect_architecture(config: &serde_json::Value) -> ModelArch {
        let model_type = config["model_type"].as_str().unwrap_or("");
        let architectures = config["architectures"].as_array();
        
        // Check model_type first
        match model_type {
            "deberta-v2" | "deberta" => return ModelArch::DebertaV2,
            "xlm-roberta" => return ModelArch::XlmRoberta,
            "roberta" => return ModelArch::Roberta,
            "bert" | "biobert" => return ModelArch::Bert,
            _ => {}
        }
        
        // Check architectures list
        if let Some(archs) = architectures {
            for arch in archs {
                if let Some(name) = arch.as_str() {
                    let name_lower = name.to_lowercase();
                    if name_lower.contains("deberta") {
                        return ModelArch::DebertaV2;
                    }
                    if name_lower.contains("xlmroberta") || name_lower.contains("xlm-roberta") {
                        return ModelArch::XlmRoberta;
                    }
                    if name_lower.contains("roberta") {
                        return ModelArch::Roberta;
                    }
                    if name_lower.contains("bert") {
                        return ModelArch::Bert;
                    }
                }
            }
        }
        
        // Default to BERT for OpenMed models
        ModelArch::Bert
    }

    /// Extract entities from text.
    pub fn extract(&self, text: &str) -> Result<Vec<NerEntity>> {
        let start = Instant::now();

        // Tokenize
        let encoding = self.tokenizer
            .encode(text, false)
            .map_err(|e| NerError::Tokenization(e.to_string()))?;

        let tokens = encoding.get_tokens();
        let input_ids = encoding.get_ids();
        
        debug!("Tokens: {:?}", tokens);
        
        if tokens.is_empty() || input_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Truncate to max length
        let input_ids: Vec<u32> = if input_ids.len() > self.config.max_length {
            input_ids[..self.config.max_length].to_vec()
        } else {
            input_ids.to_vec()
        };

        let seq_len = input_ids.len();
        
        // Create input tensors [batch=1, seq_len]
        let input_ids_tensor = Tensor::new(&input_ids[..], &self.device)?
            .unsqueeze(0)?;
        
        let attention_mask = Tensor::ones((1, seq_len), DType::F32, &self.device)?;
        let token_type_ids = Tensor::zeros((1, seq_len), DType::I64, &self.device)?;

        // Run model forward pass based on architecture
        let logits = match &self.model {
            ModelInner::Bert { model, classifier } => {
                let hidden_states = model.forward(&input_ids_tensor, &token_type_ids, Some(&attention_mask))?;
                debug!("BERT hidden states shape: {:?}", hidden_states.shape());
                classifier.forward(&hidden_states)?
            }
            ModelInner::DebertaV2 { model } => {
                model.forward(
                    &input_ids_tensor,
                    Some(token_type_ids),
                    Some(attention_mask),
                ).map_err(|e| NerError::Inference(e.to_string()))?
            }
        };
        
        debug!("Logits shape: {:?}", logits.shape());

        // Apply softmax to get probabilities
        let probs = candle_nn::ops::softmax(&logits, 2)
            .map_err(|e| NerError::Inference(e.to_string()))?;
        
        // Get max probability and label index per token
        let max_scores = probs.max(2)
            .map_err(|e| NerError::Inference(e.to_string()))?
            .to_vec2::<f32>()
            .map_err(|e| NerError::Inference(e.to_string()))?;
        
        let max_indices = logits.argmax(2)
            .map_err(|e| NerError::Inference(e.to_string()))?
            .to_vec2::<u32>()
            .map_err(|e| NerError::Inference(e.to_string()))?;

        // Extract entities using BIO tagging
        let entities = self.extract_entities_bio(
            &max_scores[0],
            &max_indices[0],
            &encoding,
            text,
        );

        debug!("Extracted {} entities in {:?}", entities.len(), start.elapsed());
        Ok(entities)
    }
    
    fn extract_entities_bio(
        &self,
        scores: &[f32],
        label_indices: &[u32],
        encoding: &tokenizers::Encoding,
        text: &str,
    ) -> Vec<NerEntity> {
        let tokens = encoding.get_tokens();
        let offsets = encoding.get_offsets();
        let special_tokens_mask = encoding.get_special_tokens_mask();
        let word_ids = encoding.get_word_ids();
        
        let mut entities = Vec::new();
        let mut current_entity: Option<(String, usize, usize, f32)> = None;
        
        // Track which word positions we've seen to avoid duplicates
        let mut last_word_id: Option<u32> = None;
        
        // Debug: log all predictions
        debug!("=== Entity Extraction Debug ===");
        debug!("Text: {}", text);
        debug!("Available labels: {:?}", self.id2label);
        
        for (i, &label_idx) in label_indices.iter().enumerate() {
            // Skip special tokens
            if i < special_tokens_mask.len() && special_tokens_mask[i] == 1 {
                continue;
            }
            
            // Skip if index out of bounds
            if i >= tokens.len() || i >= scores.len() || i >= offsets.len() {
                continue;
            }
            
            let label = self.id2label.get(&label_idx)
                .map(|s| s.clone())
                .unwrap_or_else(|| "O".to_string());
            
            let token = &tokens[i];
            let score = scores[i];
            let (offset_start, offset_end) = offsets[i];
            let word_id = word_ids.get(i).copied().flatten();
            
            // Debug logging - always log for now
            info!(
                "Token {}: '{}' label_idx={} label='{}' score={:.3} offsets=[{},{}] word_id={:?}",
                i, token, label_idx, label, score, offset_start, offset_end, word_id
            );
            
            // Skip tokens that don't map to words (e.g., subword continuations without word_id)
            if let Some(wid) = word_id {
                if Some(wid) == last_word_id {
                    // This is a continuation of the same word - update end offset
                    if let Some((ref _entity_label, _, ref mut end, _)) = current_entity {
                        *end = offset_end;
                    }
                    continue;
                }
                last_word_id = Some(wid);
            }
            
            // Parse BIO tag
            if label.starts_with("B-") {
                // Save previous entity if exists
                if let Some((entity_label, start, end, entity_score)) = current_entity.take() {
                    if let Some(entity) = self.create_entity(&entity_label, start, end, entity_score, text) {
                        entities.push(entity);
                    }
                }
                // Start new entity
                let entity_label = label[2..].to_string();
                current_entity = Some((entity_label, offset_start, offset_end, score));
            } else if label.starts_with("I-") {
                // Continue entity if we have one with matching label
                if let Some((ref entity_label, _, ref mut end, ref mut entity_score)) = current_entity {
                    let new_label = &label[2..];
                    if new_label == entity_label {
                        *end = offset_end;
                        // Update score to minimum (conservative)
                        *entity_score = (*entity_score).min(score);
                    } else {
                        // Label mismatch - save current and start new
                        let old = current_entity.take().unwrap();
                        if let Some(entity) = self.create_entity(&old.0, old.1, old.2, old.3, text) {
                            entities.push(entity);
                        }
                        let entity_label = label[2..].to_string();
                        current_entity = Some((entity_label, offset_start, offset_end, score));
                    }
                } else {
                    // I- without B- - treat as B-
                    let entity_label = label[2..].to_string();
                    current_entity = Some((entity_label, offset_start, offset_end, score));
                }
            } else {
                // "O" label - save current entity if exists
                if let Some((entity_label, start, end, entity_score)) = current_entity.take() {
                    if let Some(entity) = self.create_entity(&entity_label, start, end, entity_score, text) {
                        entities.push(entity);
                    }
                }
            }
        }
        
        // Don't forget the last entity
        if let Some((entity_label, start, end, entity_score)) = current_entity {
            if let Some(entity) = self.create_entity(&entity_label, start, end, entity_score, text) {
                entities.push(entity);
            }
        }
        
        entities
    }
    
    fn create_entity(
        &self,
        label: &str,
        start: usize,
        end: usize,
        score: f32,
        text: &str,
    ) -> Option<NerEntity> {
        // Validate offsets
        if start >= text.len() || end > text.len() || start >= end {
            return None;
        }
        
        let entity_text = text[start..end].to_string();
        let entity_type = normalize_entity_label(label);
        
        Some(NerEntity {
            text: entity_text,
            label: label.to_string(),
            entity_type,
            start,
            end,
            score,
            normalized_id: None,
        })
    }
    
    /// Get the model ID for this model
    pub fn model_id(&self) -> &str {
        &self.config.model_id
    }
    
    /// Get the label map for this model
    pub fn labels(&self) -> HashMap<i64, String> {
        self.id2label.iter()
            .map(|(k, v)| (*k as i64, v.clone()))
            .collect()
    }
    
    /// Get the architecture type
    pub fn architecture(&self) -> ModelArch {
        self.arch
    }
}
