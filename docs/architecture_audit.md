# Architecture Audit: Discrepancies between `ARCHITECTURE.md` and Code

This audit compares the system design documented in `ARCHITECTURE.md` with the current implementation in the `Ferrumyx` codebase.

## 1. Modular Structure & Crates

| Component | `ARCHITECTURE.md` | Actual Code | Status |
|---|---|---|---|
| **Query Engine** | `ferrumyx-query` (Crate) | Module within `ferrumyx-web` | ⚠️ Consolidated |
| **Feedback Loop** | `ferrumyx-feedback` (Crate) | Mostly missing / Partial modules | ❌ Out of Sync |
| **Routines** | `ferrumyx-routines` (Crate) | Module within `ferrumyx-kg` / `ferrumyx-agent` | ⚠️ Consolidated |
| **Tool Registry** | IronClaw Tool Registry integration | Unified Rust Pipeline (web-triggered) | ❌ Inconsistent |

## 2. Ingestion & Tool Layer

> [!IMPORTANT]
> `ARCHITECTURE.md` Section 1.2 and 1.8 claim that tools like `IngestPubmedTool` are implemented as IronClaw tools (WASM or Native) and registered via the registry.

- **Current Implementation**: The ingestion pipeline is a linear Rust function (`run_ingestion`) in `ferrumyx-ingestion`. It is triggered directly by the web UI or agent loop, bypassing the "Tool" trait abstraction described in the docs.
- **WASM Sandbox**: There is no evidence of the WASM sandbox for ingestion tools being active or used; all ingestion logic is native Rust.

## 3. Database Schema (LanceDB)

Several tables and fields described in Section 1.4, 3.1, and 3.5 are either simplified or missing.

| Table | Status | Missing Fields / Discrepancies |
|---|---|---|
| `papers` | ⚠️ Simplified | Missing `raw_json`, `open_access`, `retrieval_tier`. |
| `chunks` | ⚠️ Simplified | Missing `token_count`. Fields like `section` are `Option<String>` vs specific Enums. |
| `kg_facts` | ❌ Simplified | Missing `valid_from`, `valid_until`, `evidence_type`, `evidence_weight`, `sample_size`, `study_type`. |
| `target_scores`| ❌ Missing | Not found in `ferrumyx-db`. Struct exists in `ferrumyx-ranker` but not persisted. |
| `ingestion_audit`| ❌ Missing | Audit events are currently not persisted to a table. |

## 4. Confidence & Scoring Logic

- **Logic vs. Storage**: The mathematical models for "Noisy-OR" aggregation and multiplicative modifiers (Section 3.2/3.3) are implemented in `ferrumyx-common/src/confidence.rs`. However, they are **not used** by the `ferrumyx-kg/src/extraction.rs` module, which currently uses a hardcoded `evidence_count = 1`.
- **Versioning**: The "strictly append-only" versioning with `valid_from` and `valid_until` is not implemented in the current schema.

## 5. Security & Isolation

- **Docker/WASM**: Section 5.1/5.2 describes strict enforcement between Ferrumyx and Remote LLMs/Sandbox. Current code uses direct HTTP calls (`reqwest`) to Ollama/OpenAI without the intermediate IronClaw security proxying described in the conceptual diagrams.

## Summary

The codebase implements the **core functionality** (PDF parsing, NER, Vector Search, Ranking) but uses a significantly **simpler integration model** and **smaller database schema** than the aspirational design in `ARCHITECTURE.md`.

> [!TIP]
> Recommendation: Synchronize `ARCHITECTURE.md` to reflect the current unified Rust architecture, or prioritize the implementation of the "Tool" layer and expanded DB schema to match the documented vision.
