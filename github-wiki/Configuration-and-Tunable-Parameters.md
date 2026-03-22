# Configuration and Tunable Parameters

This page documents editable parameters in Ferrumyx: config file sections, settings API payload fields, and runtime environment variables.

## 1) Configuration sources and precedence

Ferrumyx reads configuration from:

1. `ferrumyx.toml` (or file selected by `FERRUMYX_CONFIG`)
2. Runtime environment variables (`FERRUMYX_*`)
3. Settings API writes (`/api/settings`) that synchronize values into runtime config/env

Primary code paths:

- Config loader: `crates/ferrumyx-agent/src/config/mod.rs`
- Settings read/write model: `crates/ferrumyx-web/src/handlers/settings.rs`

## 2) Editable parameter surfaces

### A) Settings UI / Settings API fields

`SettingsSaveRequest` (`handlers/settings.rs`) includes categories such as:

- LLM mode/backends/models
- Embedding backend/model/base URL
- Ingestion defaults and concurrency/timeouts
- Full-text cache and fingerprint cache controls
- Sci-Hub controls
- Phase-4 ranking/provider refresh controls
- Federation auth/trust/sync/HF controls
- Graph rendering limits and presets
- Provider endpoint base URLs and API timeout knobs

Use endpoint:

- `GET /api/settings`
- `POST /api/settings`

### B) Tool-level arguments (agentic calls)

From `parameters_schema()` in `crates/ferrumyx-agent/src/tools/*`:

- `ingest_literature`: gene, cancer_type, mutation, max_results, idle/max runtime caps
- `query_targets`: query_text, cancer_code, gene_symbol, mutation, max_results
- `run_autonomous_cycle`: cycle count, source profile, thresholds, adaptive toggles, timeout
- `backfill_embeddings`: paper_ids, scan_limit
- plus lab/scoring/provider/molecule/system tools

### C) Direct runtime env variables

Large set of `FERRUMYX_*` flags exist. Key high-impact groups are below.

## 3) High-impact runtime variable groups

## 3.1 LLM and provider selection

Examples:

- `FERRUMYX_LLM_FAILOVER_ORDER`
- `FERRUMYX_LLM_FAILOVER_COOLDOWN_SECS`
- `FERRUMYX_LLM_FAILOVER_FAILURE_THRESHOLD`
- `FERRUMYX_OPENAI_API_KEY`
- `FERRUMYX_ANTHROPIC_API_KEY`
- `FERRUMYX_GEMINI_API_KEY`
- `FERRUMYX_COMPAT_API_KEY`
- `FERRUMYX_COMPAT_CACHED_CHAT`

## 3.2 Ingestion throughput and reliability

Examples:

- `FERRUMYX_INGESTION_DEFAULT_MAX_RESULTS`
- `FERRUMYX_INGESTION_PERF_MODE`
- `FERRUMYX_INGESTION_SOURCE_TIMEOUT_SECS`
- `FERRUMYX_INGESTION_FULLTEXT_STEP_TIMEOUT_SECS`
- `FERRUMYX_INGESTION_FULLTEXT_TOTAL_TIMEOUT_SECS`
- `FERRUMYX_INGESTION_FULLTEXT_PREFETCH_WORKERS`
- `FERRUMYX_PAPER_PROCESS_WORKERS`
- `FERRUMYX_INGESTION_SOURCE_MAX_INFLIGHT`
- `FERRUMYX_INGESTION_SOURCE_RETRIES`

## 3.3 Cache and dedup controls

Examples:

- `FERRUMYX_INGESTION_SOURCE_CACHE_ENABLED`
- `FERRUMYX_INGESTION_SOURCE_CACHE_TTL_SECS`
- `FERRUMYX_FULLTEXT_NEGATIVE_CACHE_ENABLED`
- `FERRUMYX_FULLTEXT_NEGATIVE_CACHE_TTL_SECS`
- `FERRUMYX_FULLTEXT_SUCCESS_CACHE_ENABLED`
- `FERRUMYX_FULLTEXT_SUCCESS_CACHE_TTL_SECS`
- `FERRUMYX_CHUNK_FINGERPRINT_CACHE_ENABLED`
- `FERRUMYX_CHUNK_FINGERPRINT_CACHE_TTL_SECS`
- `FERRUMYX_CHUNK_FINGERPRINT_SCOPE`
- `FERRUMYX_STRICT_FUZZY_DEDUP`

## 3.4 Embedding behavior and performance

Examples:

- `FERRUMYX_INGESTION_ENABLE_EMBEDDINGS`
- `FERRUMYX_EMBED_SPEED_MODE`
- `FERRUMYX_EMBED_FAST_MODEL`
- `FERRUMYX_EMBED_CACHE_DIR`
- `FERRUMYX_EMBED_AUTO_FASTEMBED`
- `FERRUMYX_INGESTION_EMBED_ASYNC_BACKFILL`
- `FERRUMYX_INGESTION_EMBED_GLOBAL_BATCH`
- `FERRUMYX_EMBED_THROUGHPUT_MAX_CHUNKS_PER_PAPER`
- `FERRUMYX_EMBED_MAX_LENGTH`

