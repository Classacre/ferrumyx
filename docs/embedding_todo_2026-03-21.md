# Embedding TODO Sprint (2026-03-21)

## Scope
Harden Ferrumyx embedding usage end-to-end: downstream retrieval behavior, speed-mode clarity, config validation, and regression coverage.

## Task Board

- [x] P0 Add auto embedding speed mode and max-length controls
  Owner: Main agent
  Files: `crates/ferrumyx-agent/src/tools/ingestion_tool.rs`, `crates/ferrumyx-ingestion/src/embedding.rs`, `crates/ferrumyx-ingestion/src/embed/config.rs`
  Implementation: added `auto|fast|balanced|quality` speed mode resolution, mode-aware batch tuning in ingestion, mode-aware max length (`256/384/512`) in Rust-native embed path, and env wiring (`FERRUMYX_EMBED_SPEED_MODE`, `FERRUMYX_EMBED_MAX_LENGTH`).
  Validation: `cargo check -p ferrumyx-agent -p ferrumyx-ingestion`.

- [x] P0 Remove repeated GPU probe overhead in embedding path
  Owner: Main agent
  Files: `crates/ferrumyx-ingestion/src/embedding.rs`
  Implementation: cached `resolve_embed_use_gpu()` and speed-mode resolution with `OnceLock` to avoid repeated `nvidia-smi`/`nvcc`/PowerShell probes per embedding batch.
  Validation: `cargo check -p ferrumyx-ingestion`.

- [x] P1 Add first downstream embedding consumer in ranking workflow
  Owner: Main agent
  Files: `crates/ferrumyx-agent/src/tools/query_tool.rs`
  Implementation: added semantic reranking in `query_targets` using hybrid retrieval evidence and exposed `semantic_rerank` telemetry in tool output.
  Validation: `cargo check -p ferrumyx-agent -p ferrumyx-ranker`.

- [x] P0 Make ingestion embedding fully async-capable with targeted backfill
  Owner: Main agent
  Files: `crates/ferrumyx-agent/src/tools/ingestion_tool.rs`, `crates/ferrumyx-agent/src/tools/embedding_backfill_tool.rs`, `crates/ferrumyx-ingestion/src/pipeline.rs`, `crates/ferrumyx-ingestion/src/repository.rs`
  Implementation: added async embedding backfill mode (`ingestion.performance.embedding_async_backfill` / `FERRUMYX_INGESTION_EMBED_ASYNC_BACKFILL`), made ingestion return inserted paper IDs for precise backfill targeting, and added manual `backfill_embeddings` tool for post-ingest catch-up.
  Validation: `cargo check -p ferrumyx-agent -p ferrumyx-ingestion`.

- [x] P0 Add embedding throughput workload caps
  Owner: Main agent
  Files: `crates/ferrumyx-agent/src/tools/ingestion_tool.rs`, `crates/ferrumyx-ingestion/src/embedding.rs`
  Implementation: added throughput chunk cap control (`embedding.throughput_chunk_cap` / `FERRUMYX_EMBED_THROUGHPUT_MAX_CHUNKS_PER_PAPER`) and enforced capped pending-chunk embedding per paper in throughput mode.
  Validation: `cargo check -p ferrumyx-agent -p ferrumyx-ingestion`.

- [ ] P0 Unify embedding-mode resolution
  Owner: Main agent
  Files: `crates/ferrumyx-agent/src/tools/ingestion_tool.rs`, `crates/ferrumyx-agent/src/tools/autonomous_cycle_tool.rs`, `ferrumyx.example.toml`
  Implementation: ingestion and autonomous-cycle now both use explicit speed-mode + mode-aware batch/max-length resolution; remaining cleanup is centralizing both into one shared helper module.
  Validation: confirm `auto`, `throughput`, `balanced`, and `safe` all resolve deterministically for the same host and config.

- [ ] P0 Enforce backend/model/dimension consistency
  Owner: Main agent
  Files: `crates/ferrumyx-agent/src/config/mod.rs`, `crates/ferrumyx-agent/src/tools/ingestion_tool.rs`
  Implementation: validate that `embedding.backend`, `embedding.embedding_model`, and `embedding.embedding_dim` are compatible before ingestion starts.
  Validation: add a negative-path test for mismatched dimensions and a positive-path test for each supported backend family.

- [ ] P1 Document and wire downstream embedding consumers
  Owner: Main agent
  Files: `ARCHITECTURE.md`, `crates/ferrumyx-db/src/chunks.rs`, `crates/ferrumyx-ranker/src/lib.rs`, `crates/ferrumyx-web/src/handlers/*`
  Implementation: `query_targets` now emits the compact downstream embedding payload (RAG snippet selection, gene links, novelty signals, dedup groups, topic clusters, drift mix, and per-gene numeric features); remaining web/db consumers should read from the same canonical embedding fields in later work.
  Validation: verify each consumer still behaves correctly when embeddings are absent, 768-dim, or 1024-dim.

- [ ] P1 Add embedding-path observability
  Owner: Main agent
  Files: `crates/ferrumyx-ingestion/src/*`, `crates/ferrumyx-web/src/bin/perf_micro.rs`
  Implementation: emit counters for backend choice, batch size, embedding latency, and fallback paths.
  Validation: capture one baseline run per backend and record batch-size/latency deltas in the perf microbenchmark.

- [ ] P2 Tighten novelty/drift semantics
  Owner: Main agent
  Files: `crates/ferrumyx-kg/src/*`, `crates/ferrumyx-ranker/src/*`
  Implementation: define the exact similarity thresholding and drift heuristic for deciding whether a new paper/chunk is redundant, incremental, or novel.
  Validation: create a small fixture set with known duplicates, near-duplicates, and topical drifts; assert the classifier buckets them correctly.

- [ ] P2 Add retrieval regression coverage
  Owner: Main agent
  Files: `crates/ferrumyx-web/tests/*`, `crates/ferrumyx-db/tests/*`
  Implementation: add tests for hybrid retrieval ranking, RAG chunk selection, and embedding-backed reranking behavior.
  Validation: assert the relevant results remain stable across at least one lexical-only query, one semantic query, and one duplicate-heavy query.

## Priority Notes

- P0 work should happen first because it affects correctness of every embedding-backed path.
- P1 work should land next because it closes the loop between the docs, runtime telemetry, and the actual retrieval/ranking consumers.
- P2 work is validation hardening after the control surface is stable.

## Acceptance Criteria

- `ARCHITECTURE.md` explains where embeddings are used and how speed mode is chosen.
- Embedding configuration is deterministic and rejects incompatible combinations.
- Retrieval, reranking, novelty/drift, and dedup consumers are covered by tests or benchmarked smoke checks.
