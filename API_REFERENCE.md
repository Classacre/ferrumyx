# Complete API Reference

Router source of truth: `crates/ferrumyx-web/src/router.rs`

All endpoints are mounted on the Ferrumyx web server (default: `http://localhost:3000`).

## Authentication

Most endpoints require authentication via:
- **Bearer Token**: `Authorization: Bearer <token>`
- **API Key**: `X-API-Key: <key>` header

Federation endpoints may have additional authentication requirements.

## Error Handling

All endpoints return standard HTTP status codes:

- `200`: Success
- `400`: Bad Request (invalid parameters)
- `401`: Unauthorized (authentication required)
- `403`: Forbidden (insufficient permissions)
- `404`: Not Found
- `429`: Too Many Requests (rate limited)
- `500`: Internal Server Error

Error responses include:
```json
{
  "error": "Error message",
  "code": "ERROR_CODE",
  "details": "Additional context"
}
```

## Rate Limiting

- **Authenticated requests**: 1000/hour per user
- **Anonymous requests**: 100/hour per IP
- **Ingestion endpoints**: 10/hour per user

Rate limit headers:
```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1634567890
```

## 1) Core Discovery and Evidence APIs

### `GET /api/search`

Hybrid lexical/vector search with knowledge graph aggregation.

**Query Parameters:**
- `q` (string, required): Search query
- `limit` (int, default 20, max 100): Number of results
- `cancer_type` (optional string): Filter by cancer type
- `source` (optional string): Filter by literature source
- `date_from` (optional string): ISO date for start range
- `date_to` (optional string): ISO date for end range

**Response:**
```json
{
  "query": "KRAS pancreatic cancer",
  "results": [
    {
      "paper_id": "uuid",
      "title": "KRAS Mutations in Pancreatic Cancer",
      "chunk_text": "KRAS is frequently mutated in pancreatic ductal adenocarcinoma...",
      "similarity": 0.92,
      "section_type": "abstract",
      "source": "pubmed",
      "doi": "10.1234/example",
      "publication_date": "2023-01-15"
    }
  ],
  "kg_facts": [
    {
      "subject": "KRAS",
      "predicate": "mutated_in",
      "object": "Pancreatic Cancer",
      "confidence": 0.95,
      "evidence_count": 45
    }
  ],
  "total": 1,
  "took_ms": 150
}
```

**Examples:**
```bash
# Basic search
curl "http://localhost:3000/api/search?q=KRAS+G12D&limit=10"

# Filtered search
curl "http://localhost:3000/api/search?q=breast+cancer&cancer_type=BRCA&source=pubmed&limit=5"

# Date range search
curl "http://localhost:3000/api/search?q=immunotherapy&date_from=2023-01-01&date_to=2023-12-31"
```

### `GET /api/targets`

Retrieve ranked therapeutic targets.

**Query Parameters:**
- `cancer` (optional string): Cancer type filter
- `gene` (optional string): Gene symbol filter
- `tier` (optional string): Evidence tier (1-5)
- `limit` (optional int, default 50, max 200): Number of results
- `sort_by` (optional string): Sort field (score, gene, cancer)

**Response:**
```json
[
  {
    "id": "uuid",
    "gene_symbol": "KRAS",
    "cancer_type": "PAAD",
    "score": 9.2,
    "tier": 1,
    "evidence_count": 156,
    "clinical_trials": 12,
    "last_updated": "2024-01-15T10:30:00Z"
  }
]
```

**Examples:**
```bash
# Top targets for pancreatic cancer
curl "http://localhost:3000/api/targets?cancer=PAAD&limit=20"

# High-tier targets only
curl "http://localhost:3000/api/targets?tier=1&limit=10"

# Specific gene across cancers
curl "http://localhost:3000/api/targets?gene=TP53"
```

### `GET /api/targets/{gene}`

Detailed information for a specific target gene.

**Path Parameters:**
- `gene` (string): Gene symbol

**Query Parameters:**
- `cancer` (optional string): Specific cancer type

