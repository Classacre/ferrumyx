//! LLM backend trait and concrete implementations.
//! See ARCHITECTURE.md §8.1
//!
//! Backends:
//!   OllamaBackend          — local Ollama (OpenAI-compatible)
//!   OpenAiBackend          — OpenAI API (gpt-4o, gpt-4o-mini, o1, …)
//!   OpenAiCompatibleBackend — any OpenAI-compatible endpoint (LMStudio,
//!                             TogetherAI, Groq, OpenRouter, vLLM, …)
//!   AnthropicBackend       — Anthropic Messages API (claude-*)
//!   GeminiBackend          — Google Gemini API (gemini-1.5-pro, flash, …)
//!
//! Embedding backends (separate trait impl):
//!   OllamaBackend          — /api/embeddings
//!   OpenAiBackend          — text-embedding-3-small / text-embedding-3-large
//!   OpenAiCompatibleBackend — any /v1/embeddings endpoint

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── Error ─────────────────────────────────────────────────────────────────────

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
    #[error("API error [{status}]: {message}")]
    ApiError { status: u16, message: String },
}

// ── Request / Response ────────────────────────────────────────────────────────

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

// ── Trait ─────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait LlmBackend: Send + Sync {
    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError>;
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, LlmError>;
    fn model_id(&self) -> &str;
    fn is_local(&self) -> bool;
    fn max_context_tokens(&self) -> usize;
    fn max_output_tokens(&self) -> usize;
}

// ── Helper: parse OpenAI-style response ──────────────────────────────────────

fn parse_openai_response(json: &serde_json::Value, fallback_model: &str) -> LlmResponse {
    LlmResponse {
        content: json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        model: json["model"]
            .as_str()
            .unwrap_or(fallback_model)
            .to_string(),
        prompt_tokens:     json["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
        completion_tokens: json["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
    }
}

async fn check_response_status(resp: reqwest::Response) -> Result<serde_json::Value, LlmError> {
    let status = resp.status().as_u16();
    let body: serde_json::Value = resp.json().await?;
    if status >= 400 {
        let msg = body["error"]["message"]
            .as_str()
            .or_else(|| body["message"].as_str())
            .unwrap_or("unknown API error")
            .to_string();
        return Err(LlmError::ApiError { status, message: msg });
    }
    Ok(body)
}

// ── 1. Ollama (local) ─────────────────────────────────────────────────────────

pub struct OllamaBackend {
    pub base_url: String,
    pub model: String,
    client: reqwest::Client,
}

impl OllamaBackend {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self { base_url: base_url.into(), model: model.into(), client: reqwest::Client::new() }
    }
}

#[async_trait]
impl LlmBackend for OllamaBackend {
    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let body = serde_json::json!({
            "model":       req.model.as_deref().unwrap_or(&self.model),
            "messages":    req.messages,
            "max_tokens":  req.max_tokens.unwrap_or(4096),
            "temperature": req.temperature.unwrap_or(0.1),
        });
        let resp = self.client.post(&url).json(&body).send().await?;
        let json = check_response_status(resp).await?;
        Ok(parse_openai_response(&json, &self.model))
    }

    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, LlmError> {
        let url = format!("{}/api/embeddings", self.base_url);
        let mut out = Vec::new();
        for text in texts {
            let body = serde_json::json!({"model": &self.model, "prompt": text});
            let resp = self.client.post(&url).json(&body).send().await?;
            let json = check_response_status(resp).await?;
            let vec: Vec<f32> = serde_json::from_value(json["embedding"].clone())?;
            out.push(vec);
        }
        Ok(out)
    }

    fn model_id(&self) -> &str { &self.model }
    fn is_local(&self) -> bool { true }
    fn max_context_tokens(&self) -> usize { 32768 }
    fn max_output_tokens(&self) -> usize { 8192 }
}

// ── 2. OpenAI ─────────────────────────────────────────────────────────────────

pub struct OpenAiBackend {
    pub model: String,
    pub embedding_model: String,
    api_key: String,
    client: reqwest::Client,
}

impl OpenAiBackend {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            embedding_model: "text-embedding-3-small".to_string(),
            api_key: api_key.into(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        self.embedding_model = model.into();
        self
    }
}

