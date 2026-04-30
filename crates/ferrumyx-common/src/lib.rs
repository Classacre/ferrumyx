//! ferrumyx-common — Shared types, errors, and traits used across all Ferrumyx crates.

pub mod confidence;
pub mod entities;
pub mod error;
pub mod federation;
pub mod memory;
pub mod query;
pub mod target_config;

// Re-export commonly used types
pub use target_config::{Constraints, ScoringConfig, TargetConfig, TargetSpec};

// Re-export error types
pub use error::{
    ApiError, ChannelError, CommonError, ConfigError, DatabaseError, EstimationError,
    EvaluationError, FerrumyxError, HookError, JobError, LlmError, OrchestratorError,
    RepairError, Result, RoutineError, SafetyError, ToolError, WorkerError, WorkspaceError,
};
