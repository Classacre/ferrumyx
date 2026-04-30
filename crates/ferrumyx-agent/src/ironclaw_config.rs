use ironclaw::config::{AgentConfig, AuditConfig, ContextConfig};

pub fn load_config() -> AgentConfig {
    AgentConfig::default()
        .with_max_iterations(50)
        .with_timeout(std::time::Duration::from_secs(3600))
        .with_audit_config(AuditConfig {
            enabled: true,
            log_level: ironclaw::config::AuditLevel::Detailed,
            retention_days: 30,
            compress_old_logs: true,
        })
        .with_context_config(ContextConfig {
            max_context_length: 100_000,
            compaction_threshold: 80_000,
            auto_compaction: true,
            memory_backend: ironclaw::config::MemoryBackend::Hybrid,
        })
}