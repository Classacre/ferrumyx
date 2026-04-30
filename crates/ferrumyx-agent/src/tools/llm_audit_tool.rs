//! Tool for viewing LLM audit logs from IronClaw routing.

use std::sync::Arc;
use async_trait::async_trait;
use ferrumyx_runtime::tools::{Tool, ToolInfo};
use ferrumyx_runtime::agent::AgentDeps;
use ferrumyx_runtime::context::Context;
use anyhow::Result;

use crate::llm_routing::IronClawLlmRouter;

pub struct LlmAuditTool {
    router: Arc<IronClawLlmRouter>,
}

impl LlmAuditTool {
    pub fn new(router: Arc<IronClawLlmRouter>) -> Self {
        Self { router }
    }
}

#[async_trait]
impl Tool for LlmAuditTool {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            name: "llm_audit_viewer".to_string(),
            description: "View audit logs for LLM calls routed through IronClaw data classification gates".to_string(),
            input_description: "Optional: 'recent' to show last 10 entries, or empty for all".to_string(),
        }
    }

    async fn execute(&self, input: serde_json::Value, _context: &Context, _deps: &AgentDeps) -> Result<serde_json::Value> {
        let logs = self.router.get_audit_logs();

        let filter = input.as_str().unwrap_or("");
        let filtered_logs = if filter == "recent" {
            logs.iter().rev().take(10).cloned().collect::<Vec<_>>()
        } else {
            logs
        };

        let audit_summary = serde_json::json!({
            "total_calls": logs.len(),
            "filtered_calls": filtered_logs.len(),
            "logs": filtered_logs
        });

        Ok(audit_summary)
    }
}