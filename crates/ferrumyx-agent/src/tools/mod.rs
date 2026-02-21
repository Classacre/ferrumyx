//! IronClaw tool registration scaffold for Ferrumyx.
//!
//! Ferrumyx extends IronClaw by registering custom `FerrumyxTool` implementations
//! through a central `ToolRegistry`. Each tool wraps a specific pipeline stage
//! and is callable from IronClaw intents and routines.
//!
//! Tool lifecycle:
//!   1. Implement `FerrumyxTool` for your type.
//!   2. Register with `ToolRegistry::register`.
//!   3. IronClaw invokes tools via `ToolRegistry::invoke(name, params)`.

pub mod ingestion_tool;
pub mod ner_tool;
pub mod ranker_tool;
pub mod kg_tool;

use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;
use anyhow::Result;
use async_trait::async_trait;

// ─────────────────────────────────────────────
//  Core trait — implement for each Ferrumyx tool
// ─────────────────────────────────────────────

/// A callable Ferrumyx tool that integrates into IronClaw's agent loop.
///
/// # Minimal contract
/// - `name()` must be unique across the registry (snake_case, e.g. `"ingest_pubmed"`).
/// - `description()` is surfaced to IronClaw's planner / LLM as the tool docstring.
/// - `parameters_schema()` returns a JSON Schema object for parameter validation.
/// - `invoke()` receives validated JSON params and returns JSON output.
#[async_trait]
pub trait FerrumyxTool: Send + Sync {
    /// Unique tool name (used as the function call identifier).
    fn name(&self) -> &str;

    /// Short description shown to the LLM planner.
    fn description(&self) -> &str;

    /// JSON Schema describing the expected input parameters.
    fn parameters_schema(&self) -> Value;

    /// Execute the tool. Returns a JSON result or an anyhow error.
    async fn invoke(&self, params: Value) -> Result<Value>;

    /// Whether this tool requires human confirmation before running.
    /// Default: false. Override for destructive or externally-reaching tools.
    fn requires_approval(&self) -> bool { false }

    /// Data classification of the tool's output.
    /// Tools that access patient-level data should return "CONFIDENTIAL".
    fn output_data_class(&self) -> &str { "PUBLIC" }
}

// ─────────────────────────────────────────────
//  Tool registry
// ─────────────────────────────────────────────

/// Central registry mapping tool names → trait objects.
/// Build once at startup with `ToolRegistry::builder()`, then share via Arc.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn FerrumyxTool>>,
}

impl ToolRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self { tools: HashMap::new() }
    }

    /// Register a tool. Panics if the name is already registered.
    pub fn register<T: FerrumyxTool + 'static>(&mut self, tool: T) {
        let name = tool.name().to_string();
        assert!(
            !self.tools.contains_key(&name),
            "Duplicate tool name: {name}"
        );
        self.tools.insert(name, Arc::new(tool));
    }

    /// Invoke a registered tool by name.
    pub async fn invoke(&self, name: &str, params: Value) -> Result<Value> {
        let tool = self.tools.get(name)
            .ok_or_else(|| anyhow::anyhow!("Unknown tool: {name}"))?;

        tracing::info!(
            tool = name,
            requires_approval = tool.requires_approval(),
            data_class = tool.output_data_class(),
            "Invoking tool"
        );

        tool.invoke(params).await
    }

    /// List all registered tools as a JSON array (for IronClaw function manifest).
    pub fn manifest(&self) -> Value {
        let tools: Vec<Value> = self.tools.values().map(|t| {
            serde_json::json!({
                "name": t.name(),
                "description": t.description(),
                "parameters": t.parameters_schema(),
                "requires_approval": t.requires_approval(),
                "output_data_class": t.output_data_class(),
            })
        }).collect();
        serde_json::json!({ "tools": tools })
    }

    /// Number of registered tools.
    pub fn len(&self) -> usize { self.tools.len() }

    /// Returns true if no tools are registered.
    pub fn is_empty(&self) -> bool { self.tools.is_empty() }

    /// Get a reference to a tool by name.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn FerrumyxTool>> {
        self.tools.get(name)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self { Self::new() }
}

/// Convenience function: build the default Ferrumyx tool registry.
/// Call once at startup and store in AppState / IronClaw agent context.
pub fn build_default_registry(db: sqlx::PgPool) -> ToolRegistry {
    let mut reg = ToolRegistry::new();
    reg.register(ingestion_tool::IngestPubmedTool::new(db.clone()));
    reg.register(ingestion_tool::IngestEuropePmcTool::new(db.clone()));
    reg.register(ner_tool::NerExtractTool::new());
    reg.register(ranker_tool::ScoreTargetsTool::new(db.clone()));
    reg.register(kg_tool::KgQueryTool::new(db.clone()));
    reg.register(kg_tool::KgUpsertTool::new(db));
    tracing::info!("ToolRegistry ready with {} tools", reg.len());
    reg
}

// ─────────────────────────────────────────────
//  Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoTool;

    #[async_trait]
    impl FerrumyxTool for EchoTool {
        fn name(&self) -> &str { "echo" }
        fn description(&self) -> &str { "Echoes the input params back." }
        fn parameters_schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string" }
                },
                "required": ["message"]
            })
        }
        async fn invoke(&self, params: Value) -> Result<Value> {
            Ok(serde_json::json!({ "echo": params["message"] }))
        }
    }

    #[tokio::test]
    async fn test_registry_register_and_invoke() {
        let mut reg = ToolRegistry::new();
        reg.register(EchoTool);
        assert_eq!(reg.len(), 1);

        let result = reg.invoke("echo", serde_json::json!({ "message": "hello" })).await.unwrap();
        assert_eq!(result["echo"], "hello");
    }

    #[tokio::test]
    async fn test_registry_unknown_tool_errors() {
        let reg = ToolRegistry::new();
        let err = reg.invoke("nonexistent", serde_json::json!({})).await;
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("Unknown tool"));
    }

    #[tokio::test]
    async fn test_manifest_json() {
        let mut reg = ToolRegistry::new();
        reg.register(EchoTool);
        let manifest = reg.manifest();
        let tools = manifest["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "echo");
    }

    #[test]
    #[should_panic(expected = "Duplicate tool name")]
    fn test_duplicate_registration_panics() {
        let mut reg = ToolRegistry::new();
        reg.register(EchoTool);
        reg.register(EchoTool); // should panic
    }
}
