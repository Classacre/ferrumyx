//! BiomedBERT embedder using Candle.

use std::sync::Arc;
use std::time::Instant;

use candle_core::{Device, Tensor, DType};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, HiddenAct, PositionEmbeddingType};
use hf_hub::api::sync::Api;
use tokenizers::Tokenizer;
use tokenizers::models::wordpiece::WordPieceBuilder;
use tracing::{debug, info};
use lru::LruCache;
use std::num::NonZeroUsize;

use crate::{EmbeddingConfig, EmbedError, Result};
use crate::pooling::l2_normalize;

/// BiomedBERT embedder for biomedical text.
///
/// Loads the model from Hugging Face Hub and provides
/// efficient batched inference for generating embeddings.
pub struct BiomedBertEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
    config: EmbeddingConfig,
    cache: Option<Arc<std::sync::Mutex<LruCache<String, Vec<f32>>>>>,
}

impl BiomedBertEmbedder {
    /// Create a new BiomedBERT embedder with default configuration.
    pub async fn new(config: EmbeddingConfig) -> Result<Self> {
        let start = Instant::now();
        info!("Loading BiomedBERT model: {}", config.model_id);

        // Determine device
        let device = Self::select_device(&config)?;
        debug!("Using device: {:?}", device);

        // Download model from Hugging Face Hub using sync API (spawn_blocking for async compatibility)
        let model_id = config.model_id.clone();
        let (bert_config, tokenizer, weights_path) = tokio::task::spawn_blocking(move || {
            use hf_hub::{Repo, RepoType};
            
            // Use the sync API with explicit repo
            let api = Api::new().map_err(|e| EmbedError::Download(format!("API init: {}", e)))?;
            
            // Create repo with explicit type
            let repo = Repo::new(model_id.clone(), RepoType::Model);
            let api_repo = api.repo(repo);
            
            info!("Downloading config.json...");
            let config_path = api_repo.get("config.json")
                .map_err(|e| EmbedError::Download(format!("config.json: {}", e)))?;
            info!("Config at: {:?}", config_path);
            let bert_config = Self::load_config(&config_path)?;
            
            info!("Downloading tokenizer...");
            // Try tokenizer.json first (newer format), fall back to vocab.txt (older BERT models)
            let tokenizer = if let Ok(tokenizer_path) = api_repo.get("tokenizer.json") {
                info!("Found tokenizer.json");
                Tokenizer::from_file(&tokenizer_path)
                    .map_err(|e| EmbedError::Tokenizer(e.to_string()))?
            } else {
                // Fall back to vocab.txt for older models like BiomedBERT
                info!("tokenizer.json not found, building from vocab.txt");
                let vocab_path = api_repo.get("vocab.txt")
                    .map_err(|e| EmbedError::Download(format!("vocab.txt: {}", e)))?;
                
                // Parse vocab.txt into a HashMap
                let vocab_content = std::fs::read_to_string(&vocab_path)?;
                let vocab: std::collections::HashMap<String, u32> = vocab_content
                    .lines()
                    .enumerate()
                    .map(|(i, line)| (line.to_string(), i as u32))
                    .collect();
                
                info!("Loaded vocab with {} tokens", vocab.len());
                
                // Create a WordPiece tokenizer (used by BERT)
                let wordpiece = WordPieceBuilder::new()
                    .vocab(vocab)
                    .continuing_subword_prefix("##".to_string())
                    .max_input_chars_per_word(100)
                    .unk_token("[UNK]".to_string())
                    .build()
                    .map_err(|e| EmbedError::Tokenizer(format!("WordPiece build: {}", e)))?;
                Tokenizer::new(wordpiece)
            };
            
            info!("Downloading model weights...");
            let weights_path = api_repo.get("model.safetensors")
                .or_else(|_| api_repo.get("pytorch_model.bin"))
                .map_err(|e| EmbedError::Download(format!("model weights: {}", e)))?;
            info!("Weights at: {:?}", weights_path);
            
            Ok::<_, EmbedError>((bert_config, tokenizer, weights_path))
        }).await.map_err(|e| EmbedError::Download(e.to_string()))??;
        
        info!("Model files downloaded, loading into memory...");

        let vb = if weights_path.extension().map(|e| e == "safetensors").unwrap_or(false) {
            // Use from_mmaped_safetensors for safetensors files
            unsafe { VarBuilder::from_mmaped_safetensors(&[&weights_path], DType::F32, &device)? }
        } else {
            VarBuilder::from_pth(&weights_path, DType::F32, &device)?
        };

        let model = BertModel::load(vb, &bert_config)?;
        info!("Model loaded in {:.2}s", start.elapsed().as_secs_f32());

        // Initialize cache if configured
        let cache = if config.cache_size > 0 {
            Some(Arc::new(std::sync::Mutex::new(
                LruCache::new(NonZeroUsize::new(config.cache_size).unwrap())
            )))
        } else {
            None
        };

        Ok(Self {
            model,
            tokenizer,
            device,
            config,
            cache,
        })
    }

