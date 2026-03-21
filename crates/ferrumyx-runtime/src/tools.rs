//! Ferrumyx-owned tools contract with a runtime-core bridge.
//!
//! Ferrumyx code should implement and register tools against this module.
//! At runtime, adapters bridge these tools into the runtime core.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::context::JobContext;
use crate::llm::ToolDefinition;

/// How much approval a specific tool invocation requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalRequirement {
    Never,
    UnlessAutoApproved,
    Always,
}

/// Tool execution domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolDomain {
    Orchestrator,
    Container,
}

/// Per-tool rate limit configuration.
#[derive(Debug, Clone)]
pub struct ToolRateLimitConfig {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
}

impl Default for ToolRateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            requests_per_hour: 1000,
        }
    }
}

/// Error type for tool execution.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Timeout after {0:?}")]
    Timeout(Duration),
    #[error("Not authorized: {0}")]
    NotAuthorized(String),
    #[error("Rate limited, retry after {0:?}")]
    RateLimited(Option<Duration>),
    #[error("External service error: {0}")]
    ExternalService(String),
    #[error("Sandbox error: {0}")]
    Sandbox(String),
}

/// Output from a tool execution.
#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub result: serde_json::Value,
    pub cost: Option<Decimal>,
    pub duration: Duration,
    pub raw: Option<String>,
}

impl ToolOutput {
    pub fn success(result: serde_json::Value, duration: Duration) -> Self {
        Self {
            result,
            cost: None,
            duration,
            raw: None,
        }
    }

    pub fn text(text: impl Into<String>, duration: Duration) -> Self {
        Self {
            result: serde_json::Value::String(text.into()),
            cost: None,
            duration,
            raw: None,
        }
    }

    pub fn with_cost(mut self, cost: Decimal) -> Self {
        self.cost = Some(cost);
        self
    }

    pub fn with_raw(mut self, raw: impl Into<String>) -> Self {
        self.raw = Some(raw.into());
        self
    }
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError>;

    fn estimated_cost(&self, _params: &serde_json::Value) -> Option<Decimal> {
        None
    }

    fn estimated_duration(&self, _params: &serde_json::Value) -> Option<Duration> {
        None
    }

    fn requires_sanitization(&self) -> bool {
        true
    }

    fn requires_approval(&self, _params: &serde_json::Value) -> ApprovalRequirement {
        ApprovalRequirement::Never
    }

    fn execution_timeout(&self) -> Duration {
        Duration::from_secs(60)
    }

    fn domain(&self) -> ToolDomain {
        ToolDomain::Orchestrator
    }

    fn sensitive_params(&self) -> &[&str] {
        &[]
    }

    fn rate_limit_config(&self) -> Option<ToolRateLimitConfig> {
        None
    }
}

impl From<ToolOutput> for ferrumyx_runtime_core::tools::ToolOutput {
    fn from(value: ToolOutput) -> Self {
        Self {
            result: value.result,
            cost: value.cost,
            duration: value.duration,
            raw: value.raw,
        }
    }
}

impl From<ferrumyx_runtime_core::tools::ToolOutput> for ToolOutput {
    fn from(value: ferrumyx_runtime_core::tools::ToolOutput) -> Self {
        Self {
            result: value.result,
            cost: value.cost,
            duration: value.duration,
            raw: value.raw,
        }
    }
}

impl From<ApprovalRequirement> for ferrumyx_runtime_core::tools::ApprovalRequirement {
    fn from(value: ApprovalRequirement) -> Self {
        match value {
            ApprovalRequirement::Never => Self::Never,
            ApprovalRequirement::UnlessAutoApproved => Self::UnlessAutoApproved,
            ApprovalRequirement::Always => Self::Always,
        }
    }
}

impl From<ferrumyx_runtime_core::tools::ApprovalRequirement> for ApprovalRequirement {
    fn from(value: ferrumyx_runtime_core::tools::ApprovalRequirement) -> Self {
        match value {
            ferrumyx_runtime_core::tools::ApprovalRequirement::Never => Self::Never,
            ferrumyx_runtime_core::tools::ApprovalRequirement::UnlessAutoApproved => Self::UnlessAutoApproved,
            ferrumyx_runtime_core::tools::ApprovalRequirement::Always => Self::Always,
        }
    }
}

