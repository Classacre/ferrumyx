# CLI Reference

This page covers executable CLIs and editable arguments currently present in code.

## 1) `ferrumyx-runtime-core` CLI

Source: `crates/ferrumyx-runtime-core/src/cli/mod.rs`

### Global flags

- `--cli-only`
- `--no-db`
- `-m, --message <TEXT>`
- `-c, --config <PATH>`
- `--no-onboard`

### Top-level commands

- `run`
- `onboard [--skip-auth] [--channels-only | --provider-only]`
- `config ...`
- `tool ...`
- `registry ...`
- `mcp ...`
- `memory ...`
- `pairing ...`
- `service ...`
- `doctor`
- `status`
- `completion ...`
- internal hidden: `worker`, `claude-bridge`

### Examples

- `ferrumyx-runtime-core run`
- `ferrumyx-runtime-core config list`
- `ferrumyx-runtime-core mcp list`
- `ferrumyx-runtime-core memory search "KRAS"`
- `ferrumyx-runtime-core completion --help`

### Internal hidden command arguments

`worker`:

- `--job-id <UUID>`
- `--orchestrator-url <URL>` (default `http://host.docker.internal:50051`)
- `--max-iterations <N>` (default `50`)

`claude-bridge`:

- `--job-id <UUID>`
- `--orchestrator-url <URL>`
- `--max-turns <N>` (default `50`)
- `--model <MODEL>` (default `sonnet`)

## 2) `ferrumyx` (agent binary)

Source: `crates/ferrumyx-agent/src/main.rs`

The `ferrumyx` binary is primarily configuration-driven rather than argument-driven. It reads runtime behavior from config/env and starts the agent + web stack.

Common launch:

- `cargo run --release --bin ferrumyx`

Useful runtime environment overrides:

- `FERRUMYX_BIND` (agent bind override)
- `FERRUMYX_DISABLE_REPL`
- `FERRUMYX_CONFIG` (config file path)
- LLM/provider keys and failover variables

## 3) `ferrumyx-web` binary

Source: `crates/ferrumyx-web/src/main.rs`

Launch:

- `cargo run -p ferrumyx-web`

Editable runtime argument surface is environment-based:

- `FERRUMYX_WEB_ADDR` (default `127.0.0.1:3001`)

## 4) Agent tool parameter schemas (chat/agentic invocation)

Tool schemas are defined in `crates/ferrumyx-agent/src/tools/*` via `parameters_schema()`.

### `ingest_literature`

File: `ingestion_tool.rs`

Parameters:

- `gene` (required)
- `cancer_type` (required)
- `mutation` (optional)
- `max_results` (optional)
- `idle_timeout_secs` (optional)
- `max_runtime_secs` (optional)

### `query_targets`

File: `query_tool.rs`

Parameters:

- `query_text` (required)
- `cancer_code` (optional)
- `gene_symbol` (optional)
- `mutation` (optional)
- `max_results` (optional)

### `run_autonomous_cycle`

File: `autonomous_cycle_tool.rs`

Parameters:

- `gene` (required)
- `cancer_type` (required)
- `query_text` (optional)
- `cancer_code` (optional)
- `mutation` (optional)
- `max_results` (optional)
- `source_profile` (`auto|fast|full`)
- `max_cycles` (optional)
- `improvement_threshold` (optional)
- `adaptive_broadening` (optional)
- `novelty_pressure_mode` (`off|auto|aggressive`)
- `cycle_timeout_secs` (optional)

### `backfill_embeddings`

File: `embedding_backfill_tool.rs`

Parameters:

- `paper_ids` (optional UUID array)
- `scan_limit` (optional integer)

### Other tool schemas

Also defined in:

- `lab_autoresearch_tool.rs`
- `lab_planner_tool.rs`
- `lab_retriever_tool.rs`
- `lab_validator_tool.rs`
- `lab_run_status_tool.rs`
- `provider_refresh_tool.rs`
- `scoring_tool.rs`
- `molecule_tool.rs`
- `system_command_tool.rs`
- `workflow_status_tool.rs`

## 5) CLI help best practice

For the most accurate argument list in your local build, always run:

- `ferrumyx-runtime-core --help`
- `ferrumyx-runtime-core <subcommand> --help`

This ensures docs and binary stay aligned when upstream command trees evolve.
