# Developer Guide

## 1) Adding a new agent tool

1. Create a module in `crates/ferrumyx-agent/src/tools/`.
2. Implement runtime tool trait and `parameters_schema()`.
3. Export in `tools/mod.rs`.
4. Register in `crates/ferrumyx-agent/src/main.rs`.
5. Add web/API exposure if needed.

## 2) Adding a new ingestion source

1. Implement source client under `crates/ferrumyx-ingestion/src/sources/`.
2. Wire source enum/dispatch in `pipeline.rs`.
3. Add source selection controls in web/UI handler if operator-facing.
4. Add tests/benchmarks for source reliability and parse quality.

## 3) Adding/altering embedding backends

1. Add backend enum support in `embedding.rs`.
2. Ensure dimension compatibility with storage schema.
3. Add runtime gating for optional dependencies.
4. Update ingestion tool auto-selection behavior.
5. Document in configuration pages.

## 4) Extending ranking/provider signals

1. Add provider module in `crates/ferrumyx-ranker/src/providers/`.
2. Integrate refresh/materialization in `lib.rs`.
3. Map signal into composite scorer (`scorer.rs`).
4. Expose diagnostics in ranker/targets handlers.

## 5) Database schema changes

1. Update schema constants/types in `crates/ferrumyx-db/src/schema.rs`.
2. Update table initialization logic in `database.rs`.
3. Extend repositories in `crates/ferrumyx-db/src/*`.
4. Update ingestion/ranker code paths that write/read changed fields.
5. Add migration notes in docs/wiki pages.

## 6) Adding a web API endpoint

1. Add handler in `crates/ferrumyx-web/src/handlers/`.
2. Register route in `crates/ferrumyx-web/src/router.rs`.
3. Define request/response structs using serde.
4. Add frontend integration (template/static JS) if needed.

## 7) Federation extension points

- Schema and validation extensions: `crates/ferrumyx-common/src/federation.rs`
- Packaging/trust/lineage: `crates/ferrumyx-db/src/federation.rs`
- Transport and sync APIs: `crates/ferrumyx-web/src/handlers/federation.rs`

## 8) Testing workflow

Recommended local checks:

- `cargo check --workspace`
- `cargo test --workspace`
- targeted run of ingestion/query endpoints for changed domains

For performance-sensitive changes:

- use benchmark binaries in `crates/ferrumyx-web/src/bin/`
- compare ingestion latency and embedding throughput under representative workloads

## 9) GitHub Wiki maintenance workflow

This repo contains GitHub Wiki-ready pages under `github-wiki/`.

To publish updates:

1. `git clone https://github.com/Classacre/ferrumyx.wiki.git`
2. Copy updated Markdown files from `github-wiki/`.
3. Commit and push in the `ferrumyx.wiki` repo.

Keep page names stable when possible to avoid breaking external links.
