//! ferrumyx-web — Web GUI for Ferrumyx
//! Provides a research dashboard with:
//!   - Target query interface
//!   - Knowledge graph explorer
//!   - Ingestion pipeline monitor
//!   - Molecule pipeline viewer
//!   - Self-improvement metrics dashboard
//!   - System status & audit log

pub mod handlers;
pub mod router;
pub mod sse;
pub mod state;
