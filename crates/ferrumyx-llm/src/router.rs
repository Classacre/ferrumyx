//! LLM router — selects backend based on data classification policy.
//! See ARCHITECTURE.md §1.5 and §8.3

use std::collections::HashMap;
use std::sync::Arc;
use crate::backend::{
    LlmBackend, LlmError, LlmRequest, LlmResponse,
    OllamaBackend, OpenAiBackend, OpenAiCompatibleBackend,
    AnthropicBackend, GeminiBackend,
};
use crate::classification::{DataClass, DataClassifier};

// ── Routing policy ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RoutingPolicy {
    /// If true, INTERNAL data may be sent to remote backends (with audit log).
    pub allow_internal_remote: bool,
    /// Force all calls to local backends regardless of data class.
    pub local_only_mode: bool,
    /// Preferred backend name for PUBLIC data (e.g. "openai", "anthropic", "gemini").
    pub default_backend: String,
    /// Local backend name used for CONFIDENTIAL + INTERNAL data.
    pub local_backend: String,
}

impl Default for RoutingPolicy {
    fn default() -> Self {
        Self {
            allow_internal_remote: false,
            local_only_mode: false,
            default_backend: "openai".to_string(),
            local_backend:   "ollama".to_string(),
        }
    }
}

// ── Router ────────────────────────────────────────────────────────────────────

pub struct LlmRouter {
    backends: HashMap<String, Arc<dyn LlmBackend>>,
    policy:   RoutingPolicy,
    classifier: DataClassifier,
}

impl LlmRouter {
    pub fn new(policy: RoutingPolicy) -> Self {
        Self {
            backends: HashMap::new(),
            policy,
            classifier: DataClassifier::default(),
        }
    }

    pub fn register_backend(&mut self, name: impl Into<String>, backend: Arc<dyn LlmBackend>) {
        self.backends.insert(name.into(), backend);
    }

    /// Route a request: classify data → select backend → execute → return response.
    pub async fn route(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        let prompt_text = req.messages.iter()
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let data_class = self.classifier.classify(&prompt_text);
        let backend = self.select_backend(&data_class)?;

        tracing::info!(
            model    = backend.model_id(),
            class    = data_class.as_str(),
            is_local = backend.is_local(),
            "LLM request routed"
        );

        backend.complete(req).await
    }

    /// Embed texts using the default or local backend.
    pub async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, LlmError> {
        // Use a dedicated embedding backend if registered, else default
        let name = if self.backends.contains_key("embed") { "embed" }
                   else { &self.policy.default_backend };
        let backend = self.backends.get(name)
            .ok_or_else(|| LlmError::Unavailable(
                format!("Embedding backend '{name}' not registered")
            ))?;
        backend.embed(texts).await
    }

    fn select_backend(&self, class: &DataClass) -> Result<&Arc<dyn LlmBackend>, LlmError> {
        match class {
            DataClass::Confidential => {
                let b = self.backends.get(&self.policy.local_backend)
                    .ok_or_else(|| LlmError::Unavailable(
                        "Local backend not configured for CONFIDENTIAL data".to_string()
                    ))?;
                if !b.is_local() {
                    return Err(LlmError::PolicyBlocked(
                        "CONFIDENTIAL data cannot be sent to a remote LLM".to_string()
                    ));
                }
                Ok(b)
            }
            DataClass::Internal => {
                if self.policy.local_only_mode || !self.policy.allow_internal_remote {
                    self.backends.get(&self.policy.local_backend)
                        .ok_or_else(|| LlmError::Unavailable(
                            "Local backend not configured".to_string()
                        ))
                } else {
                    tracing::warn!("Routing INTERNAL data to remote backend — audit log active");
                    self.backends.get(&self.policy.default_backend)
                        .ok_or_else(|| LlmError::Unavailable(
                            "Default backend not configured".to_string()
                        ))
                }
            }
            DataClass::Public => {
                if self.policy.local_only_mode {
                    self.backends.get(&self.policy.local_backend)
                } else {
                    self.backends.get(&self.policy.default_backend)
                }
                .ok_or_else(|| LlmError::Unavailable(
                    "No backend configured".to_string()
                ))
            }
        }
    }

    /// List all registered backends.
    pub fn registered_backends(&self) -> Vec<(&str, bool)> {
        self.backends.iter()
            .map(|(name, b)| (name.as_str(), b.is_local()))
            .collect()
    }
}

// ── Factory: build router from config values ──────────────────────────────────

/// Backend configuration provided at startup.
#[derive(Debug, Clone)]
pub struct BackendConfig {
    pub name:    String,
    pub kind:    BackendKind,
    pub model:   String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub embedding_model: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BackendKind {
    Ollama,
    OpenAi,
    OpenAiCompatible,
    Anthropic,
    Gemini,
}

/// Build a fully configured LlmRouter from a list of BackendConfigs.
pub fn build_router(configs: Vec<BackendConfig>, policy: RoutingPolicy) -> LlmRouter {
    let mut router = LlmRouter::new(policy);

    for cfg in configs {
        let backend: Arc<dyn LlmBackend> = match cfg.kind {
            BackendKind::Ollama => {
                let url = cfg.base_url.as_deref().unwrap_or("http://localhost:11434");
                Arc::new(OllamaBackend::new(url, &cfg.model))
            }
            BackendKind::OpenAi => {
                let key = cfg.api_key.clone().unwrap_or_default();
                let mut b = OpenAiBackend::new(key, &cfg.model);
                if let Some(emb) = &cfg.embedding_model {
                    b = b.with_embedding_model(emb);
                }
                Arc::new(b)
            }
            BackendKind::OpenAiCompatible => {
                let url = cfg.base_url.as_deref().unwrap_or("http://localhost:11434");
                let mut b = OpenAiCompatibleBackend::new(url, &cfg.model, cfg.api_key.clone());
                if let Some(emb) = &cfg.embedding_model {
                    b = b.with_embedding_model(emb);
                }
                Arc::new(b)
            }
            BackendKind::Anthropic => {
                let key = cfg.api_key.clone().unwrap_or_default();
                Arc::new(AnthropicBackend::new(key, &cfg.model))
            }
            BackendKind::Gemini => {
                let key = cfg.api_key.clone().unwrap_or_default();
                let mut b = GeminiBackend::new(key, &cfg.model);
                if let Some(emb) = &cfg.embedding_model {
                    b = b.with_embedding_model(emb);
                }
                Arc::new(b)
            }
        };

        tracing::info!(
            name  = %cfg.name,
            kind  = ?cfg.kind,
            model = %cfg.model,
            "LLM backend registered"
        );
        router.register_backend(&cfg.name, backend);
    }

    router
}