#[async_trait]
impl LlmBackend for OpenAiBackend {
    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        let body = serde_json::json!({
            "model":       req.model.as_deref().unwrap_or(&self.model),
            "messages":    req.messages,
            "max_tokens":  req.max_tokens.unwrap_or(4096),
            "temperature": req.temperature.unwrap_or(0.1),
        });
        let resp = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;
        let json = check_response_status(resp).await?;
        Ok(parse_openai_response(&json, &self.model))
    }

    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, LlmError> {
        let body = serde_json::json!({
            "model": &self.embedding_model,
            "input": texts,
        });
        let resp = self.client
            .post("https://api.openai.com/v1/embeddings")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;
        let json = check_response_status(resp).await?;
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
    fn max_context_tokens(&self) -> usize { 128_000 }
    fn max_output_tokens(&self) -> usize { 16_384 }
}

// ── 3. OpenAI-Compatible (LMStudio, TogetherAI, Groq, OpenRouter, vLLM, …) ──

pub struct OpenAiCompatibleBackend {
    pub base_url: String,
    pub model: String,
    pub embedding_model: Option<String>,
    api_key: Option<String>,
    client: reqwest::Client,
}

impl OpenAiCompatibleBackend {
    pub fn new(
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key: Option<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            embedding_model: None,
            api_key,
            client: reqwest::Client::new(),
        }
    }

    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        self.embedding_model = Some(model.into());
        self
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.api_key {
            Some(k) => req.bearer_auth(k),
            None    => req,
        }
    }
}

#[async_trait]
impl LlmBackend for OpenAiCompatibleBackend {
    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        let url = format!("{}/v1/chat/completions", self.base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "model":       req.model.as_deref().unwrap_or(&self.model),
            "messages":    req.messages,
            "max_tokens":  req.max_tokens.unwrap_or(4096),
            "temperature": req.temperature.unwrap_or(0.1),
        });
        let resp = self.auth(self.client.post(&url)).json(&body).send().await?;
        let json = check_response_status(resp).await?;
        Ok(parse_openai_response(&json, &self.model))
    }

    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, LlmError> {
        let emb_model = self.embedding_model.as_deref().unwrap_or(&self.model);
        let url = format!("{}/v1/embeddings", self.base_url.trim_end_matches('/'));
        let body = serde_json::json!({"model": emb_model, "input": texts});
        let resp = self.auth(self.client.post(&url)).json(&body).send().await?;
        let json = check_response_status(resp).await?;
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
    fn max_context_tokens(&self) -> usize { 128_000 }
    fn max_output_tokens(&self) -> usize { 8_192 }
}

// ── 4. Anthropic (claude-*) ───────────────────────────────────────────────────

pub struct AnthropicBackend {
    pub model: String,
    api_key: String,
    client: reqwest::Client,
}

impl AnthropicBackend {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self { api_key: api_key.into(), model: model.into(), client: reqwest::Client::new() }
    }
}

#[async_trait]
impl LlmBackend for AnthropicBackend {
    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        // Anthropic Messages API — split system prompt from user messages
        let system = req.messages.iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.as_str())
            .unwrap_or("");

        let messages: Vec<serde_json::Value> = req.messages.iter()
            .filter(|m| m.role != "system")
            .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
            .collect();

        let model = req.model.as_deref().unwrap_or(&self.model);
        let max_tokens = req.max_tokens.unwrap_or(4096);

        let mut body = serde_json::json!({
            "model":      model,
            "messages":   messages,
            "max_tokens": max_tokens,
        });
        if !system.is_empty() {
            body["system"] = serde_json::Value::String(system.to_string());
        }

        let resp = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let json = check_response_status(resp).await?;

        let content = json["content"]
            .as_array()
            .and_then(|blocks| blocks.first())
            .and_then(|b| b["text"].as_str())
            .unwrap_or("")
            .to_string();

        Ok(LlmResponse {
            content,
            model: json["model"].as_str().unwrap_or(model).to_string(),
            prompt_tokens:     json["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: json["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
        })
    }

    /// Anthropic does not offer an embeddings API; raise an error.
    async fn embed(&self, _texts: Vec<String>) -> Result<Vec<Vec<f32>>, LlmError> {
        Err(LlmError::Unavailable(
            "Anthropic does not provide an embeddings API. \
             Use OpenAI text-embedding-3-* or a local model for embeddings.".to_string()
        ))
    }

    fn model_id(&self) -> &str { &self.model }
    fn is_local(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 200_000 }
    fn max_output_tokens(&self) -> usize { 8_192 }
}