**Response:**
```json
{
  "gene_symbol": "KRAS",
  "aliases": ["KRAS2", "C-K-RAS"],
  "description": "Kirsten rat sarcoma viral oncogene homolog",
  "cancers": [
    {
      "type": "PAAD",
      "score": 9.2,
      "tier": 1,
      "evidence_summary": "KRAS mutations found in 90% of pancreatic cancers",
      "key_mutations": ["G12D", "G12V", "G12R"],
      "clinical_trials": [
        {
          "nct_id": "NCT04380753",
          "phase": "Phase 1",
          "drug": "Sotorasib",
          "status": "Recruiting"
        }
      ]
    }
  ],
  "pathways": ["MAPK", "PI3K"],
  "drug_targets": ["G12C inhibitor", "MEK inhibitor"],
  "last_updated": "2024-01-15T10:30:00Z"
}
```

### `GET /api/kg`

Query the knowledge graph for relationships and evidence.

**Query Parameters:**
- `gene` (optional string): Focus on specific gene
- `q` (optional string): Free text search
- `predicate` (optional string): Relationship type
- `confidence_tier` (optional int): Minimum confidence (1-5)
- `max_papers` (optional int): Limit papers per fact
- `view` (optional string): View mode (network, table, timeline)
- `lens` (optional string): Analysis lens (mechanistic, therapeutic, prognostic)
- `source` (optional string): Evidence source filter
- `hops` (optional int): Graph traversal depth (1-3)
- `expanded` (optional boolean): Include full evidence

**Response:**
```json
{
  "facts": [
    {
      "id": "uuid",
      "subject": "KRAS",
      "predicate": "activates",
      "object": "MAPK Pathway",
      "confidence": 0.92,
      "tier": 1,
      "evidence_count": 45,
      "papers": [
        {
          "id": "uuid",
          "title": "KRAS signaling in cancer",
          "doi": "10.1234/example",
          "year": 2023
        }
      ],
      "mechanisms": ["Direct binding", "GTPase activity"],
      "last_updated": "2024-01-15T10:30:00Z"
    }
  ],
  "stats": {
    "total_facts": 15420,
    "unique_entities": 3847,
    "evidence_papers": 12847
  }
}
```

**Examples:**
```bash
# Gene-centric view
curl "http://localhost:3000/api/kg?gene=KRAS&hops=2"

# High-confidence relationships
curl "http://localhost:3000/api/kg?confidence_tier=1&predicate=mutated_in"

# Therapeutic lens
curl "http://localhost:3000/api/kg?lens=therapeutic&gene=EGFR"
```

### `GET /api/kg/stats`

Knowledge graph statistics and metadata.

**Response:**
```json
{
  "entities": {
    "total": 12847,
    "by_type": {
      "GENE": 5423,
      "DISEASE": 2341,
      "CHEMICAL": 1892,
      "PATHWAY": 876
    }
  },
  "facts": {
    "total": 45632,
    "by_predicate": {
      "mutated_in": 12847,
      "inhibits": 8923,
      "activates": 7234,
      "expressed_in": 5678
    },
    "by_confidence_tier": {
      "1": 8923,
      "2": 15678,
      "3": 12341,
      "4": 5678,
      "5": 3012
    }
  },
  "evidence": {
    "total_papers": 89456,
    "sources": {
      "pubmed": 45623,
      "biorxiv": 12341,
      "medrxiv": 8923,
      "clinicaltrials": 5678
    }
  },
  "last_updated": "2024-01-15T10:30:00Z"
}
```

## 2) Ranking and Enrichment APIs

### `GET /api/ranker/score`

Get ranking score for a specific gene-cancer combination.

**Query Parameters:**
- `gene` (string, required): Gene symbol
- `cancer_type` (string, required): Cancer type abbreviation

**Response:**
```json
{
  "gene_symbol": "KRAS",
  "cancer_type": "PAAD",
  "composite_score": 9.2,
  "component_scores": {
    "literature_evidence": 9.5,
    "genetic_alteration": 9.8,
    "clinical_relevance": 8.9,
    "druggability": 8.7,
    "biomarker_potential": 9.1
  },
  "tier": 1,
  "rank_percentile": 95.2,
  "evidence_summary": "Strong evidence from 156 papers, 90% mutation frequency",
  "last_calculated": "2024-01-15T10:30:00Z"
}
```

