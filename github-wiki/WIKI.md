# Ferrumyx Wiki

This wiki documents the implementation as it exists in the repository today. It is intended for engineers who need to operate, extend, or debug Ferrumyx across ingestion, graph construction, ranking, molecular workflows, and federation.

## Table of Contents

1. System Overview
2. Workspace and Crate Boundaries
3. Runtime Execution Model
4. Tooling Surface
5. End-to-End Workflow
6. Ingestion Pipeline Internals
7. Embedding System
8. Knowledge Graph and Entity Layer
9. Ranking and Provider Enrichment
10. Molecules Pipeline
11. Storage Architecture
12. Web/API Architecture
13. Federation Subsystem
14. Performance Design
15. Extension Playbooks
16. Testing and Benchmarking Paths
17. Operational Caveats

## 1) System Overview

Ferrumyx v2.0.0 is an autonomous biomedical discovery platform built on IronClaw's secure agent framework and BioClaw's bioinformatics methodology. The system centers around:

- IronClaw-powered agent orchestration with WASM sandboxing (`ferrumyx-agent`).
- Literature ingestion and BioClaw-inspired evidence extraction (`ferrumyx-ingestion`, `ferrumyx-kg`).
- Target prioritization with conversational workflows (`ferrumyx-ranker`).
- Secure molecule pipeline with container orchestration (`ferrumyx-molecules`).
- PostgreSQL + pgvector storage with encrypted secrets (`ferrumyx-db`, `ferrumyx-common`).
- Multi-channel interactive UI/API layer (`ferrumyx-web`, channels-src/).

Core entry points:

- Agent binary: `crates/ferrumyx-agent/src/main.rs`
- Web binary: `crates/ferrumyx-web/src/main.rs`
- Workspace definition: `Cargo.toml`

## 2) Workspace and Crate Boundaries

### `crates/ferrumyx-agent`

Owns IronClaw agent orchestration, WASM tool sandboxing, BioClaw skill integration, and multi-channel routing. It wires PostgreSQL + encrypted secrets + LLM providers + IronClaw agent loop + web stack.

Key files:

- `crates/ferrumyx-agent/src/main.rs`
- `crates/ferrumyx-agent/src/config/mod.rs`
- `crates/ferrumyx-agent/src/tools/mod.rs`

### `crates/ferrumyx-ingestion`

Implements IronClaw-scheduled source retrieval, dedup, full-text processing, chunking, BioClaw entity/relation extraction handoff, and pgvector embedding updates with job orchestration.

Key files:

- `crates/ferrumyx-ingestion/src/pipeline.rs`
- `crates/ferrumyx-ingestion/src/repository.rs`
- `crates/ferrumyx-ingestion/src/embedding.rs`
- `crates/ferrumyx-ingestion/src/chunker.rs`
- `crates/ferrumyx-ingestion/src/sources/*.rs`

### `crates/ferrumyx-kg`

Handles BioClaw-inspired entity/relation extraction, knowledge graph construction, and conversational scoring primitives for oncology target prioritization.

Key files:

- `crates/ferrumyx-kg/src/extraction.rs`
- `crates/ferrumyx-kg/src/scoring.rs`
- `crates/ferrumyx-kg/src/update.rs`
- `crates/ferrumyx-kg/src/repository.rs`
- `crates/ferrumyx-kg/src/conflict.rs`

### `crates/ferrumyx-ranker`

Consumes evidence, external/provider signals, and ranking logic for query outputs.

Key files:

- `crates/ferrumyx-ranker/src/lib.rs`
- `crates/ferrumyx-ranker/src/scorer.rs`
- `crates/ferrumyx-ranker/src/providers/*.rs`

### `crates/ferrumyx-molecules`

Contains structure retrieval, pocket detection, ligand generation, docking, ADMET, and molecule scoring components.

Key files:

- `crates/ferrumyx-molecules/src/pipeline.rs`
- `crates/ferrumyx-molecules/src/pdb.rs`
- `crates/ferrumyx-molecules/src/pocket.rs`
- `crates/ferrumyx-molecules/src/ligand.rs`
- `crates/ferrumyx-molecules/src/docking.rs`
- `crates/ferrumyx-molecules/src/admet.rs`
- `crates/ferrumyx-molecules/src/scoring.rs`

