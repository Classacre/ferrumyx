# Ferrumyx Architecture

**Autonomous Oncology Drug Discovery Engine**  
**Built on Ferrumyx Runtime Core (Rust AI Agent Framework)**  
**Version:** 1.0.0-mvp  
**Repository:** https://github.com/Classacre/ferrumyx  
**Status:** Active Implementation (Phase 1-3 complete; Phase 4 hardening in active iteration; Phase 5+ in progress; Phase 10 federation bootstrap started)  
**Date:** 2026-03-21
**Document ownership:** `ARCHITECTURE.md` is owned by the main agent. Keep this file aligned with the implementation state and the dated task docs under `docs/`.

---

> **Scope disclaimer:** This document describes a research-grade system intended for computational hypothesis generation, not clinical use. All outputs require expert wet-lab validation. Ferrumyx is not a replacement for medicinal chemistry expertise, and no claim of clinical predictive accuracy is made.

---

## Table of Contents

1. [Phase 1: System Architecture](#phase-1-system-architecture)
2. [Phase 2: Literature Ingestion](#phase-2-literature-ingestion)
3. [Phase 3: Knowledge Graph & Target Intelligence](#phase-3-knowledge-graph--target-intelligence)
4. [Phase 4: Target Prioritization Engine](#phase-4-target-prioritization-engine)
5. [Phase 5: Structural Analysis & Molecule Design](#phase-5-structural-analysis--molecule-design)
6. [Phase 6: Autonomous Scientific Query Handling](#phase-6-autonomous-scientific-query-handling)
7. [Phase 7: Self-Improvement Framework](#phase-7-self-improvement-framework)
8. [Phase 8: Security & LLM Strategy](#phase-8-security--llm-strategy)
9. [Phase 9: Roadmap](#phase-9-roadmap)
10. [Phase 10: Federated Knowledge Base Distribution](#phase-10-federated-knowledge-base-distribution)
11. [Deliverables](#deliverables)

---

## Current Implementation Snapshot (2026-03-21)

- Phase 1-3 functionality is operational end-to-end with Ferrumyx Runtime Core as the orchestrator and Ferrumyx domain tools registered in the runtime tool surface.
- Chat stack is now async, history-backed, and streaming-capable with markdown rendering and thread management in the web UI.
- Autonomous lab-team role tooling is now implemented (`lab_planner`, `lab_retriever`, `lab_validator`) with coordinator/status tools (`run_lab_autoresearch`, `lab_run_status`) for dynamic multi-cycle research execution.
- Lab run-state is persisted to disk (`output/lab_runs.json`, override via `FERRUMYX_LAB_STATE_PATH`) for cross-process monitoring and resilient status views.
- Settings are tab-organized and now drive secure API configuration for active providers (Ollama, OpenAI, Anthropic, Gemini, OpenAI-compatible), including cached-chat toggle support for compatible providers.
- Ingestion performance hardening is active: source caching, negative + success full-text caches, chunk fingerprint cache, canonical DOI/PMID/PMCID/title identity dedupe during source fan-in, early source-abort on unique-target saturation, adaptive worker tuning, fast-lane/heavy-lane split with optional async enrichment, explicit heavy-lane drain control, DB-side missing-embedding filtering with compatibility fallback, and batch embedding backfill updates.
- Sci-Hub fallback now includes settings-driven mirror parallelism/cooldown and adaptive fallback controls (deferred launch, failure-streak backoff, probe cadence, and adaptive step budgets) to keep full-text retrieval fast under mirror instability.
- Metrics include live ingestion performance telemetry (`/api/metrics/perf`) and persisted snapshots for run-to-run benchmarking.
- KG/query surfaces now use bounded/aggregated paths to avoid large table scans and keep UI/API latency stable under larger corpora.
- `query_targets` now emits a compact downstream embedding block: top RAG snippets, gene-link edges, novelty signals, dedup/near-dup groups, topic clusters, drift mix, and per-gene numeric feature vectors.
- KG rendering now uses deterministic topology-derived coordinates (2D projected from Rust 3D layout) with stable seeded fallback (no random jitter path), plus confidence/provenance edge metadata.
- Target scoring now consumes confidence/provenance-weighted KG evidence so source-backed high-tier facts contribute more than generic mentions.
- Sci-Hub support is multi-domain and settings-driven (domain list + timeout), aligned to currently active mirrors.
- Ranker/DepMap web APIs are now dynamic-input driven (no hardcoded cancer defaults) and hardened for web latency via bounded read paths, reduced fanout in hot endpoints, and batch entity-name resolution to avoid per-row lookup amplification.
- Web UI information architecture now follows progressive disclosure: dense tables/details are collapsed by default across KG/Query/Targets/DepMap/Molecules, with lightweight summaries first and drill-down evidence on demand.
- Chat includes a dedicated Live Lab Run Monitor panel backed by `/api/chat/lab-monitor`, with auto run-ID detection from streaming/tool events, KPI snapshots, and recent-run selection.
- Federated KB bootstrap is implemented: shared contribution manifest schema (`ferrumyx.federation.v1`), DB-side draft generation/validation, JSONL package export + digest verification, and web APIs (`/api/federation/schema`, `/api/federation/manifest/*`, `/api/federation/package/*`).

## UI Information Architecture Snapshot (2026-03-17)

- Default views prioritize concise summaries (scores, counts, top-level state) before raw evidence rows.
- High-volume artifacts (paper evidence, provider cache rows, relation breakdowns, dependency tables) are hidden behind `<details>` disclosures by default.
- Query and KG controls are split into primary vs advanced controls to reduce cognitive load while preserving full operator control.
- Entity-level drill-down remains one-click accessible (e.g., `Insights`, collapsed evidence groups, provider health/history panels).
- Chat operations now include run-level telemetry views (monitor panel) so autonomous workflow progress is inspectable without leaving the chat surface.

---

# Phase 1: System Architecture

## 1.1 High-Level System Architecture Diagram

```mermaid
flowchart TD
    %% Styling
    classDef layerBox fill:transparent,stroke:#5c6bc0,stroke-width:2px,stroke-dasharray: 5 5
    classDef coreBox fill:#1e1e24,stroke:#4fc3f7,stroke-width:2px
    classDef extBox fill:#1e1e24,stroke:#81c784,stroke-width:2px
    classDef dbBox fill:#1e1e24,stroke:#ffb74d,stroke-width:2px

    subgraph ChannelLayer [Channel Layer]
        C1[REPL]
        C2[Web Gateway]
        C3[HTTP Webhook]
        C4[Telegram/Slack WASM]
    end
    class ChannelLayer layerBox

    subgraph Ferrumyx Runtime CoreCore [Ferrumyx Runtime Core Agent Core]
        IC1[Intent Router<br/>query parsing]
        IC2{Agent Loop<br/>plan/act/obs}
        IC3[Routines Engine<br/>cron/event/webhook]
        
        subgraph ToolRegistry [Tool Registry]
            TR1[Built-in]
            TR2[MCP Tools]
            TR3[WASM Tools]
        end
        
        IC1 --> IC2
        IC2 <--> ToolRegistry
        IC3 --> IC2
    end
    class Ferrumyx Runtime CoreCore coreBox

    subgraph FerrumyxExt [Ferrumyx Extension Layer]
        direction TB
        F1[Ingestion Orchestrator]
        F2[Paper-Centric KG Builder + TrieNer]
        F3[Target Ranker + Relation Extraction]
        
        F4[Molecule Design Pipeline]
        F5[Self-Optimization Loop]
        
        F1 --> F4
        F2 --> F3
        F3 --> F5
    end
    class FerrumyxExt extBox

    subgraph SandboxLayer [Sandbox Layer]
        S1[WASM Sandbox<br/>NER tools, light processing<br/>Cap-based perms<br/>Endpoint allowlist]
    end
    class SandboxLayer layerBox

    subgraph StorageLayer [Storage Layer]
        DB1[(LanceDB Embedded Vector DB<br/>papers, chunks, embeddings, kg_facts)]
        DB2[(Workspace FS<br/>Ferrumyx Runtime Core mem.)]
        DB3[(Secrets Store<br/>AES-256-GCM)]
    end
    class StorageLayer dbBox

    subgraph LLMLayer [LLM Abstraction Layer]
        L1[Ollama local / OpenAI / Anthropic]
        L2[Data classification gate -> redaction -> routing]
        L1 --- L2
    end
    class LLMLayer layerBox

    %% Connections
    ChannelLayer --> Ferrumyx Runtime CoreCore
    Ferrumyx Runtime CoreCore <--> FerrumyxExt
    FerrumyxExt --> SandboxLayer
    FerrumyxExt <--> StorageLayer
    Ferrumyx Runtime CoreCore <--> LLMLayer
```

## 1.2 Modular Breakdown

### Orchestration Layer (Rust, Ferrumyx Runtime Core extension)

| Crate | Description | Extends Ferrumyx Runtime Core? |
|---|---|---|
| `ferrumyx-agent` | Top-level agent loop, intent routing | Yes — registers custom intents |
| `ferrumyx-common` | Shared types, utilities, and query definitions | No |
| `ferrumyx-db` | Database schema, migrations, and access patterns | No |
| `ferrumyx-ingestion` | Paper discovery, download, embeddings (Candle) | Tool implementations |
| `ferrumyx-kg` | Paper-centric Literature Graph, TrieNer (Aho-Corasick), Normalization | Storage abstraction |
| `ferrumyx-molecules` | Molecular structure and docking logic | No |
| `ferrumyx-ranker` | Target scoring, query execution, and shortlisting | Pure Rust scoring logic |
| `ferrumyx-web` | Web UI and API Gateway | No |
| `Ferrumyx Runtime Core` | Core AI agent framework dependency | Yes |

### Tool Layer (100% Rust-native)

| Tool | Implementation | Notes |
|---|---|---|
| `IngestPubmedTool` | Rust HTTP client | Native integration with PubMed API |
| `IngestEuropePmcTool` | Rust HTTP client | Native integration with Europe PMC API |
| `IngestAllSourcesTool` | Rust Orchestration | Parallel multi-source ingestion |
| `NerExtractTool` | Rust (Aho-Corasick) | Database-driven NER (HGNC, OncoTree) |
| `ScoreTargetsTool` | Rust ranking logic | Multi-factor evidence scoring & DepMap |
| `KgQueryTool` | Rust / LanceDB | Paper-centric provenance graph queries |
| `KgUpsertTool` | Rust / LanceDB | Literature Graph construction |
| `FetchStructureTool` | Rust HTTP / PDB | Fetch protein structures |
| `DetectPocketsTool` | Rust (`fpocket` wrapper) | Detect binding pockets (structural) |
| `DockMoleculeTool` | Rust (`vina` wrapper) | AutoDock Vina molecular docking |

### Storage Layer

- **LanceDB** (Embedded Vector DB)
- **Workspace filesystem** (Ferrumyx Runtime Core native): intermediate files, job artifacts
- **AES-256-GCM keychain**: API keys, DB credentials

## 1.3 Data Flow

```mermaid
flowchart TD
    classDef pipeline fill:#1e1e24,stroke:#5c6bc0,stroke-width:2px

    EXT[External APIs / PDFs]
    
    subgraph IngestionPipeline [INGESTION PIPELINE]
        direction TB
        I1[1. Paper discovery API]
        I2[2. Deduplication check]
        I3[3. Full-text retrieval]
        I4[4. PDF parse Rust/lopdf]
        I5[5. Section-aware chunking]
        I1 --> I2 --> I3 --> I4 --> I5
    end
    class IngestionPipeline pipeline
    
    subgraph EmbeddingPipeline [EMBEDDING PIPELINE]
        direction TB
        E1[6. BiomedBERT embed Candle]
        E2[7. LanceDB store]
        E3[8. Full-text index]
        E1 --> E2 --> E3
    end
    class EmbeddingPipeline pipeline
    
    subgraph NER_KGPipeline [NER + LITERATURE GRAPH CONSTRUCTION]
        direction TB
        N1[9. TrieNer: Case-insensitive AC matching]
        N2[10. Normalization: HGNC / OncoTree API]
        N3[11. RelationExtractor: Pattern-based extraction]
        N4[12. Paper-as-Hub Linkage (Mentions)]
        N5[13. Append to kg_facts (Provenance Hubs)]
        N1 --> N2 --> N3 --> N4 --> N5
    end
    class NER_KGPipeline pipeline
    
    subgraph TargetPipeline [TARGET PRIORITIZATION]
        direction TB
        T1[14. Join KG + external DBs]
        T2[15. Compute composite score]
        T3[16. Store versioned scores]
        T4[17. Shortlist candidates]
        T1 --> T2 --> T3 --> T4
    end
    class TargetPipeline pipeline
    
    subgraph StructuralPipeline [STRUCTURAL ANALYSIS]
        direction TB
        S1[18. Fetch PDB / AlphaFold]
        S2[19. fpocket detection]
        S3[20. Ligand generation]
        S4[21. AutoDock Vina docking]
        S5[22. ADMET prediction]
        S6[23. Score and rank molecules]
        S1 --> S2 --> S3 --> S4 --> S5 --> S6
    end
    class StructuralPipeline pipeline
    
    subgraph QueryPipeline [QUERY RESPONSE / REPORT]
        direction TB
        Q1[24. Assemble evidence bundle]
        Q2[25. LLM-assisted narrative]
        Q3[26. Return ranked JSON]
        Q1 --> Q2 --> Q3
    end
    class QueryPipeline pipeline

    EXT --> I1
    I5 --> E1
    E3 --> N1
    N5 --> T1
    T4 --> S1
    S6 --> Q1
```

## 1.4 Memory Design: LanceDB Schema Overview

### Core Tables

```rust
// Papers and source tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub id: uuid::Uuid,
    pub doi: Option<String>,
    pub pmid: Option<String>,
    pub pmcid: Option<String>,
    pub title: String,
    pub abstract_text: Option<String>,
    pub authors: Option<String>,         // JSON string: [{name, affiliation, orcid}]
    pub journal: Option<String>,
    pub pub_date: Option<chrono::NaiveDate>,
    pub source: String,                  // 'pubmed'|'europepmc'|'biorxiv'|...
    pub open_access: bool,
    pub full_text_url: Option<String>,
    pub parse_status: String,            // 'pending'|'parsed'|'failed'
    pub ingested_at: chrono::DateTime<chrono::Utc>,
    pub raw_json: Option<String>         // original API response (JSON string)
}
```
```rust
// Parsed document chunks (section-aware)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: uuid::Uuid,
    pub paper_id: uuid::Uuid,
    pub chunk_index: i64,
    pub section_type: Option<String>,    // 'abstract'|'intro'|'methods'|'results'|'discussion'
    pub section_heading: Option<String>,
    pub content: String,
    pub token_count: Option<i64>,
    pub embedding: Option<Vec<f32>>,     // 768-dim (BiomedBERT-base) or 1024-dim
    pub created_at: chrono::DateTime<chrono::Utc>
}
```
```rust
// Biomedical entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: uuid::Uuid,
    pub external_id: String,             // HGNC:1100, OncoTree:LUAD
    pub name: String,
    pub canonical_name: Option<String>,
    pub entity_type: String,             // 'GENE'|'CANCER_TYPE'
    pub synonyms: Option<String>,        // Pipe-separated or JSON aliasing
    pub source_db: String,               // 'HGNC'|'ONCOTREE'
    pub created_at: chrono::DateTime<chrono::Utc>
}
```
```rust
// Knowledge graph facts (append-only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgFact {
    pub id: uuid::Uuid,
    pub paper_id: uuid::Uuid,            // The provenance hub
    pub subject_id: uuid::Uuid,
    pub subject_name: String,
    pub predicate: String,               // 'inhibits'|'activates'|'associated_with'
    pub object_id: uuid::Uuid,
    pub object_name: String,
    pub confidence: f32,
    pub evidence: Option<String>,        // Supporting text snippet
    pub evidence_type: String,           // 'text_mined'|'experimental'
    pub sample_size: Option<i32>,
    pub study_type: Option<String>,      // 'rct'|'cell_line'|...
    pub valid_from: chrono::DateTime<chrono::Utc>,
    pub valid_until: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>
}
```
```rust
// Target scoring (versioned)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetScore {
    pub id: uuid::Uuid,
    pub gene_entity_id: uuid::Uuid,
    pub cancer_entity_id: uuid::Uuid,
    pub score_version: i64,
    pub composite_score: f32,
    pub component_scores: String,        // JSON string {mutation_freq, depmap, survival, ...}
    pub weight_vector: String,           // snapshot of weights used
    pub confidence_adj: Option<f32>,
    pub scored_at: chrono::DateTime<chrono::Utc>,
    pub is_current: bool
}
```
```rust
// Molecular structures and docking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Molecule {
    pub id: uuid::Uuid,
    pub smiles: String,
    pub inchi_key: Option<String>,
    pub chembl_id: Option<String>,
    pub name: Option<String>,
    pub mw: Option<f32>,
    pub logp: Option<f32>,
    pub hbd: Option<i64>,
    pub hba: Option<i64>,
    pub tpsa: Option<f32>,
    pub sa_score: Option<f32>,           // synthetic accessibility
    pub source: Option<String>,          // 'generated'|'retrieved'|'modified'
    pub parent_id: Option<uuid::Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>
}
```
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockingResult {
    pub id: uuid::Uuid,
    pub molecule_id: uuid::Uuid,
    pub target_gene_id: uuid::Uuid,
    pub pdb_id: Option<String>,
    pub pocket_id: Option<String>,
    pub vina_score: Option<f32>,
    pub gnina_score: Option<f32>,
    pub pose_file: Option<String>,       // path in workspace FS
    pub admet_scores: Option<String>,    // JSON string
    pub run_params: Option<String>,      // JSON string
    pub docked_at: chrono::DateTime<chrono::Utc>
}
```
```rust
// Feedback and self-improvement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEvent {
    pub id: uuid::Uuid,
    pub event_type: String,              // 'drugbank_validation'|'chembl_correlation'|'trial_outcome'
    pub target_gene_id: uuid::Uuid,
    pub cancer_id: uuid::Uuid,
    pub metric_name: String,
    pub metric_value: f32,
    pub evidence_source: Option<String>,
    pub recorded_at: chrono::DateTime<chrono::Utc>
}
```
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightUpdateLog {
    pub id: uuid::Uuid,
    pub previous_weights: String,        // JSON string
    pub new_weights: String,             // JSON string
    pub trigger_event: Option<String>,
    pub algorithm: Option<String>,       // 'bayesian'|'manual'|'gradient'
    pub approved_by: Option<String>,     // human reviewer ID or 'auto'
    pub delta_summary: Option<String>,   // JSON string
    pub updated_at: chrono::DateTime<chrono::Utc>
}
```
```rust
// Audit log for all LLM calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmAuditLog {
    pub id: uuid::Uuid,
    pub session_id: Option<String>,
    pub model: String,
    pub backend: String,                 // 'ollama'|'openai'|'anthropic'|'custom'
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub data_class: String,              // 'PUBLIC'|'INTERNAL'|'CONFIDENTIAL'
    pub output_hash: String,
    pub latency_ms: Option<i64>,
    pub called_at: chrono::DateTime<chrono::Utc>
}
```
```rust
// Ingestion audit log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionAudit {
    pub id: uuid::Uuid,
    pub paper_doi: Option<String>,
    pub paper_pmid: Option<String>,
    pub action: String,                  // 'discovered'|'downloaded'|'parsed'|'embedded'|'failed'
    pub source: String,
    pub detail: Option<String>,          // JSON string
    pub occurred_at: chrono::DateTime<chrono::Utc>
}
```

### LanceDB Usage

- **Embedding dimension:** 768 (BiomedBERT-base) or 1024 (PubMedBERT-large). Choose at project start — changing dimension requires full re-embedding.
- **Index type:** IVF-PQ for MVP (good enough up to ~1M vectors); migrate to HNSW for production scale.
- **Hybrid search:** Reciprocal Rank Fusion (RRF) combining cosine similarity from LanceDB and BM25-style full-text ranking.

```rust
// Example hybrid search query (RRF) via LanceDB
let query = table
    .search(query_embedding)
    .limit(100)
    .execute()
    .await?;
```

## 1.5 LLM Backend Abstraction Layer

```mermaid
flowchart TD
    classDef abstract fill:#1e1e24,stroke:#ffb74d,stroke-width:2px,stroke-dasharray: 5 5
    classDef concrete fill:#1e1e24,stroke:#4fc3f7,stroke-width:2px
    classDef gate fill:#1e1e24,stroke:#ef5350,stroke-width:2px

    Llm[ferrumyx_llm::LlmBackend<br/>Rust trait]
    class Llm abstract

    subgraph Backends [Backend Implementations]
        direction LR
        Ollama[OllamaBackend<br/>local HTTP]
        OpenAI[OpenAIBackend<br/>REST API]
        Anthropic[AnthropicBackend<br/>REST API]
    end
    class Ollama,OpenAI,Anthropic concrete

    Custom[CustomHttp<br/>Backend]
    class Custom concrete
    
    Gate[Data Classification<br/>Gate pre-call]
    class Gate gate

    Llm --> Ollama
    Llm --> OpenAI
    Llm --> Anthropic
    
    Ollama --> Custom
    OpenAI --> Gate
    Anthropic --> Gate
```

**Trait definition (conceptual Rust):**

```rust
#[async_trait]
pub trait LlmBackend: Send + Sync {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>;
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, LlmError>;
    fn model_id(&self) -> &str;
    fn supports_local(&self) -> bool;
    fn max_context_tokens(&self) -> usize;
}

pub struct LlmRouter {
    backends: HashMap<String, Arc<dyn LlmBackend>>,
    policy: RoutingPolicy,
    data_gate: DataClassificationGate,
    audit_logger: AuditLogger,
}
```

**Routing policy:**
- `DataClass::Public` → any backend (prefer local if available)
- `DataClass::Internal` → local only OR explicit override with audit log
- `DataClass::Confidential` → local only; remote call = hard block + alert

### Default Configuration (Phase 3 Completed)
In Phase 3, we transitioned from a scaffolded LLM backend to leveraging **Ollama** natively as the engine. The system includes an automated hardware detection routine embedded within startup scripts to dynamically optimize the chosen Ollama model (e.g., `llama3.2`, `qwen2.5-coder`, `mistral`) depending on available system RAM and resources. The agent runs transparently via the `rig-core` integration within `ferrumyx-agent`. The primary node of the entire application architecture is the Ferrumyx Runtime Core event loop hosted inside `ferrumyx-agent::main.rs`.

## 1.6 Self-Improvement Feedback Loop Architecture

```mermaid
flowchart TD
    classDef stage fill:#1e1e24,stroke:#81c784,stroke-width:2px
    classDef policy fill:#1e1e24,stroke:#ffb74d,stroke-width:2px,stroke-dasharray: 5 5

    subgraph Feedback [FEEDBACK COLLECTION LAYER]
        direction TB
        F1[scheduled routines: daily/weekly]
        F2[ChEMBL activity data pull]
        F3[ClinicalTrials.gov outcome updates]
        F4[DrugBank approved drug list diff]
        F5[Target ranking stability measurement]
    end
    class Feedback stage

    subgraph Metric [METRIC COMPUTATION]
        direction TB
        M1[Retrospective recall@N]
        M2[Docking-IC50 Pearson correlation]
        M3[Ranking Kendall-tau stability]
        M4[False positive accumulation rate]
    end
    class Metric stage

    subgraph Threshold [THRESHOLD EVALUATION automated]
        direction TB
        T1{If metric_delta > threshold}
        T2[Generate weight update PROPOSAL]
        T3[Log metric, no action]
        T1 -->|Yes| T2
        T1 -->|No| T3
    end
    class Threshold stage

    subgraph Policy [OPTIONAL POLICY CHECKPOINT]
        direction TB
        H1[Autonomous default: apply if within safety bounds]
        H2[Optional manual gate via settings/policy]
        H3[Projected impact and diff shown in UI]
        H4((Escalate to operator only on high-risk deltas))
    end
    class Policy policy

    subgraph Update [WEIGHT APPLICATION + AUDIT]
        direction TB
        U1[Atomic write to weight_update_log]
        U2[Re-score all targets with new weights]
        U3[Mark old target_scores as is_current=FALSE]
    end
    class Update stage

    Feedback --> Metric
    Metric --> Threshold
    T2 --> Policy
    Policy --> Update
```

Current runtime policy is autonomy-first: parameter updates are applied automatically when safety constraints are met, with optional operator gating for stricter environments.

## 1.7 Security Boundary Definitions

| Boundary | Description | Enforcement |
|---|---|---|
| Host ↔ WASM | WASM tools cannot access filesystem, network, or secrets directly | WASM capability model (Ferrumyx Runtime Core) |
| Host ↔ Docker | Docker containers network-isolated; no direct DB access | Docker network policy + Ferrumyx Runtime Core orchestrator |
| Ferrumyx ↔ Remote LLM | Data classification gate blocks INTERNAL/CONFIDENTIAL | Rust middleware in LlmRouter |
| DB credentials | Never passed to tool layer; only accessed by host process | Ferrumyx Runtime Core AES-256-GCM keychain |
| API keys | Injected at host boundary; WASM tools receive only scoped tokens | Ferrumyx Runtime Core credential injection |
| Public API calls | All outbound calls logged with endpoint + response hash | Ingestion audit log |

## 1.8 How Ferrumyx Extends Ferrumyx Runtime Core Without Forking

**Decision: Extension, not fork.**

Rationale: Forking Ferrumyx Runtime Core means carrying the maintenance burden of diverging from upstream improvements to the agent loop, WASM sandbox, and security layer — areas where Ferrumyx has no domain-specific requirements. The extension model preserves upgradability.

**Extension mechanisms used:**

1.  **Custom tool registration:** Ferrumyx registers its specialized domain tools (`IngestPubmedTool`, `KgQueryTool`, `DockMoleculeTool`, etc.) via Ferrumyx Runtime Core's tool registry.
2.  **Autonomous Tool Creation (framework capability):** Ferrumyx Runtime Core exposes primitives for dynamic routines/tooling, but current Ferrumyx runtime primarily uses a fixed registered tool surface plus guarded host-command execution (`run_system_command`) for autonomous remediation.
3.  **Core Agent Loop Orchestration:** Ferrumyx embeds the `Agent` runtime within `ferrumyx-agent`. The agent is the primary decision-maker, autonomously interpreting results, adjusting parameters, and executing complex multi-step workflows without human intervention.
4.  **Custom routines:** Ferrumyx leverages the Ferrumyx Runtime Core routines engine for continuous background optimization, with agents deciding when to re-score or re-prioritize based on incoming data streams.
5.  **LLM execution:** Ferrumyx Runtime Core's native LLM abstraction handles routing to local (Ollama) or remote (OpenAI/Anthropic) backends based on data classification and agent needs.

**What requires direct Ferrumyx Runtime Core code changes (minimal, tracked):**
- Potentially: exposing Docker sandbox orchestration as a stable API if not already public. This would be contributed back upstream rather than forked.

---

## 1.9 Phase 1 Implementation Audit (2026-03-14)

Reality check against current codebase (`crates/ferrumyx-*` + vendored `Ferrumyx Runtime Core`):

- [x] Workspace/crate topology matches Phase 1 modular design (`ferrumyx-agent`, `ferrumyx-db`, `ferrumyx-ingestion`, `ferrumyx-kg`, `ferrumyx-ranker`, `ferrumyx-web`, `Ferrumyx Runtime Core`).
- [x] LanceDB core tables are present and initialized (`papers`, `chunks`, `entities`, `entity_mentions`, `kg_facts`, `kg_conflicts`, `target_scores`, `ingestion_audit`).
- [x] DB startup paths are hardened: table presence is resolved from a single table-name snapshot per init/stats call, and vector-index creation is idempotent with duplicate-index tolerance.
- [x] Target score history semantics are implemented (`score_version`, `is_current`) with backward-compatible handling for legacy tables.
- [x] Ferrumyx Runtime Core extension model is in active use (custom Ferrumyx tools are registered into Ferrumyx Runtime Core tool registry; no fork-only orchestration path).
- [x] Web gateway + live event streaming are implemented (`/api/events` SSE), including chat-thread endpoints and history-backed async chat UX.
- [x] Secrets/security primitives are available through Ferrumyx Runtime Core (`SecretsStore`, crypto/keychain-backed paths, sandbox/WASM runtime support).
- [~] Earlier text in this document labels most connectors as “WASM tools”; current runtime implementation is predominantly native Rust modules exposed through Ferrumyx/Ferrumyx Runtime Core handlers.

Added capabilities now present in code:
- Settings-driven provider/env sync into Ferrumyx Runtime Core runtime (OpenAI/Anthropic/Gemini/OpenAI-compatible/Ollama).
- OpenAI-compatible cached-chat toggle persisted in settings and synced to runtime env.
- Secure key save behavior in web settings (keys are not returned to browser after save).
- Streaming chat delivery and markdown-safe rendering in the web chat surface.
- Chat thread lifecycle endpoints/UI (create/list/switch/delete) with persisted history-backed async polling.

# Phase 2: Literature Ingestion

## 2.1 Source Evaluation

### PubMed / NCBI E-utilities API

| Property | Value |
|---|---|
| API availability | Free, REST (https://eutils.ncbi.nlm.nih.gov/entrez/eutils/) |
| Rate limits | 3 req/sec unauthenticated; 10 req/sec with API key |
| Authentication | Optional API key (NCBI account, free) |
| Data formats | XML (PubMed, PubMed Central), JSON (efetch supports both) |
| Full-text access | PMC Open Access subset (PMCID required) — structured XML preferred |
| Ferrumyx approach | WASM tool wrapping HTTP calls; prefer PMC XML when available |
| Key endpoints | `esearch`, `efetch`, `elink`, `esummary` |
| Notes | Most reliable source for curated biomedical abstracts; XML includes MeSH terms |

### Europe PMC REST API

| Property | Value |
|---|---|
| API availability | Free, REST (https://www.ebi.ac.uk/europepmc/webservices/rest/) |
| Rate limits | No hard limit documented; 500 req/min recommended |
| Authentication | None required for basic search |
| Data formats | JSON, XML |
| Full-text access | Open access articles available; `fullTextXML` endpoint for OA papers |
| Ferrumyx approach | WASM tool; good for preprints + European clinical trial refs |
| Notes | Contains cross-references to patents, clinical trials, grants |

### bioRxiv / medRxiv API

| Property | Value |
|---|---|
| API availability | REST (https://api.biorxiv.org/) |
| Rate limits | Not publicly documented; conservative 2 req/sec |
| Authentication | None |
| Data formats | JSON |
| Full-text access | PDF only (no structured XML); Ferrules parser |
| Ferrumyx approach | WASM tool for metadata; Ferrules parser for full text |
| Notes | High signal for cutting-edge methods; NOT peer-reviewed — confidence weight lower |

### arXiv API

| Property | Value |
|---|---|
| API availability | REST + OAI-PMH (http://export.arxiv.org/api/query) |
| Rate limits | 3 req/sec |
| Authentication | None |
| Data formats | Atom XML |
| Full-text access | PDF + source LaTeX (when available) |
| Ferrumyx approach | WASM tool; relevant for ML/computational biology / ML-for-drug-discovery papers; lower priority than PubMed for clinical oncology |
| Notes | Parse Atom XML with quick-xml crate; harvest LaTeX source for formula extraction where available; filter by primary category: q-bio.GN, q-bio.BM, cs.LG with cancer MeSH-equivalent keyword intersection |

---

### ClinicalTrials.gov API v2

| Property | Value |
|---|---|
| API availability | REST (https://clinicaltrials.gov/api/v2/studies) |
| Rate limits | 10 req/sec (unauthenticated); no key required |
| Authentication | None |
| Data formats | JSON (default), CSV |
| Full-text access | Structured trial metadata only; no PDFs |
| Ferrumyx approach | WASM tool; poll for trials matching target gene + cancer type; extract arm descriptions, intervention names, phase, status, NCT IDs |
| Notes | Invaluable for evidence component of KG: confirms clinical translation stage; use `query.term` for free-text and `filter.overallStatus` for RECRUITING/COMPLETED; parse `interventions[].name` against ChEMBL for compound cross-reference |

---

### CrossRef REST API

| Property | Value |
|---|---|
| API availability | REST (https://api.crossref.org/works) |
| Rate limits | Polite pool: ~50 req/sec with `mailto=` param; fast pool: 150 req/sec with registered token |
| Authentication | Optional Bearer token (Crossref Metadata Plus) |
| Data formats | JSON |
| Full-text access | DOI metadata only; links to publisher full-text (often paywalled) |
| Ferrumyx approach | WASM tool; DOI resolution, citation graph retrieval, journal/publisher metadata, open-access flag via `license` field |
| Notes | Essential for DOI resolution pipeline (§2.3); check `link[].content-type` for `application/pdf` links; `is-referenced-by-count` provides citation count signal for evidence weighting; use `mailto` param to avoid rate throttle |

---

### Semantic Scholar API

| Property | Value |
|---|---|
| API availability | REST (https://api.semanticscholar.org/graph/v1) |
| Rate limits | 100 req/sec with API key; 1 req/sec unauthenticated |
| Authentication | API key (free, no approval required) |
| Data formats | JSON |
| Full-text access | Open-access PDFs linked via `openAccessPdf.url`; S2 corpus IDs for cross-reference |
| Ferrumyx approach | Native Rust ingestion source; Graph API search + metadata retrieval + OA PDF URL resolution via `openAccessPdf` |
| Notes | API key is wired through Ferrumyx settings and ingestion tools; citation graph expansion and SPECTER2 embedding endpoint usage are implemented with bounded expansion controls |

---

## 2.2 Paper Discovery Tool

The paper discovery path is implemented as native Rust Ferrumyx/Ferrumyx Runtime Core tooling that translates a structured `DiscoveryRequest` (gene symbol, mutation, cancer type, date range, optional keyword modifiers) into source-specific query strings and fans out to all enabled sources in parallel.

### Query Construction Logic

```json
DiscoveryRequest {
  "gene":        "KRAS",
  "mutation":    "G12D",
  "cancer_type": "pancreatic ductal adenocarcinoma",
  "aliases":     ["PDAC", "pancreatic cancer"],
  "date_from":   "2018-01-01",
  "max_results": 200
}
```

**PubMed E-utilities query string:**
```
(KRAS[Title/Abstract] AND G12D[Title/Abstract]) AND
("pancreatic ductal adenocarcinoma"[MeSH Terms] OR
"pancreatic cancer"[Title/Abstract] OR PDAC[Title/Abstract])
AND ("2018/01/01"[PDat] : "3000/12/31"[PDat])
AND (hasabstract[text])
```

**Europe PMC REST query:**
```
(ABSTRACT:"KRAS G12D") AND
(ABSTRACT:"pancreatic cancer" OR ABSTRACT:"PDAC" OR
 MeSH:"Carcinoma, Pancreatic Ductal")
AND FIRST_PDATE:[2018-01-01 TO *]
AND (HAS_FULLTEXT:y OR OPEN_ACCESS:y)
```

**bioRxiv/medRxiv query:**
```
/search/biology kras+g12d+pancreatic?cursor=0&email_alerts=0
(POST body with query: "KRAS G12D pancreatic")
```

**arXiv API:**
```
http://export.arxiv.org/api/query?
  search_query=all:KRAS+AND+all:G12D+AND+all:pancreatic
  &cat=q-bio.GN+OR+q-bio.BM+OR+cs.LG
  &start=0&max_results=50
  &sortBy=submittedDate&sortOrder=descending
```

**Semantic Scholar:**
```
GET /graph/v1/paper/search
  ?query=KRAS G12D pancreatic cancer targeted therapy
  &fields=paperId,title,abstract,year,openAccessPdf,
          citationCount,influentialCitationCount,authors
  &limit=100
```

### Query Expansion Rules

1.  Gene aliases resolved from HGNC (e.g., KRAS → {KRAS, RASK2, c-Ki-ras})
2.  Mutation notation variants: G12D → {G12D, Gly12Asp, p.G12D, c.35G>A, rs121913529}
3.  Cancer synonyms from OncoTree: PDAC → {pancreatic adenocarcinoma, exocrine pancreatic cancer, pancreatic ductal carcinoma}
4.  Boolean logic: (gene OR alias1 OR alias2) AND (mutation OR notation2 OR notation3) AND (cancer OR synonym1 OR synonym2)

### Deduplication on Ingestion

Results from all sources are immediately deduplicated by DOI (§2.10) before downstream processing. A paper returned by both PubMed and Europe PMC counts as one record; the PubMed record is preferred (richer MeSH/structured metadata).

---

## 2.3 DOI Resolution Workflow

```mermaid
flowchart TD
    Start([Input: Paper Metadata OR Known DOI]) --> IsDoi{DOI present?}
    
    IsDoi -- No --> CrossRefSearch[CrossRef Search by title + author]
    CrossRefSearch --> Found{DOI found? <br/>Confidence > 0.92?}
    
    Found -- No --> Unresolved[Flag: Unresolved DOI<br/>Store metadata-only]
    Unresolved --> End([DOI Resolution Complete])
    
    IsDoi -- Yes --> Lookup[CrossRef /works/DOI lookup]
    Found -- Yes --> Lookup
    
    Lookup --> Extract[Extract: Journal, ISSN, Publisher, <br/>license, link, OA flag, citations, refs]
    Extract --> Unpaywall[Unpaywall API lookup<br/>api.unpaywall.org/v2/DOI]
    Unpaywall --> Status[Determine OA status:<br/>Gold/Green/Hybrid/Closed]
    
    Status --> IsOA{Open Access?}
    IsOA -- Yes --> Store[Store in papers.doi, <br/>papers.oa_status, papers.full_text_url]
    IsOA -- No --> SciHub{Sci-Hub enabled?}
    
    SciHub -- Yes --> FetchSciHub[Attempt Sci-Hub PDF download]
    FetchSciHub --> SciHubSuccess{Success?}
    
    SciHubSuccess -- Yes --> Store
    SciHubSuccess -- No --> StoreMetadata[Store metadata-only]
    SciHub -- No --> StoreMetadata
    
    Store --> End
    StoreMetadata --> End
```

**CrossRef title-matching confidence** is computed as:
```
score = jaro_winkler(query_title, result_title)
      + 0.2 * (query_year == result_year ? 1 : 0)
      + 0.1 * author_overlap_ratio
Threshold: score >= 0.92 to accept
```

**Unpaywall integration** is now implemented as a native Rust connector targeting `https://api.unpaywall.org/v2/{doi}?email=...`; `best_oa_location.url_for_pdf` is used as an OA full-text retrieval tier when configured with contact email.

---

## 2.4 Open-Access Detection and Full-Text Retrieval Strategy

Ferrumyx operates a tiered retrieval strategy. The goal is to maximise the fraction of papers where full structured text (not just abstract) is available, without violating terms of service.

```mermaid
graph TD
    A[Start Retrieval] --> Explicit{Explicit OA URL?}
    
    Explicit -- Yes --> T1[Tier 1 (preferred): Direct PDF<br/>Fetch & Parse]
    Explicit -- No --> B{PMCID available?}
    
    B -- Yes --> T2[Tier 2: PMC Direct PDF API<br/>Fetch & Parse]
    B -- No --> C{OA Status via Unpaywall?}
    
    C -- Yes --> T3[Tier 3: Unpaywall PDF<br/>Fetch & Parse]
    C -- No --> D{Europe PMC ID?}
    
    D -- Yes --> T4[Tier 4: Europe PMC XML/PDF<br/>Fetch via REST]
    D -- No --> E{bioRxiv/medRxiv?}
    
    E -- Yes --> T5[Tier 5: Preprint PDF<br/>Fetch & Parse]
    E -- No --> G{Sci-Hub fallback<br/>enabled?}
    
    G -- Yes --> SciHub[Initiate Sci-Hub Multi-Domain Loop]
    SciHub --> SH1[Try sci-hub.al] --> SH2{Success?}
    SH2 -- No --> SH3[Try sci-hub.mk] --> SH4{Success?}
    SH4 -- No --> SH5[Try sci-hub.ee / sci-hub.vg / sci-hub.st] --> T7[Tier 7: Sci-Hub PDF<br/>Scrape & Download]
    SH2 -- Yes --> T7
    SH4 -- Yes --> T7
    SH5 -- Failed --> T6[Tier 6 (final): Abstract Only]
    
    G -- No --> T6
```

**Tiered Retrieval Definitions:**

1.  **Tier 1: Explicit OA URL** (Preferred). Direct fetch of a publisher-provided PDF link included in search metadata.
2.  **Tier 2: PMC Direct API**. Resolution of `PMCID` directly to the `europepmc.org/backend/ptpmcrender` endpoint for open-access PDF downloads.
3.  **Tier 3: Unpaywall resolved OA PDF**. High-quality PDF retrieval from official publisher repositories via unpaywall.org.
4.  **Tier 4: Europe PMC**. Fallback for papers not in PMC USA.
5.  **Tier 5: bioRxiv/medRxiv PDF**. Preprint retrieval (lower confidence score applied).
6.  **Tier 7: Sci-Hub Multi-Domain Fallback**. (Optional) Scrapes active Sci-Hub mirrors sequentially (default list: `sci-hub.al`, `sci-hub.mk`, `sci-hub.ee`, `sci-hub.vg`, `sci-hub.st`; configurable in Settings). Disabled by default.
7.  **Tier 6: Abstract Only**. Fallback when no full-text can be legally or technically retrieved.

**Decision stored in DB:** `papers.full_text_status` indicates if full-text was successfully assembled. Typical expectation for recent oncology literature with Sci-Hub enabled: >90% full-text coverage.

---

## 2.5 Structured XML Preference

When PubMed Central XML is available, it is **strongly preferred** over PDF parsing. PMC XML provides deterministic section boundaries, inline citation links, and structured table/formula markup.

### PMC XML Schema (Relevant Elements)

```xml
<article>
  <front>
    <article-meta>
      <article-id pub-id-type="pmid">...</article-id>
      <article-id pub-id-type="pmc">...</article-id>
      <article-id pub-id-type="doi">...</article-id>
      <title-group><article-title>...</article-title></title-group>
      <abstract><p>...</p></abstract>
      <kwd-group><kwd>KRAS</kwd><kwd>G12D</kwd></kwd-group>
    </article-meta>
  </front>
  <body>
    <sec sec-type="intro"><title>Introduction</title><p>...</p></sec>
    <sec sec-type="methods"><title>Methods</title>
      <sec><title>Cell lines</title><p>...</p></sec>
    </sec>
    <sec sec-type="results"><title>Results</title>
      <table-wrap id="T1"><table>...</table></table-wrap>
      <fig id="F1"><caption><p>...</p></caption></fig>
    </sec>
    <sec sec-type="discussion"><title>Discussion</title><p>...</p></sec>
  </body>
  <back>
    <ref-list>
      <ref id="R1">
        <element-citation publication-type="journal">
          <pub-id pub-id-type="doi">...</pub-id>
        </element-citation>
      </ref>
    </ref-list>
  </back>
</article>
```

**Ferrumyx XML parser** (built in Rust using `quick-xml`) extracts:
- Section type from `sec-type` attribute → maps to `section_type` enum: {Abstract, Introduction, Methods, Results, Discussion, Conclusion, SupplementaryMethods, Other}
- `<table-wrap>` contents → serialized to TSV for structured extraction
- `<formula>` (MathML or TeX) → preserved as-is for optional downstream parsing
- `<xref ref-type="bibr">` → inline citation IDs → resolved to DOIs via ref-list

### XML vs PDF Decision Matrix

| Scenario | Preferred format | Reason |
|---|---|---|
| PMC ID available | PMC XML | Deterministic section boundaries |
| Europe PMC indexed | Europe PMC XML | Same benefit, good fallback |
| bioRxiv preprint | PDF + Ferrules | No structured XML available |
| Older journal paper (pre-2005) | PDF + Ferrules | PMC coverage sparse |
| Conference proceedings | PDF + Ferrules | Rarely in PMC |
| Supplementary material | PDF + Ferrules | Never structured in PMC |

---

## 2.6 PDF Parsing Integration: Ferrules (Rust-Native)

### Overview

Ferrumyx uses **Ferrules**, a Rust-native PDF parser built on `lopdf`, for fast and lightweight PDF text extraction. This replaces the previously considered Python-based Docker approaches to eliminate container overhead and keep the stack 100% Rust.

### Implementation

**Location:** `crates/ferrumyx-ingestion/src/pdf_parser.rs`

```rust
/// Parse a PDF file and extract structured sections.
pub fn parse_pdf_sections(pdf_path: &Path) -> Result<ParsedPdf> {
    use lopdf::Document as PdfDoc;
    
    let pdf = PdfDoc::load(pdf_path)?;
    
    // Extract text from all pages
    for (page_num, page) in pdf.get_pages() {
        let content = pdf.get_page_content(page)?;
        // Parse content stream for text operators
        // ...
    }
    
    // Section detection via keyword heuristics
    let sections = detect_sections(&full_text, &pages);
    
    Ok(ParsedPdf { title, sections, full_text, page_count })
}
```

### Section Detection

Heuristic-based section mapping from heading text:

| Heading Pattern | SectionType |
|-----------------|-------------|
| "Abstract" | Abstract |
| "Introduction", "Background" | Introduction |
| "Methods", "Materials and Methods" | Methods |
| "Results" | Results |
| "Discussion" | Discussion |
| "Conclusion" | Conclusion |
| "References" | References |

### Strengths

| Strength | Detail |
|----------|--------|
| Pure Rust | No Docker, no Python, no external dependencies |
| Fast | ~100ms per 10-page PDF (vs 8-12s for Python-based alternatives) |
| Lightweight | ~5MB memory vs 2-4GB VRAM for heavy model-based parsers |
| Local execution | Data never leaves the machine |
| No network calls | Works offline entirely |

### Limitations

| Limitation | Severity | Detail |
|------------|----------|--------|
| No table extraction | High | Tables extracted as plain text; structure lost |
| No figure OCR | High | Images and figures not processed |
| Basic text extraction | Medium | Complex layouts may have ordering issues |
| No formula parsing | Low | Math expressions as plain text |

### Future: Enhanced Table/Figure Extraction

For papers where table/figure extraction is critical:
1.  Primary: Ferrules (fast, for text and entity extraction)
2.  Future: Custom Rust table extraction (for complex tables/figures when needed)

---

## 2.6.1 Entity Recognition Pipeline

### Overview

Fast, database-driven biomedical entity recognition using `TrieNer` (Aho-Corasick) for mention detection and `OncoTree`/`HGNC` normalizers for canonicalization.

**Location:** `crates/ferrumyx-kg/src/ner/`

### Components

1.  **TrieNer** (`trie_ner.rs`)
    - Aho-Corasick automaton for O(n) streaming mention detection.
    - Case-insensitive matching across 30k+ combined patterns.
    - Normalization: Maps detected text to canonical IDs (HGNC Symbol, OncoTree Code).

2.  **CancerNormaliser** (`cancer_normaliser.rs`)
    - Pulls from MSKCC OncoTree JSON API.
    - Recursively parses 897 tumor types across 5 levels of the hierarchy.
    - Provides canonical mapping for common cancer names and synonyms.

3.  **HgncNormaliser** (`hgnc_normaliser.rs`)
    - Loads ~27,000 gene symbols and aliases from HGNC.
    - Handles canonical symbol resolution (e.g., `HER2` -> `ERBB2`).

4.  **RelationExtractor** (`relation_extractor.rs`)
    - Pattern-based extraction of scientific relations (e.g., "associated_with", "inhibits").
    - Extracts evidence snippets for every fact.

### Integration with Ingestion Pipeline

```mermaid
flowchart LR
    Fetch[1. Fetch Paper] --> Parse[2. Parse PDF]
    Parse --> Chunk[3. Chunk Sections]
    Chunk --> NER[4. Extract Entities]
    NER --> Embed[5. Embed Chunks]
    Embed --> Store[6. Store in LanceDB]
```

### Database Schema (entities table)

```sql
CREATE TABLE entities (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    paper_id        UUID REFERENCES papers(id) ON DELETE CASCADE,
    chunk_id        UUID REFERENCES paper_chunks(id) ON DELETE CASCADE,
    entity_type     TEXT NOT NULL,  -- 'GENE', 'DISEASE', 'CHEMICAL'
    entity_text     TEXT NOT NULL,  -- matched text from document
    normalized_id   TEXT,           -- HGNC:6407, MESH:D009369, CHEMBL:xxx
    score           FLOAT,          -- confidence (0.0-1.0)
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (paper_id, chunk_id, entity_type, entity_text)
);
```

### Performance

| Component | Speed | Memory |
|-----------|-------|--------|
| Ferrules PDF parsing | ~100ms/PDF | ~5MB |
| Trie NER | 10M chars/sec | ~5MB |
| Full HGNC load | ~2s (once) | ~50MB |
| Typical paper (10K chars) | <10ms | - |

### Comparison: Alternative PDF Parsers

**Note:** The following section documents alternative PDF parsing approaches considered during design.

#### Ferrules (Selected)
- Pure Rust, no Docker required
- ~100ms per 10-page PDF
- ~5MB memory footprint
- Text extraction with section detection

#### Future Enhancements
For complex table/figure extraction needs, consider:
- Custom Rust table detection on top of lopdf
- Integration with specialized chemistry OCR for molecular structures
- Section-aware parsing with heading detection heuristics

### PDF Output Schema (Ferrules)

```rust
pub struct ParsedPdf {
    pub title: Option<String>,
    pub sections: Vec<DocumentSection>,
    pub full_text: String,
    pub page_count: usize,
}

pub struct DocumentSection {
    pub section_type: SectionType,
    pub heading: Option<String>,
    pub content: String,
    pub page_number: usize,
}
```

The Ferrumyx PDF parser performs heading-to-section-type inference using a case-insensitive keyword lookup table:

```rust
fn infer_section_type(heading: &str) -> SectionType {
    let h = heading.to_lowercase();
    if h.contains("abstract")             { SectionType::Abstract }
    else if h.contains("introduction")    { SectionType::Introduction }
    else if h.contains("method")
         || h.contains("material")        { SectionType::Methods }
    else if h.contains("result")          { SectionType::Results }
    else if h.contains("discussion")      { SectionType::Discussion }
    else if h.contains("conclusion")      { SectionType::Conclusion }
    else                                  { SectionType::Other }
}
```

---

## 2.7 Chunking Strategy

Chunking converts full-text documents into retrieval-optimised units stored in `paper_chunks`. The strategy is section-aware: chunk boundaries follow scientific paper structure rather than arbitrary token windows.

### Rules

| Section type | Chunking rule | Rationale |
|---|---|---|
| Abstract | Single chunk always | Abstract is a semantic unit; never split |
| Introduction | 512-token window, 64-token overlap | Moderate density; context carries across paragraphs |
| Methods | 512-token window, 64-token overlap | Step-by-step detail; overlap preserves procedural continuity |
| Results | 512-token window, 64-token overlap | Data-dense; tables treated separately (see below) |
| Discussion | 512-token window, 64-token overlap | Interpretive; overlap preserves logical flow |
| Conclusion | Single chunk (if ≤ 512 tokens) or 512+64 | Usually short |
| Table | One chunk per table row-group (≤512 tokens) | Tables serialised as "col1: val1 \| col2: val2" |
| Figure caption | Single chunk per figure | Captions are self-contained |
| Supplementary | 512-token window, 64-token overlap | Treated same as methods |

**Token counting:** `tiktoken` Python library (cl100k_base encoding) via a lightweight Docker tool; for Rust-native, `tiktoken-rs` crate. Token count is based on the **embedding model's tokenizer**, not the LLM tokenizer — BiomedBERT uses WordPiece with a 512 subword token limit.

**Important:** BiomedBERT has a hard 512-token limit per input. The 512-token chunk size with 64-token overlap is calibrated to fit within this limit including special tokens ([CLS], [SEP]). Effective content window = 510 tokens.

### Chunk Metadata Schema

Every chunk stored in `paper_chunks` carries:

```sql
paper_id        UUID        -- FK to papers table
section_type    TEXT        -- Abstract|Introduction|Methods|
                            -- Results|Discussion|Conclusion|
                            -- Table|FigureCaption|Other
chunk_index     INTEGER     -- 0-based within section
page_number     INTEGER     -- Source page from Ferrules/PMC XML
token_count     INTEGER     -- Actual token count of this chunk
char_offset     INTEGER     -- Character offset in reconstructed full text
text            TEXT        -- Raw chunk text
embedding       VECTOR(768) -- BiomedBERT-base or VECTOR(1024) for large
created_at      TIMESTAMPTZ
```

**Cross-reference:** This maps directly to the `paper_chunks` table in the Phase 1 LanceDB schema.

### Overlap Implementation

```mermaid
graph LR
    C0[Chunk 0:<br/>t0...t511]
    C1[Chunk 1:<br/>t448...t959]
    C2[Chunk 2:<br/>t896...]
    C0 -.->|64-token overlap| C1
    C1 -.->|64-token overlap| C2
```

Overlap is computed at the token level, not character level, to ensure consistent chunk sizes. The reconstructed overlap is stored only in the later chunk (chunk_index n+1 carries the last 64 tokens of chunk n as its prefix). During retrieval, duplicate text from overlapping chunks is deduplicated by the query handler before passing to LLM context.

---

## 2.8 Embedding Pipeline

### Model Selection

| Model | Dimensions | Max tokens | Intended use | Deployment |
|---|---|---|---|---|
| `jinaai/jina-embeddings-v2-base-en` / `BiomedBERT` | 768 | 8192 / 512 | Default; MVP | Rust Native (Candle) |
| `microsoft/BiomedNLP-PubMedBERT-base-uncased-abstract-fulltext` | 768 | 512 | Default alternative | Rust Native (Candle) |
| `NationalLibraryOfMedicine/BiomedBERT-large-uncased-abstract-fulltext` | 1024 | 512 | High-precision mode | Rust Native (Candle) |
| SPECTER2 (from Semantic Scholar) | 768 | 512 | Citation-aware embeddings, optional | Rust Native (Candle) |

**Default selection:** `microsoft/BiomedNLP-PubMedBERT-base-uncased-abstract-fulltext` — trained on 14M+ PubMed abstracts + full-text articles; strong performance on biomedical STS benchmarks; freely available via HuggingFace.

**High-precision mode** uses the 1024-dim BiomedBERT-large path; enable it by setting `[embedding].backend = "biomedbert"` and `[embedding].embedding_dim = 1024`. Embeddings are stored in the separate LanceDB column `embedding_large VECTOR(1024)`.

### Embedding Service

```yaml
[Ferrumyx Runtime Core Tool "embed_chunks"]
  Crate: ferrumyx-ingestion
  Tech: Rust + Candle Framework

Input (Rust struct):
  EmbedRequest { chunks: vec!["text1", "text2", ...], model: "pubmedbert-base" }

Output (Rust struct):
  EmbedResponse { embeddings: vec![vec![0.123, ...], vec![0.456, ...]], dim: 768 }
```

**Batch size:** 32 chunks per inference call. Larger batches risk OOM on CPU-only environments. On GPU (CUDA), batch size 128 is feasible.

**Throughput estimate:**
- CPU (8-core): ~50 chunks/sec → 1,000-chunk paper ≈ 20s
- GPU (RTX 3080): ~800 chunks/sec → 1,000-chunk paper ≈ 1.25s

### Downstream Embedding Uses

Embeddings are not just a storage primitive in Ferrumyx. They feed multiple downstream retrieval and ranking paths:

- **Hybrid retrieval:** vector similarity in LanceDB is fused with full-text search using reciprocal-rank style merging so semantic matches and lexical matches both survive.
- **RAG context selection:** embeddings help pick the most relevant paper chunks before the LLM receives context, especially when the query terms are sparse or domain-specific.
- **Semantic reranking:** embedding similarity acts as a second-pass relevance signal for shortlisting and ordering candidates after the coarse retrieval stage.
- **Novelty and drift checks:** chunk and paper embeddings can be compared over time to detect whether new literature is semantically close to existing evidence or meaningfully drifting into a new subspace.
- **Deduplication support:** embeddings complement DOI/PMID/title heuristics by catching near-duplicate abstracts, preprint/published variants, and repeated retrieval artifacts.
- **Query response enrichment:** `query_targets` now packages retrieved evidence into a compact downstream embedding payload with RAG snippet selection, gene similarity links, novelty signals, dedup groups, topic clusters, drift mix, and per-gene feature blocks.

These uses primarily consume `paper_chunks.embedding` today, with `embedding_large` reserved for the high-precision 1024-dim path.

### Embedding Speed Mode and Controls

Ferrumyx now supports embedding speed mode auto-determination (`fast|balanced|quality`) with runtime-profile-aware tuning.

**Auto-determination behavior:**

- `FERRUMYX_INGESTION_PERF_MODE` defaults to `auto` and accepts `throughput`, `balanced`, or `safe`.
- When `auto` is selected, the runtime profile is loaded from `FERRUMYX_RUNTIME_PROFILE_PATH` or computed from the current host.
- `RuntimeProfile::detect_and_prepare()` probes RAM, logical CPU count, NVIDIA GPU presence, and CUDA availability.
- In the ingestion tool path, speed mode defaults to `auto` and resolves to:
  - `fast` for throughput-heavy CPU paths,
  - `quality` for smaller GPU-backed runs,
  - `balanced` otherwise.
- Effective embedding batch size is mode-aware (`fast` favors throughput, `quality` favors stability/precision).
- Rust-native embedding max token length is mode-aware:
  - `fast = 256`, `balanced = 384`, `quality = 512`.

**Primary controls:**

- `[ingestion].enable_embeddings` or `FERRUMYX_INGESTION_ENABLE_EMBEDDINGS`
- `[embedding].speed_mode` or `FERRUMYX_EMBED_SPEED_MODE` (`auto|fast|balanced|quality`)
- `[ingestion.performance].embedding_async_backfill` or `FERRUMYX_INGESTION_EMBED_ASYNC_BACKFILL` (non-blocking ingestion completion; vector backfill queued in background)
- `[embedding].throughput_chunk_cap` or `FERRUMYX_EMBED_THROUGHPUT_MAX_CHUNKS_PER_PAPER` (cap per-paper embedding workload in throughput mode)
- `[embedding].backend` values: `rust_native`, `fastembed`, `biomedbert`, `openai`, `gemini`, `openai_compatible`, `ollama` (`fastembed` requires building `ferrumyx-ingestion` with feature `fastembed_backend`)
- `[embedding].embedding_model`
- `[embedding].fast_model` or `FERRUMYX_EMBED_FAST_MODEL` (optional 768-d fast-path model used in `fast` mode; defaults to a FastEmbed-compatible 768-d model when auto-switching)
- `[embedding].embedding_dim`
- `[embedding].batch_size`
- `[embedding].base_url` for local or compatible services
- `[embedding].api_key` or provider env vars
- `FERRUMYX_EMBED_CACHE_DIR` (explicit HF/Candle model cache location)
- `FERRUMYX_EMBED_AUTO_FASTEMBED` (default `true`; auto-switches Rust-native/biomed configs to FastEmbed in `fast` mode for throughput, but only when `fastembed_backend` is compiled in)
- `FERRUMYX_INGESTION_EMBED_GLOBAL_BATCH` (default `true`; defers per-paper embedding and executes one global cross-paper batch pass for better utilization)

**Provider env vars:**

- `FERRUMYX_OPENAI_API_KEY` or `OPENAI_API_KEY`
- `FERRUMYX_GEMINI_API_KEY` or `GEMINI_API_KEY`
- `FERRUMYX_COMPAT_API_KEY` or `LLM_API_KEY`

**Operational notes:**

- `FERRUMYX_CONFIG` selects the TOML file used for auto-resolution.
- `FERRUMYX_PAPER_PROCESS_WORKERS` may be tuned by the ingestion runtime when batch processing is active.
- `FERRUMYX_EMBED_MAX_LENGTH` is set by the tooling to the resolved speed-mode length (`256/384/512`) for runtime transparency.
- Candle/HF model artifacts are cached on disk; if `FERRUMYX_EMBED_CACHE_DIR` is unset, Ferrumyx defaults to `data/cache/hf-hub`.
- The safe path is to keep `embedding_dim` aligned with the selected backend and model; mismatches should be treated as configuration errors rather than silently coerced.
- Query-time downstream semantic rerank can be controlled via `FERRUMYX_QUERY_SEMANTIC_RERANK`, `FERRUMYX_QUERY_SEMANTIC_TOPK`, and `FERRUMYX_QUERY_SEMANTIC_WEIGHT`.
- Downstream embedding payload generation in `query_targets` can be toggled with `FERRUMYX_QUERY_DOWNSTREAM_EMBEDDING`.
- Manual catch-up is exposed via the `backfill_embeddings` runtime tool (optional `paper_ids` and/or `scan_limit`).

### LanceDB Storage

```rust
// Index creation (handled transparently by LanceDB)
// Typical parameters for the IVF-PQ index in LanceDB:
// IVF lists = sqrt(row_count)
// PQ subvectors = 96 (for 768-dim) or 128 (for 1024-dim)
```

**Hybrid search (RRF):** Ferrumyx Runtime Core's built-in hybrid search combines vector similarity (ANN via LanceDB) with BM25/FTS full-text search on `paper_chunks.content`. Reciprocal Rank Fusion weight: `rrf_k = 60` (default). Vector search weight 0.7, keyword weight 0.3 for domain-specific biomedical queries.

```rust
// Hybrid search query pattern via LanceDB
let mut stream = chunks_table
    .search(query_vector)
    .limit(100)
    .execute()
    .await?;
// In-memory or LanceDB-native FTS filter is applied combining RRF scores.
```

---

## 2.9 Biomedical NER Pipeline Evaluation

### MVP Configuration

**Primary:** `TrieNer` (Native Rust Aho-Corasick Automaton)

| Component | Entity types | Speed | Deployment |
|---|---|---|---|
| `TrieNer` Core | General biomedical: disease, chemical, gene, protein, cell line | 10M+ chars/sec | Native Rust (`ferrumyx-kg`) |
| `EntityLoader` | HGNC / MeSH / ChEMBL | Auto-downloads | Native Rust |

**Why TrieNer for MVP:**
- 100% Native Rust execution (no Python/Docker overhead)
- O(n) guaranteed matching speed via Aho-Corasick algorithmic properties
- Extremely lightweight (~5MB memory footprint)
- Mature dictionary-based extraction is highly precise for known ontology IDs (HGNC, MeSH)

### High-Recall Mode: Future LLM Integration

Future versions plan to integrate local LLM-based extraction (e.g., using Ollama structured output) for discovering truly novel entities outside of mapped ontology databases, combined with `TrieNer` acting as a fast first-pass filter.
| Entity normalisation (mutations) | **Build** | HGVS notation normalisation using Rust hgvs crate + custom regex for informal notations (G12D → p.Gly12Asp) |
| Entity normalisation (diseases) | **Integrate** | OLS (Ontology Lookup Service) REST API for MeSH/OMIM lookups |
| Custom oncology gazetteer | **Build** | Curated list of KRAS/RAS pathway members, common oncology abbreviations; used as pre-filter |
| NER result storage | **Build** | Custom `entity_mentions` table in LanceDB |

### NER Result Schema

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMention {
    pub id: uuid::Uuid,
    pub chunk_id: uuid::Uuid,
    pub paper_id: uuid::Uuid,
    pub mention_text: String,
    pub entity_type: String,     // 'gene'|'mutation'|'disease'|'drug'|'cell_line'|'pathway'
    pub norm_id: Option<String>, // 'HGNC:1097', 'MESH:D010190', 'rs121913529'
    pub norm_source: Option<String>, // 'HGNC'|'MESH'|'OMIM'|'DBSNP'|'CHEMBL'
    pub confidence: Option<f32>,
    pub char_start: Option<i64>,
    pub char_end: Option<i64>,
    pub model_source: Option<String>, // 'TrieNer'|'bern2'|'gazetteer'|'trie_ner'
    pub created_at: chrono::DateTime<chrono::Utc>,
}
```

---

## 2.10 Deduplication Strategy

Duplicate papers arise when the same work is indexed by multiple sources (e.g., a paper appears
in both PubMed and Europe PMC). Three deduplication tiers are applied in sequence:

**Tier 0 — Canonical identity key during source fan-in:**
- Every fetched paper is normalized to an identity key in this order: `doi:*` -> `pmid:*` -> `pmcid:*` -> `title:*` (normalized text).
- Cross-source dedupe is applied before upsert and before expensive full-text/NER work.
- Source fan-in can terminate early once a unique-result target is reached, preventing unnecessary duplicate fetch work.

**Tier 1 — DOI match (primary):**
- If `doi` field is non-null and matches an existing `papers.doi`: skip ingestion, log as duplicate in `ingestion_audit`
- DOIs are normalised before comparison: lowercase, strip `https://doi.org/` prefix, trim whitespace
- This catches ~85% of true duplicates for recent literature

**Tier 2 — Abstract SimHash (secondary):**
- Compute 64-bit SimHash of the abstract text (after stripping whitespace/punctuation)
- If Hamming distance to any existing abstract SimHash ≤ 3: flag as probable duplicate, queue for manual review
- SimHash stored in `papers.abstract_simhash` (BIGINT column)
- Threshold of 3 bits allows for minor OCR/formatting differences between sources

**Tier 3 — Fuzzy title + author match (tertiary):**
- Applied only when DOI is null AND SimHash check is inconclusive
- Jaccard similarity on title token set: if ≥ 0.95 AND first-author surname matches: flag as duplicate
- Jaccard(A, B) = |A ∩ B| / |A ∪ B| where A, B are sets of lowercase title tokens (stopwords removed)
- Computationally expensive; applied only to papers from the same year ± 1

**Source priority for canonical record:**
1.  PubMed (richest structured metadata, MeSH terms)
2.  Europe PMC (good fallback, grants/patent cross-refs)
3.  bioRxiv/medRxiv (preprint; superseded when PubMed record appears)
4.  Semantic Scholar / CrossRef (metadata-only fallback)

When a duplicate is detected across sources, the canonical record is updated to merge: all available IDs (PMID, PMCID, DOI, S2 paper ID), open-access URLs from all sources, and citation counts.

**Ingestion audit log:** Cross-reference to the `ingestion_audit` table defined in Phase 1 schema. Every deduplication event (tier matched, action taken) is logged with `action = 'deduplicated'` and `detail = {tier: 1, matched_paper_id: "..."}`.

---

Deduplication runs at ingestion time, before any downstream processing.

### Deduplication Algorithm (Ordered by Priority)

**Stage 1 — DOI exact match (primary key)**
```text
IF incoming.doi IS NOT NULL:
    IF EXISTS (SELECT 1 FROM papers WHERE doi = incoming.doi):
        → DUPLICATE: skip ingestion, log to ingestion_audit as 'deduplicated'
    ELSE:
        → PROCEED
```

**Stage 2 — Abstract SimHash (secondary)**
- Compute 64-bit SimHash of abstract text (normalised: lowercase, strip punctuation, remove stopwords)
- Hamming distance < 4 between incoming and any stored abstract SimHash → flag as likely duplicate
- Store SimHash in `papers.abstract_simhash` (BIGINT column with index)
- Trigger manual review if SimHash match but DOI differs (could be preprint → published version pair)

**Stage 3 — Fuzzy title + first-author match (tertiary)**
- Jaccard similarity on title trigrams ≥ 0.92 AND first author family name matches → flag as probable duplicate
- Used only when DOI is absent AND abstract SimHash is unavailable (e.g., very short abstracts)

**Preprint → Published Pairing**
- bioRxiv DOI `10.1101/XXXXXX` matched to published DOI via CrossRef `relation.is-preprint-of` field
- When pairing detected: retain both records, link via `papers.published_version_doi` FK
- Published version takes precedence in scoring; preprint remains for provenance

### Deduplication Audit Log
Cross-reference: handled by `ingestion_audit` table (Phase 1, §1.4) with `action = 'deduplicated'` and `detail = {duplicate_of: <paper_id>, method: 'doi'|'simhash'|'fuzzy_title'}`.

---

## 2.11 Build vs Integrate vs Wrap Summary (Phase 2)

| Component | Decision | Justification |
|---|---|---|
| PubMed E-utilities client | **Build** (Rust native module) | Simple REST; Rust HTTP client sufficient |
| Europe PMC client | **Build** (Rust native module) | Same |
| bioRxiv/medRxiv client | **Build** (Rust native module) | Simple JSON API |
| arXiv client | **Build** (Rust native module) | Atom XML; `quick-xml` crate |
| ClinicalTrials.gov client | **Build** (Rust native module) | REST JSON API |
| CrossRef client | **Build** (Rust native module) | DOI resolution; simple REST |
| Semantic Scholar client | **Build** (Rust native module) | REST; citation expansion + SPECTER2 field support |
| Unpaywall client | **Build** (Rust native module) | DOI OA lookup |
| PDF parser | **Build** (Rust, lopdf) | Ferrules - fast, Rust-native text extraction |
| BiomedBERT embeddings | **Build** (Rust, Candle) | HuggingFace models run natively via Candle framework |
| TrieNer NER | **Build** (Rust, Aho-Corasick) | 100% native Rust memory-safe fast NER |
| BERN2 Fallback | **Scrapped** | Local PyTorch ensemble is too heavy and violates 100% Rust architecture goals. Relying entirely on native `TrieNer`. |
| Entity normalisation (genes) | **Build** (Rust) | Custom HGNC lookup table + hgvs crate |
| Entity normalisation (mutations) | **Build** (Rust) | HGVS regex + variant notation normaliser |
| Entity normalisation (diseases) | **Integrate** (OLS REST API) | MeSH/OMIM lookup via EBI OLS |
| Deduplication logic | **Build** (Rust) | SimHash implementation; straightforward |
| Chunking pipeline | **Build** (Rust) | Section-aware logic; tightly coupled to PMC XML parser |
| PMC XML parser | **Build** (Rust, `quick-xml`) | Ferrumyx-specific section mapping |

---

## 2.12 Phase 2 Implementation Audit (2026-03-14)

Implemented in production code:

- [x] Multi-source ingestion orchestrator (`run_ingestion`) with source fan-out and stage progress events.
- [x] Source clients: PubMed, Europe PMC, bioRxiv, medRxiv, ClinicalTrials.gov, CrossRef, Semantic Scholar (core Graph API search + metadata + OA PDF URL resolution).
- [x] Optional Sci-Hub fallback client and pipeline path (`enable_scihub_fallback`).
- [x] Deduplication at ingestion with DOI/PMID checks and repository-level guardrails; SimHash utilities exist (`dedup.rs`) and schema support exists (`papers.abstract_simhash`).
- [x] Ferrules/lopdf-based PDF parsing integrated in pipeline (`parse_pdf_sections`).
- [x] Section-aware chunking + optional embedding pass with pluggable backends (Rust-native BiomedBERT, OpenAI, Gemini, OpenAI-compatible, Ollama).
- [x] Ingestion watchdog controls (idle timeout + max runtime) and settings exposure.
- [x] Ingestion API key settings include PubMed + Semantic Scholar and are wired to agent + web ingestion paths.
- [x] Source-search cache + TTL controls and persistence are active.
- [x] Full-text negative cache + TTL controls are active to avoid repeated dead-end fetch attempts.
- [x] Full-text success cache + TTL controls are active to skip repeated successful fetch/parse work.
- [x] Chunk-fingerprint cache + TTL controls are active to skip redundant heavy NER/relation passes.
- [x] Batch DOI/PMID prefetch dedup is implemented to reduce per-paper DB round-trips.
- [x] Adaptive worker tuning and fast-lane/heavy-lane async enrichment are implemented for higher sustained throughput.
- [x] Heavy-lane async flow now supports explicit completion draining (`FERRUMYX_INGESTION_HEAVY_LANE_DRAIN`) and corrected process telemetry (`process_ms` reflects process stage, not prefetch time).
- [x] New-paper prefetch handoff avoids duplicate `PaperMetadata` cloning on the upsert-to-prefetch path.
- [x] Missing-embedding discovery now uses DB-side `embedding IS NULL` filtering first, with safe fallback when filter execution is unsupported.
- [x] Embedding backfill updates now use chunk-ID keyed batch merge-upsert (`FERRUMYX_INGESTION_EMBED_UPDATE_BATCH_SIZE`), replacing per-chunk read/delete/reinsert loops in the bulk path.
- [x] Dedupe decisions persist structured `ingestion_audit` rows (DOI/PMID/title/strict-fuzzy) in addition to trace logs.
- [x] Sci-Hub fallback is now adaptive and settings-driven (mirror parallelism/cooldown + defer/backoff/probe/budget controls) for better degraded-mode throughput.
- [x] Ingestion performance snapshots are persisted and exposed via `/api/metrics/perf`.

Partially implemented / pending:

- [x] PMC XML strategy is active (PubMed/EuropePMC XML fetch hooks + section-aware parsing path using `quick-xml`, preferred before PDF fallback when PMCID is available).
- [x] SimHash/fuzzy-title dedup hardening is enforced in repository upsert flow (DOI/PMID exact + abstract SimHash + fuzzy title/author guardrail).
- [x] arXiv ingestion source client is implemented (Atom API search + metadata mapping + PDF URL derivation).
- [x] Unpaywall integration is implemented (DOI lookup tier with configured contact email).
- [x] Semantic Scholar citation-graph traversal is integrated with bounded expansion and final dedup.
- [x] Semantic Scholar SPECTER2 embedding fetch support is implemented in the source client (`embedding.specter_v2` field path).

Implementation note:
- Several Phase 2 tables describe connectors as “WASM tools.” Current Ferrumyx implementation runs these connectors as native Rust modules in the ingestion crate and exposes them through Ferrumyx/Ferrumyx Runtime Core tools and web handlers.

# Phase 3: Knowledge Graph & Target Intelligence

## 3.1 Entity Type Schemas

```rust
// Gene / Protein
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntGene {
    pub id: uuid::Uuid,
    pub hgnc_id: Option<String>,
    pub symbol: String,
    pub name: Option<String>,
    pub uniprot_id: Option<String>,
    pub ensembl_id: Option<String>,
    pub entrez_id: Option<String>,
    pub gene_biotype: Option<String>,
    pub chromosome: Option<String>,
    pub strand: Option<i16>,
    pub aliases: Option<Vec<String>>,
    pub oncogene_flag: bool,
    pub tsg_flag: bool,
    pub created_at: chrono::DateTime<chrono::Utc>
}

// Mutation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntMutation {
    pub id: uuid::Uuid,
    pub gene_id: uuid::Uuid,
    pub hgvs_p: Option<String>,
    pub hgvs_c: Option<String>,
    pub rs_id: Option<String>,
    pub aa_ref: Option<String>,
    pub aa_alt: Option<String>,
    pub aa_position: Option<i32>,
    pub oncogenicity: Option<String>,
    pub hotspot_flag: bool,
    pub vaf_context: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>
}

// Cancer Type (OncoTree)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntCancerType {
    pub id: uuid::Uuid,
    pub oncotree_code: Option<String>,
    pub oncotree_name: Option<String>,
    pub icd_o3_code: Option<String>,
    pub tissue: Option<String>,
    pub parent_code: Option<String>,
    pub level: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>
}

// Pathway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntPathway {
    pub id: uuid::Uuid,
    pub kegg_id: Option<String>,
    pub reactome_id: Option<String>,
    pub go_term: Option<String>,
    pub name: String,
    pub gene_members: Option<Vec<String>>, // Array of HGNC symbols
    pub source: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>
}

// Clinical Evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntClinicalEvidence {
    pub id: uuid::Uuid,
    pub nct_id: Option<String>,
    pub pmid: Option<String>,
    pub doi: Option<String>,
    pub phase: Option<String>,
    pub intervention: Option<String>,
    pub target_gene_id: uuid::Uuid,
    pub cancer_id: uuid::Uuid,
    pub primary_endpoint: Option<String>,
    pub outcome: Option<String>,
    pub evidence_grade: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>
}

// Compounds / Inhibitors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntCompound {
    pub id: uuid::Uuid,
    pub chembl_id: Option<String>,
    pub name: Option<String>,
    pub smiles: Option<String>,
    pub inchi_key: Option<String>,
    pub moa: Option<String>,
    pub patent_status: Option<String>,
    pub max_phase: Option<i32>,
    pub target_gene_ids: Option<Vec<uuid::Uuid>>,
    pub created_at: chrono::DateTime<chrono::Utc>
}

// Structural Availability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntStructure {
    pub id: uuid::Uuid,
    pub gene_id: uuid::Uuid,
    pub pdb_ids: Option<Vec<String>>,
    pub best_resolution: Option<f32>,
    pub exp_method: Option<String>,
    pub af_accession: Option<String>,
    pub af_plddt_mean: Option<f32>,
    pub af_plddt_active: Option<f32>,
    pub has_pdb: bool,
    pub has_alphafold: bool,
    pub updated_at: chrono::DateTime<chrono::Utc>
}

// Druggability Score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntDruggability {
    pub id: uuid::Uuid,
    pub structure_id: uuid::Uuid,
    pub fpocket_score: Option<f32>,
    pub fpocket_volume: Option<f32>,
    pub fpocket_pocket_count: Option<i32>,
    pub dogsitescorer: Option<f32>,
    pub overall_assessment: Option<String>,
    pub assessed_at: chrono::DateTime<chrono::Utc>
}

// Synthetic Lethality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntSyntheticLethality {
    pub id: uuid::Uuid,
    pub gene1_id: uuid::Uuid,
    pub gene2_id: uuid::Uuid,
    pub cancer_id: uuid::Uuid,
    pub evidence_type: Option<String>,
    pub source_db: Option<String>,
    pub screen_id: Option<String>,
    pub effect_size: Option<f32>,
    pub confidence: Option<f32>,
    pub pmid: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>
}
```

## 3.2 Deterministic NER Confidence Scoring Model

Every `kg_facts` mention and relationship has a `confidence` value in [0, 1] computed dynamically. During Phase 3.2, Ferrumyx moved away from hardcoded ML confidence estimates to a deterministic scaling formula evaluated directly within the Aho-Corasick `TrieNer`.

### Confidence Modifiers (Calculated at Match Time)

```text
confidence = base_weight * tier_modifier * text_length_modifier * boundary_validation
```

| Entity & Match Type | Base Weight / Modifier | Rationale |
|---|---|---|
| **Gene (Preferred Symbol)** | 1.00 | Exact HGNC approved symbol (e.g., `KRAS`, `BRAF`) |
| **Gene (Alias)** | 0.85 | Common historical alias (e.g., `c-Ki-ras`) |
| **Gene (Previous)** | 0.75 | Outdated/Legacy symbol mapping |
| **Cancer (OncoTree Code)** | 1.00 | Pure classification code (e.g., `LUAD`, `SKCM`) |
| **Cancer (OncoTree Name)** | 0.90 | Textual description (e.g., `Lung Adenocarcinoma`) |

### Ambiguity and Length Penalties

Short acronyms frequently trigger false positives (e.g., `RB` matching inside `VERB`, or `ABL` inside `VARIABLE`). To counteract this:

1. **Length Penalty:** If a matched symbol length is `< 4` characters, an automatic `-0.15` penalty is applied to the confidence score (unless it's a structural OncoTree Code).
2. **Word Boundary Check:** If a symbol's length is `<= 3`, `TrieNer` enforces a strict word-boundary check on the surrounding characters (must be space, punctuation, or string ends). If it fails the boundary check, the match is outright rejected (confidence = 0.0).
3. **Hard Threshold:** Any match whose final calculated confidence drops below `0.75` is discarded before insertion into the Knowledge Graph to maintain high data fidelity.

**Example 1:** Finding "KRAS" (Preferred, Length=4) -> Score = 1.00
**Example 2:** Finding "c-Ki-ras" (Alias, Length=8) -> Score = 0.85
**Example 3:** Finding "RB1" (Preferred, Length=3) -> Boundary OK -> Score = 1.00 - 0.15 (short) = 0.85

**Implementation:** Handled internally via `SymbolTier` and `CancerPatternKind` enums within `hgnc.rs` and `cancer_normaliser.rs`, resolving down to `PatternMeta` within the `TrieNer` automaton map.

## 3.3 Evidence Weighting & Aggregation

When multiple independent facts support the same (subject, predicate, object) triple:

```text
aggregate_confidence = 1 - Π(1 - confidence_i)
```

This is the noisy-OR model — each independent piece of evidence adds to aggregate certainty.

**Contradictory evidence** (e.g., two facts with opposite directionality on the same predicate):

```text
net_confidence = |Σ(signed_confidence_i)|
where: signed_confidence_i = +confidence_i (supporting) or -confidence_i (contradicting)
```

Contradiction handling:
- Both facts stored with `contradiction_flag = TRUE`
- `kg_conflicts` table records the conflict pair
- Query output always surfaces both supporting and contradicting evidence explicitly
- Aggregate confidence of the net fact is penalised: final_confidence = net_confidence × 0.7

```sql
CREATE TABLE kg_conflicts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fact_a_id       UUID REFERENCES kg_facts(id),
    fact_b_id       UUID REFERENCES kg_facts(id),
    conflict_type   TEXT,   -- 'directional'|'magnitude'|'existence'
    net_confidence  FLOAT,
    resolution      TEXT DEFAULT 'unresolved', -- 'unresolved'|'resolved'|'manual_review'
    detected_at     TIMESTAMPTZ DEFAULT NOW()
);
```

## 3.4 Update Rules

Update pipeline is **event-driven**, not batch-scheduled:

```text
[Ferrumyx Runtime Core routines event:
 "kg_fact_changed" → triggers
 "score_recompute" routine
 for affected (gene, cancer) pairs]
```

Re-scoring is asynchronous: queued jobs processed by the routines engine during low-activity windows (configurable: e.g., off-peak hours or immediately if queue depth < 10).

## 3.5 Versioning Strategy

- `kg_facts` is **strictly append-only** — rows are never updated or deleted
- `valid_from` set on INSERT; `valid_until` set only on supersession
- Current facts: `WHERE valid_until IS NULL`
- Supersession example (retraction):
  ```sql
  BEGIN;
    UPDATE kg_facts SET valid_until = NOW()
    WHERE id = <retracted_fact_id>;
    INSERT INTO kg_facts (..., confidence = 0.0, evidence_type = 'retraction', valid_from = NOW());
  COMMIT;
  ```
- **Schema versioning**: `schema_migrations` table with migration ID, applied_at, checksum
- **Score versioning**: `target_scores.score_version` is an integer monotonically increasing per (gene_id, cancer_id) pair; old versions preserved with `is_current = FALSE`

## 3.6 Conflict Resolution Logic

```mermaid
flowchart TD
    Detect[1. DETECT: Same triple, opposite direction<br/>OR confidence delta > 0.4] --> Log[2. LOG: INSERT into kg_conflicts]
    Log --> Compute[3. COMPUTE net_confidence<br/>net = |conf_a - conf_b|]
    Compute --> Classify{4. CLASSIFY net_confidence}
    
    Classify -- "< 0.30" --> Disputed[Mark as DISPUTED<br/>Exclude from default scoring]
    Classify -- "0.30 - 0.60" --> Penalty[Include with DISPUTED flag<br/>Confidence penalty: x0.70]
    Classify -- "> 0.60" --> Dominant[Treat dominant direction as current<br/>Note minority evidence]
    
    Disputed --> Review{5. HUMAN REVIEW?}
    Penalty --> Review
    Dominant --> Review
    
    Review -- "Both conf > 0.70" --> HighPri[INSERT into human_review_queue<br/>Priority: HIGH<br/>Notify operator]
```

## 3.7 Graph Traversal Query Patterns

JOIN ent_compounds c ON kf2.subject_id = c.id
WHERE kf1.confidence > 0.5
GROUP BY g.symbol, g.hgnc_id
ORDER BY inhibitor_count DESC;
```

### Pattern 2: Synthetic lethal partners of KRAS G12D with structural availability

```sql
SELECT
    g2.symbol AS sl_partner,
    sl.effect_size,
    sl.confidence AS sl_confidence,
    es.has_pdb,
    es.af_plddt_mean,
    ed.fpocket_score
FROM ent_mutations m
JOIN ent_genes g1 ON m.gene_id = g1.id AND g1.symbol = 'KRAS'
                 AND m.hgvs_p = 'p.Gly12Asp'
JOIN ent_synthetic_lethality sl ON sl.gene1_id = g1.id
JOIN ent_genes g2 ON sl.gene2_id = g2.id
LEFT JOIN ent_structures es ON es.gene_id = g2.id
LEFT JOIN ent_druggability ed ON ed.structure_id = es.id
WHERE sl.confidence > 0.5
  AND (es.has_pdb = TRUE OR es.af_plddt_mean > 70)
ORDER BY sl.effect_size ASC, ed.fpocket_score DESC;
```

### Pattern 3: Semantic similarity — find targets mechanistically similar to KRAS

```sql
-- Assumes entity embeddings stored in ent_genes.embedding VECTOR(768)
SELECT g2.symbol, g2.hgnc_id,
       1 - (g1.embedding <=> g2.embedding) AS cosine_similarity
FROM ent_genes g1, ent_genes g2
WHERE g1.symbol = 'KRAS'
  AND g2.id != g1.id
ORDER BY g1.embedding <=> g2.embedding
LIMIT 20;
```

## 3.8 LanceDB-only vs Hybrid Graph Approach: Assessment

| Dimension | LanceDB-only | + Dedicated Graph DB |
|---|---|---|
| MVP operational complexity | Low (1 DB) | High (2 DBs, sync layer) |
| Native graph traversal | Moderate (JOINs, CTEs) | Excellent (Cypher, native graph) |
| >3-hop path queries | Slow above 5M facts | Fast |
| Vector search | Native (LanceDB) | Not supported natively |
| Ferrumyx Runtime Core integration | Already integrated | Requires new adapter |
| Licensing | Open source | Community edition free; Enterprise: $$ |
| Sync complexity | N/A | CDC pipeline required (Debezium) |

**Recommendation: LanceDB-only through Month 6.**

Rationale: For the MVP cancer domain (KRAS G12D PDAC), the knowledge graph will contain <500K facts. All required traversal patterns (up to 4-hop joins) are feasible in LanceDB with proper indexing. The operational cost of maintaining a second database and synchronisation layer is not justified at this scale.

**Trigger for dedicated graph adoption:** If at Month 12, path traversal queries on >5M facts exceed 500ms P95 latency for common patterns. If adopted, the graph DB is a read-only analytical mirror updated via CDC; LanceDB remains the write-primary source of truth.

## 3.9 Completion Status (2026-03-15)

Phase 3 is now treated as complete for the current codebase baseline.

- [x] Paper-centric KG ingestion and relation extraction are live.
- [x] Dynamic entity typeahead and performance-capped KG rendering are live.
- [x] KG graph coordinates are deterministic/topology-aware (2D projected from Rust 3D layout, stable seeded fallback; no random layout dependency in runtime rendering).
- [x] KG confidence/provenance tiering is surfaced in API/UI (`high|medium|low`, `provider|extracted|generic`) for focused filtering and interpretation.
- [x] Chat gateway flow is asynchronous and thread/history-backed (no longer assuming immediate sync replies).
- [x] Agent tool surface now includes autonomous loop controls:
  - `ingest_literature`
  - `query_targets`
  - `workflow_status`
  - `recompute_target_scores`
  - `run_molecule_pipeline`
  - `run_autonomous_cycle`
  - `run_system_command` (guarded host command execution for autonomous diagnostics/remediation)
- [x] Settings-driven provider/env sync into Ferrumyx Runtime Core is live (including OpenAI-compatible cached-chat toggle).

These changes satisfy the Phase 3 target for autonomous KG-driven operation and transition readiness into Phase 4 scoring/quality work.

---

# Phase 4: Target Prioritization Engine

## 4.1 Composite Score Formula

The composite target priority score for gene *g* in cancer context *c* is:

```text
S(g, c) = [ Σᵢ wᵢ × nᵢ(g, c) ] − P(g, c)

constrained to: S(g, c) ∈ [0, 1]

Confidence-adjusted:
S_adj(g, c) = S(g, c) × C(g, c)

where C(g, c) = weighted_mean(confidence_i, weight_i)
               over KG facts contributing to the 9 component scores
               with weight_i derived from:
               - provenance tier (provider > extracted > generic),
               - confidence tier (high > medium > low),
               - predicate specificity (typed > associated_with > mentions)
```

### Components

| i | Component | Description |
|---|---|---|
| 1 | `mutation_freq` | Frequency of gain-of-function mutations in cancer c (COSMIC/cBioPortal) |
| 2 | `crispr_dependency` | CERES dependency score (DepMap Achilles): inverted & normalised |
| 3 | `survival_correlation` | Kaplan-Meier log-rank p-value / hazard ratio from TCGA expression data |
| 4 | `expression_specificity` | Tumour/normal expression ratio: mean_tumour_TPM / (mean_normal_GTEx_TPM + ε) |
| 5 | `structural_tractability` | Composite: PDB coverage weight + AlphaFold pLDDT + pocket druggability |
| 6 | `pocket_detectability` | fpocket best_score normalised; DoGSiteScorer if available |
| 7 | `novelty_score` | Inverse inhibitor density: 1 / (1 + ChEMBL_inhibitor_count) |
| 8 | `pathway_independence` | Inverse redundancy: 1 / (1 + Reactome_escape_pathway_count) |
| 9 | `literature_novelty` | Underexplored ratio: inverted 2yr citation velocity |

### Initial Weight Vector W

```python
W = {
  w1: 0.20,   # mutation_freq         — highest weight; biological relevance anchor
  w2: 0.18,   # crispr_dependency     — strong functional evidence
  w3: 0.15,   # survival_correlation  — clinical translational signal
  w4: 0.12,   # expression_specificity — therapeutic window
  w5: 0.12,   # structural_tractability — chemical tractability gate
  w6: 0.08,   # pocket_detectability  — docking feasibility
  w7: 0.07,   # novelty_score         — differentiation from existing drugs
  w8: 0.05,   # pathway_independence  — resistance risk proxy
  w9: 0.03,   # literature_novelty    — discovery opportunity signal
}
# Sum = 1.00
```

### Penalty Term P(g, c)

```text
P(g, c) = Σ penalty_k

where:
  inhibitor_saturation_penalty = 0.15  IF ChEMBL_count > 50 ELSE 0
  low_specificity_penalty      = 0.10  IF expression_ratio < 1.5 ELSE 0
  structural_void_penalty      = 0.08  IF no PDB AND af_pLDDT < 50 ELSE 0

P(g, c) = inhibitor_saturation_penalty
         + low_specificity_penalty
         + structural_void_penalty
```

### structural_tractability Sub-Formula

```text
structural_tractability(g) =
    0.40 × pdb_coverage_score
  + 0.35 × (af_plddt_mean / 100)     [if no PDB]
  + 0.25 × pocket_druggability_norm

where:
  pdb_coverage_score = min(pdb_structure_count / 5, 1.0)  [5+ structures = full score]
  pocket_druggability_norm = fpocket_score / 1.0  [fpocket scores typically 0–1]
```

## 4.2 Normalization Strategy

**Default: Rank-based normalization** (chosen for outlier robustness)

```text
For component i, across all N candidate (gene, cancer) pairs:
  rank_i(g, c) = rank of (g, c) among all candidates by raw component value
  n_i(g, c) = rank_i(g, c) / N

So the top-ranked candidate gets n_i = 1.0, last-ranked gets n_i ≈ 0.
```

**Exception — CRISPR dependency (component 2):**
CERES scores have biological meaning at specific thresholds (< -1.0 = strongly essential). Apply min-max within [-2.0, 0.0] range before ranking:
```text
ceres_normalised = (ceres_score - (-2.0)) / (0.0 - (-2.0))
                 = (ceres_score + 2.0) / 2.0
# More negative (more essential) → lower raw → inverted for scoring:
n2 = 1.0 - ceres_normalised   [so more essential = higher component score]
```

**Rationale for rank over min-max:** A single dominant outlier (e.g., KRAS mutation frequency 0.95 in PDAC vs 0.02 for all others) would compress all other scores to near-zero under min-max, losing discrimination among the remaining candidates. Rank normalization preserves relative ordering across the cohort.

## 4.3 Data Sources Per Component

| Component | Source | Endpoint | Update Freq | License |
|---|---|---|---|---|
| mutation_freq | COSMIC v3.4 | cancer.sanger.ac.uk/cosmic/download (registration) | Annual | Academic free |
| mutation_freq (alt) | cBioPortal REST | cbioportal.org/api | Continuous | Open |
| crispr_dependency | DepMap Achilles | depmap.org/portal/download (CERES CSV) | Quarterly | CC BY 4.0 |
| survival_correlation | GDC TCGA | api.gdc.cancer.gov | Stable | Public domain |
| expression_specificity | GTEx v10 | gtexportal.org/api/v2 + TCGA GDC | Annual | dbGaP open |
| structural_tractability | RCSB PDB REST | data.rcsb.org/rest/v1 | Weekly | Open |
| structural_tractability | AlphaFold DB | alphafold.ebi.ac.uk/api | Stable | CC BY 4.0 |
| pocket_detectability | fpocket (local) | Compute on-demand | On-demand | BSD |
| novelty_score | ChEMBL REST | ebi.ac.uk/chembl/api/data | Quarterly | CC BY-SA 3.0 |
| pathway_independence | Reactome REST | reactome.org/ContentService | Quarterly | CC0 |
| literature_novelty | Semantic Scholar | api.semanticscholar.org/graph/v1 | Continuous | Open |

## 4.4 Score Storage and Versioning

Handled by `target_scores` table (Phase 1, §1.4). Key properties:
- Each scoring run inserts new rows; `is_current` flag flipped atomically within a transaction
- `component_scores JSONB` stores all 9 raw values, normalised values, and the weight vector used
- Historical scores are never deleted — full audit trail for reproducibility
- Re-scoring trigger: any `kg_fact_changed` event affecting a component data source, or manual trigger via REPL

```sql
-- Query current top targets for KRAS G12D PDAC
SELECT g.symbol, ct.oncotree_code, ts.composite_score,
       ts.component_scores, ts.confidence_adj, ts.scored_at
FROM target_scores ts
JOIN ent_genes g ON ts.gene_entity_id = g.id
JOIN ent_cancer_types ct ON ts.cancer_entity_id = ct.id
WHERE ct.oncotree_code = 'PAAD'
  AND ts.is_current = TRUE
ORDER BY ts.composite_score DESC
LIMIT 20;
```

## 4.5 Threshold Logic for Shortlisting

```text
PRIMARY SHORTLIST (high confidence):
  S_adj > 0.60
  AND mutation_freq_raw > 0.05      (present in ≥5% of tumours)
  AND structural_tractability > 0.40

SECONDARY SHORTLIST (exploratory):
  S_adj > 0.45
  AND no hard exclusions

HARD EXCLUSION RULES:
  - ChEMBL inhibitor count > 50 AND novelty_score < 0.20:
    → EXCLUDED unless query explicitly requests "known targets"
  - expression_specificity < 1.20:
    → INCLUDE with WARNING: "not tumour-enriched; normal tissue toxicity risk"
  - No PDB AND af_pLDDT < 50:
    → INCLUDE with FLAG: "structurally unresolved; docking unreliable"
  - DISPUTED relationship in KG (conflict_resolution = DISPUTED):
    → INCLUDE with DISPUTED badge; exclude from default rankings

OUTPUT FIELDS (per candidate):
  rank, composite_score, confidence_adjusted_score, percentile,
  component_score_breakdown[9], shortlist_tier, flags[], warnings[]
```

## 4.6 Phase 4 Kickoff Status (2026-03-14)

- [x] Cohort scoring now runs at **gene-level** rather than per-fact row expansion.
- [x] Random/hash fallback metrics removed from target query path.
- [x] Ranking metrics now derived deterministically from KG evidence aggregates (with explicit proxy fields where external providers are not yet fully wired).
- [x] Shortlist tiering now enforces the architecture hard exclusion rule for saturated/low-novelty targets.
- [x] DISPUTED and quality-risk flags are surfaced directly in query results.
- [x] Ranker/targets/metrics handlers now avoid large full-table scans and use bounded reads + aggregated counters for stable latency on larger corpora.

Remaining Phase 4 hardening work:
- [x] Replace proxy-derived components with fully source-backed components for all 9 metrics (DepMap/TCGA/GTEx/Reactome/ChEMBL joins) via source-missing defaults + provider/entity-stage fills.
- [x] `n1` mutation frequency supports source-backed cBioPortal + COSMIC caching (`ent_cbio_mutation_frequency`, `ent_cosmic_mutation_frequency`) with runtime fetch + TTL-backed reuse, same-cancer fallback, and any-cancer fallback paths for broader cohort coverage.
- [x] `n5`/`n6` structural components run in source-backed mode for larger cohorts (`STRUCTURAL_SOURCE_MISSING` surfaced when absent), with optional strict mode for all cohorts via `ranker.phase4.structural_source_only`.
- [x] Add explicit percentile field and richer component breakdown in API output.
- [x] Add score-run versioning/is_current semantics exactly as specified for `target_scores` history.
  - Note: legacy databases without these columns are handled via backward-compatible runtime dedupe; new/updated tables use native `score_version` + `is_current`.
- [x] CRISPR dependency (`n2`) now uses local DepMap cache when present (`data/depmap`), with source-missing baseline when absent in strict source-backed mode.
- [x] Query API output now includes per-component source provenance (`component_sources`) so source-backed vs proxy values are explicit.
- [x] `n3` (survival) and `n4` (expression) are now source-first (TCGA/GTEx) with optional semantic fallback only when explicitly enabled (`ranker.phase4.n3n4_semantic_fallback`).
- [x] `n4` supports GTEx-backed enrichment with persistent cache table (`ent_gtex_expression`) and bounded runtime fallback (`gtex_api`) for small cohorts; large cohorts remain cache-only for latency control.
- [x] `n3` supports TCGA-backed enrichment with persistent cache table (`ent_tcga_survival`) and bounded runtime fallback (`tcga_api`) for small cohorts with cancer context; large cohorts remain cache-only.
- [x] `n7` supports ChEMBL-backed inhibitor counts with persistent cache table (`ent_chembl_targets`) and bounded runtime fallback (`chembl_api`) for small cohorts; large cohorts remain cache-only.
- [x] `n8` supports Reactome-backed pathway counts with persistent cache table (`ent_reactome_genes`) and bounded runtime fallback (`reactome_api`) for small cohorts; large cohorts remain cache-only.
- [x] Provider cache freshness controls added (TTL-based reads from persisted signal tables) to avoid stale long-lived values.
- [x] Large-cohort mode now keeps query latency bounded while asynchronously prewarming top candidates into provider cache tables for subsequent source-backed runs.
- [x] Explicit staged refresh path added (`refresh_provider_signals`) with bounded batch size, per-provider retries, and refresh telemetry for cBioPortal/COSMIC/TCGA/GTEx/ChEMBL/Reactome.
- [x] Provider refresh execution now uses bounded concurrent provider calls per gene with deterministic merge ordering (removes fully serial per-provider inner loops).
- [x] Runtime provider fallback caches (CBIO/COSMIC/GTEx/TCGA/ChEMBL/Reactome paths) are bounded with TTL + pruning to prevent unbounded memory growth.
- [x] Autonomous cycle now runs provider refresh before ranking so iterative runs progressively replace proxy/semantic fallbacks with source-backed cache signals.
- [x] `n9` (literature novelty) now derives from paper publication + citation metadata (`papers.raw_json`/`published_at`) when available (`papers_metadata_citations`), with source-missing baseline instead of proxy-only coupling.
- [x] Staged refresh now persists per-provider run history (`ent_provider_refresh_runs`), applies adaptive cadence (error-rate + staleness aware), and is externalized through a continuous background scheduling/alerting loop (`ranker.phase4.background_refresh`).

### 4.7 Phase 4 Review (2026-03-15)

Reality check against code:

- Scoring core (weights, rank-normalization, penalties, shortlist thresholds, `DISPUTED` and hard-exclusion flags) aligns with architecture design.
- Versioned score persistence is aligned (`score_version`, `is_current`) with backward compatibility for legacy tables.
- Source-backed provider integration is materially advanced for `n1` through `n4` and `n7`/`n8`; `n1` now uses cBioPortal + COSMIC mutation-frequency cache rows (`ent_cbio_mutation_frequency`, `ent_cosmic_mutation_frequency`) with bounded runtime fetch + TTL freshness checks.
- Adaptive refresh persistence now records provider refresh outcomes in `ent_provider_refresh_runs` and uses recent error-rate/staleness policy to modulate refresh cadence.
- Literature novelty (`n9`) now uses publication + citation metadata when available, reducing dependency on pure evidence-density proxy behavior.
- `component_sources` is exposed per result so proxy-vs-source-backed provenance is explicit at query time.
- Confidence adjustment now uses provenance/tier-weighted KG evidence in the ranker path (provider-backed high-tier typed relations are upweighted versus generic mention edges).
- Phase 4 architecture targets are functionally met; remaining operational risk is provider data sparsity/availability (runtime source-missing on sparse cohorts), not framework-level gap.

---

# Phase 5: Structural Analysis & Molecule Design

## 5.1 Tool Inventory

| Tool | Function | Language | Deployment | License |
|---|---|---|---|---|
| PDB REST API client | Structure retrieval by PDB ID | Rust | Native (`FetchStructureTool`) | Open |
| fpocket wrapper | Binding pocket detection | Rust / C binary | Native (`DetectPocketsTool`) | BSD |
| AutoDock Vina wrapper | Molecular docking | Rust / C++ binary | Native (`DockMoleculeTool`) | Apache 2.0 |
| AlphaFold DB API client (Future) | Predicted structure retrieval | Rust | Native | CC BY 4.0 |
| RDKit / ADMET (Future) | Molecule manipulation, properties | Rust | Native Extensions | BSD/MIT |

**Rust Native wrappers:**
- The MVP relies on fast native Rust wrappers executing trusted local binaries (`fpocket` and `vina`). This limits overhead compared to Docker isolation and fits the local processing paradigm of Ferrumyx.

## 5.2 Ferrumyx Runtime Core Tool Orchestration Sequence

```mermaid
flowchart TD
    Start[Target Shortlist] --> Step1[STEP 1: Structure Retrieval<br/>Tool: pdb_fetch WASM<br/>Fallback: alphafold_fetch]
    Step1 --> Step2[STEP 2: Structure Validation<br/>PDB: resolution < 3.0 Å<br/>AlphaFold: pLDDT > 70]
    Step2 --> Step3[STEP 3: Pocket Detection<br/>Tool: fpocket_run Docker<br/>Select best pocket]
    Step3 --> Step4[STEP 4: Ligand Preparation<br/>Tool: rdkit_ops Rust Native<br/>Seed from ChEMBL + RDKit grow]
    Step4 --> Step5[STEP 5: Molecular Docking<br/>Tool: vina_dock Rust Bin<br/>Alt: gnina_dock rescoring]
    Step5 --> Step6[STEP 6: ADMET Prediction<br/>Tool: admet_predict Rust]
    Step6 --> Step7[STEP 7: Multi-Objective Ranking<br/>Vina + GNINA + ADMET + SA + Novelty]
    Step7 --> Step8[STEP 8: Store Results<br/>Trigger report generation]
```

## 5.3 Iterative Molecule Optimisation Loop

```text
REPEAT (max 5 iterations OR convergence):

  1. Score current batch:
     multi_obj = 0.40 × norm(vina_score)
               + 0.20 × norm(gnina_score)
               + 0.20 × admet_pass_ratio
               + 0.10 × norm(1 / sa_score)
               + 0.10 × novelty_vs_chembl

  2. Select top 20% by multi_obj_score

  3. Generate variants:
     a) Scaffold hopping via RDKit fragment replacement
     b) R-group enumeration on top scaffolds
     c) Reinvent4 (Month 7+): generative sampling
        conditioned on pocket pharmacophore

  4. Filter new batch:
     - Lipinski Ro5: MW < 500, LogP < 5, HBD ≤ 5, HBA ≤ 10
     - SA score < 6
     - No PAINS alerts (RDKit)
     - Not exact ChEMBL match (InChI key check)

  5. Dock filtered batch → repeat from step 1

CONVERGENCE: mean(multi_obj) improvement < 0.02
             OR top score unchanged for 2 iterations
```

## 5.4 Multi-Objective Optimisation Targets

| Objective | Target | Hard Constraint? |
|---|---|---|
| Vina docking score | < −7.0 kcal/mol | No (soft) |
| Gnina CNN score | > 0.5 | No (soft) |
| Molecular weight | 300–500 Da | Yes |
| LogP | 1–5 | Yes |
| HBD | ≤ 5 | Yes |
| HBA | ≤ 10 | Yes |
| TPSA | < 140 Å² | Yes |
| Synthetic accessibility (SA) | < 6 | Yes |
| hERG toxicity | Not flagged | Soft (warning) |
| Hepatotoxicity | Not flagged | Soft (warning) |
| ChEMBL novelty | InChI not in ChEMBL | Soft (log only) |

## 5.5 Intermediate Result Storage

All intermediate files in Ferrumyx Runtime Core workspace:
```text
/workspace/structural/{gene_symbol}/{job_id}/
  ├── structures/     # .pdb, .pdbqt
  ├── pockets/        # fpocket output
  ├── ligands/        # .sdf, .mol2
  ├── docking/        # Vina/Gnina poses
  ├── admet/          # prediction CSVs
  └── report.json     # job summary
```

Database: `molecules` and `docking_results` (Phase 1 schema). Each job UUID links workspace files to DB rows. Re-runs create new records; old preserved.

## 5.6 Failure Handling

| Scenario | Handling |
|---|---|
| No PDB entry | Fallback to AlphaFold; FLAG "predicted structure only" |
| AlphaFold pLDDT < 50 | WARNING: "low-confidence structure; docking unreliable" |
| fpocket: 0 pockets found | Try AlphaFold structure; if still 0: FLAG "no detectable pocket" |
| Vina docking fails | Retry with wider search box; if fails: log, skip, record failure |
| ADMET tool timeout | Retry once; proceed without ADMET, flag "not assessed" |
| No ChEMBL seed ligands | Start from built-in RDKit 1000-fragment library |

---

# Phase 6: Autonomous Scientific Query Handling

## 6.1 Example Query

> "What are promising synthetic lethal targets in KRAS G12D pancreatic cancer with structural druggability and low prior inhibitor exploration?"

## 6.2 LLM-Driven Intent Parsing

The NL query is autonomously parsed by an Ferrumyx Runtime Core agent into a structured `ScientificQuery` object. The agent leverages the LLM's reasoning capabilities to identify implicit filters and map them to the system's scoring components.

```json
{
  "query_type": "target_prioritization",
  "entities": {
    "gene": {"symbol": "KRAS", "mutation": "G12D", "hgvs_p": "p.Gly12Asp"},
    "cancer_type": {"oncotree_code": "PAAD"},
    "relationship": "synthetic_lethality"
  },
  "filters": {
    "require_structural_druggability": true,
    "max_chembl_inhibitor_count": 20,
    "min_sl_confidence": 0.5
  },
  "output_preferences": {
    "ranked": true,
    "include_evidence": true,
    "include_next_steps": true,
    "max_results": 10
  }
}
```

**Entity extraction pipeline:**
1.  Run NER on query text → gene (KRAS), mutation (G12D), cancer (pancreatic cancer), relationship (synthetic lethal)
2.  Normalise: KRAS → HGNC:6407; G12D → p.Gly12Asp / rs121913529; pancreatic cancer → PAAD
3.  Map filter intent: "structural druggability" → structural_tractability > 0.4; "low prior inhibitor" → ChEMBL count < 20

## 6.3 Query Plan Generation

```text
PLAN: synthetic lethal targets in KRAS G12D PAAD

Step 1: Validate KRAS G12D entity in ent_mutations
Step 2: Retrieve SL partners from ent_synthetic_lethality
        WHERE gene1 = KRAS AND cancer = PAAD AND confidence > 0.5
Step 3: For each SL partner:
        a. Get target_scores (is_current = TRUE, cancer = PAAD)
        b. Filter: structural_tractability > 0.40
        c. Filter: ChEMBL count < 20
Step 4: Retrieve structural data (ent_structures + ent_druggability)
Step 5: Retrieve supporting evidence (kg_facts + papers)
Step 6: Rank by composite_score × SL evidence confidence
Step 7: Format output JSON with full citations
```

## 6.4 Tool Invocation Sequence

```text
ferrumyx_ranker.execute(query):

1. ner_extract(query_text)                  → entities JSON
2. entity_normalise(entities)               → canonical IDs
3. kg_query.synthetic_lethality(            → SL partner list
       gene=KRAS, cancer=PAAD, conf>0.5)
4. FOR each SL partner:
   a. target_scores.get_current(gene, PAAD) → score object
   b. structural_data.get(gene)             → structure + druggability
   c. chembl.inhibitor_count(gene)          → count
5. filter_and_rank(candidates, filters)     → ranked list
6. evidence_bundle.assemble(top_N=10)       → citations + KG facts
7. llm_backend.narrate(ranked, evidence)    → human-readable summary
8. format_output(ranked, narrative)         → final JSON
```

## 6.5 Output JSON Schema

```json
{
  "query_id": "uuid",
  "query_text": "...",
  "generated_at": "ISO8601",
  "overall_confidence": 0.72,
  "caveats": [
    "SL evidence primarily from cell line screens; in vivo validation absent for 7/10 candidates.",
    "AlphaFold structures used for 3 candidates; docking reliability lower than crystallographic."
  ],
  "ranked_targets": [
    {
      "rank": 1,
      "gene_symbol": "POLQ",
      "hgnc_id": "HGNC:9177",
      "composite_score": 0.81,
      "confidence_adjusted_score": 0.74,
      "percentile": 94,
      "shortlist_tier": "primary",
      "flags": [],
      "warnings": [],
      "score_breakdown": {
        "mutation_freq":          {"raw": 0.03, "normalised": 0.45, "weight": 0.20},
        "crispr_dependency":      {"raw": -1.42, "normalised": 0.89, "weight": 0.18},
        "survival_correlation":   {"raw": 0.71, "normalised": 0.82, "weight": 0.15},
        "expression_specificity": {"raw": 3.2,  "normalised": 0.78, "weight": 0.12},
        "structural_tractability":{"raw": 0.67, "normalised": 0.71, "weight": 0.12},
        "pocket_detectability":   {"raw": 0.55, "normalised": 0.63, "weight": 0.08},
        "novelty_score":          {"raw": 0.91, "normalised": 0.93, "weight": 0.07},
        "pathway_independence":   {"raw": 0.75, "normalised": 0.70, "weight": 0.05},
        "literature_novelty":     {"raw": 0.82, "normalised": 0.85, "weight": 0.03}
      },
      "synthetic_lethality_evidence": {
        "confidence": 0.78,
        "effect_size": -1.42,
        "sources": ["DepMap Achilles Q4 2024", "PMID:35123456"],
        "screen_count": 3
      },
      "structural_feasibility": {
        "pdb_ids": ["7JO8"],
        "resolution_angstrom": 2.1,
        "fpocket_score": 0.73,
        "best_pocket_volume_A3": 842,
        "assessment": "druggable"
      },
      "novelty_assessment": {
        "chembl_inhibitor_count": 4,
        "most_advanced_phase": 1,
        "assessment": "early-stage; significant novelty headroom"
      },
      "evidence_citations": [
        {"pmid": "35123456", "doi": "10.1038/s41586-022-XXXX", "confidence": 0.82},
        {"source_db": "DepMap", "record_id": "Achilles_23Q4", "confidence": 0.75}
      ],
      "suggested_next_steps": [
        "Run AutoDock Vina against PDB 7JO8 with ChEMBL seed ligands",
        "Check SynLethDB for additional KRAS-POLQ evidence",
        "Review DepMap Achilles 23Q4 raw screen data"
      ]
    }
  ]
}
```

## 6.6 Confidence Score Calculation

- Per-claim confidence = confidence of underlying `kg_facts` row(s)
- Composite claims (e.g., "structurally druggable"): minimum of constituent fact confidences
- Overall query confidence = weighted mean of top-N `confidence_adjusted_score` values

## 6.7 Citation Traceability Rule

Every factual claim must link to ≥1 of: PMID, DOI, or database record ID (DepMap, ChEMBL, COSMIC, etc.).

Claims without traceable source → labelled `"source": "INFERRED"`, confidence ≤ 0.3.

## 6.8 Lab Team Orchestration + Live Monitor (2026-03-17)

Ferrumyx now exposes a role-oriented autonomous lab loop on top of Ferrumyx Runtime Core:

- `lab_planner`: creates hypotheses and run context.
- `lab_retriever`: executes ingestion with adaptive retrieval sizing based on novelty/duplicate pressure.
- `lab_validator`: evaluates ranking progress and recommends next action.
- `run_lab_autoresearch`: coordinator that executes dynamic planner→retriever→validator cycles with plateau-aware stopping.
- `lab_run_status`: returns single-run or recent-run status snapshots.

To make this observable in the web UI, run-state is persisted as JSON (`output/lab_runs.json` by default), and `ferrumyx-web` exposes:

- `GET /api/chat/lab-monitor?run_id=<id>` for one run.
- `GET /api/chat/lab-monitor?limit=<n>` for recent runs.

The `/chat` page includes a **Live Lab Run Monitor** panel that:

- auto-detects `lab-<uuid>` IDs from stream/tool/output events,
- polls monitor state independently of chat message completion,
- renders cycle KPIs (papers/chunks/duplicates/top score/novelty),
- lets users switch between recent runs quickly.

# Phase 7: Autonomous Self-Optimization Framework

Ferrumyx leverages Ferrumyx Runtime Core agents to monitor its own performance and autonomously optimize parameters, tool configurations, and orchestration logic.

## 7.1 Feedback Metrics

(Metrics remain as defined in §7.1: Recall@N, Docking Correlation, Stability, etc.)

## 7.2 Autonomous Optimization Loop

```mermaid
flowchart TD
    classDef stage fill:#1e1e24,stroke:#81c784,stroke-width:2px
    classDef agent fill:#1e1e24,stroke:#4fc3f7,stroke-width:2px

    subgraph Feedback [DATA COLLECTION]
        direction TB
        F1[Scheduled routines]
        F2[ChEMBL/DrugBank diffs]
        F3[ClinicalTrials outcomes]
    end
    class Feedback stage

    subgraph Analysis [AGENT ANALYSIS]
        direction TB
        A1[Compute metrics]
        A2[LLM-driven bottleneck identification]
        A3[Self-query: 'Why did recall drop?']
    end
    class Analysis agent

    subgraph Optimization [AUTONOMOUS ACTION]
        direction TB
        O1[Propose parameter updates]
        O2[Draft new tool/routine logic]
        O3[Vulnerability/Logic check]
    end
    class Optimization agent

    subgraph Apply [ATOMIC UPDATE]
        direction TB
        U1[Apply new weights/params]
        U2[Re-deploy logic to routines]
        U3[Log to audit trail]
    end
    class Apply stage

    Feedback --> Analysis
    Analysis --> Optimization
    Optimization --> Apply
```

## 7.3 Autonomous Parameter Re-Weighting

**Algorithm: Agent-Mediated Bayesian Update**

The Ferrumyx Runtime Core agent evaluates the Bayesian proposals (§7.3) and decides whether to apply them based on the correlation strength and ranking stability.
- **Automatic Updates:** By default, the agent applies updates atomically if the `ranking_volatility` is within safe bounds.
- **Safety Gate:** This can be disabled in `ferrumyx.toml` to require human approval, but the system is designed for high-autonomy operation.

## 7.4 Agent-Driven Tool Creation

When a specific ingestion or parsing bottleneck is identified (e.g., a new data format from a high-signal source), the Ferrumyx Runtime Core agent can autonomously:
1.  Draft a new Rust/WASM tool implementation.
2.  Register it within a temporary sandbox for validation.
3.  Propose its integration into the main pipeline if it improves `literature_recall` metrics.

## 7.5 Audit Trail & Transparency

Transparency is preserved through the `weight_update_log` and Ferrumyx Runtime Core's internal agent logs. Every autonomous decision is documented with its underlying evidence (metrics, LLM reasoning trace, and projected impact) in the `weight_update_log` table.

---

# Phase 8: Security & LLM Strategy

Ferrumyx leverages Ferrumyx Runtime Core's built-in security and LLM orchestration layers, extending them with domain-specific data classification rules.

## 8.1 Autonomous LLM Orchestration (Ferrumyx Runtime Core)

The system uses Ferrumyx Runtime Core's native multi-backend support to securely route requests to:
- **Local:** Ollama (default for INTERNAL/CONFIDENTIAL data).
- **Remote:** OpenAI, Anthropic, or custom HTTP backends (preferred for high-reasoning PUBLIC analysis).

Ferrumyx Runtime Core agents autonomously select the model based on:
1.  **Data Class:** Redaction policies enforced before any remote call.
2.  **Context Needs:** Large context models chosen for multi-paper synthesis.
3.  **Hardware State:** Optimized Ollama model selection based on detected RAM/GPU.

## 8.2 Agent-Driven Data Classification

Ferrumyx Runtime Core agents autonomously classify data at the prompt-construction stage:
- `PUBLIC`: Literature, database records.
- `INTERNAL`: Scores, fact triples, molecule candidates.
- `CONFIDENTIAL`: Proprietary partner data.

## 8.3 Autonomous Redaction & Routing

The agent loop enforces strict routing:
- **Confidential/Internal:** Hard-routed to local backends.
- **Public:** Routed based on performance/cost optimization.
- **Redaction:** Sensitive identifiers (UUIDs, specific SMILES) are stripped or hashed by the agent before external transmission.

## 8.4 Security Boundary Definitions (Ferrumyx Runtime Core Native)

| Boundary | Description | Enforcement |
|---|---|---|
| Agent ↔ Tool | Tools run in capability-based sandboxes | Ferrumyx Runtime Core WASM Sandbox |
| Data ↔ Network | Endpoint allowlisting for literature APIs | Ferrumyx Runtime Core Firewall |
| Host ↔ LLM | Data classification gated routing | Ferrumyx Runtime Core Redaction Layer |
| Secrets | AES-256-GCM encrypted retrieval | Ferrumyx Runtime Core Keychain |

All agent activities, tool executions, and LLM calls are tracked in the **Audit Log** for full traceability of autonomous decisions.

```toml
[llm.limits]
max_tokens_per_day_openai    = 500000
max_tokens_per_day_anthropic = 500000
max_cost_per_day_usd         = 20.0   # hard stop
alert_cost_threshold_usd     = 15.0   # soft alert

[llm.rate_limits]
openai_rpm    = 60
anthropic_rpm = 40
ollama_rpm    = 120
```

On daily limit hit: pause remote calls, route to Ollama, notify operator.


---

# Phase 9: Roadmap

## 9.1 Three-Month MVP (Months 1–3)

**Focus:** KRAS G12D Pancreatic Ductal Adenocarcinoma (PDAC)

Chosen because: highest unmet clinical need, well-characterised mutation, rich public datasets (TCGA, COSMIC, DepMap), active ClinicalTrials landscape, and tractable scope for validation.

### Month 1: Foundation
**Deliverable:** Full pipeline operational for KRAS G12D PAAD. Literature → KG → target scores → structural analysis → ranked output with citations. Retrospective validation: top-10 vs DrugBank known PDAC targets.
- [x] Initialise Cargo workspace: `ferrumyx-agent`, `ferrumyx-common`, `ferrumyx-db`, `ferrumyx-ingestion`, `ferrumyx-kg`, `ferrumyx-molecules`, `ferrumyx-ranker`, `ferrumyx-web`, `Ferrumyx Runtime Core`
- [x] LanceDB deployed; Phase 1 schema migrations run
- [x] PubMed E-utilities WASM tool (esearch + efetch XML) (Implemented in `pubmed.rs`)
- [x] Europe PMC WASM tool (Implemented in `europepmc.rs`)
- [x] PMC XML section-aware parser (Rust, `quick-xml`)
- [x] Ferrules PDF parser (Rust-native)
- [x] Section-aware chunker (Implemented in `chunker.rs`)
- [x] BiomedBERT embedding Native tool (/crates/ferrumyx-ingestion)
- [x] LanceDB IVFFlat index setup
- [x] Sci-Hub fallback client (Optional) (Implemented in `scihub.rs`)

**Deliverable:** Ingest a PubMed query result, parse full text, chunk, embed, store in LanceDB. Manual verification of 50 KRAS PDAC papers.

### Month 2: Knowledge Graph
- [x] TrieNer NER Native tool (High-speed Rust Aho-Corasick)
- [x] Entity normalisation (HGNC, HGVS, OncoTree)
- [x] kg_facts population from NER output
- [x] External DB pulls: COSMIC, DepMap CERES, ChEMBL, ClinicalTrials.gov (connectors implemented; runtime coverage depends on source/API availability)
- [x] Target score computation (all 9 components + persisted `target_scores`)
- [x] Hybrid search (LanceDB vector + LanceDB FTS with RRF fusion)
- [x] Basic REPL + Gateway query interface (`query_targets`, `ingest_literature`, async chat history-backed replies)
- [x] Autonomous workflow tooling (`workflow_status`, `recompute_target_scores`, `run_autonomous_cycle`)

**Deliverable:** Can answer "What are the top KRAS G12D targets in PAAD?" with ranked list and source citations.

### Month 3: Structural Pipeline + Query Handler
- [ ] PDB fetch + AlphaFold WASM tools
- [ ] fpocket Docker tool
- [ ] AutoDock Vina Docker tool
- [ ] RDKit Docker tool (SMILES → properties + Lipinski filter)
- [ ] ADMET-AI Docker tool
- [ ] Molecule pipeline orchestration
- [/] NL query handler (intent parsing → structured plan → tool calls) (Gateway async flow and autonomous tool invocation implemented; refinement still ongoing)
- [ ] Output JSON schema (§6.5)
- [ ] LLM router with Ollama + OpenAI backends
- [ ] Audit logging populated

**Deliverable:** Full pipeline operational for KRAS G12D PAAD. Literature → KG → target scores → structural analysis → ranked output with citations. Retrospective validation: top-10 vs DrugBank known PDAC targets.

### MVP Explicit Limitations
- Single cancer subtype only (KRAS G12D PAAD)
- No self-improvement loop (metrics collected but weights not updated)
- No generative molecule design (Reinvent4 not integrated)
- LanceDB-only; no Neo4j
- LLM narration quality depends on Ollama model capability
- Web UI available; some advanced autonomy controls still maturing

---

## 9.2 Six-Month Expansion (Months 4–6)

**Expansion criteria:** MVP retrospective Recall@20 > 0.55 for PDAC domain.

**New capabilities:**
- [x] bioRxiv/medRxiv ingestion tools (Implemented in `biorxiv.rs`)
- [x] ClinicalTrials.gov structured ingestion (trial outcomes → KG) (Implemented in `clinicaltrials.rs`)
- [x] Semantic Scholar integration (Graph API search/metadata + citation graph expansion + SPECTER2 field support)
- [x] arXiv ingestion tool (Implemented in `arxiv.rs`)
- [x] Unpaywall OA retrieval integration (Implemented in `unpaywall.rs`)
- [ ] Expand to 3 cancer subtypes: KRAS G12D PDAC + EGFR-mutant NSCLC + BRCA1/2 ovarian
- [ ] BERN2 high-recall NER for high-citation papers
- [ ] Basic generative design (RDKit fragment growing)
- [ ] DeepPurpose binding affinity prediction
- [ ] Feedback metrics collection activated (weights NOT yet auto-updated)
- [ ] Deduplication pipeline hardened (preprint→published pairing)
- [x] Web Gateway query interface with async/streaming chat, thread management, and Live Lab Run Monitor (`/api/chat/lab-monitor`)

**Validation strategy:** For each new cancer subtype, run retrospective Recall@20 vs DrugBank before declaring that domain operational.

---

## 9.3 Twelve-Month Autonomous Optimisation (Months 7–12)

**Self-improvement activation criteria:**
- ≥3 complete feedback cycles collected
- Docking-IC50 Pearson r > 0.40 on ≥2 target genes
- Recall@20 stable (±0.05) for 2 consecutive months
- Safety policy configured for deployment mode (autonomous-default or manual-gated)

**New capabilities:**
- [ ] Self-improvement loop fully active (autonomous-default with optional manual gate)
- [ ] Reinvent4 generative molecule design (CUDA GPU required)
- [ ] Expand to 10+ cancer subtypes; pan-cancer analysis
- [ ] Retrospective validation against all FDA oncology approvals (1990–present)
- [ ] Synthetic lethality network analysis (multi-hop Reactome traversal)
- [ ] External validation pipeline: submit top candidates to wet-lab partners
- [ ] Neo4j evaluation: benchmark if fact count > 2M and traversal latency > 500ms P95
- [ ] Full audit report generation (Markdown → PDF)

---

# Phase 10: Federated Knowledge Base Distribution

## 10.1 Objective

Enable Ferrumyx nodes to publish and consume shared knowledge-base snapshots so users can bootstrap from a large global corpus instead of rebuilding from scratch, while preserving provenance, integrity, and merge safety.

## 10.2 Canonical Storage Strategy

- **Canonical write path:** object-storage-backed, immutable snapshot lineage (append-only).
- **Distribution mirrors:** Hugging Face/GitHub release artifacts are read-only mirrors for download convenience.
- **Merge control:** contributions are validated against manifest + quality gates before entering canonical lineage.
- **Tamper model:** manifest-level integrity checks, signature envelope support, and rejection of malformed/low-quality snapshots.

This keeps distribution simple for users while preventing mirror drift from becoming source-of-truth drift.

## 10.3 Contribution Manifest Schema (`ferrumyx.federation.v1`)

Shared schema now lives in `crates/ferrumyx-common/src/federation.rs` and is exposed via `/api/federation/schema`.

| Field | Type | Required | Purpose |
|---|---|---|---|
| `schema_version` | string | yes | Strict schema gate (`ferrumyx.federation.v1`) |
| `manifest_id` | UUID | yes | Unique manifest identity |
| `dataset_id` | string | yes | Logical dataset namespace |
| `snapshot_id` | string | yes | Immutable snapshot identifier |
| `parent_snapshot_id` | string \| null | no | Lineage parent for merge graph |
| `created_at` | RFC3339 datetime | yes | Snapshot creation timestamp |
| `contributor` | object | yes | Node identity (`instance_id`, `display_name`, contact, key id) |
| `provenance` | object | yes | App/runtime context used to generate snapshot |
| `stats` | object | yes | Table counts + relation cardinality metrics |
| `artifacts` | array | no | Exported file digests (path/hash/rows/bytes) |
| `quality` | object | yes | Parse success, duplicate identity rate, generic predicate share |
| `signature` | object \| null | no | Cryptographic signature envelope |
| `annotations` | map | no | Extensible metadata tags |

## 10.4 Validation Rules (v1)

Validation now checks:

- schema version exact match
- non-empty dataset/snapshot identifiers
- no parent-snapshot self-cycle
- quality metrics finite and bounded to `[0,1]`
- artifact path uniqueness and SHA-256 format checks (if artifacts provided)
- signature envelope completeness (if provided)
- future timestamp skew warning

Endpoint: `/api/federation/manifest/validate`

## 10.5 Current Implementation Status (2026-03-21)

Implemented:

- shared manifest schema and validation report types in `ferrumyx-common`
- DB-side draft manifest generation from live tables (`build_contribution_manifest_draft`)
- draft quality signal extraction (parse success, DOI duplicate estimate, generic predicate share)
- JSONL package export with SHA-256 + byte/row digests (`export_contribution_package`)
- package integrity verification against manifest digests (`validate_contribution_package`)
- local Ed25519 manifest signing with trust-registry key resolution (`sign_contribution_package`)
- merge-gate moderation queue with status transitions (`submit_package_for_merge`, `decide_merge_queue`, `list_merge_queue`)
- canonical lineage index for approved snapshots (`get_canonical_lineage`)
- sync transport bootstrap: local/remote snapshot index, sync planning, resumable artifact pull, and push trigger (`/api/federation/sync/*`)
- web APIs for schema inspection, manifest draft/validation, and package export/validation
- trust-registry APIs for key listing/upsert/revoke (`/api/federation/trust/*`)
- merge queue admission policy for trusted signatures (`FERRUMYX_FED_REQUIRE_SIGNATURE_FOR_QUEUE`, default `true`)
- federation endpoint authn/authz controls with read/write bearer scopes and replay guard (`FERRUMYX_FED_AUTH_ENABLED`, `FERRUMYX_FED_READ_TOKEN`, `FERRUMYX_FED_WRITE_TOKEN`, `FERRUMYX_FED_REPLAY_*`)
- federation audit logging (`FERRUMYX_FED_AUDIT_LOG_PATH`) and settings-driven runtime env sync
- Hugging Face distribution bridge: server-side publish/pull/status endpoints (`/api/federation/hf/*`) with settings/env-driven repo, prefix, revision, timeout, pull-root, and optional token

Not yet implemented (next pass):

- Parquet export mode for high-volume snapshots (JSONL is implemented first)
- remote trust bootstrap and key governance policy across independent operators (local trust-registry APIs are implemented)
- cloud-hosted canonical merge gate service (current moderation queue is local-first bootstrap)
- content-addressed artifact store + true delta transfer (current transport is resumable chunked artifact transfer)

## 10.6 API Surface (Bootstrap)

- `GET /api/federation/schema`
- `POST /api/federation/manifest/draft`
- `POST /api/federation/manifest/validate`
- `POST /api/federation/package/export`
- `POST /api/federation/package/validate`
- `POST /api/federation/package/sign`
- `POST /api/federation/merge/submit`
- `GET /api/federation/merge/queue`
- `POST /api/federation/merge/decide`
- `GET /api/federation/canonical/lineage`
- `GET /api/federation/trust/list`
- `POST /api/federation/trust/upsert`
- `POST /api/federation/trust/revoke`
- `GET /api/federation/sync/index`
- `GET /api/federation/sync/snapshot`
- `GET /api/federation/sync/artifact`
- `POST /api/federation/sync/plan`
- `POST /api/federation/sync/pull`
- `POST /api/federation/sync/push`
- `GET /api/federation/hf/status`
- `POST /api/federation/hf/publish`
- `POST /api/federation/hf/pull`

These endpoints intentionally ship as a bootstrap layer so the federation contract can be tested before enabling cross-node write paths.

---

# Deliverables

## Tool Inventory

| Tool / Library | Type | Language | Deployment | License | Notes |
|---|---|---|---|---|---|
| PubMed E-utilities client | Implemented | Rust | WASM | Open | esearch + efetch; API key optional |
| Europe PMC client | Implemented | Rust | WASM | Open | fullTextXML for OA papers |
| bioRxiv/medRxiv client | Implemented | Rust | WASM | Open | PDF metadata only |
| arXiv client | Implemented | Rust | Native | Open | Atom XML; quick-xml |
| ClinicalTrials.gov v2 client | Implemented | Rust | WASM | Open | REST JSON |
| CrossRef client | Implemented | Rust | WASM | Open | DOI resolution |
| Semantic Scholar client | Implemented | Rust | Native | Open | Graph API search + metadata + OA URL + bounded citation expansion + SPECTER2 field support |
| Unpaywall client | Implemented | Rust | Native | Open | DOI OA lookup + PDF URL resolution |
| Sci-Hub client | Implemented | Rust | Native | Open | (Optional) Fallback scraper |
| PMC XML parser | Implemented (core) | Rust | Native | Open | quick-xml section extraction integrated in ingestion pipeline |
| Ferrules PDF parser | Implemented | Rust | Native | Open | lopdf-based text extraction |
| BiomedBERT / PubMedBERT | Implemented | Rust | Native | Apache 2.0 | Candle; 768-dim embeddings (ferrumyx-ingestion) |
| TrieNer | Implemented | Rust | Native | MIT | Fast AC-based matching (HGNC + OncoTree) |
| Gene entity normaliser | Implemented | Rust | Native | — | HGNC cache subset |
| fpocket wrapper | Implemented | Rust | Native | BSD | Pocket detection runtime execution |
| AutoDock Vina wrapper| Implemented | Rust | Native | Apache 2.0 | Molecular docking runtime execution |
| LanceDB | Implemented | Rust / C++ | Embedded | Apache 2.0 | Vector similarity search + FTS |
| Ferrumyx Runtime Core | Implemented | Rust | Native | Open | Agent loop, Web UI, routines |

---

## Risk Analysis

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| Docking score ≠ binding affinity | H | H | Gnina CNN rescoring; validate against ChEMBL IC50; treat as hypothesis generator only |
| AlphaFold low confidence for disordered regions | H | M | Check pLDDT at active site; prefer PDB; flag structurally unresolved targets |
| NER accuracy on novel nomenclature | M | M | Gazetteer supplement; BERN2 for high-citation papers; manual curation queue |
| LanceDB scaling > 10M chunks | M | L (MVP) | HNSW optimization; S3/Azure Blob offloading; read replicas |
| LLM hallucination in query narration | H | M | Strict grounding: narration only from KG-verified evidence bundle; no training-data facts allowed |
| WASM performance overhead on hot-path tools | L | M | Profile at Month 2; migrate to native Rust if >100ms overhead |
| Data licensing violation | M | L | COSMIC needs registration; ChEMBL CC BY-SA; full audit before any commercial use |
| Stochastic docking non-reproducibility | M | H | Fix random seed in Vina/Gnina; log all run params in docking_results |
| CRISPR data cell-line artefacts | H | H | Supplement DepMap with in vivo data; lower CRISPR weight for cell-line-only targets |
| Feedback loop overfitting to DrugBank | M | M | Multiple validation sources (CIViC + literature); enforce expert approval on all weight updates |

---

## Technical Bottlenecks

1. **Docking throughput at scale.** Vina: ~100 poses/min per CPU core. At 100 molecules × 10 targets × 3 pockets = 3,000 runs → ~30 min on 8 cores. GPU-accelerated Gnina reduces to ~3 min but requires CUDA. A docking job queue (Docker batch) is necessary at Month 6+ scale.

2. **Embedding throughput.** BiomedBERT on CPU: ~50 chunks/sec. 100K papers × 50 chunks avg = 5M chunks → ~28 hours CPU. GPU (RTX 3080): ~800 chunks/sec → ~1.7 hours. GPU is mandatory for production ingestion speed.

3. **KG aggregation correctness.** The noisy-OR aggregation and contradiction detection must be rigorously tested. Bugs propagate silently into target scores. Requires extensive unit tests with synthetic fact sets before production.

4. **NER precision on ambiguous gene symbols.** Symbols like "CAT", "SET", "MAX" are real English words. TrieNer context window is limited. False positives create spurious KG edges. Requires precision-recall tuning and a periodic manual audit.

5. **Feedback loop data sparsity at MVP.** Underexplored targets (the most interesting ones) have sparse ChEMBL data — the exact targets Ferrumyx is designed to find. Docking-IC50 correlation is hardest to compute for novel targets. The self-improvement loop matures most at 12+ months with more coverage.

6. **LLM context window for complex queries.** Assembling a 10-target evidence bundle with full citations can exceed 32K tokens. Evidence prioritisation logic is needed to trim context without dropping key citations.

7. **Ferrumyx Runtime Core WASM toolchain constraints.** The wasm32-wasip1 target has no threading and limited system calls. Tools requiring parallelism or native crypto must be Docker containers. This limits how many Ferrumyx tools can run in the lightweight WASM sandbox.

---

## Scientific Validity Risks

1. **Docking is not binding.** AutoDock Vina scores correlate weakly with experimental IC50 (r ≈ 0.4–0.6). A good docking score is a necessary but not sufficient condition for a real binder. All docking outputs are labelled "computational hypotheses."

2. **In silico ≠ in vivo.** DepMap CRISPR data is from cancer cell lines. Cell line models have altered metabolism, absent microenvironment, and unlimited passage artefacts. Essential targets in cell lines frequently fail in vivo.

3. **Publication bias.** Negative results are systematically underreported. Ferrumyx learns from published literature, which skews toward positive findings. Targets with known failures may appear more promising if failure papers are absent from the corpus.

4. **KRAS G12D cancer-type specificity.** G12D biology varies by tissue context. A synthetic lethal target identified in pancreatic CRISPR screens may not be lethal in KRAS G12D lung cancer. Cancer subtype context must be preserved in all KG queries — this is enforced by the (gene, cancer) pairing in target_scores but requires ongoing attention.

5. **AlphaFold disordered region inaccuracy.** Many oncology targets (c-Myc, p53 transactivation domain) are intrinsically disordered. AlphaFold pLDDT < 70 indicates low confidence. Docking against disordered regions is unreliable regardless of pocket detection scores.

6. **Synthetic lethality context dependency.** SL pairs validated in one genetic background may not hold in another co-occurring mutation context. All SL evidence must carry cancer context and genetic background metadata.

---

## 9.8 Value Proposition

Ferrumyx provides a specialized research infrastructure for target discovery with the following key advantages:

- **Full auditability:** every hypothesis traces to a specific PMID, DOI, or DB record. No black box decisions.
- **Privacy-preserving by default:** local LLM mode ensures that sensitivity-classified data does not leave the local environment unless explicitly configured.
- **Researcher-controlled:** scoring weights are transparent, inspectable, and modifiable, allowing for domain-specific tuning.
- **Ferrumyx Runtime Core-native:** inherits a production-grade agent loop, WASM sandbox, routines engine, and hybrid search.
- **Integrated hypothesis-to-molecule pipeline:** single autonomous system from literature ingestion to ranked docking candidates.

Target user: a computational biology research group wanting an auditable, privacy-respecting, continuously-learning literature mining and target prioritisation system.

---

## Next Engineering Steps

| # | Step | Complexity | Notes |
|---|---|---|---|
| 1 | Initialise Cargo workspace: ferrumyx-agent, ferrumyx-ingestion, ferrumyx-kg, ferrumyx-ranker | [x] | Standard Rust workspace |
| 2 | Initialize LanceDB embedded; run Phase 1 schema setup via Rust crate | [x] | Local file storage in workspace |
| 3 | Implement PubMed E-utilities WASM tool (esearch + efetch XML) | [x] | quick-xml + reqwest |
| 4 | Implement PMC XML section-aware parser | M | Map sec-type → SectionType enum; extract tables separately |
| 5 | Implement Ferrules PDF parser | [x] | Rust-native lopdf-based text extraction |
| 6 | Implement BiomedBERT embedding native Rust tool | [x] | Candle-based inference |
| 7 | Implement TrieNer NER Tool | [x] | Database-driven AC matching |
| 8 | Build entity normalisation (HGNC + HGVS) | L | HGNC REST bulk download; HGVS regex; edge cases numerous |
| 9 | Populate kg_facts from NER + external DB pulls (COSMIC, DepMap, ChEMBL) | L | Most complex ingestion step; each source needs separate parser |
| 10 | Implement target_scores computation (9 components + composite formula + versioning) | L | Rank normalisation; penalties; confidence adjustment; atomic versioning |