### `GET /api/ranker/top`

Get top-ranked targets, optionally filtered by cancer type.

**Query Parameters:**
- `cancer_type` (optional string): Filter by cancer
- `limit` (optional int, default 50, max 200): Number of results
- `min_tier` (optional int, 1-5): Minimum evidence tier

**Response:**
```json
[
  {
    "rank": 1,
    "gene_symbol": "KRAS",
    "cancer_type": "PAAD",
    "score": 9.2,
    "tier": 1,
    "evidence_count": 156,
    "clinical_trials": 12,
    "druggable": true
  }
]
```

### `GET /api/depmap/gene`

Get DepMap (dependency map) data for gene-cancer combinations.

**Query Parameters:**
- `gene` (string, required): Gene symbol
- `cancer_type` (optional string): Cancer type filter

**Response:**
```json
{
  "gene_symbol": "KRAS",
  "depmap_data": {
    "dependency_score": -0.45,
    "essentiality_probability": 0.12,
    "cell_lines_tested": 342,
    "cancer_types": [
      {
        "type": "PAAD",
        "score": -0.67,
        "cell_lines": 23,
        "percentile": 5.2
      }
    ]
  },
  "last_updated": "2024-01-15T10:30:00Z"
}
```

## 3) Ingestion and Processing APIs

### `POST /api/ingestion/run`

Start a literature ingestion job.

**Request Body:**
```json
{
  "gene": "KRAS",
  "cancer_type": "PAAD",
  "mutation": "G12D",
  "max_results": 100,
  "sources": {
    "pubmed": true,
    "europepmc": true,
    "biorxiv": false,
    "medrxiv": false,
    "clinicaltrials": true
  },
  "embedding": {
    "backend": "fastembed",
    "model": "BAAI/bge-small-en-v1.5",
    "api_key": "optional-key"
  },
  "options": {
    "enable_scihub": false,
    "force_refresh": false,
    "skip_duplicates": true
  }
}
```

**Response:**
```json
{
  "job_id": "uuid",
  "status": "queued",
  "estimated_duration": "15-30 minutes",
  "message": "Ingestion job started for KRAS in pancreatic cancer"
}
```

### `GET /api/ingestion/status/{job_id}`

Check status of an ingestion job.

**Response:**
```json
{
  "job_id": "uuid",
  "status": "running",
  "progress": {
    "phase": "embedding_generation",
    "completed": 145,
    "total": 200,
    "percentage": 72.5
  },
  "stats": {
    "papers_found": 180,
    "papers_ingested": 145,
    "entities_extracted": 2340,
    "facts_generated": 4560
  },
  "started_at": "2024-01-15T10:00:00Z",
  "estimated_completion": "2024-01-15T10:25:00Z"
}
```

### `GET /api/ingestion/history`

Get history of ingestion jobs.

**Query Parameters:**
- `limit` (optional int, default 20): Number of jobs
- `status` (optional string): Filter by status

**Response:**
```json
{
  "jobs": [
    {
      "id": "uuid",
      "query": "KRAS PAAD",
      "status": "completed",
      "started_at": "2024-01-15T09:00:00Z",
      "completed_at": "2024-01-15T09:45:00Z",
      "stats": {
        "papers_ingested": 234,
        "entities_extracted": 3456,
        "facts_generated": 6789
      }
    }
  ],
  "total": 45
}
```

## 4) Chat and Conversational APIs

### `POST /api/chat`

Send a message to the conversational agent.

**Request Body:**
```json
{
  "message": "Find KRAS targets in pancreatic cancer",
  "thread_id": "optional-thread-uuid",
  "context": {
    "cancer_focus": "PAAD",
    "evidence_level": "clinical"
  }
}
```