impl From<ToolDomain> for ferrumyx_runtime_core::tools::ToolDomain {
    fn from(value: ToolDomain) -> Self {
        match value {
            ToolDomain::Orchestrator => Self::Orchestrator,
            ToolDomain::Container => Self::Container,
        }
    }
}

impl From<ferrumyx_runtime_core::tools::ToolDomain> for ToolDomain {
    fn from(value: ferrumyx_runtime_core::tools::ToolDomain) -> Self {
        match value {
            ferrumyx_runtime_core::tools::ToolDomain::Orchestrator => Self::Orchestrator,
            ferrumyx_runtime_core::tools::ToolDomain::Container => Self::Container,
        }
    }
}

impl From<ToolRateLimitConfig> for ferrumyx_runtime_core::tools::ToolRateLimitConfig {
    fn from(value: ToolRateLimitConfig) -> Self {
        Self {
            requests_per_minute: value.requests_per_minute,
            requests_per_hour: value.requests_per_hour,
        }
    }
}

impl From<ferrumyx_runtime_core::tools::ToolRateLimitConfig> for ToolRateLimitConfig {
    fn from(value: ferrumyx_runtime_core::tools::ToolRateLimitConfig) -> Self {
        Self {
            requests_per_minute: value.requests_per_minute,
            requests_per_hour: value.requests_per_hour,
        }
    }
}

impl From<ToolError> for ferrumyx_runtime_core::tools::ToolError {
    fn from(value: ToolError) -> Self {
        match value {
            ToolError::InvalidParameters(v) => Self::InvalidParameters(v),
            ToolError::ExecutionFailed(v) => Self::ExecutionFailed(v),
            ToolError::Timeout(v) => Self::Timeout(v),
            ToolError::NotAuthorized(v) => Self::NotAuthorized(v),
            ToolError::RateLimited(v) => Self::RateLimited(v),
            ToolError::ExternalService(v) => Self::ExternalService(v),
            ToolError::Sandbox(v) => Self::Sandbox(v),
        }
    }
}

impl From<ferrumyx_runtime_core::tools::ToolError> for ToolError {
    fn from(value: ferrumyx_runtime_core::tools::ToolError) -> Self {
        match value {
            ferrumyx_runtime_core::tools::ToolError::InvalidParameters(v) => Self::InvalidParameters(v),
            ferrumyx_runtime_core::tools::ToolError::ExecutionFailed(v) => Self::ExecutionFailed(v),
            ferrumyx_runtime_core::tools::ToolError::Timeout(v) => Self::Timeout(v),
            ferrumyx_runtime_core::tools::ToolError::NotAuthorized(v) => Self::NotAuthorized(v),
            ferrumyx_runtime_core::tools::ToolError::RateLimited(v) => Self::RateLimited(v),
            ferrumyx_runtime_core::tools::ToolError::ExternalService(v) => Self::ExternalService(v),
            ferrumyx_runtime_core::tools::ToolError::Sandbox(v) => Self::Sandbox(v),
        }
    }
}

struct FerrumyxToCoreTool {
    inner: Arc<dyn Tool>,
}

#[async_trait]
impl ferrumyx_runtime_core::tools::Tool for FerrumyxToCoreTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.inner.parameters_schema()
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: &crate::context::JobContext,
    ) -> Result<ferrumyx_runtime_core::tools::ToolOutput, ferrumyx_runtime_core::tools::ToolError> {
        let out = self
            .inner
            .execute(params, ctx)
            .await
            .map_err(ferrumyx_runtime_core::tools::ToolError::from)?;
        Ok(out.into())
    }

    fn estimated_cost(&self, params: &serde_json::Value) -> Option<Decimal> {
        self.inner.estimated_cost(params)
    }

    fn estimated_duration(&self, params: &serde_json::Value) -> Option<Duration> {
        self.inner.estimated_duration(params)
    }

    fn requires_sanitization(&self) -> bool {
        self.inner.requires_sanitization()
    }

    fn requires_approval(
        &self,
        params: &serde_json::Value,
    ) -> ferrumyx_runtime_core::tools::ApprovalRequirement {
        self.inner.requires_approval(params).into()
    }

    fn execution_timeout(&self) -> Duration {
        self.inner.execution_timeout()
    }

    fn domain(&self) -> ferrumyx_runtime_core::tools::ToolDomain {
        self.inner.domain().into()
    }

    fn sensitive_params(&self) -> &[&str] {
        self.inner.sensitive_params()
    }

    fn rate_limit_config(&self) -> Option<ferrumyx_runtime_core::tools::ToolRateLimitConfig> {
        self.inner.rate_limit_config().map(Into::into)
    }
}

