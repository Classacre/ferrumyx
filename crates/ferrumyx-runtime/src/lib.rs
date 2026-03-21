//! ferrumyx-runtime
//!
//! Thin compatibility facade over the Ferrumyx runtime core.
//! This lets Ferrumyx migrate subsystem-by-subsystem to local implementations
//! without changing every call site at once.

#![forbid(unsafe_code)]

pub mod llm;
pub mod tools;

pub use ferrumyx_runtime_core::{agent, channels, config, context, hooks, safety, skills};

