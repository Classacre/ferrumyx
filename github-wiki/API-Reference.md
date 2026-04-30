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

**Advanced example with filtering:**
```bash
curl "http://127.0.0.1:3001/api/search?q=KRAS%20G12D&limit=10&cancer_type=PAAD&source=pubmed"
```

**Python example:**
```python
import requests

def search_literature(query, cancer_type=None, limit=20):
    params = {'q': query, 'limit': limit}
    if cancer_type:
        params['cancer_type'] = cancer_type

    response = requests.get('http://127.0.0.1:3001/api/search', params=params)
    response.raise_for_status()
    return response.json()

# Example usage
results = search_literature("KRAS G12D pancreatic cancer", cancer_type="PAAD")
for result in results.get('results', []):
    print(f"Title: {result['title']}")
    print(f"Similarity: {result['similarity']:.3f}")
    print("---")
```

### Ranker top targets

```bash
curl "http://127.0.0.1:3001/api/ranker/top?cancer_type=PAAD&limit=20"
```

**Response example:**
```json
[
  {
    "gene_symbol": "KRAS",
    "cancer_type": "PAAD",
    "composite_score": 9.2,
    "component_scores": {
      "literature_evidence": 8.5,
      "genetic_alteration": 9.8,
      "clinical_trials": 7.1
    },
    "evidence_count": 145,
    "last_updated": "2024-01-15T10:30:00Z"
  }
]
```

**JavaScript example:**
```javascript
async function getTopTargets(cancerType, limit = 20) {
    const params = new URLSearchParams({
        cancer_type: cancerType,
        limit: limit.toString()
    });

    const response = await fetch(`http://127.0.0.1:3001/api/ranker/top?${params}`);
    if (!response.ok) {
        throw new Error(`API error: ${response.status}`);
    }

    return await response.json();
}

// Usage
getTopTargets('PAAD', 10).then(targets => {
    targets.forEach(target => {
        console.log(`${target.gene_symbol}: ${target.composite_score}`);
    });
});
```

### Chat submit

```bash
curl -X POST "http://127.0.0.1:3001/api/chat" \
  -H "content-type: application/json" \
  -d '{"message":"Rank KRAS targets in pancreatic cancer"}'
```

**With thread management:**
```bash
# Create new thread
curl -X POST "http://127.0.0.1:3001/api/chat/thread/new" \
  -H "content-type: application/json" \
  -d '{"title":"KRAS Research Analysis"}'

# Submit message to thread
curl -X POST "http://127.0.0.1:3001/api/chat" \
  -H "content-type: application/json" \
  -d '{"message":"Analyze KRAS mutations in PDAC","thread_id":"550e8400-e29b-41d4-a716-446655440000"}'
```

**Streaming response handling (WebSocket):**
```javascript
const ws = new WebSocket('ws://127.0.0.1:3001/ws/chat');

ws.onopen = () => {
    ws.send(JSON.stringify({
        type: 'chat_message',
        message: 'Find KRAS targets in lung cancer',
        thread_id: '550e8400-e29b-41d4-a716-446655440000'
    }));
};

ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    if (data.type === 'chat_response') {
        console.log('Response:', data.content);
    } else if (data.type === 'chat_stream') {
        process.stdout.write(data.chunk); // Stream chunks
    }
};
```

### Molecule run

```bash
curl -X POST "http://127.0.0.1:3001/api/molecules/run" \
  -H "content-type: application/json" \
  -d '{"uniprot_id":"P01116"}'
```

**Advanced molecular analysis:**
```bash
curl -X POST "http://127.0.0.1:3001/api/molecules/run" \
  -H "content-type: application/json" \
  -d '{
    "uniprot_id":"P01116",
    "analysis_type":"structure_prediction",
    "include_docking":true,
    "ligands":["CHEMBL12345", "CHEMBL67890"]
  }'
```

**Response with detailed results:**
```json
{
  "status": "success",
  "job_id": "550e8400-e29b-41d4-a716-446655440001",
  "results": {
    "structure_url": "/api/molecules/structure/P01116",
    "binding_sites": [
      {
        "residue": "GLY12",
        "confidence": 0.95,
        "ligands": ["CHEMBL12345"]
      }
    ],
    "docking_scores": [
      {
        "ligand": "CHEMBL12345",
        "binding_energy": -8.2,
        "kd": "1.5nM"
      }
    ]
  }
}
```

### Federation sync plan

```bash
curl -X POST "http://127.0.0.1:3001/api/federation/sync/plan" \
  -H "content-type: application/json" \
  -d '{"remote_base_url":"https://node.example.org"}'
