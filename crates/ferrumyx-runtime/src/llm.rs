//! Ferrumyx-owned LLM contract with a runtime-core bridge.
//!
//! This module is the first migration step away from direct runtime-core coupling:
//! - Ferrumyx code depends on `ferrumyx_runtime::llm::LlmProvider`
//! - Runtime bridges providers into the runtime core where needed.

use std::sync::Arc;

use async_trait::async_trait;
use rig::completion::CompletionModel;
use rust_decimal::Decimal;
use ferrumyx_runtime_core::llm::LlmProvider as CoreLlmProvider;

pub use ferrumyx_runtime_core::error::LlmError;
pub use ferrumyx_runtime_core::llm::{
    ChatMessage, CompletionRequest, CompletionResponse, ContentPart, CooldownConfig, FinishReason,
    ImageUrl, ModelMetadata, Role, ToolCall, ToolCompletionRequest, ToolCompletionResponse,
    ToolDefinition, ToolResult,
};

/// Ferrumyx-owned LLM provider contract.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn model_name(&self) -> &str;
    fn cost_per_token(&self) -> (Decimal, Decimal);
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError>;
    async fn complete_with_tools(
        &self,
        request: ToolCompletionRequest,
    ) -> Result<ToolCompletionResponse, LlmError>;

    async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(Vec::new())
    }

    async fn model_metadata(&self) -> Result<ModelMetadata, LlmError> {
        Ok(ModelMetadata {
            id: self.model_name().to_string(),
            context_length: None,
        })
    }

    fn effective_model_name(&self, requested_model: Option<&str>) -> String {
        requested_model
            .map(std::borrow::ToOwned::to_owned)
            .unwrap_or_else(|| self.active_model_name())
    }

    fn active_model_name(&self) -> String {
        self.model_name().to_string()
    }

    fn set_model(&self, _model: &str) -> Result<(), LlmError> {
        Err(LlmError::RequestFailed {
            provider: "unknown".to_string(),
            reason: "Runtime model switching not supported by this provider".to_string(),
        })
    }

    fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> Decimal {
        let (input_cost, output_cost) = self.cost_per_token();
        input_cost * Decimal::from(input_tokens) + output_cost * Decimal::from(output_tokens)
    }

    fn cache_write_multiplier(&self) -> Decimal {
        Decimal::ONE
    }

    fn cache_read_discount(&self) -> Decimal {
        Decimal::ONE
    }
}

/// Rig-backed LLM adapter implementing the Ferrumyx contract.
pub struct RigAdapter<M: CompletionModel> {
    inner: ferrumyx_runtime_core::llm::RigAdapter<M>,
}

impl<M: CompletionModel> RigAdapter<M> {
    pub fn new(model: M, model_name: impl Into<String>) -> Self {
        Self {
            inner: ferrumyx_runtime_core::llm::RigAdapter::new(model, model_name),
        }
    }
}

#[async_trait]
impl<M> LlmProvider for RigAdapter<M>
where
    M: CompletionModel + Send + Sync + 'static,
{
    fn model_name(&self) -> &str {
        self.inner.model_name()
    }

    fn cost_per_token(&self) -> (Decimal, Decimal) {
        self.inner.cost_per_token()
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        self.inner.complete(request).await
    }

    async fn complete_with_tools(
        &self,
        request: ToolCompletionRequest,
    ) -> Result<ToolCompletionResponse, LlmError> {
        self.inner.complete_with_tools(request).await
    }

    async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        self.inner.list_models().await
    }

    async fn model_metadata(&self) -> Result<ModelMetadata, LlmError> {
        self.inner.model_metadata().await
    }

    fn effective_model_name(&self, requested_model: Option<&str>) -> String {
        self.inner.effective_model_name(requested_model)
    }

    fn active_model_name(&self) -> String {
        self.inner.active_model_name()
    }

    fn set_model(&self, model: &str) -> Result<(), LlmError> {
        self.inner.set_model(model)
    }

    fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> Decimal {
        self.inner.calculate_cost(input_tokens, output_tokens)
    }

    fn cache_write_multiplier(&self) -> Decimal {
        self.inner.cache_write_multiplier()
    }

    fn cache_read_discount(&self) -> Decimal {
        self.inner.cache_read_discount()
    }
}

/// Ensure tool result messages reference known assistant tool calls.
///
/// Rewrites orphaned `Role::Tool` messages as `Role::User` messages to avoid
/// provider protocol errors while preserving content.
pub fn sanitize_tool_messages(messages: &mut [ChatMessage]) {
    use std::collections::HashSet;

    let mut known_ids: HashSet<String> = HashSet::new();
    for msg in messages.iter() {
        if msg.role != Role::Assistant {
            continue;
        }
        if let Some(ref calls) = msg.tool_calls {
            for tc in calls {
                known_ids.insert(tc.id.clone());
            }
        }
    }

    for msg in messages.iter_mut() {
        if msg.role != Role::Tool {
            continue;
        }
        let is_orphaned = match &msg.tool_call_id {
            Some(id) => !known_ids.contains(id),
            None => true,
        };
        if is_orphaned {
            let tool_name = msg.name.as_deref().unwrap_or("unknown");
            msg.role = Role::User;
            msg.content = format!("[Tool `{}` returned: {}]", tool_name, msg.content);
            msg.tool_call_id = None;
            msg.name = None;
        }
    }
}

