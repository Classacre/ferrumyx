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
pub mod molecules_tool;

use std::sync::Arc;
use ironclaw::tools::ToolRegistry;

/// Convenience function: build the default Ferrumyx tool registry natively using IronClaw's registry.
/// Call once at startup and store in AppState / IronClaw agent context.
pub fn build_default_registry(db: Arc<ferrumyx_db::Database>) -> ToolRegistry {
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(ingestion_tool::IngestPubmedTool::new(db.clone())));
    reg.register(Arc::new(ingestion_tool::IngestEuropePmcTool::new(db.clone())));
    reg.register(Arc::new(ingestion_tool::IngestAllSourcesTool::new(db.clone())));
    reg.register(Arc::new(ner_tool::NerExtractTool::new()));
    reg.register(Arc::new(ranker_tool::ScoreTargetsTool::new(db.clone())));
    reg.register(Arc::new(kg_tool::KgQueryTool::new(db.clone())));
    reg.register(Arc::new(kg_tool::KgUpsertTool::new(db)));
    
    // Register Molecule tools
    let cache_dir = std::path::PathBuf::from("./data/cache");
    reg.register(Arc::new(molecules_tool::FetchStructureTool::new(cache_dir)));
    reg.register(Arc::new(molecules_tool::DetectPocketsTool::new(std::path::PathBuf::from("fpocket"))));
    reg.register(Arc::new(molecules_tool::DockMoleculeTool::new(std::path::PathBuf::from("vina"))));

    tracing::info!("ToolRegistry ready");
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