// ── 5. Google Gemini ──────────────────────────────────────────────────────────

pub struct GeminiBackend {
    pub model: String,
    pub embedding_model: String,
    api_key: String,
    client: reqwest::Client,
}

impl GeminiBackend {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            embedding_model: "text-embedding-004".to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        self.embedding_model = model.into();
        self
    }
}

#[async_trait]
impl LlmBackend for GeminiBackend {
    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        let model = req.model.as_deref().unwrap_or(&self.model);
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, self.api_key
        );

        // Convert messages to Gemini `contents` format
        // System message → systemInstruction
        let system_text = req.messages.iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.clone());

        let contents: Vec<serde_json::Value> = req.messages.iter()
            .filter(|m| m.role != "system")
            .map(|m| {
                let role = if m.role == "assistant" { "model" } else { "user" };
                serde_json::json!({
                    "role": role,
                    "parts": [{ "text": m.content }]
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "contents": contents,
            "generationConfig": {
                "maxOutputTokens": req.max_tokens.unwrap_or(4096),
                "temperature":     req.temperature.unwrap_or(0.1),
            }
        });
        if let Some(sys) = system_text {
            body["systemInstruction"] = serde_json::json!({
                "parts": [{ "text": sys }]
            });
        }

        let resp = self.client.post(&url).json(&body).send().await?;
        let json = check_response_status(resp).await?;

        let content = json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let prompt_tokens = json["usageMetadata"]["promptTokenCount"]
            .as_u64().unwrap_or(0) as u32;
        let completion_tokens = json["usageMetadata"]["candidatesTokenCount"]
            .as_u64().unwrap_or(0) as u32;

        Ok(LlmResponse {
            content,
            model: model.to_string(),
            prompt_tokens,
            completion_tokens,
        })
    }

    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, LlmError> {
        // Gemini batch embed endpoint
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:batchEmbedContents?key={}",
            self.embedding_model, self.api_key
        );
        let requests: Vec<serde_json::Value> = texts.iter().map(|t| serde_json::json!({
            "model": format!("models/{}", self.embedding_model),
            "content": { "parts": [{ "text": t }] }
        })).collect();

        let body = serde_json::json!({ "requests": requests });
        let resp = self.client.post(&url).json(&body).send().await?;
        let json = check_response_status(resp).await?;

        let embeddings: Vec<Vec<f32>> = json["embeddings"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|e| {
                e["values"].as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                    .collect()
            })
            .collect();
        Ok(embeddings)
    }

    fn model_id(&self) -> &str { &self.model }
    fn is_local(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 1_000_000 }  // Gemini 1.5 Pro
    fn max_output_tokens(&self) -> usize { 8_192 }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_backend_is_not_local() {
        let b = OpenAiBackend::new("sk-test", "gpt-4o");
        assert!(!b.is_local());
        assert_eq!(b.model_id(), "gpt-4o");
    }

    #[test]
    fn test_anthropic_backend_is_not_local() {
        let b = AnthropicBackend::new("sk-ant-test", "claude-sonnet-4-5");
        assert!(!b.is_local());
        assert_eq!(b.model_id(), "claude-sonnet-4-5");
    }

    #[test]
    fn test_gemini_backend_is_not_local() {
        let b = GeminiBackend::new("AIza-test", "gemini-1.5-pro");
        assert!(!b.is_local());
        assert_eq!(b.model_id(), "gemini-1.5-pro");
        assert_eq!(b.max_context_tokens(), 1_000_000);
    }

    #[test]
    fn test_openai_compatible_with_no_key() {
        let b = OpenAiCompatibleBackend::new("http://localhost:1234", "local-model", None);
        // No API key is valid for LMStudio / vLLM
        assert_eq!(b.model_id(), "local-model");
    }

    #[test]
    fn test_ollama_is_local() {
        let b = OllamaBackend::new("http://localhost:11434", "llama3:8b");
        assert!(b.is_local());
    }

    #[test]
    fn test_openai_embedding_model_override() {
        let b = OpenAiBackend::new("sk-test", "gpt-4o")
            .with_embedding_model("text-embedding-3-large");
        assert_eq!(b.embedding_model, "text-embedding-3-large");
    }
}