**Response:**
```json
{
  "thread_id": "uuid",
  "response": {
    "message": "I found several high-priority KRAS-related targets in pancreatic cancer...",
    "targets": [
      {
        "gene": "KRAS",
        "score": 9.2,
        "evidence": "156 papers, 90% mutation frequency"
      }
    ],
    "follow_up_questions": [
      "Would you like me to analyze specific mutations?",
      "Should I check for clinical trials?"
    ]
  },
  "timestamp": "2024-01-15T10:30:00Z"
}
```

### `GET /api/chat/threads`

List conversation threads.

**Query Parameters:**
- `limit` (optional int, default 20): Number of threads
- `active_only` (optional boolean): Show only active threads

**Response:**
```json
{
  "threads": [
    {
      "id": "uuid",
      "title": "KRAS in Pancreatic Cancer",
      "created_at": "2024-01-15T09:00:00Z",
      "last_message_at": "2024-01-15T10:30:00Z",
      "message_count": 12,
      "active": true
    }
  ]
}
```

### `GET /api/chat/history`

Get message history for a thread.

**Query Parameters:**
- `thread_id` (string, required): Thread identifier
- `limit` (optional int, default 50): Number of messages
- `before` (optional string): Cursor for pagination

**Response:**
```json
{
  "thread_id": "uuid",
  "messages": [
    {
      "id": "uuid",
      "role": "user",
      "content": "Find KRAS targets in pancreatic cancer",
      "timestamp": "2024-01-15T10:00:00Z"
    },
    {
      "id": "uuid",
      "role": "assistant",
      "content": "I found several targets...",
      "timestamp": "2024-01-15T10:01:00Z",
      "metadata": {
        "targets_found": 5,
        "evidence_reviewed": 156
      }
    }
  ],
  "has_more": false
}
```

## 5) Settings and Administration APIs

### `GET /api/settings`

Get current system settings.

**Response:**
```json
{
  "llm": {
    "provider": "openai",
    "model": "gpt-4",
    "temperature": 0.7,
    "max_tokens": 4096
  },
  "ingestion": {
    "default_max_results": 100,
    "timeout_seconds": 300,
    "concurrency_limit": 5
  },
  "security": {
    "audit_logging": true,
    "rate_limiting": true,
    "data_classification": true
  },
  "federation": {
    "enabled": false,
    "node_id": "local-node",
    "sync_interval_hours": 24
  }
}
```

### `POST /api/settings`

Update system settings (admin only).

**Request Body:**
```json
{
  "llm": {
    "model": "gpt-4-turbo"
  },
  "ingestion": {
    "default_max_results": 200
  }
}
```

## 6) Monitoring and Metrics APIs

### `GET /api/metrics/perf`

Get performance metrics.

**Response:**
```json
{
  "ingestion": {
    "avg_duration_seconds": 420,
    "papers_per_hour": 45,
    "success_rate": 0.94,
    "error_rate": 0.03
  },
  "search": {
    "avg_response_time_ms": 150,
    "queries_per_hour": 1200,
    "cache_hit_rate": 0.75
  },
  "system": {
    "cpu_usage_percent": 45.2,
    "memory_usage_gb": 8.3,
    "disk_usage_gb": 156.7,
    "uptime_hours": 168
  },
  "timestamp": "2024-01-15T10:30:00Z"
}
```

### `GET /api/health`

System health check.

**Query Parameters:**
- `detailed` (optional boolean): Include detailed component status

**Response:**
```json
{
  "status": "healthy",
  "version": "2.0.0",
  "timestamp": "2024-01-15T10:30:00Z",
  "services": {
    "database": "healthy",
    "agent": "healthy",
    "web": "healthy",
    "redis": "healthy"
  },
  "metrics": {
    "response_time_ms": 45,
    "active_connections": 12,
    "memory_usage_percent": 68
  }
}
```

## 7) Federation APIs

### `POST /api/federation/package/export`

Export knowledge package for sharing.

**Request Body:**
```json
{
  "name": "KRAS_PAAD_2024",
  "description": "KRAS targets in pancreatic cancer analysis",
  "filters": {
    "genes": ["KRAS"],
    "cancer_types": ["PAAD"],
    "date_from": "2023-01-01"
  },
  "include_evidence": true,
  "anonymize": false
}
```

