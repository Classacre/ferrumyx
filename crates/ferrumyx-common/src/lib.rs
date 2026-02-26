//! ferrumyx-common — Shared types, errors, and traits used across all Ferrumyx crates.

pub mod error;
pub mod entities;
pub mod confidence;
pub mod target_config;
pub mod sandbox;

// Re-export commonly used types
pub use target_config::{TargetConfig, TargetSpec, Constraints, ScoringConfig};