/// Ferrumyx wrapper around runtime-core failover provider.
pub struct FailoverProvider {
    inner: ferrumyx_runtime_core::llm::FailoverProvider,
}

impl FailoverProvider {
    pub fn new(providers: Vec<Arc<dyn LlmProvider>>) -> Result<Self, LlmError> {
        Self::with_cooldown(providers, CooldownConfig::default())
    }

    pub fn with_cooldown(
        providers: Vec<Arc<dyn LlmProvider>>,
        cooldown_config: CooldownConfig,
    ) -> Result<Self, LlmError> {
        let providers: Vec<Arc<dyn CoreLlmProvider>> =
            providers.into_iter().map(to_core_provider).collect();
        let inner = ferrumyx_runtime_core::llm::FailoverProvider::with_cooldown(providers, cooldown_config)?;
        Ok(Self { inner })
    }
}

#[async_trait]
impl LlmProvider for FailoverProvider {
    fn model_name(&self) -> &str {
        self.inner.model_name()
    }

    fn cost_per_token(&self) -> (Decimal, Decimal) {
        self.inner.cost_per_token()
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        self.inner.complete(request).await
    }

    async fn complete_with_tools(
        &self,
        request: ToolCompletionRequest,
    ) -> Result<ToolCompletionResponse, LlmError> {
        self.inner.complete_with_tools(request).await
    }

    async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        self.inner.list_models().await
    }

    async fn model_metadata(&self) -> Result<ModelMetadata, LlmError> {
        self.inner.model_metadata().await
    }

    fn effective_model_name(&self, requested_model: Option<&str>) -> String {
        self.inner.effective_model_name(requested_model)
    }

    fn active_model_name(&self) -> String {
        self.inner.active_model_name()
    }

    fn set_model(&self, model: &str) -> Result<(), LlmError> {
        self.inner.set_model(model)
    }

    fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> Decimal {
        self.inner.calculate_cost(input_tokens, output_tokens)
    }

    fn cache_write_multiplier(&self) -> Decimal {
        self.inner.cache_write_multiplier()
    }

    fn cache_read_discount(&self) -> Decimal {
        self.inner.cache_read_discount()
    }
}

struct FerrumyxToCore {
    inner: Arc<dyn LlmProvider>,
}

#[async_trait]
impl CoreLlmProvider for FerrumyxToCore {
    fn model_name(&self) -> &str {
        self.inner.model_name()
    }

    fn cost_per_token(&self) -> (Decimal, Decimal) {
        self.inner.cost_per_token()
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        self.inner.complete(request).await
    }

    async fn complete_with_tools(
        &self,
        request: ToolCompletionRequest,
    ) -> Result<ToolCompletionResponse, LlmError> {
        self.inner.complete_with_tools(request).await
    }

    async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        self.inner.list_models().await
    }

    async fn model_metadata(&self) -> Result<ModelMetadata, LlmError> {
        self.inner.model_metadata().await
    }

    fn effective_model_name(&self, requested_model: Option<&str>) -> String {
        self.inner.effective_model_name(requested_model)
    }

    fn active_model_name(&self) -> String {
        self.inner.active_model_name()
    }

    fn set_model(&self, model: &str) -> Result<(), LlmError> {
        self.inner.set_model(model)
    }

    fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> Decimal {
        self.inner.calculate_cost(input_tokens, output_tokens)
    }

    fn cache_write_multiplier(&self) -> Decimal {
        self.inner.cache_write_multiplier()
    }

    fn cache_read_discount(&self) -> Decimal {
        self.inner.cache_read_discount()
    }
}

struct CoreToFerrumyx {
    inner: Arc<dyn CoreLlmProvider>,
}

#[async_trait]
impl LlmProvider for CoreToFerrumyx {
    fn model_name(&self) -> &str {
        self.inner.model_name()
    }

    fn cost_per_token(&self) -> (Decimal, Decimal) {
        self.inner.cost_per_token()
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        self.inner.complete(request).await
    }

    async fn complete_with_tools(
        &self,
        request: ToolCompletionRequest,
    ) -> Result<ToolCompletionResponse, LlmError> {
        self.inner.complete_with_tools(request).await
    }

    async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        self.inner.list_models().await
    }

    async fn model_metadata(&self) -> Result<ModelMetadata, LlmError> {
        self.inner.model_metadata().await
    }

    fn effective_model_name(&self, requested_model: Option<&str>) -> String {
        self.inner.effective_model_name(requested_model)
    }

    fn active_model_name(&self) -> String {
        self.inner.active_model_name()
    }

    fn set_model(&self, model: &str) -> Result<(), LlmError> {
        self.inner.set_model(model)
    }

    fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> Decimal {
        self.inner.calculate_cost(input_tokens, output_tokens)
    }

    fn cache_write_multiplier(&self) -> Decimal {
        self.inner.cache_write_multiplier()
    }

    fn cache_read_discount(&self) -> Decimal {
        self.inner.cache_read_discount()
    }
}

/// Bridge a Ferrumyx LLM provider into the runtime-core provider trait.
pub fn to_core_provider(provider: Arc<dyn LlmProvider>) -> Arc<dyn CoreLlmProvider> {
    Arc::new(FerrumyxToCore { inner: provider })
}

/// Bridge a runtime-core LLM provider into the Ferrumyx provider trait.
pub fn from_core_provider(provider: Arc<dyn CoreLlmProvider>) -> Arc<dyn LlmProvider> {
    Arc::new(CoreToFerrumyx { inner: provider })
}

