//! LLM backend trait and concrete implementations.
//! See ARCHITECTURE.md §8.1

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Backend unavailable: {0}")]
    Unavailable(String),
    #[error("Data classification policy blocked this request: {0}")]
    PolicyBlocked(String),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
}

// ---------------------------------------------------------------------------
// Request / Response
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,   // "system" | "user" | "assistant"
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub messages: Vec<Message>,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait LlmBackend: Send + Sync {
    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError>;
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, LlmError>;
    fn model_id(&self) -> &str;
    fn is_local(&self) -> bool;
    fn max_context_tokens(&self) -> usize;
    fn max_output_tokens(&self) -> usize;
}

// ---------------------------------------------------------------------------
// Ollama backend (local)
// ---------------------------------------------------------------------------

pub struct OllamaBackend {
    pub base_url: String,
    pub model: String,
    client: reqwest::Client,
}

impl OllamaBackend {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmBackend for OllamaBackend {
    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        // Ollama OpenAI-compatible /v1/chat/completions endpoint
        let url = format!("{}/v1/chat/completions", self.base_url);
        let body = serde_json::json!({
            "model": req.model.as_deref().unwrap_or(&self.model),
            "messages": req.messages,
            "max_tokens": req.max_tokens.unwrap_or(4096),
            "temperature": req.temperature.unwrap_or(0.1),
        });

        let resp = self.client.post(&url).json(&body).send().await?;
        let json: serde_json::Value = resp.json().await?;

        Ok(LlmResponse {
            content: json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            model: json["model"].as_str().unwrap_or(&self.model).to_string(),
            prompt_tokens: json["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: json["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
        })
    }

    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, LlmError> {
        // Ollama /v1/embeddings
        let url = format!("{}/v1/embeddings", self.base_url);
        let mut embeddings = Vec::new();
        for text in texts {
            let body = serde_json::json!({"model": &self.model, "input": text});
            let resp = self.client.post(&url).json(&body).send().await?;
            let json: serde_json::Value = resp.json().await?;
            let vec: Vec<f32> = serde_json::from_value(json["data"][0]["embedding"].clone())?;
            embeddings.push(vec);
        }
        Ok(embeddings)
    }

    fn model_id(&self) -> &str { &self.model }
    fn is_local(&self) -> bool { true }
    fn max_context_tokens(&self) -> usize { 32768 }
    fn max_output_tokens(&self) -> usize { 8192 }
}

// ---------------------------------------------------------------------------
// OpenAI-compatible backend (remote)
// ---------------------------------------------------------------------------

pub struct OpenAiBackend {
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    client: reqwest::Client,
}

impl OpenAiBackend {
    pub fn new(
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            api_key: api_key.into(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmBackend for OpenAiBackend {
    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = serde_json::json!({
            "model": req.model.as_deref().unwrap_or(&self.model),
            "messages": req.messages,
            "max_tokens": req.max_tokens.unwrap_or(4096),
            "temperature": req.temperature.unwrap_or(0.1),
        });

        let resp = self.client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let json: serde_json::Value = resp.json().await?;
        Ok(LlmResponse {
            content: json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            model: json["model"].as_str().unwrap_or(&self.model).to_string(),
            prompt_tokens: json["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: json["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
        })
    }

    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, LlmError> {
        let url = format!("{}/embeddings", self.base_url);
        let body = serde_json::json!({"model": &self.model, "input": texts});
        let resp = self.client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;
        let json: serde_json::Value = resp.json().await?;
        let embeddings: Vec<Vec<f32>> = json["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|item| serde_json::from_value(item["embedding"].clone()).unwrap_or_default())
            .collect();
        Ok(embeddings)
    }

    fn model_id(&self) -> &str { &self.model }
    fn is_local(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128000 }
    fn max_output_tokens(&self) -> usize { 16384 }
}