### `crates/ferrumyx-db`

Owns PostgreSQL + pgvector schema/repositories, encrypted secrets management, and federation package/trust/lineage persistence with comprehensive monitoring.

Key files:

- `crates/ferrumyx-db/src/database.rs`
- `crates/ferrumyx-db/src/schema.rs`
- `crates/ferrumyx-db/src/chunks.rs`
- `crates/ferrumyx-db/src/papers.rs`
- `crates/ferrumyx-db/src/kg_facts.rs`
- `crates/ferrumyx-db/src/federation.rs`

### `crates/ferrumyx-web`

Exposes IronClaw web gateway with multi-channel support, Axum routes for UI/APIs: ingestion, query, KG, ranker, chat, federation, monitoring, settings.

Key files:

- `crates/ferrumyx-web/src/router.rs`
- `crates/ferrumyx-web/src/state.rs`
- `crates/ferrumyx-web/src/sse.rs`
- `crates/ferrumyx-web/src/handlers/*.rs`

### Runtime bridge crates

- `crates/ferrumyx-runtime`: IronClaw runtime adapters with WASM tool support and container orchestration.
- `crates/ferrumyx-runtime-core`: shared runtime-core with Docker orchestration and encrypted secrets management.
- `crates/ferrumyx-monitoring`: performance metrics, health checks, and Prometheus integration.

## 3) Runtime Execution Model

Startup sequence in `crates/ferrumyx-agent/src/main.rs`:

1. Load and merge runtime config.
2. Initialize DB and repositories.
3. Initialize provider chains and scoring queue(s).
4. Register tool set.
5. Construct runtime agent and spawn loop.
6. Start web router and event streams.

Design properties:

- Tools are first-class runtime actions.
- Agent loop and web surfaces share stateful infrastructure.
- Long-running jobs emit progress/events and can be monitored externally.

## 4) Tooling Surface

Tool modules are in `crates/ferrumyx-agent/src/tools/`.

### Ingestion and query tools

- `ingestion_tool.rs`: end-to-end literature ingestion execution.
- `query_tool.rs`: target query + evidence response path.
- `embedding_backfill_tool.rs`: asynchronous/explicit embedding catch-up.

### Autonomy and lab orchestration

- `autonomous_cycle_tool.rs`: iterative cycles over ingest/score/rank.
- `lab_autoresearch_tool.rs`, `lab_planner_tool.rs`, `lab_retriever_tool.rs`, `lab_validator_tool.rs`: role-based autonomous research execution.
- `lab_run_status_tool.rs`, `lab_state.rs`: run-state tracking and visibility.

### Scoring and maintenance

- `scoring_tool.rs`: trigger/drive scoring refresh.
- `provider_refresh_tool.rs`: refresh external provider signals.
- `workflow_status_tool.rs`: status introspection.

### System/molecule tools

- `molecule_tool.rs`: molecular flow invocation.
- `system_command_tool.rs`: controlled command execution path.
- `runtime_profile.rs`: profile detection and runtime tuning decisions.

## 5) End-to-End Workflow

A typical pipeline run:

1. **Source retrieval** from configured literature providers.
2. **Cross-source identity dedup** (DOI/PMID/PMCID/title/fuzzy logic).
3. **Paper upsert** into storage.
4. **Full-text acquisition** (OA + fallback paths).
5. **Section-aware parse/chunk**.
6. **Entity + relation extraction** and fact construction.
7. **Chunk and fact persistence**.
8. **Embedding generation/backfill**.
9. **Ranking and query retrieval**.
10. **Optional molecular downstream processing**.

This workflow is orchestrated primarily in `crates/ferrumyx-ingestion/src/pipeline.rs` and consumed by tools/web handlers.

## 6) Ingestion Pipeline Internals

`crates/ferrumyx-ingestion/src/pipeline.rs` contains the main orchestration.

Key internal behaviors:

- Multi-source fan-out with bounded concurrency.
- Step-wise and total timeouts for source and full-text operations.
- Cache layers for source responses, full-text outcomes, and parse artifacts.
- Heavy-lane vs fast-lane patterns for throughput-sensitive phases.
- Progress heartbeat emission (`IngestionProgress`) through broadcast channels.
- Post-ingestion KG fact extraction and typed predicate tracking.

Supporting components:

- `sources/*.rs`: source adapters.
- `pdf_parser.rs`: full-text parse paths.
- `chunker.rs`: chunk strategy and boundaries.
- `repository.rs`: DB access + batch operations.

## 7) Embedding System

Primary code: `crates/ferrumyx-ingestion/src/embedding.rs`.

Implemented backend model:

- Multiple backends with a unified client abstraction (`EmbeddingClient`).
- Runtime- and mode-driven batching and max-length behavior.
- Global pending-embedding pass support for throughput.
- Batch writeback + normalization before persistence.

Important implementation notes:

- `fastembed` support is compile-time gated (`fastembed_backend` feature).
- Auto-switch logic in ingestion tooling should only select FastEmbed when compiled.
- Embeddings are used both for chunk storage and hybrid retrieval paths.

Hybrid retrieval path:

- Implemented in `embedding.rs` as lexical + vector rank fusion (RRF-style) and used by web search handlers.

## 8) Knowledge Graph and Entity Layer

Core extraction/scoring code is under `crates/ferrumyx-kg/src`.

Responsibilities:

- Build relation facts from chunk text and entity context.
- Normalize and upsert evidence-bearing facts.
- Resolve contradictory/conflicting evidence states.
- Compute target-related scores from accumulated graph evidence.

Persistence targets:

- `entities`
- `entity_mentions`
- `kg_facts`
- `kg_conflicts`
- `target_scores`

Repository access in `crates/ferrumyx-db/src/*` supports these flows.

## 9) Ranking and Provider Enrichment

Main entry: `crates/ferrumyx-ranker/src/lib.rs`.

Composition:

- Composite scoring logic in `scorer.rs`.
- External/provider adapters under `providers/` and dedicated provider files.
- Batch refresh and score persistence are integrated with agent tools and web handlers.

Usage surfaces:

- Tool-driven query and autonomous cycle flows.
- Web APIs in `crates/ferrumyx-web/src/handlers/ranker.rs` and `handlers/targets.rs`.

## 10) Molecules Pipeline

Main orchestrator: `crates/ferrumyx-molecules/src/pipeline.rs`.

Pipeline components:

- Structure acquisition (`pdb.rs`).
- Pocket detection (`pocket.rs`).
- Ligand handling/generation (`ligand.rs`).
- Docking pipeline (`docking.rs`).
- ADMET scoring (`admet.rs`).
- Composite molecule scoring (`scoring.rs`).

The agent tooling can invoke this pipeline through `molecule_tool.rs`.

## 11) Storage Architecture

Ferrumyx uses two distinct storage subsystems.

### A) Biomedical corpus and KG: LanceDB

Code: `crates/ferrumyx-db/src/database.rs`, `schema.rs`, repositories.

Characteristics:

- Embedded storage (no external DB service requirement).
- Table creation/init handled in DB initialization paths.
- Vector index creation on chunk embeddings.
- Repository APIs for batch upsert/search/update.

Notable schema behavior:

- Chunks include `embedding` (768) and `embedding_large` (1024) columns.
- Different embedding paths may populate different vector columns.

### B) Runtime/workspace persistence: libSQL (runtime-core)

Code: `crates/ferrumyx-runtime-core/src/db/libsql/*`.

Characteristics:

- Separate from LanceDB biomedical tables.
- Managed via runtime-core migration and DB adapters.
- Used for runtime/workspace-level state and related metadata paths.

## 12) Web/API Architecture

Router: `crates/ferrumyx-web/src/router.rs`.

Includes:

- UI pages (`/`, `/query`, `/targets`, `/ingestion`, `/kg`, `/chat`, etc.).
- API routes (`/api/*`) for ingestion, KG, ranking, search, chat, federation, settings, and metrics.
- SSE endpoint (`/api/events`) and chat event proxy paths.

State and events:

- Shared app state in `state.rs`.
- Streaming support in `sse.rs`.