## 3.5 Query-time semantic rerank/downstream embedding payload

Examples:

- `FERRUMYX_QUERY_SEMANTIC_RERANK`
- `FERRUMYX_QUERY_SEMANTIC_TOPK`
- `FERRUMYX_QUERY_SEMANTIC_WEIGHT`
- `FERRUMYX_QUERY_DOWNSTREAM_EMBEDDING`

## 3.6 Sci-Hub/full-text fallback controls

Examples:

- `FERRUMYX_SCIHUB_DOMAINS`
- `FERRUMYX_SCIHUB_REQUEST_TIMEOUT_SECS`
- `FERRUMYX_SCIHUB_DOMAIN_PARALLELISM`
- `FERRUMYX_SCIHUB_DOMAIN_COOLDOWN_SECS`
- `FERRUMYX_SCIHUB_DEFER_MS`
- `FERRUMYX_SCIHUB_ADAPTIVE_ENABLED`
- `FERRUMYX_SCIHUB_ADAPTIVE_FAIL_STREAK`
- `FERRUMYX_SCIHUB_ADAPTIVE_BACKOFF_SECS`
- `FERRUMYX_SCIHUB_ADAPTIVE_PROBE_EVERY`
- `FERRUMYX_SCIHUB_ADAPTIVE_MIN_STEP_TIMEOUT_SECS`

## 3.7 Ranker/provider refresh controls

Examples:

- `FERRUMYX_PHASE4_PROVIDER_REFRESH_ADAPTIVE_ENABLED`
- `FERRUMYX_PHASE4_PROVIDER_REFRESH_BASE_INTERVAL_SECS`
- `FERRUMYX_PHASE4_PROVIDER_REFRESH_MIN_INTERVAL_SECS`
- `FERRUMYX_PHASE4_PROVIDER_REFRESH_MAX_INTERVAL_SECS`
- `FERRUMYX_PHASE4_PROVIDER_REFRESH_STALE_FORCE_AFTER_SECS`
- `FERRUMYX_PHASE4_BG_REFRESH_ENABLED`
- `FERRUMYX_PHASE4_BG_REFRESH_INTERVAL_SECS`
- `FERRUMYX_PHASE4_BG_REFRESH_MAX_GENES`
- `FERRUMYX_PHASE4_BG_REFRESH_BATCH_SIZE`
- `FERRUMYX_PHASE4_BG_REFRESH_RETRIES`

## 3.8 Federation security and sync controls

Examples:

- `FERRUMYX_FED_AUTH_ENABLED`
- `FERRUMYX_FED_READ_TOKEN`
- `FERRUMYX_FED_WRITE_TOKEN`
- `FERRUMYX_FED_REPLAY_REQUIRED`
- `FERRUMYX_FED_REPLAY_WINDOW_SECS`
- `FERRUMYX_FED_REQUIRE_SIGNATURE_FOR_QUEUE`
- `FERRUMYX_FED_AUDIT_LOG_PATH`
- `FERRUMYX_FED_DEFAULT_REMOTE_BASE_URL`
- `FERRUMYX_FED_NODE_PUBLIC_BASE_URL`
- `FERRUMYX_FED_SYNC_CHUNK_BYTES`
- `FERRUMYX_FED_SYNC_TIMEOUT_SECS`
- `FERRUMYX_FED_PULL_AUTO_SUBMIT`
- `FERRUMYX_FED_HF_ENABLED`
- `FERRUMYX_FED_HF_REPO_ID`
- `FERRUMYX_FED_HF_SNAPSHOTS_PREFIX`
- `FERRUMYX_FED_HF_REVISION`
- `FERRUMYX_FED_HF_TIMEOUT_SECS`
- `FERRUMYX_FED_HF_PULL_ROOT`
- `FERRUMYX_FED_HF_TOKEN`

## 4) Configuration in TOML

Primary config example: `ferrumyx.example.toml`.

Editable sections include:

- `[llm]` and backend-specific model/API settings
- `[embedding]` backend/model/dimension/base URL
- `[ingestion]` defaults and source/perf controls
- provider/federation sections aligned with settings UI

## 5) Practical tuning recipes

### Throughput-focused ingestion

- Increase source/full-text worker counts moderately.
- Keep embedding in fast mode and enable global batch.
- Reduce per-run `max_results` when sources are noisy.

### Stability-focused runs

- Lower in-flight workers and retries to avoid cascading failures.
- Use stricter timeout budgets and cache longer.
- Keep autonomous cycle `max_cycles` bounded.

### Federation-safe deployment

- Enable auth + replay required.
- Use separate read/write tokens.
- Require signed queue submissions.
- Persist and monitor audit log.

## 6) Where to inspect exact field names

- Settings payload model and save/read flow:
  - `crates/ferrumyx-web/src/handlers/settings.rs`
- Runtime defaults and env resolution:
  - `crates/ferrumyx-agent/src/tools/ingestion_tool.rs`
  - `crates/ferrumyx-agent/src/main.rs`
- Embedding speed/auto-backend logic:
  - `crates/ferrumyx-ingestion/src/embedding.rs`
  - `crates/ferrumyx-agent/src/tools/ingestion_tool.rs`
