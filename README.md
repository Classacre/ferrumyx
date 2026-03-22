# Ferrumyx

<div align="center">
  <img src="crates/ferrumyx-web/static/logo.svg" alt="Ferrumyx Logo" width="200"/>
</div>

<div align="center">
  <a href="https://colab.research.google.com/github/Classacre/ferrumyx/blob/main/ferrumyx_colab.ipynb">
    <img src="https://colab.research.google.com/assets/colab-badge.svg" alt="Open In Colab"/>
  </a>
</div>

**Open-source autonomous oncology discovery system built in Rust.**

Ferrumyx is an agentic platform for literature-driven target discovery and downstream molecular exploration. It combines autonomous ingestion, biomedical extraction, graph-backed evidence modeling, target ranking, and web/agent interfaces in a single Rust workspace.

## What Ferrumyx Does

- Ingests and deduplicates biomedical literature from multiple sources.
- Extracts entities and evidence relations from text.
- Builds a queryable evidence graph backed by embedded storage.
- Produces ranked target outputs using multi-signal scoring.
- Supports downstream molecular steps (structure, pockets, ligand/docking flow).
- Exposes interactive workflows through both agent tools and web APIs.
- Supports federated package export/validation/signing/sync for shared knowledge distribution.

## Implementation Highlights

- **Rust-native architecture:** Core pipeline, storage access, orchestration, ranking, and web server are all implemented in Rust.
- **Agent-first orchestration:** `ferrumyx-agent` registers domain tools and runs autonomous cycles over ingestion, scoring, retrieval, and validation.
- **Embedded data layer:** Primary biomedical corpus is stored in LanceDB; runtime/workspace persistence is handled separately by runtime-core DB support.
- **Hybrid retrieval:** Query-time retrieval combines lexical and vector signals and returns evidence-rich responses.
- **Operational focus:** Includes performance telemetry, batched ingestion paths, background embedding backfill, and structured run monitoring.

## Repository Layout

| Crate | Purpose |
|---|---|
| `crates/ferrumyx-agent` | Agent entrypoint, tool registration, autonomous and lab workflows |
| `crates/ferrumyx-ingestion` | Literature ingestion, chunking, full-text flow, embeddings |
| `crates/ferrumyx-kg` | Entity/relation extraction, KG update and scoring primitives |
| `crates/ferrumyx-ranker` | Target ranking and provider-backed enrichment logic |
| `crates/ferrumyx-molecules` | Structure/pocket/ligand/docking pipeline components |
| `crates/ferrumyx-db` | LanceDB schema, repositories, and federation persistence |
| `crates/ferrumyx-web` | Axum web UI and API handlers |
| `crates/ferrumyx-runtime` | Runtime adapter layer used by the agent stack |
| `crates/ferrumyx-runtime-core` | Shared runtime-core infrastructure used by the system |
| `crates/ferrumyx-common` | Shared schema/types and cross-crate contracts |

## Interfaces

- **Agent runtime:** `cargo run --release --bin ferrumyx`
- **Web app/API:** `cargo run -p ferrumyx-web`

## Quick Start

```powershell
# Optional (Windows): ensure protoc is available for build dependencies
$env:PROTOC = "C:\protoc\bin\protoc.exe"

# Start helper
.\start.ps1

# Or run components manually
cargo run --release --bin ferrumyx
cargo run -p ferrumyx-web
```

## Documentation

- High-level architecture: [ARCHITECTURE.md](ARCHITECTURE.md)
- In-depth implementation wiki: [docs/WIKI.md](docs/WIKI.md)

## License

Apache-2.0 OR MIT