```

**Complete federation workflow:**
```bash
# 1. Create manifest
curl -X POST "http://127.0.0.1:3001/api/federation/manifest/draft" \
  -H "content-type: application/json" \
  -d '{"dataset_name":"kras-study-2024","description":"KRAS mutation analysis"}'

# 2. Validate manifest
curl -X POST "http://127.0.0.1:3001/api/federation/manifest/validate" \
  -H "content-type: application/json" \
  -d @manifest.json

# 3. Export package
curl -X POST "http://127.0.0.1:3001/api/federation/package/export" \
  -H "content-type: application/json" \
  -d '{"manifest_id":"550e8400-e29b-41d4-a716-446655440002"}'

# 4. Sign and sync
curl -X POST "http://127.0.0.1:3001/api/federation/sync/push" \
  -H "content-type: application/json" \
  -d '{"remote_url":"https://federation.example.org","package_id":"550e8400-e29b-41d4-a716-446655440003"}'
```

## 8) Error handling and status codes

### Common HTTP Status Codes

| Status Code | Description | Common Causes |
|-------------|-------------|---------------|
| 200 | Success | Request completed successfully |
| 400 | Bad Request | Invalid parameters, malformed JSON |
| 401 | Unauthorized | Missing or invalid authentication |
| 403 | Forbidden | Insufficient permissions |
| 404 | Not Found | Resource or endpoint doesn't exist |
| 429 | Too Many Requests | Rate limit exceeded |
| 500 | Internal Server Error | Server-side error |
| 503 | Service Unavailable | Service temporarily unavailable |

### Error Response Format

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid query parameter: limit must be between 1 and 100",
    "details": {
      "parameter": "limit",
      "provided_value": 150,
      "allowed_range": {"min": 1, "max": 100}
    },
    "request_id": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

### Rate Limiting

#### Rate Limit Headers

All API responses include rate limit information:

```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 950
X-RateLimit-Reset: 1640995200
X-RateLimit-Retry-After: 60
```

#### Rate Limit Tiers

| User Type | Requests per Hour | Burst Limit |
|-----------|-------------------|-------------|
| Anonymous | 100 | 10 |
| Registered | 1000 | 100 |
| Premium | 10000 | 1000 |

#### Rate Limit Exceeded Response

```json
{
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "API rate limit exceeded",
    "details": {
      "limit": 1000,
      "remaining": 0,
      "reset_time": "2024-01-15T12:00:00Z",
      "retry_after_seconds": 3600
    }
  }
}
```

### Request Validation

#### Input Validation Rules

- **Query parameters**: URL-encoded, maximum 2048 characters
- **JSON payloads**: Maximum 1MB, valid JSON structure
- **File uploads**: Maximum 100MB per file for ingestion endpoints
- **Timeouts**: 30 seconds for standard requests, 300 seconds for long-running operations

#### Validation Error Examples

**Invalid gene symbol:**
```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid gene symbol format",
    "details": {
      "field": "gene_symbol",
      "value": "INVALID_GENE_123",
      "pattern": "^[A-Z0-9-]{1,20}$"
    }
  }
}
```

**Missing required field:**
```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Required field missing",
    "details": {
      "field": "message",
      "error": "field_required"
    }
  }
}
```

## 9) Authentication and authorization

### Authentication Methods

#### API Key Authentication

```bash
curl -H "X-API-Key: your-api-key-here" \
     "http://127.0.0.1:3001/api/search?q=KRAS"
```

#### Bearer Token Authentication

```bash
curl -H "Authorization: Bearer your-jwt-token" \
     "http://127.0.0.1:3001/api/targets"
```

#### OAuth 2.0 Flow

```python
import requests
from requests_oauthlib import OAuth2Session

# OAuth 2.0 client credentials flow
client_id = 'your-client-id'
client_secret = 'your-client-secret'
token_url = 'http://127.0.0.1:3001/oauth/token'

client = OAuth2Session(client_id)
token = client.fetch_token(token_url, client_secret=client_secret)

# Use token for API calls
response = client.get('http://127.0.0.1:3001/api/search?q=KRAS')
```

### Authorization Scopes

| Scope | Description | Example Endpoints |
|-------|-------------|-------------------|
| `read:literature` | Read literature data | GET /api/search, GET /api/kg |
| `read:targets` | Read target data | GET /api/targets, GET /api/ranker |
| `write:chat` | Send chat messages | POST /api/chat |
| `admin:settings` | Modify settings | POST /api/settings |
| `federation:read` | Read federation data | GET /api/federation/* |
| `federation:write` | Modify federation | POST /api/federation/* |

### Session Management

#### WebSocket Authentication

```javascript
const ws = new WebSocket('ws://127.0.0.1:3001/ws');

