//! LLM router — selects backend based on data classification policy.
//! See ARCHITECTURE.md §1.5 and §8.3

use std::collections::HashMap;
use std::sync::Arc;
use crate::backend::{LlmBackend, LlmError, LlmRequest, LlmResponse};
use crate::classification::{DataClass, DataClassifier};

/// Routing policy controlling which backends are allowed for each data class.
#[derive(Debug, Clone)]
pub struct RoutingPolicy {
    /// If true, INTERNAL data may be sent to remote backends (with audit log).
    pub allow_internal_remote: bool,
    /// If true, all calls are forced to local backends regardless of data class.
    pub local_only_mode: bool,
    /// Preferred backend name for PUBLIC data.
    pub default_backend: String,
    /// Fallback local backend name (used when remote is blocked).
    pub local_backend: String,
}

impl Default for RoutingPolicy {
    fn default() -> Self {
        Self {
            allow_internal_remote: false,
            local_only_mode: false,
            default_backend: "ollama".to_string(),
            local_backend: "ollama".to_string(),
        }
    }
}

/// Routes LLM requests to appropriate backends based on data classification.
pub struct LlmRouter {
    backends: HashMap<String, Arc<dyn LlmBackend>>,
    policy: RoutingPolicy,
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

    /// Route a request: classify data, select backend, execute.
    pub async fn route(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        // Classify prompt content
        let prompt_text = req.messages.iter()
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let data_class = self.classifier.classify(&prompt_text);

        // Enforce policy
        let backend = self.select_backend(&data_class)?;

        tracing::info!(
            model = backend.model_id(),
            data_class = data_class.as_str(),
            is_local = backend.is_local(),
            "LLM request routed"
        );

        backend.complete(req).await
    }

    fn select_backend(&self, class: &DataClass) -> Result<&Arc<dyn LlmBackend>, LlmError> {
        match class {
            DataClass::Confidential => {
                // HARD BLOCK: confidential data never leaves local
                let b = self.backends.get(&self.policy.local_backend)
                    .ok_or_else(|| LlmError::Unavailable(
                        "Local backend not available for CONFIDENTIAL data".to_string()
                    ))?;
                if !b.is_local() {
                    return Err(LlmError::PolicyBlocked(
                        "CONFIDENTIAL data cannot be sent to remote LLM".to_string()
                    ));
                }
                Ok(b)
            }

            DataClass::Internal => {
                if self.policy.local_only_mode || !self.policy.allow_internal_remote {
                    // Route to local
                    self.backends.get(&self.policy.local_backend)
                        .ok_or_else(|| LlmError::Unavailable(
                            "Local backend not configured".to_string()
                        ))
                } else {
                    // Allow remote with audit (caller handles audit logging)
                    tracing::warn!("Routing INTERNAL data to remote backend — ensure audit log is active");
                    self.backends.get(&self.policy.default_backend)
                        .ok_or_else(|| LlmError::Unavailable(
                            "Default backend not configured".to_string()
                        ))
                }
            }

            DataClass::Public => {
                if self.policy.local_only_mode {
                    self.backends.get(&self.policy.local_backend)
                        .ok_or_else(|| LlmError::Unavailable(
                            "Local backend not configured".to_string()
                        ))
                } else {
                    self.backends.get(&self.policy.default_backend)
                        .ok_or_else(|| LlmError::Unavailable(
                            "Default backend not configured".to_string()
                        ))
                }
            }
        }
    }
}