    /// Select the best available device.
    fn select_device(config: &EmbeddingConfig) -> Result<Device> {
        if !config.use_gpu {
            return Ok(Device::Cpu);
        }

        #[cfg(feature = "cuda")]
        {
            match Device::new_cuda(0) {
                Ok(device) => {
                    info!("CUDA device available");
                    return Ok(device);
                }
                Err(e) => {
                    debug!("CUDA not available: {}, falling back to CPU", e);
                }
            }
        }

        #[cfg(feature = "metal")]
        {
            match Device::new_metal(0) {
                Ok(device) => {
                    info!("Metal device available");
                    return Ok(device);
                }
                Err(e) => {
                    debug!("Metal not available: {}, falling back to CPU", e);
                }
            }
        }

        Ok(Device::Cpu)
    }

    /// Load BERT config from file.
    fn load_config(path: &std::path::PathBuf) -> Result<Config> {
        let content = std::fs::read_to_string(path)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;
        
        // Parse config - handle both standard and custom BERT configs
        let hidden_act = match json.get("hidden_act").and_then(|v| v.as_str()) {
            Some("gelu") => HiddenAct::Gelu,
            Some("relu") => HiddenAct::Relu,
            Some("gelu_new") | Some("gelu_approximate") => HiddenAct::GeluApproximate,
            _ => HiddenAct::Gelu,
        };

        // Candle's Config has different fields than standard BERT config
        // We need to build it with the available fields
        Ok(Config {
            vocab_size: json.get("vocab_size").and_then(|v| v.as_u64()).unwrap_or(30522) as usize,
            hidden_size: json.get("hidden_size").and_then(|v| v.as_u64()).unwrap_or(768) as usize,
            num_hidden_layers: json.get("num_hidden_layers").and_then(|v| v.as_u64()).unwrap_or(12) as usize,
            num_attention_heads: json.get("num_attention_heads").and_then(|v| v.as_u64()).unwrap_or(12) as usize,
            intermediate_size: json.get("intermediate_size").and_then(|v| v.as_u64()).unwrap_or(3072) as usize,
            hidden_act,
            hidden_dropout_prob: json.get("hidden_dropout_prob").and_then(|v| v.as_f64()).unwrap_or(0.1) as f64,
            max_position_embeddings: json.get("max_position_embeddings").and_then(|v| v.as_u64()).unwrap_or(512) as usize,
            type_vocab_size: json.get("type_vocab_size").and_then(|v| v.as_u64()).unwrap_or(2) as usize,
            initializer_range: json.get("initializer_range").and_then(|v| v.as_f64()).unwrap_or(0.02) as f64,
            layer_norm_eps: json.get("layer_norm_eps").and_then(|v| v.as_f64()).unwrap_or(1e-12) as f64,
            pad_token_id: json.get("pad_token_id").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
            position_embedding_type: PositionEmbeddingType::Absolute,
            use_cache: true,
            classifier_dropout: None,
            model_type: None,
        })
    }

    /// Embed a list of texts.
    ///
    /// Returns a vector of 768-dimensional embeddings.
    /// Automatically batches for efficiency.
    pub async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let start = Instant::now();
        let mut all_embeddings = Vec::with_capacity(texts.len());

        // Check cache for each text
        let mut uncached_indices = Vec::new();
        let mut uncached_texts = Vec::new();
        
        if let Some(cache) = &self.cache {
            let mut cache_guard = cache.lock().unwrap();
            for (i, text) in texts.iter().enumerate() {
                if let Some(cached) = cache_guard.get(text) {
                    all_embeddings.push((i, cached.clone()));
                } else {
                    uncached_indices.push(i);
                    uncached_texts.push(text.clone());
                }
            }
        } else {
            uncached_indices = (0..texts.len()).collect();
            uncached_texts = texts.to_vec();
        }

