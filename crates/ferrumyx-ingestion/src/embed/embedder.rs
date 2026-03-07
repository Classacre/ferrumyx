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

use crate::embed::{EmbeddingConfig, EmbedError, Result};
use crate::embed::pooling::l2_normalize;

pub struct BiomedBertEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
    config: EmbeddingConfig,
    cache: Option<Arc<std::sync::Mutex<LruCache<String, Vec<f32>>>>>,
}

impl BiomedBertEmbedder {
    pub async fn new(config: EmbeddingConfig) -> Result<Self> {
        let start = Instant::now();
        info!("Loading BiomedBERT model: {}", config.model_id);
        let device = Self::select_device(&config)?;
        
        let model_id = config.model_id.clone();
        let (bert_config, tokenizer, weights_path) = tokio::task::spawn_blocking(move || {
            let api = Api::new().map_err(|e| EmbedError::Download(format!("API init: {}", e)))?;
            let repo = hf_hub::Repo::new(model_id.clone(), hf_hub::RepoType::Model);
            let api_repo = api.repo(repo);
            
            let config_path = api_repo.get("config.json")?;
            let bert_config = Self::load_config(&config_path)?;
            
            let tokenizer = if let Ok(tokenizer_path) = api_repo.get("tokenizer.json") {
                Tokenizer::from_file(&tokenizer_path).map_err(|e| EmbedError::Tokenizer(e.to_string()))?
            } else {
                let vocab_path = api_repo.get("vocab.txt")?;
                let wordpiece = tokenizers::models::wordpiece::WordPiece::from_file(vocab_path.to_str().unwrap())
                    .unk_token("[UNK]".to_string())
                    .build()
                    .map_err(|e| EmbedError::Tokenizer(e.to_string()))?;
                Tokenizer::new(wordpiece)
            };
            
            let weights_path = api_repo.get("model.safetensors").or_else(|_| api_repo.get("pytorch_model.bin"))?;
            Ok::<_, EmbedError>((bert_config, tokenizer, weights_path))
        }).await.map_err(|e| EmbedError::Download(e.to_string()))??;

        let vb = if weights_path.extension().map(|e| e == "safetensors").unwrap_or(false) {
            unsafe { VarBuilder::from_mmaped_safetensors(&[&weights_path], DType::F32, &device)? }
        } else {
            VarBuilder::from_pth(&weights_path, DType::F32, &device)?
        };

        let model = BertModel::load(vb, &bert_config)?;
        info!("Model loaded in {:.2}s", start.elapsed().as_secs_f32());

        let cache = if config.cache_size > 0 {
            Some(Arc::new(std::sync::Mutex::new(LruCache::new(NonZeroUsize::new(config.cache_size).unwrap()))))
        } else { None };

        Ok(Self { model, tokenizer, device, config, cache })
    }

    fn select_device(config: &EmbeddingConfig) -> Result<Device> {
        if !config.use_gpu { return Ok(Device::Cpu); }
        #[cfg(feature = "cuda")] { if let Ok(d) = Device::new_cuda(0) { return Ok(d); } }
        #[cfg(feature = "metal")] { if let Ok(d) = Device::new_metal(0) { return Ok(d); } }
        Ok(Device::Cpu)
    }

    fn load_config(path: &std::path::PathBuf) -> Result<Config> {
        let content = std::fs::read_to_string(path)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;
        let hidden_act = match json.get("hidden_act").and_then(|v| v.as_str()) {
            Some("relu") => HiddenAct::Relu,
            Some("gelu_new") => HiddenAct::GeluApproximate,
            _ => HiddenAct::Gelu,
        };
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

    pub async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() { return Ok(Vec::new()); }
        let mut all_embeddings = Vec::with_capacity(texts.len());
        let mut uncached_indices = Vec::new();
        let mut uncached_texts = Vec::new();
        
        if let Some(cache) = &self.cache {
            let mut cache_guard = cache.lock().unwrap();
            for (i, text) in texts.iter().enumerate() {
                if let Some(cached) = cache_guard.get(text) { all_embeddings.push((i, cached.clone())); }
                else { uncached_indices.push(i); uncached_texts.push(text.clone()); }
            }
        } else {
            uncached_indices = (0..texts.len()).collect();
            uncached_texts = texts.to_vec();
        }

        if !uncached_texts.is_empty() {
            for batch_start in (0..uncached_texts.len()).step_by(self.config.batch_size) {
                let batch_end = (batch_start + self.config.batch_size).min(uncached_texts.len());
                let batch = &uncached_texts[batch_start..batch_end];
                let batch_embeddings = self.embed_batch(batch).await?;
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
        all_embeddings.sort_by_key(|(i, _)| *i);
        Ok(all_embeddings.into_iter().map(|(_, e)| e).collect())
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let encodings = self.tokenizer.encode_batch(text_refs, true).map_err(|e| EmbedError::Tokenizer(e.to_string()))?;
        let mut input_ids_vec = Vec::new();
        let mut attention_mask_vec = Vec::new();
        let mut token_type_ids_vec = Vec::new();

        for encoding in &encodings {
            let ids = encoding.get_ids();
            let mask = encoding.get_attention_mask();
            let type_ids = encoding.get_type_ids();
            let len = ids.len().min(self.config.max_length.min(512));
            input_ids_vec.push(ids[..len].to_vec());
            attention_mask_vec.push(mask[..len].to_vec());
            token_type_ids_vec.push(type_ids[..len].to_vec());
        }

        let max_len = input_ids_vec.iter().map(|v| v.len()).max().unwrap_or(0);
        for ((ids, mask), type_ids) in input_ids_vec.iter_mut().zip(attention_mask_vec.iter_mut()).zip(token_type_ids_vec.iter_mut()) {
            let pad_len = max_len - ids.len();
            ids.extend(std::iter::repeat(0).take(pad_len));
            mask.extend(std::iter::repeat(0).take(pad_len));
            type_ids.extend(std::iter::repeat(0).take(pad_len));
        }

        let batch_size = texts.len();
        let input_ids = Tensor::new(input_ids_vec, &self.device)?.reshape((batch_size, max_len))?;
        let attention_mask = Tensor::new(attention_mask_vec, &self.device)?.reshape((batch_size, max_len))?.to_dtype(DType::F32)?;
        let token_type_ids = Tensor::new(token_type_ids_vec, &self.device)?.reshape((batch_size, max_len))?;

        let embeddings = self.model.forward(&input_ids, &token_type_ids, Some(&attention_mask))?;
        let pooled = self.config.pooling.apply(&embeddings, &attention_mask)?;
        let normalized = if self.config.normalize { l2_normalize(&pooled)? } else { pooled };
        Ok(normalized.to_vec2::<f32>()?)
    }
}
