# Workflows and Architecture

## Runtime topology

Ferrumyx is a multi-crate Rust workspace.

- `ferrumyx-agent`: orchestration and tool registration
- `ferrumyx-ingestion`: literature retrieval, full-text handling, chunking, embedding
- `ferrumyx-kg`: extraction, relation logic, score-relevant evidence transforms
- `ferrumyx-ranker`: target scoring and provider enrichment
- `ferrumyx-molecules`: structure/pocket/ligand/docking/admet components
- `ferrumyx-db`: LanceDB repositories and federation persistence
- `ferrumyx-web`: pages + APIs + SSE
- `ferrumyx-runtime-core`: runtime-core facilities and CLI

## Startup execution flow

From `crates/ferrumyx-agent/src/main.rs`:

1. Load config and runtime/provider settings.
2. Initialize database and repositories.
3. Build LLM provider chain and failover behavior.
4. Register all runtime tools.
5. Start agent loop.
6. Start web router and event streaming.

## End-to-end discovery workflow

Primary path (ingestion -> evidence -> ranking):

1. Collect paper candidates from configured sources.
2. Perform identity dedup (DOI/PMID/PMCID/title + fuzzy guards).
3. Upsert papers.
4. Retrieve/parse full text where available.
5. Chunk documents.
6. Extract entities + relations -> KG facts.
7. Generate/write embeddings for chunk rows.
8. Run ranking and return evidence-backed target outputs.

Main implementation files:

- Pipeline: `crates/ferrumyx-ingestion/src/pipeline.rs`
- Ingestion DB ops: `crates/ferrumyx-ingestion/src/repository.rs`
- Embedding/hybrid search: `crates/ferrumyx-ingestion/src/embedding.rs`
- KG extraction/scoring: `crates/ferrumyx-kg/src/*.rs`
- Ranker: `crates/ferrumyx-ranker/src/lib.rs`, `scorer.rs`

## Agent tool workflows

Important tools under `crates/ferrumyx-agent/src/tools/`:

- `ingestion_tool.rs`: one-shot ingestion run with watchdog/timeouts.
- `query_tool.rs`: ranked query response (supports semantic rerank/downstream embedding blocks).
- `autonomous_cycle_tool.rs`: multi-cycle ingest/score/rank with adaptive broadening and plateau stop logic.
- `embedding_backfill_tool.rs`: targeted or scanned embedding backfill.
- `provider_refresh_tool.rs`: provider signal refresh.
- `lab_*` tools: planner/retriever/validator/coordinator/status for autonomous research runs.

## Storage architecture (important distinction)

Ferrumyx uses two storage subsystems:

1. **Biomedical corpus + KG**: LanceDB (`crates/ferrumyx-db/src/*`).
2. **Runtime/workspace state**: runtime-core DB (libSQL path in `crates/ferrumyx-runtime-core/src/db/libsql/*`).

Do not treat them as one DB layer; they serve different responsibilities.

## Web and API architecture

Router: `crates/ferrumyx-web/src/router.rs`

- UI routes (`/`, `/query`, `/targets`, `/ingestion`, `/kg`, `/molecules`, `/settings`, `/chat`, ...)
- API routes (`/api/*`) for ingestion, ranker, KG, depmap, molecules, federation, chat, settings, metrics
- Streaming endpoint: `/api/events`

State and eventing:

- `crates/ferrumyx-web/src/state.rs`
- `crates/ferrumyx-web/src/sse.rs`

## Federation workflow

Federation contract and implementation:

- Schema contract: `crates/ferrumyx-common/src/federation.rs`
- Manifest/package/trust/merge logic: `crates/ferrumyx-db/src/federation.rs`
- HTTP/sync/hf routes: `crates/ferrumyx-web/src/handlers/federation.rs`

Typical federation flow:

1. Draft manifest
2. Validate manifest
3. Export package
4. Validate package
5. Optional signing
6. Submit to merge queue / decide merge
7. Sync/push/pull snapshots
