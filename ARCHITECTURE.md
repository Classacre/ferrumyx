# Ferrumyx Architecture

**Autonomous Oncology Drug Discovery Engine**  
**Built on IronClaw (Rust AI Agent Framework)**  
**Version:** 0.1.0-draft  
**Repository:** https://github.com/Classacre/ferrumyx  
**Status:** Implementation Phase (LanceDB Migration)  
**Date:** 2026-02-25

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
10. [Deliverables](#deliverables)

---

# Phase 1: System Architecture

## 1.1 High-Level System Architecture Diagram

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                            FERRUMYX SYSTEM                                   │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │                        CHANNEL LAYER                                   │  │
│  │   REPL │ Web Gateway │ HTTP Webhook │ Telegram/Slack (WASM channel)    │  │
│  └───────────────────────────┬────────────────────────────────────────────┘  │
│                                │                                             │
│  ┌───────────────────────────▼────────────────────────────────────────────┐  │
│  │                     IRONCLAW AGENT CORE                                │  │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────────────┐    │  │
│  │  │  Intent Router  │  │  Agent Loop     │  │  Routines Engine     │    │  │
│  │  │  (query parsing)│  │  (plan/act/obs) │  │  (cron/event/        │    │  │
│  │  └─────────────────┘  └────────┬────────┘  │   webhook)           │    │  │
│  │                                 │          └──────────────────────┘    │  │
│  │  ┌──────────────────────────────▼────────────────────────────────────┐ │  │
│  │  │                    TOOL REGISTRY                                  │ │  │
│  │  │  Built-in │ MCP Tools │ WASM Tools │ Docker-backed Tools          │ │  │
│  │  └───────────────────────────────────────────────────────────────────┘ │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │                    FERRUMYX EXTENSION LAYER                            │  │
│  │                                                                        │  │
│  │  ┌─────────────────┐  ┌──────────────────┐  ┌─────────────────────┐    │  │
│  │  │  Ingestion      │  │  KG Builder      │  │  Target Ranker      │    │  │
│  │  │  Orchestrator   │  │  (LanceDB)       │  │  (Scoring Engine)   │    │  │
│  │  └────────┬────────┘  └────────┬─────────┘  └─────────┬───────────┘    │  │
│  │           │                    │                      │                │  │
│  │  ┌────────▼────────┐  ┌────────▼─────────┐  ┌─────────▼───────────┐    │  │
│  │  │  Molecule       │  │  Query Handler   │  │  Feedback Loop      │    │  │
│  │  │  Design Pipeline│  │  (NL → Struct)   │  │  (Self-Improve)     │    │  │
│  │  │  (Future Phase) │  │                  │  │                     │    │  │
│  │  └─────────────────┘  └──────────────────┘  └─────────────────────┘    │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │                      SANDBOX LAYER                                     │  │
│  │                                                                        │  │
│  │  ┌──────────────────────┐       ┌──────────────────────────────┐       │  │
│  │  │    WASM Sandbox      │       │      Docker Sandbox          │       │  │
│  │  │  (NER tools, light   │       │  (RDKit, docking, ADMET,     │       │  │
│  │  │   processing)        │       │   DeepPurpose)               │       │  │
│  │  │  Cap-based perms     │       │  Orchestrator-worker pattern │       │  │
│  │  │  Endpoint allowlist  │       │  Resource limits enforced    │       │  │
│  │  │  Credential injection│       └──────────────────────────────┘       │  │
│  │  └──────────────────────┘                                              │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │                       STORAGE LAYER                                    │  │
│  │                                                                        │  │
│  │  ┌─────────────────────────────────────────────────────────────────┐   │  │
│  │  │                    LanceDB (Embedded Vector DB)                 │   │  │
│  │  │  papers │ chunks │ embeddings │ entities │ kg_facts │ scores    │   │  │
│  │  │  molecules │ docking_results │ feedback │ audit_log             │   │  │
│  │  └─────────────────────────────────────────────────────────────────┘   │  │
│  │                                                                        │  │
│  │  ┌──────────────────┐   ┌─────────────────────┐                        │  │
│  │  │  Workspace FS    │   │  Secrets Store      │                        │  │
│  │  │  (IronClaw mem.) │   │  (AES-256-GCM)      │                        │  │
│  │  └──────────────────┘   └─────────────────────┘                        │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │                      LLM ABSTRACTION LAYER                             │  │
│  │  OpenAI │ Anthropic │ Ollama │ vLLM │ Groq │ DeepSeek              │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────────────┘
```

## 1.2 Core Technology Stack

- **Language:** Rust (100% host layer)
- **Agent Framework:** IronClaw
- **Database:** LanceDB (Embedded, Columnar, Vector-native)
- **Embeddings:** Candle (Rust-native) + BiomedBERT
- **NER:** Aho-Corasick Trie (Rust-native) + Candle
- **Web Framework:** Axum
- **Serialization:** Serde, Apache Arrow (via LanceDB)
- **Containerization:** Docker (only for structural analysis tools in Phase 5)

## 1.3 Storage Strategy: LanceDB

Ferrumyx uses **LanceDB** as its primary storage engine. LanceDB is an embedded, serverless vector database built on the Apache Lance columnar format.

### Why LanceDB?
- **Zero External Dependencies:** No need for a separate PostgreSQL/pgvector process.
- **Native Vector Search:** High-performance HNSW indexing for BiomedBERT embeddings.
- **Columnar Performance:** Extremely fast scans for knowledge graph facts and scoring.
- **Rust-Native:** First-class support for the Rust ecosystem.

### Schema Overview
- **papers:** Metadata for ingested literature (PMID, DOI, Title, Journal).
- **chunks:** Text segments with associated vector embeddings.
- **entities:** Canonical biomedical entities (Genes, Diseases, Chemicals).
- **kg_facts:** Extracted relationships (Subject-Predicate-Object) with evidence links.
- **entity_mentions:** Co-occurrence tracking for graph building.

## 1.4 Hybrid Search Design
Ferrumyx implements hybrid search by combining:
1. **Vector Search:** Cosine similarity on BiomedBERT embeddings (via LanceDB HNSW).
2. **Full-Text Search:** Columnar filtering and keyword matching.
3. **RRF (Reciprocal Rank Fusion):** Merging results for maximum relevance.

---

# Phase 2: Literature Ingestion

## 2.1 Pipeline Flow
1. **Discovery:** PubMed/EuropePMC API queries.
2. **Extraction:** PDF parsing via `lopdf` (Rust-native).
3. **Chunking:** Semantic chunking with overlap.
4. **Embedding:** BiomedBERT inference via Candle.
5. **Storage:** Atomic upsert into LanceDB.

---

# Phase 3: Knowledge Graph & Target Intelligence

## 3.1 Fact Extraction
Relationships are extracted using a combination of:
- **Trie-based NER:** Fast matching against HGNC, MeSH, and ChEMBL.
- **LLM-based Extraction:** High-precision relationship classification.

## 3.2 Scoring Engine
Targets are ranked based on:
- **Literature Evidence:** Frequency and recency of mentions.
- **Genetic Validation:** CRISPR/Dependency scores (DepMap).
- **Clinical Status:** Phase of associated trials.
- **Druggability:** Structural analysis results.

---

# Phase 4: Target Prioritization Engine

(Details on multi-factor scoring and ranking algorithms...)

---

# Phase 5: Structural Analysis & Molecule Design

(Details on RDKit, Vina, and AlphaFold integration...)

---

# Phase 6: Autonomous Scientific Query Handling

(Details on agentic reasoning and tool use...)

---

# Phase 7: Self-Improvement Framework

(Details on feedback loops and weight optimization...)

---

# Phase 8: Security & LLM Strategy

(Details on credential management and model routing...)

---

# Phase 9: Roadmap

- **Month 1:** Core Architecture & LanceDB Migration (Current)
- **Month 2:** Advanced NER & KG Fact Extraction
- **Month 3:** Target Scoring & Web Dashboard
- **Month 4:** Structural Analysis Pipeline
- **Month 5:** Autonomous Agent Integration
- **Month 6:** Self-Improvement & Feedback Loops

---

# Deliverables

1. **Ferrumyx Binary:** Single executable containing all logic and embedded DB.
2. **Knowledge Base:** Local LanceDB directory with ingested oncology data.
3. **Web Interface:** Dashboard for target exploration and system monitoring.