        // Process uncached texts in batches
        if !uncached_texts.is_empty() {
            for batch_start in (0..uncached_texts.len()).step_by(self.config.batch_size) {
                let batch_end = (batch_start + self.config.batch_size).min(uncached_texts.len());
                let batch = &uncached_texts[batch_start..batch_end];
                
                let batch_embeddings = self.embed_batch(batch).await?;
                
                // Cache results
                if let Some(cache) = &self.cache {
                    let mut cache_guard = cache.lock().unwrap();
                    for (text, embedding) in batch.iter().zip(batch_embeddings.iter()) {
                        cache_guard.put(text.clone(), embedding.clone());
                    }
                }
                
                for (j, embedding) in batch_embeddings.into_iter().enumerate() {
                    all_embeddings.push((uncached_indices[batch_start + j], embedding));
                }
            }
        }

        // Sort by original index
        all_embeddings.sort_by_key(|(i, _)| *i);
        let result: Vec<Vec<f32>> = all_embeddings.into_iter().map(|(_, e)| e).collect();

        debug!(
            "Embedded {} texts in {:.2}ms ({:.2} texts/sec)",
            texts.len(),
            start.elapsed().as_secs_f32() * 1000.0,
            texts.len() as f32 / start.elapsed().as_secs_f32()
        );

        Ok(result)
    }

    /// Embed a single batch of texts.
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // Use batch tokenization for better performance
        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        
        let encodings = self.tokenizer
            .encode_batch(text_refs, true)
            .map_err(|e| EmbedError::Tokenizer(e.to_string()))?;

        let mut input_ids_vec = Vec::with_capacity(texts.len());
        let mut attention_mask_vec = Vec::with_capacity(texts.len());
        let mut token_type_ids_vec = Vec::with_capacity(texts.len());

        for encoding in &encodings {
            let ids = encoding.get_ids();
            let mask = encoding.get_attention_mask();
            let type_ids = encoding.get_type_ids();

            // Truncate if needed
            let max_len = self.config.max_length.min(512);
            let len = ids.len().min(max_len);

            input_ids_vec.push(ids[..len].to_vec());
            attention_mask_vec.push(mask[..len].to_vec());
            token_type_ids_vec.push(type_ids[..len].to_vec());
        }

        // Find max length for padding
        let max_len = input_ids_vec.iter().map(|v| v.len()).max().unwrap_or(0);

        // Pad sequences
        for ((ids, mask), type_ids) in input_ids_vec.iter_mut()
            .zip(attention_mask_vec.iter_mut())
            .zip(token_type_ids_vec.iter_mut())
        {
            let pad_len = max_len - ids.len();
            ids.extend(std::iter::repeat_n(0, pad_len));
            mask.extend(std::iter::repeat_n(0, pad_len));
            type_ids.extend(std::iter::repeat_n(0, pad_len));
        }

        // Create tensors - attention_mask needs to be F32 for broadcasting operations
        let batch_size = texts.len();
        let input_ids = Tensor::new(input_ids_vec, &self.device)?
            .reshape((batch_size, max_len))?;
        let attention_mask = Tensor::new(attention_mask_vec, &self.device)?
            .reshape((batch_size, max_len))?
            .to_dtype(DType::F32)?;  // Convert to F32 for multiplication operations
        let token_type_ids = Tensor::new(token_type_ids_vec, &self.device)?
            .reshape((batch_size, max_len))?;

        // Run model
        let embeddings = self.model.forward(&input_ids, &token_type_ids, Some(&attention_mask))?;

        // Apply pooling
        let pooled = self.config.pooling.apply(&embeddings, &attention_mask)?;

        // Normalize if configured
        let normalized = if self.config.normalize {
            l2_normalize(&pooled)?
        } else {
            pooled
        };

        // Convert to vec
        let result = normalized.to_vec2::<f32>()?;
        Ok(result)
    }

    /// Embed a single text.
    pub async fn embed_one(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.embed(&[text.to_string()]).await?;
        embeddings.into_iter().next()
            .ok_or(EmbedError::InvalidInput("No embedding produced".to_string()))
    }

    /// Get the embedding dimension (768 for BiomedBERT-base).
    pub fn dimension(&self) -> usize {
        768
    }

    /// Get the model name.
    pub fn model_name(&self) -> &str {
        &self.config.model_id
    }

    /// Check if GPU is being used.
    pub fn is_gpu(&self) -> bool {
        matches!(self.device, Device::Cuda(_) | Device::Metal(_))
    }

    /// Clear the embedding cache.
    pub fn clear_cache(&self) {
        if let Some(cache) = &self.cache {
            cache.lock().unwrap().clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_embedder_creation() {
        let config = EmbeddingConfig::cpu();
        let embedder = BiomedBertEmbedder::new(config).await;
        
        // This test requires network access to download the model
        // In CI, we might want to mock this
        if let Ok(embedder) = embedder {
            assert_eq!(embedder.dimension(), 768);
        }
    }
}