**Response:**
```json
{
  "package_id": "uuid",
  "status": "exporting",
  "estimated_size_mb": 45.2,
  "download_url": "/api/federation/package/uuid/download"
}
```

### `POST /api/federation/sync/plan`

Plan synchronization with remote federation node.

**Request Body:**
```json
{
  "remote_base_url": "https://federation-node.example.com",
  "auth_token": "bearer-token",
  "sync_scope": {
    "cancer_types": ["PAAD", "LUAD"],
    "date_from": "2023-01-01"
  }
}
```

**Response:**
```json
{
  "plan_id": "uuid",
  "estimated_packages": 12,
  "estimated_size_gb": 2.3,
  "sync_duration_estimate": "4-6 hours",
  "conflicts_expected": 3
}
```

## 8) Webhook and Integration APIs

### `POST /api/webhooks/ingestion`

Webhook for ingestion completion notifications.

**Headers:**
```
X-Webhook-Signature: sha256=signature
Content-Type: application/json
```

**Request Body:**
```json
{
  "event": "ingestion.completed",
  "job_id": "uuid",
  "query": "KRAS PAAD",
  "stats": {
    "papers_ingested": 234,
    "entities_extracted": 3456,
    "duration_seconds": 1800
  },
  "timestamp": "2024-01-15T10:30:00Z"
}
```

### `POST /api/integrations/slack`

Send notifications to Slack channels.

**Configuration:**
```json
{
  "webhook_url": "https://hooks.slack.com/services/...",
  "channel": "#research-alerts",
  "events": ["ingestion.completed", "high_priority_target_found"]
}
```

## 9) Advanced Query APIs

### `POST /api/query/advanced`

Execute complex multi-step queries.

**Request Body:**
```json
{
  "query": {
    "target": {
      "gene": "KRAS",
      "cancer": "PAAD"
    },
    "filters": {
      "evidence_tier": 1,
      "publication_year": { "gte": 2020 },
      "clinical_trials": { "exists": true }
    },
    "analysis": {
      "pathway_enrichment": true,
      "drug_interactions": true,
      "biomarker_correlation": true
    }
  },
  "output_format": "comprehensive"
}
```

**Response:**
```json
{
  "query_id": "uuid",
  "status": "processing",
  "estimated_completion": "2024-01-15T11:00:00Z",
  "results_url": "/api/query/results/uuid"
}
```

## Error Codes

| Code | Description | Resolution |
|------|-------------|------------|
| `INVALID_QUERY` | Malformed search query | Check query syntax |
| `RATE_LIMITED` | Too many requests | Wait and retry |
| `AUTH_REQUIRED` | Authentication missing | Provide valid token |
| `PERMISSION_DENIED` | Insufficient permissions | Check user role |
| `SERVICE_UNAVAILABLE` | Service temporarily down | Retry later |
| `DATABASE_ERROR` | Database connectivity issue | Check system status |
| `INGESTION_FAILED` | Literature ingestion error | Check job status |
| `VALIDATION_ERROR` | Input validation failed | Fix request parameters |

## SDKs and Libraries

### Python Client

```python
from ferrumyx import FerrumyxClient

client = FerrumyxClient(api_key="your-key", base_url="http://localhost:3000")

# Search for targets
results = client.search_targets(cancer="PAAD", limit=20)

# Get detailed analysis
analysis = client.analyze_target("KRAS", cancer="PAAD")

# Start ingestion
job = client.ingest_literature("KRAS", cancer="PAAD")
```

### JavaScript Client

```javascript
import { FerrumyxAPI } from 'ferrumyx-js';

const client = new FerrumyxAPI({
  apiKey: 'your-key',
  baseURL: 'http://localhost:3000'
});

// Conversational query
const response = await client.chat({
  message: 'Find KRAS targets in pancreatic cancer'
});

// Get metrics
const metrics = await client.getMetrics();
```

This comprehensive API reference covers all major endpoints with examples, parameters, and response formats. For the latest updates, check the source code in `crates/ferrumyx-web/src/router.rs`.</content>
<parameter name="filePath">D:\AI\Ferrumyx\API_REFERENCE.md