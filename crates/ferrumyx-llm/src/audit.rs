//! Audit logging for LLM calls.
//! See ARCHITECTURE.md §8.5

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmAuditEntry {
    pub id: Uuid,
    pub session_id: Option<String>,
    pub model: String,
    pub backend: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub data_class: String,
    pub output_hash: String,
    pub latency_ms: u64,
    pub called_at: chrono::DateTime<Utc>,
}

impl LlmAuditEntry {
    pub fn new(
        session_id: Option<String>,
        model: String,
        backend: String,
        prompt_tokens: u32,
        completion_tokens: u32,
        data_class: String,
        output: &str,
        latency_ms: u64,
    ) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(output.as_bytes());
        let output_hash = format!("{:x}", hasher.finalize());

        Self {
            id: Uuid::new_v4(),
            session_id,
            model,
            backend,
            prompt_tokens,
            completion_tokens,
            data_class,
            output_hash,
            latency_ms,
            called_at: Utc::now(),
        }
    }
}
