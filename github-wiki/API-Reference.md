# API Reference

Router source of truth: `crates/ferrumyx-web/src/router.rs`

All endpoints below are mounted on the Ferrumyx web server.

## 1) Core discovery and evidence APIs

### `GET /api/search`

Hybrid lexical/vector search with KG aggregation.

Query params (`SearchQuery` in `handlers/search.rs`):

- `q` (string, required in practice)
- `limit` (int, default 20, clamped 1..100)
- `cancer_type` (optional string)

Response (`HybridSearchResponse`):

- `query`
- `results[]` (`paper_id`, `title`, `chunk_text`, `similarity`, `section_type`, `source`)
- `kg_facts[]`
- `total`

### `GET /api/targets`

Query params (`TargetFilter` in `handlers/targets.rs`):

- `cancer` (optional)
- `gene` (optional)
- `tier` (optional)
- `page` (optional, used as bounded limit in API handler)

Response: array of `ApiTarget`.

### `GET /api/targets/{gene}`

Query params:

- `cancer` (optional)

Response: `ApiTargetDetail`.

### `GET /api/kg`

Query params (`KgFilter` in `handlers/kg.rs`):

- `gene`, `q`, `predicate`, `confidence_tier`, `max_papers`, `view`, `lens`, `preset`, `source`, `target`, `hops`, `expanded`

Response: array of `ApiKgFact`.

### `GET /api/kg/stats`

No params.

Response: `ApiKgStats`.

### `GET /api/entities/suggest`

Query params (`EntitySuggestQuery`):

- `q` (optional string)
- `limit` (optional int, clamped 5..30)

Response: array of `{ value }`.

## 2) Ranking and dependency APIs

### `GET /api/ranker/score`

Query params (`RankerFilter` in `handlers/ranker.rs`):

- `gene` (required logically)
- `cancer_type` (optional)

Response: `RankedTarget`.

### `GET /api/ranker/top`

Query params:

- `cancer_type` (optional)
- `limit` (optional int, clamped 1..100)

Response: array of `RankedTarget`.

### `GET /api/ranker/stats`

Response: `RankerStats`.

### `GET /api/depmap/gene`

Query params (`DepMapFilter`):

- `gene` (optional but expected)
- `cancer_type` (optional but expected)

Response: `DepMapGeneStats`.

### `GET /api/depmap/celllines`

Query params: same as above.

Response: array of `DepMapCellLine` (currently may be empty depending on mode/data).

## 3) Ingestion/NER/molecule APIs

### `POST /ingestion/run`

Form body (`IngestionForm` in `handlers/ingestion.rs`):

- `gene` (required)
- `mutation` (optional)
- `cancer` (required)
- `max_results` (optional)
- source toggles (`src_pubmed`, `src_europepmc`, `src_biorxiv`, `src_medrxiv`, `src_arxiv`, `src_clinicaltrials`, `src_crossref`, `src_semanticscholar`)
- embedding fields (`embed_backend`, `embed_api_key`, `embed_model`)
- `enable_scihub`

Response: HTML page (not JSON).

### `GET /api/ner/stats`

Returns NER stats payload.

### `POST /api/ner/extract`

JSON body based on `NerForm` in `handlers/ner.rs`.

### `POST /api/molecules/run`

JSON body (`MolRunParams` in `handlers/molecules.rs`):

- `uniprot_id` (string)

Response:

- `status: success|error`
- `results` or `error`

## 4) Chat APIs

### `POST /api/chat`

JSON body (`ChatRequest` in `handlers/chat.rs`):

- `message` (string)
- `thread_id` (optional string)

### `GET /api/chat/threads`

Returns threads payload.

### `POST /api/chat/thread/new`

Creates thread.

### `GET /api/chat/history`

Query params:

- `thread_id`
- `limit`

### `GET /api/chat/events`

Proxy stream for chat events.

### `GET /api/chat/lab-monitor`

Lab run monitoring payload.

## 5) Settings and metrics APIs

### `GET /api/settings`

Returns `SettingsView` in `handlers/settings.rs` with large editable config surface.

### `POST /api/settings`

Accepts `SettingsSaveRequest` with runtime/provider/ingestion/ranker/federation tuning fields.

### `GET /api/metrics/perf`

Returns ingestion/run performance telemetry (`PerfResponse`).

## 6) Federation APIs

Federation handlers are in `handlers/federation.rs`.

### Schema and manifest

- `GET /api/federation/schema`
- `POST /api/federation/manifest/draft` (`ManifestDraftRequest`)
- `POST /api/federation/manifest/validate` (`ContributionManifest`)

### Package lifecycle

- `POST /api/federation/package/export` (`PackageExportRequest`)
- `POST /api/federation/package/validate` (`PackageValidationRequest`)
- `POST /api/federation/package/sign` (`PackageSignRequest`)

### Merge queue and lineage

- `POST /api/federation/merge/submit` (`MergeSubmitRequest`)
- `GET /api/federation/merge/queue`
- `POST /api/federation/merge/decide` (`MergeDecisionRequest`)
- `GET /api/federation/canonical/lineage`

### Trust registry

- `GET /api/federation/trust/list`
- `POST /api/federation/trust/upsert` (`TrustKeyUpsertRequest`)
- `POST /api/federation/trust/revoke` (`TrustKeyRevokeRequest`)

### Sync transport

- `GET /api/federation/sync/index`
- `GET /api/federation/sync/snapshot` (`dataset_id`, `snapshot_id`)
- `GET /api/federation/sync/artifact` (`dataset_id`, `snapshot_id`, `relative_path`, `offset`, `max_bytes`)
- `POST /api/federation/sync/plan` (`SyncPlanRequest`)
- `POST /api/federation/sync/pull` (`SyncPullRequest`)
- `POST /api/federation/sync/push` (`SyncPushRequest`)

### Hugging Face bridge

- `GET /api/federation/hf/status`
- `POST /api/federation/hf/publish` (`HfPublishRequest`)
- `POST /api/federation/hf/pull` (`HfPullRequest`)

## 7) API usage examples

### Hybrid search

```bash
curl "http://127.0.0.1:3001/api/search?q=KRAS%20G12D&limit=10"
```

### Ranker top targets

```bash
curl "http://127.0.0.1:3001/api/ranker/top?cancer_type=PAAD&limit=20"
```

### Chat submit

```bash
curl -X POST "http://127.0.0.1:3001/api/chat" \
  -H "content-type: application/json" \
  -d '{"message":"Rank KRAS targets in pancreatic cancer"}'
```

### Molecule run

```bash
curl -X POST "http://127.0.0.1:3001/api/molecules/run" \
  -H "content-type: application/json" \
  -d '{"uniprot_id":"P01116"}'
```

### Federation sync plan

```bash
curl -X POST "http://127.0.0.1:3001/api/federation/sync/plan" \
  -H "content-type: application/json" \
  -d '{"remote_base_url":"https://node.example.org"}'
```

## 8) Notes on authentication/security

- Federation routes can enforce bearer auth and replay checks depending on runtime settings.
- Chat endpoints proxy to runtime gateway and use local authorization between web and gateway.
- Settings endpoint mutates runtime/config state and should be protected at deployment layer.