struct CoreToFerrumyxTool {
    inner: Arc<dyn ferrumyx_runtime_core::tools::Tool>,
}

#[async_trait]
impl Tool for CoreToFerrumyxTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.inner.parameters_schema()
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let out = self
            .inner
            .execute(params, ctx)
            .await
            .map_err(ToolError::from)?;
        Ok(out.into())
    }

    fn estimated_cost(&self, params: &serde_json::Value) -> Option<Decimal> {
        self.inner.estimated_cost(params)
    }

    fn estimated_duration(&self, params: &serde_json::Value) -> Option<Duration> {
        self.inner.estimated_duration(params)
    }

    fn requires_sanitization(&self) -> bool {
        self.inner.requires_sanitization()
    }

    fn requires_approval(&self, params: &serde_json::Value) -> ApprovalRequirement {
        self.inner.requires_approval(params).into()
    }

    fn execution_timeout(&self) -> Duration {
        self.inner.execution_timeout()
    }

    fn domain(&self) -> ToolDomain {
        self.inner.domain().into()
    }

    fn sensitive_params(&self) -> &[&str] {
        self.inner.sensitive_params()
    }

    fn rate_limit_config(&self) -> Option<ToolRateLimitConfig> {
        self.inner.rate_limit_config().map(Into::into)
    }
}

/// Bridge a Ferrumyx tool into the runtime-core tool trait.
pub fn to_core_tool(tool: Arc<dyn Tool>) -> Arc<dyn ferrumyx_runtime_core::tools::Tool> {
    Arc::new(FerrumyxToCoreTool { inner: tool })
}

/// Bridge a runtime-core tool into the Ferrumyx tool trait.
pub fn from_core_tool(tool: Arc<dyn ferrumyx_runtime_core::tools::Tool>) -> Arc<dyn Tool> {
    Arc::new(CoreToFerrumyxTool { inner: tool })
}

/// Ferrumyx-owned tool registry.
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Arc<dyn Tool>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, tool: Arc<dyn Tool>) {
        self.register_sync(tool);
    }

    pub fn register_sync(&self, tool: Arc<dyn Tool>) {
        if let Ok(mut map) = self.tools.write() {
            map.insert(tool.name().to_string(), tool);
        }
    }

    pub async fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.get_sync(name)
    }

    pub fn get_sync(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools
            .read()
            .ok()
            .and_then(|m| m.get(name).map(Arc::clone))
    }

    pub async fn list(&self) -> Vec<String> {
        self.list_sync()
    }

    pub fn list_sync(&self) -> Vec<String> {
        self.tools
            .read()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn count(&self) -> usize {
        self.tools.read().map(|m| m.len()).unwrap_or(0)
    }

    pub async fn tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tool_definitions_sync()
    }

    pub fn tool_definitions_sync(&self) -> Vec<ToolDefinition> {
        let mut defs: Vec<ToolDefinition> = self
            .tools
            .read()
            .map(|m| {
                m.values()
                    .map(|tool| ToolDefinition {
                        name: tool.name().to_string(),
                        description: tool.description().to_string(),
                        parameters: tool.parameters_schema(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        defs.sort_unstable_by(|a, b| a.name.cmp(&b.name));
        defs
    }

    /// Convert this Ferrumyx registry into a runtime-core registry.
    ///
    /// This is the integration boundary for the current migration phase.
    pub fn to_core_registry(&self) -> Arc<ferrumyx_runtime_core::tools::ToolRegistry> {
        let out = Arc::new(ferrumyx_runtime_core::tools::ToolRegistry::new());
        if let Ok(map) = self.tools.read() {
            for tool in map.values() {
                out.register_sync(to_core_tool(Arc::clone(tool)));
            }
        }
        out
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

