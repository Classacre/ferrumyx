use async_trait::async_trait;
use ironclaw::context::JobContext;
use ironclaw::tools::{Tool, ToolError, ToolOutput};
use serde_json::json;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

/// Controlled host command execution tool for autonomous remediation tasks.
pub struct SystemCommandTool;

impl SystemCommandTool {
    pub fn new() -> Self {
        Self
    }

    fn is_command_allowed(command: &str) -> bool {
        let c = command.trim().to_lowercase();
        if c.is_empty() {
            return false;
        }
        if c.contains("git reset --hard") {
            return false;
        }

        let blocked_prefixes = [
            "rm -rf",
            "del /f /s /q",
            "format",
            "mkfs",
            "shutdown",
            "reboot",
            "poweroff",
            "diskpart",
            "reg delete",
        ];

        // Evaluate each command segment independently (e.g., `cmd1; cmd2 | cmd3`)
        // so option flags like `-Format` do not get treated as destructive shell commands.
        let segments = c
            .split(|ch| ch == ';' || ch == '|')
            .map(str::trim)
            .filter(|s| !s.is_empty());
        for segment in segments {
            if blocked_prefixes.iter().any(|p| {
                segment == *p
                    || segment.starts_with(&format!("{p} "))
                    || segment.starts_with(&format!("{p}."))
            }) {
                return false;
            }
        }
        true
    }
}

#[async_trait]
impl Tool for SystemCommandTool {
    fn name(&self) -> &str {
        "run_system_command"
    }

    fn description(&self) -> &str {
        "Runs a non-destructive host command for diagnostics/automation and returns stdout/stderr."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The command string to execute."
                },
                "timeout_secs": {
                    "type": "integer",
                    "description": "Execution timeout in seconds (default: 45, max: 300)."
                }
            },
            "required": ["command"]
        })
    }

    fn execution_timeout(&self) -> Duration {
        Duration::from_secs(5 * 60)
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ToolError::InvalidParameters("missing 'command' parameter".to_string()))?
            .to_string();

        if !Self::is_command_allowed(&command) {
            return Err(ToolError::ExecutionFailed(
                "command blocked by safety policy".to_string(),
            ));
        }

        let timeout_secs = params
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(45)
            .clamp(1, 300);

        let child = Command::new("powershell")
            .arg("-NoProfile")
            .arg("-Command")
            .arg(&command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ToolError::ExecutionFailed(format!("spawn failed: {e}")))?;

        let timed = tokio::time::timeout(Duration::from_secs(timeout_secs), child.wait_with_output())
            .await;
        let output = match timed {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => return Err(ToolError::ExecutionFailed(format!("command failed: {e}"))),
            Err(_) => {
                return Err(ToolError::Timeout(Duration::from_secs(timeout_secs)));
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let status = output.status.code().unwrap_or(-1);

        Ok(ToolOutput::success(
            json!({
                "status_code": status,
                "stdout": stdout,
                "stderr": stderr,
                "ok": output.status.success()
            }),
            Duration::from_millis(0),
        ))
    }
}