Major handler groups:

- Query/search: `handlers/query.rs`, `handlers/search.rs`
- Ingestion: `handlers/ingestion.rs`
- KG: `handlers/kg.rs`
- Ranker/targets: `handlers/ranker.rs`, `handlers/targets.rs`
- Chat/lab monitor: `handlers/chat.rs`
- Federation: `handlers/federation.rs`

## 13) Federation Subsystem

Federation contracts and logic:

- Shared schema/contracts: `crates/ferrumyx-common/src/federation.rs`
- Export/validate/sign/merge/trust/lineage: `crates/ferrumyx-db/src/federation.rs`
- Web transport and sync endpoints: `crates/ferrumyx-web/src/handlers/federation.rs`

Capabilities:

- Manifest drafting/validation.
- Package export and artifact digest validation.
- Signature operations and trust key registry operations.
- Merge queue and canonical lineage tracking.
- Snapshot sync and remote publish/pull endpoints.

Implementation caveat:

- Federation control-plane state is file-backed (JSON/JSONL artifact sets and metadata), not a fully normalized relational DB model.

## 14) Performance Design

Performance-sensitive patterns in code include:

- Ingestion lane splitting and bounded concurrent workers.
- Timeout and retry guardrails around network/full-text steps.
- Cache layers for source/full-text/parse/fingerprint flows.
- Batch-first write patterns for chunks/embeddings/facts where possible.
- Background or deferred embedding paths for throughput mode.
- Runtime profile-aware tuning in agent tools.

Reference files:

- `crates/ferrumyx-ingestion/src/pipeline.rs`
- `crates/ferrumyx-ingestion/src/repository.rs`
- `crates/ferrumyx-ingestion/src/embedding.rs`
- `crates/ferrumyx-agent/src/tools/runtime_profile.rs`
- `crates/ferrumyx-web/src/handlers/metrics.rs`

## 15) Extension Playbooks

### Add a new agent tool

1. Implement new module in `crates/ferrumyx-agent/src/tools/`.
2. Export from `tools/mod.rs`.
3. Register in `crates/ferrumyx-agent/src/main.rs`.

### Add a new ingestion source

1. Implement `LiteratureSource` in `crates/ferrumyx-ingestion/src/sources/`.
2. Wire source selection/dispatch in `pipeline.rs`.
3. Add web/agent trigger surfaces if needed.

### Add new ranking/provider signal

1. Add provider module under `crates/ferrumyx-ranker/src/providers/`.
2. Integrate into refresh/materialization path in `lib.rs`.
3. Expose in query outputs and relevant handlers.

### Add a new web endpoint/page

1. Implement handler under `crates/ferrumyx-web/src/handlers/`.
2. Register route in `router.rs`.
3. Add template/static assets if UI-facing.

### Add schema/storage surface

1. Update `crates/ferrumyx-db/src/schema.rs` + table init logic.
2. Add/extend repository accessors.
3. Wire through ingestion/ranking/query layers.

## 16) Testing and Benchmarking Paths

Common validation/benchmark surfaces in-repo:

- Unit/integration tests in crate modules.
- Benchmark helpers in `crates/ferrumyx-web/src/bin/benchmark.rs` and `crates/ferrumyx-web/src/bin/perf_micro.rs`.
- Federation import/helper binary in `crates/ferrumyx-web/src/bin/federation_import.rs`.

Recommended basic checks before merge:

- `cargo check --workspace`
- `cargo test --workspace` (where runtime/resources allow)
- Targeted end-to-end ingestion/query run for changed paths

## 17) Operational Caveats

- LanceDB biomedical corpus and runtime-core libSQL are separate by design; treat them as distinct subsystems.
- Hybrid retrieval in biomedical flow is lexical + vector fusion; it is not equivalent to a dedicated BM25 engine implementation.
- Dual embedding columns exist (`embedding`, `embedding_large`); ensure docs/config match the active backend/model path.
- Some output/log files are runtime-generated and should remain out of source-control workflows unless explicitly needed for diagnostics.

---

For broader conceptual context, see `ARCHITECTURE.md`. For implementation-level details, use this wiki and follow file paths directly.