ws.onopen = () => {
    ws.send(JSON.stringify({
        type: 'auth',
        token: 'your-jwt-token'
    }));
};

ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    if (data.type === 'auth_success') {
        // Now authenticated for chat
        ws.send(JSON.stringify({
            type: 'chat_message',
            message: 'Hello!'
        }));
    }
};
```

## 10) Advanced API features

### Pagination

#### Offset-based pagination

```bash
# Get first page
curl "http://127.0.0.1:3001/api/search?q=KRAS&page=1&limit=20"

# Get second page
curl "http://127.0.0.1:3001/api/search?q=KRAS&page=2&limit=20"
```

#### Cursor-based pagination (for large datasets)

```bash
# Initial request
curl "http://127.0.0.1:3001/api/kg?limit=100"

# Response includes cursor
{
  "results": [...],
  "pagination": {
    "has_more": true,
    "next_cursor": "eyJwYWdlIjoyLCJsaW1pdCI6MTAwfQ=="
  }
}

# Next page using cursor
curl "http://127.0.0.1:3001/api/kg?cursor=eyJwYWdlIjoyLCJsaW1pdCI6MTAwfQ==&limit=100"
```

### Filtering and sorting

#### Advanced filtering

```bash
# Multiple filters
curl "http://127.0.0.1:3001/api/targets?cancer=PAAD&gene=KRAS&tier=high"

# Date range filtering
curl "http://127.0.0.1:3001/api/search?q=KRAS&pub_date_from=2023-01-01&pub_date_to=2024-01-01"

# Score filtering
curl "http://127.0.0.1:3001/api/ranker/top?min_score=8.0&max_score=10.0"
```

#### Sorting options

```bash
# Sort by relevance (default)
curl "http://127.0.0.1:3001/api/search?q=KRAS&sort=relevance"

# Sort by publication date
curl "http://127.0.0.1:3001/api/search?q=KRAS&sort=pub_date&order=desc"

# Sort by citation count
curl "http://127.0.0.1:3001/api/search?q=KRAS&sort=citations&order=desc"
```

### Bulk operations

#### Bulk target analysis

```bash
curl -X POST "http://127.0.0.1:3001/api/targets/bulk" \
  -H "content-type: application/json" \
  -d '{
    "genes": ["KRAS", "TP53", "EGFR"],
    "cancer_types": ["PAAD", "LUAD"],
    "include_evidence": true
  }'
```

#### Bulk literature ingestion

```bash
curl -X POST "http://127.0.0.1:3001/api/ingestion/bulk" \
  -H "content-type: application/json" \
  -d '{
    "queries": [
      {"gene": "KRAS", "cancer": "PAAD", "max_results": 100},
      {"gene": "EGFR", "cancer": "LUAD", "max_results": 100}
    ],
    "priority": "high"
  }'
```

### Webhooks and callbacks

#### Webhook registration

```bash
curl -X POST "http://127.0.0.1:3001/api/webhooks" \
  -H "content-type: application/json" \
  -d '{
    "url": "https://your-app.com/webhook",
    "events": ["chat.completed", "analysis.finished"],
    "secret": "your-webhook-secret"
  }'
```

#### Webhook payload example

```json
{
  "event": "chat.completed",
  "timestamp": "2024-01-15T10:30:00Z",
  "data": {
    "thread_id": "550e8400-e29b-41d4-a716-446655440000",
    "message": "Analysis of KRAS targets completed",
    "results": {
      "targets_found": 15,
      "top_target": "KRAS"
    }
  },
  "signature": "sha256=abc123..."
}
```

### API versioning

#### Version specification

```bash
# Explicit version
curl -H "Accept: application/vnd.ferrumyx.v2+json" \
     "http://127.0.0.1:3001/api/search?q=KRAS"

# URL-based versioning
curl "http://127.0.0.1:3001/v2/api/search?q=KRAS"
```

#### Breaking changes policy

- Major version bumps (v1 → v2) for breaking changes
- Minor versions for additive changes
- Patch versions for bug fixes
- 12-month support window for deprecated versions

## 11) Notes on authentication/security

- Federation routes can enforce bearer auth and replay checks depending on runtime settings.
- Chat endpoints proxy to runtime gateway and use local authorization between web and gateway.
- Settings endpoint mutates runtime/config state and should be protected at deployment layer.
- All API calls are logged for audit purposes with PHI-safe content hashing.
