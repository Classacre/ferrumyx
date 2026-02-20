# Ferrumyx Architecture

**Autonomous Oncology Drug Discovery Engine**  
**Built on IronClaw (Rust AI Agent Framework)**  
**Version:** 0.1.0-draft  
**Repository:** https://github.com/Classacre/ferrumyx  
**Status:** Pre-implementation design  
**Date:** 2026-02-21

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

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                            FERRUMYX SYSTEM                                  â”‚
â”‚                                                                             â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”‚
â”‚  â”‚                        CHANNEL LAYER                                 â”‚   â”‚
â”‚  â”‚   REPL â”‚ Web Gateway â”‚ HTTP Webhook â”‚ Telegram/Slack (WASM channel)  â”‚   â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â”‚
â”‚                                â”‚                                            â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â–¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”‚
â”‚  â”‚                     IRONCLAW AGENT CORE                              â”‚   â”‚
â”‚  â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚   â”‚
â”‚  â”‚  â”‚  Intent Router   â”‚  â”‚  Agent Loop     â”‚  â”‚  Routines Engine â”‚    â”‚   â”‚
â”‚  â”‚  â”‚  (query parsing) â”‚  â”‚  (plan/act/obs) â”‚  â”‚  (cron/event/    â”‚    â”‚   â”‚
â”‚  â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â””â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â”‚   webhook)       â”‚    â”‚   â”‚
â”‚  â”‚                                 â”‚           â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚   â”‚
â”‚  â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â–¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â” â”‚   â”‚
â”‚  â”‚  â”‚                    TOOL REGISTRY                               â”‚ â”‚   â”‚
â”‚  â”‚  â”‚  Built-in â”‚ MCP Tools â”‚ WASM Tools â”‚ Docker-backed Tools       â”‚ â”‚   â”‚
â”‚  â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€-â”€â”˜ â”‚   â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â”‚
â”‚                                                                             â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”‚
â”‚  â”‚                    FERRUMYX EXTENSION LAYER                          â”‚   â”‚
â”‚  â”‚                                                                      â”‚   â”‚
â”‚  â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”‚   â”‚
â”‚  â”‚  â”‚  Ingestion      â”‚  â”‚  KG Builder      â”‚  â”‚  Target Ranker     â”‚  â”‚   â”‚
â”‚  â”‚  â”‚  Orchestrator   â”‚  â”‚  (PostgreSQL)    â”‚  â”‚  (Scoring Engine)  â”‚  â”‚   â”‚
â”‚  â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â””â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â”‚   â”‚
â”‚  â”‚           â”‚                    â”‚                       â”‚             â”‚   â”‚
â”‚  â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â–¼â”€â”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â–¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â–¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”‚   â”‚
â”‚  â”‚  â”‚  Molecule       â”‚  â”‚  Query Handler   â”‚  â”‚  Feedback Loop     â”‚  â”‚   â”‚
â”‚  â”‚  â”‚  Design Pipelineâ”‚  â”‚  (NL â†’ Struct)   â”‚  â”‚  (Self-Improve)    â”‚  â”‚   â”‚
â”‚  â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â”‚   â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â”‚
â”‚                                                                             â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”‚
â”‚  â”‚                      SANDBOX LAYER                                   â”‚   â”‚
â”‚  â”‚                                                                      â”‚   â”‚
â”‚  â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”       â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”‚   â”‚
â”‚  â”‚  â”‚    WASM Sandbox       â”‚       â”‚      Docker Sandbox          â”‚   â”‚   â”‚
â”‚  â”‚  â”‚  (NER tools, light    â”‚       â”‚  (RDKit, docking, ADMET,     â”‚   â”‚   â”‚
â”‚  â”‚  â”‚   processing)         â”‚       â”‚   Docling, DeepPurpose)      â”‚   â”‚   â”‚
â”‚  â”‚  â”‚  Cap-based perms      â”‚       â”‚  Orchestrator-worker pattern â”‚   â”‚   â”‚
â”‚  â”‚  â”‚  Endpoint allowlist   â”‚       â”‚  Resource limits enforced    â”‚   â”‚   â”‚
â”‚  â”‚  â”‚  Credential injection â”‚       â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â”‚   â”‚
â”‚  â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                                           â”‚   â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â”‚
â”‚                                                                             â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”‚
â”‚  â”‚                       STORAGE LAYER                                  â”‚   â”‚
â”‚  â”‚                                                                      â”‚   â”‚
â”‚  â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”‚   â”‚
â”‚  â”‚  â”‚                    PostgreSQL + pgvector                      â”‚   â”‚   â”‚
â”‚  â”‚  â”‚  papers â”‚ chunks â”‚ embeddings â”‚ entities â”‚ kg_facts â”‚ scores  â”‚   â”‚   â”‚
â”‚  â”‚  â”‚  molecules â”‚ docking_results â”‚ feedback â”‚ audit_log           â”‚   â”‚   â”‚
â”‚  â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â”‚   â”‚
â”‚  â”‚                                                                      â”‚   â”‚
â”‚  â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                      â”‚   â”‚
â”‚  â”‚  â”‚  Workspace FS    â”‚   â”‚  Secrets Store     â”‚                      â”‚   â”‚
â”‚  â”‚  â”‚  (IronClaw mem.) â”‚   â”‚  (AES-256-GCM)     â”‚                      â”‚   â”‚
â”‚  â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                      â”‚   â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â”‚
â”‚                                                                             â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”‚
â”‚  â”‚                      LLM ABSTRACTION LAYER                           â”‚   â”‚
â”‚  â”‚  Ollama (local) â”‚ OpenAI â”‚ Anthropic â”‚ Custom HTTP endpoint          â”‚   â”‚
â”‚  â”‚  Data classification gate â†’ redaction â†’ routing decision             â”‚   â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

## 1.2 Modular Breakdown

### Orchestration Layer (Rust, IronClaw extension)

| Module | Responsibility | Extends IronClaw |
|---|---|---|
| `ferrumyx-agent` | Top-level agent loop, intent routing | Yes â€” registers custom intents |
| `ferrumyx-ingestion` | Paper discovery, download, parse orchestration | Tool implementations |
| `ferrumyx-kg` | Knowledge graph build/query/update | Storage abstraction |
| `ferrumyx-ranker` | Target scoring, normalization, shortlisting | Pure Rust scoring logic |
| `ferrumyx-query` | NL query â†’ structured plan â†’ execution | Intent handler |
| `ferrumyx-feedback` | Metric collection, weight update proposals | Routine/event handler |
| `ferrumyx-routines` | Scheduled ingestion, re-scoring, validation | IronClaw routines |

### Tool Layer (mix of Rust + sandboxed Python/binary)

| Tool | Implementation | Sandbox |
|---|---|---|
| `pubmed_search` | Rust HTTP client (WASM-wrappable) | WASM |
| `europepmc_search` | Rust HTTP client | WASM |
| `docling_parse` | Python via Docker | Docker |
| `ner_extract` | Python (SciSpacy/BERN2) via Docker | Docker |
| `embed_batch` | Python (sentence-transformers) via Docker | Docker |
| `fpocket_run` | Binary via Docker | Docker |
| `vina_dock` | Binary via Docker | Docker |
| `rdkit_ops` | Python via Docker | Docker |
| `admet_predict` | Python via Docker | Docker |
| `deepchem_affinity` | Python via Docker | Docker |

### Storage Layer

- **PostgreSQL 16+** with **pgvector 0.7+** extension
- **Workspace filesystem** (IronClaw native): intermediate files, job artifacts
- **AES-256-GCM keychain**: API keys, DB credentials

## 1.3 Data Flow

```
[External APIs / PDFs]
        â”‚
        â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚     INGESTION PIPELINE          â”‚
â”‚  1. Paper discovery (API query) â”‚
â”‚  2. Deduplication check         â”‚
â”‚  3. Full-text retrieval         â”‚
â”‚  4. Docling parse (Docker)      â”‚
â”‚  5. Section-aware chunking      â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
               â”‚
               â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚     EMBEDDING PIPELINE          â”‚
â”‚  6. BiomedBERT embedding batch  â”‚
â”‚  7. pgvector store              â”‚
â”‚  8. Full-text index (tsvector)  â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
               â”‚
               â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚     NER + KG CONSTRUCTION       â”‚
â”‚  9. SciSpacy / BERN2 NER        â”‚
â”‚  10. Entity normalization       â”‚
â”‚  11. Fact triple extraction     â”‚
â”‚  12. Confidence scoring         â”‚
â”‚  13. Append to kg_facts table   â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
               â”‚
               â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚     TARGET PRIORITIZATION       â”‚
â”‚  14. Join KG with external DBs  â”‚
â”‚  15. Compute composite score    â”‚
â”‚  16. Store versioned scores     â”‚
â”‚  17. Shortlist candidates       â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
               â”‚
               â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚     STRUCTURAL ANALYSIS         â”‚
â”‚  18. Fetch PDB / AlphaFold      â”‚
â”‚  19. fpocket pocket detection   â”‚
â”‚  20. Generate / retrieve ligandsâ”‚
â”‚  21. AutoDock Vina docking      â”‚
â”‚  22. ADMET prediction           â”‚
â”‚  23. Score and rank molecules   â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
               â”‚
               â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚     QUERY RESPONSE / REPORT     â”‚
â”‚  24. Assemble evidence bundle   â”‚
â”‚  25. LLM-assisted narrative     â”‚
â”‚  26. Return ranked JSON output  â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

## 1.4 Memory Design: PostgreSQL Schema Overview

### Core Tables

```sql
-- Papers and source tracking
CREATE TABLE papers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    doi             TEXT UNIQUE,
    pmid            TEXT UNIQUE,
    pmcid           TEXT,
    title           TEXT NOT NULL,
    abstract        TEXT,
    authors         JSONB,         -- [{name, affiliation, orcid}]
    journal         TEXT,
    pub_date        DATE,
    source          TEXT NOT NULL, -- 'pubmed'|'europepmc'|'biorxiv'|...
    open_access     BOOLEAN DEFAULT FALSE,
    full_text_url   TEXT,
    parse_status    TEXT DEFAULT 'pending', -- 'pending'|'parsed'|'failed'
    ingested_at     TIMESTAMPTZ DEFAULT NOW(),
    raw_json        JSONB          -- original API response
);

-- Parsed document chunks (section-aware)
CREATE TABLE chunks (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    paper_id        UUID REFERENCES papers(id) ON DELETE CASCADE,
    chunk_index     INTEGER NOT NULL,
    section_type    TEXT,          -- 'abstract'|'intro'|'methods'|'results'|'discussion'
    section_heading TEXT,
    content         TEXT NOT NULL,
    token_count     INTEGER,
    embedding       vector(768),   -- BiomedBERT-base dimension
    ts_vector       tsvector,      -- for full-text search
    created_at      TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX ON chunks USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);
CREATE INDEX ON chunks USING GIN (ts_vector);

-- Biomedical entities
CREATE TABLE entities (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    canonical_id    TEXT,          -- HGNC:1100, MESH:D009374, etc.
    entity_type     TEXT NOT NULL, -- 'gene'|'mutation'|'cancer_type'|'compound'|'pathway'
    name            TEXT NOT NULL,
    aliases         TEXT[],
    external_ids    JSONB,         -- {hgnc, uniprot, ensembl, chebi, ...}
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (canonical_id, entity_type)
);

-- Knowledge graph facts (append-only)
CREATE TABLE kg_facts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subject_id      UUID REFERENCES entities(id),
    predicate       TEXT NOT NULL, -- 'inhibits'|'activates'|'mutated_in'|'synthetic_lethal_with'
    object_id       UUID REFERENCES entities(id),
    confidence      FLOAT NOT NULL CHECK (confidence BETWEEN 0 AND 1),
    evidence_type   TEXT NOT NULL, -- 'experimental'|'computational'|'text_mined'
    evidence_weight FLOAT NOT NULL,
    source_pmid     TEXT,
    source_doi      TEXT,
    source_db       TEXT,          -- 'cosmic'|'depmap'|'chembl'|...
    sample_size     INTEGER,
    study_type      TEXT,          -- 'rct'|'cohort'|'in_vitro'|'cell_line'|...
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    valid_from      TIMESTAMPTZ DEFAULT NOW(),
    valid_until     TIMESTAMPTZ   -- NULL = currently valid
);
CREATE INDEX ON kg_facts (subject_id, predicate, object_id);
CREATE INDEX ON kg_facts (created_at);

-- Target scoring (versioned)
CREATE TABLE target_scores (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene_entity_id      UUID REFERENCES entities(id),
    cancer_entity_id    UUID REFERENCES entities(id),
    score_version       INTEGER NOT NULL,
    composite_score     FLOAT NOT NULL,
    component_scores    JSONB NOT NULL,  -- {mutation_freq, depmap, survival, ...}
    weight_vector       JSONB NOT NULL,  -- snapshot of weights used
    confidence_adj      FLOAT,
    scored_at           TIMESTAMPTZ DEFAULT NOW(),
    is_current          BOOLEAN DEFAULT TRUE
);
CREATE INDEX ON target_scores (gene_entity_id, cancer_entity_id, is_current);

-- Molecular structures and docking
CREATE TABLE molecules (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    smiles          TEXT NOT NULL,
    inchi_key       TEXT UNIQUE,
    chembl_id       TEXT,
    name            TEXT,
    mw              FLOAT,
    logp            FLOAT,
    hbd             INTEGER,
    hba             INTEGER,
    tpsa            FLOAT,
    sa_score        FLOAT,          -- synthetic accessibility
    source          TEXT,           -- 'generated'|'retrieved'|'modified'
    parent_id       UUID REFERENCES molecules(id),
    created_at      TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE docking_results (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    molecule_id     UUID REFERENCES molecules(id),
    target_gene_id  UUID REFERENCES entities(id),
    pdb_id          TEXT,
    pocket_id       TEXT,
    vina_score      FLOAT,
    gnina_score     FLOAT,
    pose_file       TEXT,           -- path in workspace FS
    admet_scores    JSONB,
    run_params      JSONB,
    docked_at       TIMESTAMPTZ DEFAULT NOW()
);

-- Feedback and self-improvement
CREATE TABLE feedback_events (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type      TEXT NOT NULL,  -- 'drugbank_validation'|'chembl_correlation'|'trial_outcome'
    target_gene_id  UUID REFERENCES entities(id),
    cancer_id       UUID REFERENCES entities(id),
    metric_name     TEXT NOT NULL,
    metric_value    FLOAT NOT NULL,
    evidence_source TEXT,
    recorded_at     TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE weight_update_log (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    previous_weights JSONB NOT NULL,
    new_weights      JSONB NOT NULL,
    trigger_event    TEXT,
    algorithm        TEXT,          -- 'bayesian'|'manual'|'gradient'
    approved_by      TEXT,          -- human reviewer ID or 'auto'
    delta_summary    JSONB,
    updated_at       TIMESTAMPTZ DEFAULT NOW()
);

-- Audit log for all LLM calls
CREATE TABLE llm_audit_log (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id      TEXT,
    model           TEXT NOT NULL,
    backend         TEXT NOT NULL,  -- 'ollama'|'openai'|'anthropic'|'custom'
    prompt_tokens   INTEGER,
    completion_tokens INTEGER,
    data_class      TEXT NOT NULL,  -- 'PUBLIC'|'INTERNAL'|'CONFIDENTIAL'
    output_hash     TEXT NOT NULL,
    latency_ms      INTEGER,
    called_at       TIMESTAMPTZ DEFAULT NOW()
);

-- Ingestion audit log
CREATE TABLE ingestion_audit (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    paper_doi       TEXT,
    paper_pmid      TEXT,
    action          TEXT NOT NULL,  -- 'discovered'|'downloaded'|'parsed'|'embedded'|'failed'
    source          TEXT NOT NULL,
    detail          JSONB,
    occurred_at     TIMESTAMPTZ DEFAULT NOW()
);
```

### pgvector Usage

- **Embedding dimension:** 768 (BiomedBERT-base) or 1024 (PubMedBERT-large). Choose at project start â€” changing dimension requires full re-embedding.
- **Index type:** IVFFlat for MVP (good enough up to ~1M vectors); migrate to HNSW for production scale.
- **Hybrid search:** Reciprocal Rank Fusion (RRF) combining cosine similarity from pgvector and BM25-style tsvector ranking.
- **pgvector version:** 0.7+ required for HNSW index support.

```sql
-- Example hybrid search query (RRF)
WITH vector_results AS (
    SELECT id, paper_id, content,
           1 - (embedding <=> $1::vector) AS vec_score,
           ROW_NUMBER() OVER (ORDER BY embedding <=> $1::vector) AS vec_rank
    FROM chunks
    LIMIT 100
),
text_results AS (
    SELECT id, paper_id, content,
           ts_rank(ts_vector, plainto_tsquery('english', $2)) AS text_score,
           ROW_NUMBER() OVER (ORDER BY ts_rank(ts_vector, plainto_tsquery('english', $2)) DESC) AS text_rank
    FROM chunks
    WHERE ts_vector @@ plainto_tsquery('english', $2)
    LIMIT 100
),
rrf AS (
    SELECT COALESCE(v.id, t.id) AS id,
           COALESCE(v.paper_id, t.paper_id) AS paper_id,
           COALESCE(v.content, t.content) AS content,
           (1.0 / (60 + COALESCE(v.vec_rank, 100))) + 
           (1.0 / (60 + COALESCE(t.text_rank, 100))) AS rrf_score
    FROM vector_results v
    FULL OUTER JOIN text_results t ON v.id = t.id
)
SELECT * FROM rrf ORDER BY rrf_score DESC LIMIT 20;
```

## 1.5 LLM Backend Abstraction Layer

```
                    â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
                    â”‚   ferrumyx_llm::LlmBackend   â”‚
                    â”‚   (Rust trait)               â”‚
                    â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
                                   â”‚
          â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
          â”‚                        â”‚                        â”‚
          â–¼                        â–¼                        â–¼
 â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
 â”‚  OllamaBackend  â”‚    â”‚  OpenAIBackend  â”‚    â”‚  AnthropicBackend    â”‚
 â”‚  (local HTTP)   â”‚    â”‚  (REST API)     â”‚    â”‚  (REST API)          â”‚
 â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
          â”‚                        â”‚
          â–¼                        â–¼
 â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
 â”‚  CustomHttp     â”‚    â”‚  Data Classification â”‚
 â”‚  Backend        â”‚    â”‚  Gate (pre-call)     â”‚
 â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
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
- `DataClass::Public` â†’ any backend (prefer local if available)
- `DataClass::Internal` â†’ local only OR explicit override with audit log
- `DataClass::Confidential` â†’ local only; remote call = hard block + alert

## 1.6 Self-Improvement Feedback Loop Architecture

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚           FEEDBACK COLLECTION LAYER              â”‚
â”‚  (scheduled routines: daily/weekly)              â”‚
â”‚  - ChEMBL activity data pull                     â”‚
â”‚  - ClinicalTrials.gov outcome updates            â”‚
â”‚  - DrugBank approved drug list diff              â”‚
â”‚  - Target ranking stability measurement          â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
                       â”‚
                       â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚           METRIC COMPUTATION                     â”‚
â”‚  - Retrospective recall@N                        â”‚
â”‚  - Docking-IC50 Pearson correlation              â”‚
â”‚  - Ranking Kendall-tau stability                 â”‚
â”‚  - False positive accumulation rate             â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
                       â”‚
                       â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚        THRESHOLD EVALUATION (automated)          â”‚
â”‚  If metric_delta > threshold:                    â”‚
â”‚    â†’ Generate weight update PROPOSAL             â”‚
â”‚  Else:                                           â”‚
â”‚    â†’ Log metric, no action                       â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
                       â”‚
                       â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚         HUMAN-IN-THE-LOOP CHECKPOINT             â”‚
â”‚  Proposal presented to operator via:            â”‚
â”‚  - REPL / Web Gateway notification               â”‚
â”‚  - Detailed diff of old vs new weights          â”‚
â”‚  - Projected impact on current shortlist        â”‚
â”‚  REQUIRED APPROVAL before weights applied       â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
                       â”‚
                       â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚          WEIGHT APPLICATION + AUDIT              â”‚
â”‚  - Atomic write to weight_update_log             â”‚
â”‚  - Re-score all targets with new weights         â”‚
â”‚  - Mark old target_scores as is_current=FALSE    â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

## 1.7 Security Boundary Definitions

| Boundary | Description | Enforcement |
|---|---|---|
| Host â†” WASM | WASM tools cannot access filesystem, network, or secrets directly | WASM capability model (IronClaw) |
| Host â†” Docker | Docker containers network-isolated; no direct DB access | Docker network policy + IronClaw orchestrator |
| Ferrumyx â†” Remote LLM | Data classification gate blocks INTERNAL/CONFIDENTIAL | Rust middleware in LlmRouter |
| DB credentials | Never passed to tool layer; only accessed by host process | IronClaw AES-256-GCM keychain |
| API keys | Injected at host boundary; WASM tools receive only scoped tokens | IronClaw credential injection |
| Public API calls | All outbound calls logged with endpoint + response hash | Ingestion audit log |

## 1.8 How Ferrumyx Extends IronClaw Without Forking

**Decision: Extension, not fork.**

Rationale: Forking IronClaw means carrying the maintenance burden of diverging from upstream improvements to the agent loop, WASM sandbox, and security layer â€” areas where Ferrumyx has no domain-specific requirements. The extension model preserves upgradability.

**Extension mechanisms used:**

1. **Custom tool registration:** Ferrumyx registers all domain tools (`pubmed_search`, `fpocket_run`, etc.) via IronClaw's tool registry interface. Tools implement the `Tool` trait.

2. **Custom intent handlers:** `FerrumyxQueryHandler` implements IronClaw's intent handler interface, intercepting scientific queries before the generic fallback.

3. **Custom routines:** Ferrumyx defines its ingestion, re-scoring, and feedback routines via the routines engine API (cron triggers + event triggers).

4. **Workspace conventions:** Ferrumyx uses IronClaw's workspace filesystem with a defined directory structure (`/kg/`, `/molecules/`, `/docking/`, `/reports/`).

5. **Storage extension:** pgvector is already a PostgreSQL extension. Ferrumyx adds its schema on top of IronClaw's existing DB. No IronClaw table is modified.

6. **LLM abstraction:** Ferrumyx implements additional `LlmBackend` adapters on top of IronClaw's existing abstraction.

**What requires direct IronClaw code changes (minimal, tracked):**
- Potentially: exposing Docker sandbox orchestration as a stable API if not already public. This would be contributed back upstream rather than forked.

---

# Phase 2: Literature Ingestion

## 2.1 Source Evaluation

### PubMed / NCBI E-utilities API

| Property | Value |
|---|---|
| API availability | Free, REST (https://eutils.ncbi.nlm.nih.gov/entrez/eutils/) |
| Rate limits | 3 req/sec unauthenticated; 10 req/sec with API key |
| Authentication | Optional API key (NCBI account, free) |
| Data formats | XML (PubMed, PubMed Central), JSON (efetch supports both) |
| Full-text access | PMC Open Access subset (PMCID required) â€” structured XML preferred |
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
| Full-text access | PDF only (no structured XML); Docling required |
| Ferrumyx approach | WASM tool for metadata; Docker Docling for full text |
| Notes | High signal for cutting-edge methods; NOT peer-reviewed â€” confidence weight lower |

### arXiv API

| Property | Value |
|---|---|
| API availability | REST + OAI-PMH (http://export.arxiv.org/api/query) |
| Rate limits | 3 req/sec |
| Authentication | None |
| Data formats | Atom XML |
| Full-text access | PDF + source LaTeX (when available) |
| Ferrumyx approach | WASM tool; relevant for ML/
computational biology / ML-for-drug-discovery papers; lower priority than PubMed for clinical oncology |
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
| Notes | Essential for DOI resolution pipeline (Â§2.3); check `link[].content-type` for `application/pdf` links; `is-referenced-by-count` provides citation count signal for evidence weighting; use `mailto` param to avoid rate throttle |

---

### Semantic Scholar API

| Property | Value |
|---|---|
| API availability | REST (https://api.semanticscholar.org/graph/v1) |
| Rate limits | 100 req/sec with API key; 1 req/sec unauthenticated |
| Authentication | API key (free, no approval required) |
| Data formats | JSON |
| Full-text access | Open-access PDFs linked via `openAccessPdf.url`; S2 corpus IDs for cross-reference |
| Ferrumyx approach | WASM tool; citation graph traversal, influential paper detection via `influentialCitationCount`, embedding retrieval via `/paper/{id}/embedding` (SPECTER2 vectors) |
| Notes | SPECTER2 embeddings are a useful secondary signal; `tldr` field provides model-generated abstract summaries; `fieldsOfStudy` for pre-filtering; citation velocity (citations per year) computable from `year` + `citationCount` |

---

## 2.2 Paper Discovery Tool

The paper discovery tool is an IronClaw WASM tool that translates a structured `DiscoveryRequest` (gene symbol, mutation, cancer type, date range, optional keyword modifiers) into source-specific query strings and fans out to all enabled sources in parallel.

### Query Construction Logic

```
DiscoveryRequest {
  gene:        "KRAS",
  mutation:    "G12D",
  cancer_type: "pancreatic ductal adenocarcinoma",
  aliases:     ["PDAC", "pancreatic cancer"],
  date_from:   "2018-01-01",
  max_results: 200,
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

1. Gene aliases resolved from HGNC (e.g., KRAS â†’ {KRAS, RASK2, c-Ki-ras})
2. Mutation notation variants: G12D â†’ {G12D, Gly12Asp, p.G12D, c.35G>A, rs121913529}
3. Cancer synonyms from OncoTree: PDAC â†’ {pancreatic adenocarcinoma, exocrine pancreatic cancer, pancreatic ductal carcinoma}
4. Boolean logic: (gene OR alias1 OR alias2) AND (mutation OR notation2 OR notation3) AND (cancer OR synonym1 OR synonym2)

### Deduplication on Ingestion

Results from all sources are immediately deduplicated by DOI (Â§2.10) before downstream processing. A paper returned by both PubMed and Europe PMC counts as one record; the PubMed record is preferred (richer MeSH/structured metadata).

---

## 2.3 DOI Resolution Workflow

```
Input: raw paper metadata (title, authors, journal, year)
         OR known DOI string

        [DOI present?]
             |
      Yes â”€â”€â”€â”¤
             â”‚                    No
             â”‚               [CrossRef search by
             â”‚                title + first author]
             â”‚                      â”‚
             â”‚               [DOI found? confidence > 0.92?]
             â”‚                /              \
             â”‚           Yes                  No
             â”‚                                â”‚
             â”‚                         [Flag: unresolved DOI]
             â”‚                         [Store metadata-only]
             â†“
     [CrossRef /works/{DOI} lookup]
             â”‚
     [Extract: journal, ISSN, publisher,
      license[], link[], open-access flag,
      citation count, reference list]
             â”‚
     [Unpaywall API lookup]
     (https://api.unpaywall.org/v2/{DOI}
      ?email=ferrumyx@local)
             â”‚
     [OA status: gold/green/hybrid/closed]
             â”‚
     [Store in papers.doi,
      papers.oa_status,
      papers.full_text_url]
             â†“
     [DOI Resolution Complete]
```

**CrossRef title-matching confidence** is computed as:
```
score = jaro_winkler(query_title, result_title)
      + 0.2 * (query_year == result_year ? 1 : 0)
      + 0.1 * author_overlap_ratio
Threshold: score >= 0.92 to accept
```

**Unpaywall integration** is a WASM tool calling `https://api.unpaywall.org/v2/{doi}?email=...`; the `best_oa_location.url_for_pdf` field, when non-null, feeds directly into the full-text retrieval pipeline. No authentication required; polite usage enforced by IronClaw rate-limiter (3 req/sec).

---

## 2.4 Open-Access Detection and Full-Text Retrieval Strategy

Ferrumyx operates a tiered retrieval strategy. The goal is to maximise the fraction of papers where full structured text (not just abstract) is available, without violating terms of service.

```
Tier 1 (preferred): PubMed Central XML
  â””â”€ Check pmc_id != null in PubMed efetch response
  â””â”€ Fetch: https://eutils.ncbi.nlm.nih.gov/entrez/eutils/
             efetch.fcgi?db=pmc&id={PMC_ID}&rettype=xml
  â””â”€ Full structured XML with <sec>, <table-wrap>,
     <fig>, <formula> tags

Tier 2: Unpaywall PDF (gold/green OA)
  â””â”€ oa_status IN ('gold', 'green', 'hybrid')
  â””â”€ url_for_pdf != null
  â””â”€ Fetch PDF â†’ Docling parse pipeline

Tier 3: Europe PMC full-text XML
  â””â”€ https://www.ebi.ac.uk/europepmc/webservices/rest/
     {PMCID}/fullTextXML
  â””â”€ Structured, similar to PMC XML

Tier 4: bioRxiv/medRxiv PDF
  â””â”€ doi matches 10.1101/* pattern
  â””â”€ https://www.biorxiv.org/content/{doi}v{version}.full.pdf
  â””â”€ Fetch PDF â†’ Docling

Tier 5: Semantic Scholar OA PDF
  â””â”€ openAccessPdf.url != null
  â””â”€ Fetch PDF â†’ Docling

Tier 6 (fallback): Abstract only
  â””â”€ No full-text available or accessible
  â””â”€ Store abstract as single chunk
  â””â”€ Flag papers.full_text_available = false
```

**Decision stored in DB:** `papers.retrieval_tier` (1â€“6) enables retrospective analysis of corpus coverage. Typical expectation for recent oncology literature: ~60% Tier 1â€“3, ~20% Tier 4â€“5, ~20% abstract-only.

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
- Section type from `sec-type` attribute â†’ maps to `section_type` enum: {Abstract, Introduction, Methods, Results, Discussion, Conclusion, SupplementaryMethods, Other}
- `<table-wrap>` contents â†’ serialized to TSV for structured extraction
- `<formula>` (MathML or TeX) â†’ preserved as-is for optional downstream parsing
- `<xref ref-type="bibr">` â†’ inline citation IDs â†’ resolved to DOIs via ref-list

### XML vs PDF Decision Matrix

| Scenario | Preferred format | Reason |
|---|---|---|
| PMC ID available | PMC XML | Deterministic section boundaries |
| Europe PMC indexed | Europe PMC XML | Same benefit, good fallback |
| bioRxiv preprint | PDF + Docling | No structured XML available |
| Older journal paper (pre-2005) | PDF + Docling | PMC coverage sparse |
| Conference proceedings | PDF + Docling | Rarely in PMC |
| Supplementary material | PDF + Docling | Never structured in PMC |

---

## 2.6 PDF Parsing Integration: Docling Deep Evaluation

### Overview

Docling (IBM Research, Apache 2.0) is the selected PDF parser for Ferrumyx. It is deployed as a Docker container and invoked as an IronClaw Docker tool. Below is a detailed evaluation.

### Strengths

| Strength | Detail |
|---|---|
| Complex PDF table extraction | Heron layout model accurately identifies multi-column tables with merged cells; outputs structured table objects in JSON (rows Ã— columns) |
| Local execution | No external API calls; data never leaves the machine; critical for CONFIDENTIAL data classification |
| MCP server mode | `docling-serve` exposes an MCP-compatible HTTP endpoint; IronClaw can register it as an MCP tool |
| Formula detection | Detects display-math regions; extracts as LaTeX strings when possible |
| Figure captioning | Associates figure bounding boxes with their captions |
| Multi-format input | PDF, DOCX, HTML, LaTeX, images â€” one unified interface |
| Markdown + JSON export | JSON export preserves document structure (headings, tables, paragraphs with bounding boxes) |
| Page-number metadata | Every text element tagged with source page number |

### Limitations

| Limitation | Severity | Detail |
|---|---|---|
| No scientific section inference | Medium | Does not infer "Methods" vs "Results" from content; relies on explicit heading text â†’ requires Ferrumyx post-processing to map headings to section_type enum |
| OCR accuracy on chemical structures | High | SMILES / Markush structures in figures are not extracted; only bounding box coordinates. Requires separate dedicated chemistry OCR (e.g., OSRA, MolScribe) if chemical structure images are needed |
| Compute requirements | Medium | Heron layout model requires ~2â€“4 GB VRAM (GPU accelerated) or ~8â€“12s per page on CPU; batch processing needed for large corpora |
| Scanned PDFs (bitmap) | Medium | OCR quality degrades on low-DPI scans; pre-2000 literature is often poorly scanned |
| No citation link extraction | Low | Citation cross-references (e.g., "[12]") extracted as plain text; resolution requires separate pass |
| Non-deterministic table borders | Low | Edge cases with borderless tables occasionally misidentify column count |

### Integration Path

**Option A: Docker binary tool (recommended for MVP)**

```
IronClaw orchestrator
  â”‚
  â”œâ”€ DockerTool "docling-parse"
  â”‚    image: quay.io/docling/docling-serve:latest
  â”‚    command: ["docling", "--from", "pdf", "--to", "json",
  â”‚              "--output", "/output/", "/input/paper.pdf"]
  â”‚    volumes:
  â”‚      - {tmp_dir}/input:/input:ro
  â”‚      - {tmp_dir}/output:/output:rw
  â”‚    gpu: optional (cuda device if available)
  â”‚    timeout: 120s per document
  â”‚
  â””â”€ Output: /output/paper.json
       parsed by Rust struct DoclingDocument
```

**Option B: MCP server mode (for interactive/streaming use)**

```
# Start once, persist for session
docker run -d --name docling-mcp \
  -p 5001:5001 \
  quay.io/docling/docling-serve:latest

# Register in IronClaw tool registry
[[tools]]
name = "docling_mcp"
type = "mcp"
endpoint = "http://localhost:5001"
tools = ["convert_document"]
```

Option B reduces Docker startup overhead (~3s per invocation) for high-throughput batch ingestion (>100 papers/day). Option A is simpler for MVP; Option B should be adopted at Month 2 when batch ingestion begins.

### Docling JSON Output Schema (Ferrumyx-relevant fields)

```json
{
  "name": "paper.pdf",
  "pages": [{"page_no": 1, "width": 612, "height": 792}],
  "texts": [
    {
      "text": "Introduction",
      "label": "section_header",
      "page_no": 1,
      "bbox": {"l": 72, "t": 120, "r": 300, "b": 135}
    },
    {
      "text": "KRAS G12D mutations drive...",
      "label": "paragraph",
      "page_no": 1,
      "bbox": {"l": 72, "t": 140, "r": 540, "b": 200}
    }
  ],
  "tables": [
    {
      "page_no": 3,
      "data": {
        "grid": [
          [{"text": "Gene"}, {"text": "IC50 (nM)"}],
          [{"text": "KRAS G12D"}, {"text": "42.3"}]
        ]
      }
    }
  ]
}
```

The Ferrumyx `DoclingParser` Rust struct consumes this JSON and performs heading-to-section-type inference using a case-insensitive keyword lookup table:

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
| Conclusion | Single chunk (if â‰¤ 512 tokens) or 512+64 | Usually short |
| Table | One chunk per table row-group (â‰¤512 tokens) | Tables serialised as "col1: val1 | col2: val2" |
| Figure caption | Single chunk per figure | Captions are self-contained |
| Supplementary | 512-token window, 64-token overlap | Treated same as methods |

**Token counting:** `tiktoken` Python library (cl100k_base encoding) via a lightweight Docker tool; for Rust-native, `tiktoken-rs` crate. Token count is based on the **embedding model's tokenizer**, not the LLM tokenizer â€” BiomedBERT uses WordPiece with a 512 subword token limit.

**Important:** BiomedBERT has a hard 512-token limit per input. The 512-token chunk size with 64-token overlap is calibrated to fit within this limit including special tokens ([CLS], [SEP]). Effective content window = 510 tokens.

### Chunk Metadata Schema

Every chunk stored in `paper_chunks` carries:

```sql
paper_id        UUID        -- FK to papers table
section_type    TEXT        -- Abstract|Introduction|Methods|
                            -- Results|Discussion|Conclusion|
                            -- Table|FigureCaption|Other
chunk_index     INTEGER     -- 0-based within section
page_number     INTEGER     -- Source page from Docling/PMC XML
token_count     INTEGER     -- Actual token count of this chunk
char_offset     INTEGER     -- Character offset in reconstructed full text
text            TEXT        -- Raw chunk text
embedding       VECTOR(768) -- BiomedBERT-base or VECTOR(1024) for large
created_at      TIMESTAMPTZ
```

**Cross-reference:** This maps directly to the `paper_chunks` table in the Phase 1 PostgreSQL schema.

### Overlap Implementation

```
Section text (tokenized):
[t0 t1 t2 ... t511 | t448 t449 ... t959 | t896 t897 ... ]
                   ^--- 64-token overlap between chunk 0 and chunk 1
```

Overlap is computed at the token level, not character level, to ensure consistent chunk sizes. The reconstructed overlap is stored only in the later chunk (chunk_index n+1 carries the last 64 tokens of chunk n as its prefix). During retrieval, duplicate text from overlapping chunks is deduplicated by the query handler before passing to LLM context.

---

## 2.8 Embedding Pipeline

### Model Selection

| Model | Dimensions | Max tokens | Intended use | Deployment |
|---|---|---|---|---|
| `dmis-lab/biobert-base-cased-v1.2` (BiomedBERT-base) | 768 | 512 | Default; MVP | Docker Python |
| `microsoft/BiomedNLP-PubMedBERT-base-uncased-abstract-fulltext` | 768 | 512 | Default alternative | Docker Python |
| `NationalLibraryOfMedicine/BiomedBERT-large-uncased-abstract-fulltext` | 1024 | 512 | High-precision mode | Docker Python (GPU recommended) |
| SPECTER2 (from Semantic Scholar) | 768 | 512 | Citation-aware embeddings, optional | Docker Python |

**Default selection:** `microsoft/BiomedNLP-PubMedBERT-base-uncased-abstract-fulltext` â€” trained on 14M+ PubMed abstracts + full-text articles; strong performance on biomedical STS benchmarks; freely available via HuggingFace.

**High-precision mode** activates when `embedding_mode = "high_precision"` in Ferrumyx config; uses BiomedBERT-large (1024-dim); embeddings stored in a separate pgvector column `embedding_large VECTOR(1024)`.

### Embedding Service

```
[IronClaw DockerTool "embed_chunks"]
  image: ferrumyx/embed-service:latest
  # Dockerfile:
  #   FROM python:3.11-slim
  #   RUN pip install sentence-transformers torch --index-url ...
  #   COPY embed_service.py .
  #   ENTRYPOINT ["python", "embed_service.py"]

Input (stdin JSON):
  {"chunks": ["text1", "text2", ...], "model": "pubmedbert-base"}

Output (stdout JSON):
  {"embeddings": [[0.123, ...], [0.456, ...]], "dim": 768}
```

**Batch size:** 32 chunks per inference call. Larger batches risk OOM on CPU-only environments. On GPU (CUDA), batch size 128 is feasible.

**Throughput estimate:**
- CPU (8-core): ~50 chunks/sec â†’ 1,000-chunk paper â‰ˆ 20s
- GPU (RTX 3080): ~800 chunks/sec â†’ 1,000-chunk paper â‰ˆ 1.25s

### pgvector Storage

```sql
-- Index creation (run once after initial bulk load)
CREATE INDEX ON paper_chunks
  USING ivfflat (embedding vector_cosine_ops)
  WITH (lists = 200);
-- lists = sqrt(row_count) is rule of thumb
-- At 1M chunks: lists = 1000
-- At 10M chunks: lists = 3162

-- For large embedding column:
CREATE INDEX ON paper_chunks
  USING ivfflat (embedding_large vector_cosine_ops)
  WITH (lists = 200);
```

**Hybrid search (RRF):** IronClaw's built-in hybrid search combines vector similarity (ANN via ivfflat) with BM25 full-text search on `paper_chunks.text`. Reciprocal Rank Fusion weight: `rrf_k = 60` (default). Vector search weight 0.7, keyword weight 0.3 for domain-specific biomedical queries.

```sql
-- Hybrid search query pattern
WITH vector_results AS (
  SELECT paper_id, chunk_index,
         1 - (embedding <=> $query_vec) AS score,
         ROW_NUMBER() OVER (ORDER BY embedding <=> $query_vec) AS rank
  FROM paper_chunks
  ORDER BY embedding <=> $query_vec
  LIMIT 100
),
keyword_results AS (
  SELECT paper_id, chunk_index,
         ts_rank(to_tsvector('english', text),
                 plainto_tsquery('english', $query_text)) AS score,
         ROW_NUMBER() OVER (
           ORDER BY ts_rank(to_tsvector('english', text),
                            plainto_tsquery('english', $query_text)) DESC
         ) AS rank
  FROM paper_chunks
  WHERE to_tsvector('english', text) @@ plainto_tsquery('english', $query_text)
  LIMIT 100
)
SELECT
  COALESCE(v.paper_id, k.paper_id) AS paper_id,
  COALESCE(v.chunk_index, k.chunk_index) AS chunk_index,
  (1.0/(60 + COALESCE(v.rank, 101)) +
   1.0/(60 + COALESCE(k.rank, 101))) AS rrf_score
FROM vector_results v
FULL OUTER JOIN keyword_results k
  ON v.paper_id = k.paper_id AND v.chunk_index = k.chunk_index
ORDER BY rrf_score DESC
LIMIT 20;
```

---

## 2.9 Biomedical NER Pipeline Evaluation

### MVP Configuration

**Primary:** SciSpacy `en_core_sci_lg` + `en_ner_bc5cdr_md`

| Model | Entity types | F1 (BC5CDR) | Deployment |
|---|---|---|---|
| `en_core_sci_lg` | General biomedical: disease, chemical, gene, protein, cell line, species, DNA, RNA | ~85% | Docker Python |
| `en_ner_bc5cdr_md` | Chemical + Disease (BC5CDR corpus) | ~88% chem, ~85% dis | Docker Python |
| Combined pipeline | Run both; merge overlapping spans preferring higher-confidence model | â€” | Docker Python |

**Why SciSpacy for MVP:**
- Lightweight (en_core_sci_lg ~580MB vs BERN2 ~4GB+)
- Deterministic, no GPU required
- Apache 2.0 license
- Mature Python API, IronClaw Docker tool wraps trivially
- Sufficient precision for gene/mutation/disease entity extraction

### High-Recall Mode: BERN2

BERN2 (Biomedical Entity Recognition and Normalisation 2) is a neural NER+linking system covering 9 entity types with normalization to standard ontologies.

| Property | Value |
|---|---|
| Entity types | Gene, disease, drug, mutation, species, cell line, cell type, DNA, RNA |
| Normalization | NCBI Gene ID, OMIM, MeSH, DrugBank, dbSNP |
| Architecture | BioBERT-based sequence labeler + entity linker |
| Compute | ~6GB VRAM (GPU); ~45s/doc on CPU |
| Deployment | Docker container via official image |
| API | REST (local): POST /plain with {"text": "..."} |
| Ferrumyx use | Activated for high-value papers (citation count > 50, or direct match to target gene) |

**BERN2 output example:**
```json
{
  "annotations": [
    {
      "mention": "KRAS G12D",
      "obj": "mutation",
      "norm_ids": ["rs121913529"],
      "start": 45,
      "end": 54,
      "score": 0.97
    },
    {
      "mention": "pancreatic cancer",
      "obj": "disease",
      "norm_ids": ["MESH:D010190"],
      "start": 73,
      "end": 90,
      "score": 0.94
    }
  ]
}
```

### Build vs Integrate vs Wrap

| Component | Decision | Rationale |
|---|---|---|
| SciSpacy NER | **Wrap** (Docker Python IronClaw tool) | Mature library; no benefit to re-implementing |
| BERN2 NER | **Integrate** (REST client in Rust WASM tool) | Deploy as Docker service; call via HTTP |
| Entity normalisation (genes) | **Build** | Map SciSpacy gene mentions to HGNC IDs using custom lookup table seeded from HGNC REST API |
| Entity normalisation (mutations) | **Build** | HGVS notation normalisation using Rust hgvs crate + custom regex for informal notations (G12D â†’ p.Gly12Asp) |
| Entity normalisation (diseases) | **Integrate** | OLS (Ontology Lookup Service) REST API for MeSH/OMIM lookups |
| Custom oncology gazetteer | **Build** | Curated list of KRAS/RAS pathway members, common oncology abbreviations; used as pre-filter |
| NER result storage | **Build** | Custom `entity_mentions` table in PostgreSQL |

### NER Result Schema

```sql
CREATE TABLE entity_mentions (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  chunk_id      UUID REFERENCES paper_chunks(id),
  paper_id      UUID REFERENCES papers(id),
  mention_text  TEXT NOT NULL,
  entity_type   TEXT NOT NULL,  -- gene|mutation|disease|drug|cell_line|pathway
  norm_id       TEXT,           -- HGNC:1097, MESH:D010190, rs121913529
  norm_source   TEXT,           -- HGNC|MESH|OMIM|DBSNP|CHEMBL
  confidence    FLOAT,
  char_start    INTEGER,
  char_end      INTEGER,
  model_source  TEXT,           -- scispacy|bern2|gazetteer
  created_at    TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX ON entity_mentions (paper_id);
CREATE INDEX ON entity_mentions (norm_id);
CREATE INDEX ON entity_mentions (entity_type, norm_id);
```

---

## 2.10 Deduplication Strategy

Duplicate papers arise when the same work is indexed by multiple sources (e.g., a paper appears
in both PubMed and Europe PMC). Three deduplication tiers are applied in sequence:

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
1. PubMed (richest structured metadata, MeSH terms)
2. Europe PMC (good fallback, grants/patent cross-refs)
3. bioRxiv/medRxiv (preprint; superseded when PubMed record appears)
4. Semantic Scholar / CrossRef (metadata-only fallback)

When a duplicate is detected across sources, the canonical record is updated to merge: all available IDs (PMID, PMCID, DOI, S2 paper ID), open-access URLs from all sources, and citation counts.

**Ingestion audit log:** Cross-reference to the `ingestion_audit` table defined in Phase 1 schema. Every deduplication event (tier matched, action taken) is logged with `action = 'deduplicated'` and `detail = {tier: 1, matched_paper_id: "..."}`.

---

## 2.11 Phase 2 Build vs Integrate vs Wrap Summary

| Component | Decision | Notes |
|---|---|---|
| PubMed/NCBI E-utilities client | **Build** (Rust WASM tool) | Simple HTTP; well-documented API |
| Europe PMC REST client | **Build** (Rust WASM tool) | Same pattern as PubMed |
| bioRxiv/medRxiv client | **Build** (Rust WASM tool) | Simple JSON REST |
| arXiv Atom XML client | **Build** (Rust WASM tool) | `quick-xml` crate |
| ClinicalTrials.gov v2 client | **Build** (Rust WASM tool) | JSON REST |
| CrossRef REST client | **Build** (Rust WASM tool) | DOI resolution |
| Semantic Scholar client | **Build** (Rust WASM tool) | API key injected at host boundary |
| Unpaywall client | **Build** (Rust WASM tool) | OA detection |
| DOI resolution logic | **Build** (Rust, host layer) | Jaro-Winkler title matching |
| PMC XML parser | **Build** (Rust, `quick-xml`) | Section extraction, citation links |
| Docling PDF parser | **Integrate** (Docker container) | IBM Research; do not re-implement |
| Chunking logic | **Build** (Rust, host layer) | Section-aware, token-counted |
| Embedding service | **Integrate** + wrap (Docker Python) | HuggingFace `sentence-transformers` |
| SciSpacy NER | **Wrap** (Docker Python IronClaw tool) | Do not re-implement |
| BERN2 NER | **Integrate** (Docker service + Rust REST client) | Deploy separately; call via HTTP |
| HGNC gene normalisation | **Build** (Rust lookup table, seeded from HGNC REST) | Map gene mentions → HGNC IDs |
| HGVS mutation normalisation | **Build** (Rust, regex + `hgvs` crate) | Normalise informal notations |
| Disease normalisation (OLS) | **Integrate** (Rust WASM REST client → OLS API) | MeSH/OMIM lookups |
| Deduplication (DOI + SimHash) | **Build** (Rust, host layer) | SimHash via `simhash` crate |
| pgvector storage | **Integrate** (IronClaw existing DB) | Add Ferrumyx schema on top |

in both PubMed and Europe PMC). Deduplication runs at ingestion time, before any downstream processing.

### Deduplication Algorithm (Ordered by Priority)

**Stage 1 — DOI exact match (primary key)**
```
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
| PubMed E-utilities client | **Build** (Rust, WASM tool) | Simple REST; Rust HTTP client sufficient |
| Europe PMC client | **Build** (Rust, WASM tool) | Same |
| bioRxiv/medRxiv client | **Build** (Rust, WASM tool) | Simple JSON API |
| arXiv client | **Build** (Rust, WASM tool) | Atom XML; `quick-xml` crate |
| ClinicalTrials.gov client | **Build** (Rust, WASM tool) | REST JSON API |
| CrossRef client | **Build** (Rust, WASM tool) | DOI resolution; simple REST |
| Semantic Scholar client | **Build** (Rust, WASM tool) | REST; SPECTER2 embeddings bonus |
| Unpaywall client | **Build** (Rust, WASM tool) | DOI OA lookup |
| Docling PDF parser | **Integrate** (Docker container) | IBM Research; mature; not worth reimplementing |
| BiomedBERT embeddings | **Integrate** (Docker Python, HuggingFace) | Pre-trained; no fine-tuning needed for MVP |
| SciSpacy NER | **Integrate** (Docker Python) | Mature biomedical NLP library |
| BERN2 NER | **Integrate** (Docker, REST) | Neural NER+linking; deploy as service |
| Entity normalisation (genes) | **Build** (Rust) | Custom HGNC lookup table + hgvs crate |
| Entity normalisation (mutations) | **Build** (Rust) | HGVS regex + variant notation normaliser |
| Entity normalisation (diseases) | **Integrate** (OLS REST API) | MeSH/OMIM lookup via EBI OLS |
| Deduplication logic | **Build** (Rust) | SimHash implementation; straightforward |
| Chunking pipeline | **Build** (Rust) | Section-aware logic; tightly coupled to PMC XML parser |
| PMC XML parser | **Build** (Rust, `quick-xml`) | Ferrumyx-specific section mapping |

---

# Phase 3: Knowledge Graph & Target Intelligence

## 3.1 Entity Type Schemas

```sql
-- Gene / Protein
CREATE TABLE ent_genes (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    hgnc_id         TEXT UNIQUE,          -- HGNC:1097
    symbol          TEXT NOT NULL,        -- KRAS
    name            TEXT,                 -- KRAS proto-oncogene, GTPase
    uniprot_id      TEXT,                 -- P01116
    ensembl_id      TEXT,                 -- ENSG00000133703
    entrez_id       TEXT,                 -- 3845
    gene_biotype    TEXT,                 -- protein_coding
    chromosome      TEXT,                 -- 12
    strand          SMALLINT,             -- 1 or -1
    aliases         TEXT[],
    oncogene_flag   BOOLEAN DEFAULT FALSE,
    tsg_flag        BOOLEAN DEFAULT FALSE, -- tumour suppressor
    created_at      TIMESTAMPTZ DEFAULT NOW()
);

-- Mutation
CREATE TABLE ent_mutations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene_id         UUID REFERENCES ent_genes(id),
    hgvs_p          TEXT,    -- p.Gly12Asp
    hgvs_c          TEXT,    -- c.35G>A
    rs_id           TEXT,    -- rs121913529
    aa_ref          TEXT,    -- G
    aa_alt          TEXT,    -- D
    aa_position     INTEGER, -- 12
    oncogenicity    TEXT,    -- 'Oncogenic'|'Likely Oncogenic'|'VUS'|'Benign'
    hotspot_flag    BOOLEAN DEFAULT FALSE,
    vaf_context     TEXT,    -- 'somatic'|'germline'|'unknown'
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (gene_id, hgvs_p)
);

-- Cancer Type (OncoTree)
CREATE TABLE ent_cancer_types (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    oncotree_code   TEXT UNIQUE,   -- PAAD
    oncotree_name   TEXT,          -- Pancreatic Adenocarcinoma
    icd_o3_code     TEXT,          -- 8500/3
    tissue          TEXT,          -- Pancreas
    parent_code     TEXT,          -- PANCREAS
    level           INTEGER,       -- depth in OncoTree hierarchy
    created_at      TIMESTAMPTZ DEFAULT NOW()
);

-- Pathway
CREATE TABLE ent_pathways (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    kegg_id         TEXT,          -- hsa04010
    reactome_id     TEXT,          -- R-HSA-5673001
    go_term         TEXT,          -- GO:0007265
    name            TEXT NOT NULL,
    gene_members    TEXT[],        -- array of HGNC symbols
    source          TEXT,          -- 'KEGG'|'Reactome'|'GO'
    created_at      TIMESTAMPTZ DEFAULT NOW()
);

-- Clinical Evidence
CREATE TABLE ent_clinical_evidence (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    nct_id          TEXT,          -- NCT04330664
    pmid            TEXT,
    doi             TEXT,
    phase           TEXT,          -- 'Phase 1'|'Phase 2'|'Phase 3'|'Phase 4'|'Approved'
    intervention    TEXT,          -- drug name
    target_gene_id  UUID REFERENCES ent_genes(id),
    cancer_id       UUID REFERENCES ent_cancer_types(id),
    primary_endpoint TEXT,
    outcome         TEXT,          -- 'positive'|'negative'|'inconclusive'|'ongoing'
    evidence_grade  TEXT,          -- ESMO A/B/C or ASCO high/medium/low
    created_at      TIMESTAMPTZ DEFAULT NOW()
);

-- Inhibitor / Compound
CREATE TABLE ent_compounds (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chembl_id       TEXT UNIQUE,
    name            TEXT,
    smiles          TEXT,
    inchi_key       TEXT UNIQUE,
    moa             TEXT,          -- mechanism of action
    patent_status   TEXT,          -- 'patented'|'generic'|'unpatented'
    max_phase       INTEGER,       -- 0-4 (4 = approved)
    target_gene_ids UUID[],        -- array of gene entity IDs
    created_at      TIMESTAMPTZ DEFAULT NOW()
);

-- Structural Availability
CREATE TABLE ent_structures (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene_id         UUID REFERENCES ent_genes(id),
    pdb_ids         TEXT[],        -- ['4OBE', '6OIM', ...]
    best_resolution FLOAT,         -- Angstrom, lowest = best
    exp_method      TEXT,          -- 'X-ray'|'Cryo-EM'|'NMR'
    af_accession    TEXT,          -- AlphaFold UniProt accession
    af_plddt_mean   FLOAT,         -- mean pLDDT across residues
    af_plddt_active FLOAT,         -- pLDDT at predicted active site
    has_pdb         BOOLEAN DEFAULT FALSE,
    has_alphafold   BOOLEAN DEFAULT FALSE,
    updated_at      TIMESTAMPTZ DEFAULT NOW()
);

-- Druggability Score
CREATE TABLE ent_druggability (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    structure_id    UUID REFERENCES ent_structures(id),
    fpocket_score   FLOAT,
    fpocket_volume  FLOAT,         -- Angstrom^3
    fpocket_pocket_count INTEGER,
    dogsitescorer   FLOAT,         -- 0-1
    overall_assessment TEXT,       -- 'druggable'|'difficult'|'undruggable'
    assessed_at     TIMESTAMPTZ DEFAULT NOW()
);

-- Synthetic Lethality
CREATE TABLE ent_synthetic_lethality (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gene1_id        UUID REFERENCES ent_genes(id),
    gene2_id        UUID REFERENCES ent_genes(id),
    cancer_id       UUID REFERENCES ent_cancer_types(id),
    evidence_type   TEXT,          -- 'CRISPR_screen'|'RNAi'|'computational'|'clinical'
    source_db       TEXT,          -- 'SynLethDB'|'ISLE'|'DepMap'
    screen_id       TEXT,          -- internal CRISPR screen identifier
    effect_size     FLOAT,         -- e.g. CERES delta
    confidence      FLOAT,
    pmid            TEXT,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (gene1_id, gene2_id, cancer_id, source_db)
);
```

## 3.2 Confidence Scoring Model

Every `kg_facts` row has a `confidence` value in [0, 1] computed as:

```
confidence = base_weight × Π(applicable_modifiers)  [capped at 1.0]
```

### Base Weights by Evidence Type

| Evidence Type | Base Weight |
|---|---|
| Experimental in vivo | 1.00 |
| Experimental in vitro | 0.85 |
| Clinical trial Phase 3+ (positive outcome) | 1.00 |
| Clinical trial Phase 1–2 | 0.75 |
| Computational (ML-based prediction) | 0.50 |
| Computational (rule-based / pathway inference) | 0.35 |
| Text-mined (NER extraction, unverified) | 0.30 |
| Database assertion (no traceable primary source) | 0.40 |

### Modifiers (Multiplicative)

| Modifier Condition | Factor |
|---|---|
| Sample size > 1,000 | ×1.20 |
| Replicated in ≥2 independent studies | ×1.15 |
| Published in journal with IF > 10 | ×1.05 |
| Preprint only (not peer-reviewed) | ×0.70 |
| Single-cell line only (not in vivo) | ×0.85 |
| Retracted paper | ×0.00 |

**Example:** An in vitro result (0.85) from a preprint (×0.70) with sample size 50 (no size modifier) = 0.85 × 0.70 = **0.595**

**Implementation:** Modifiers are computed at fact-insertion time in the `ferrumyx-kg` Rust module, using metadata from the `papers` table (journal IF from CrossRef, peer_review_status, retraction flag via RetractionWatch API).

## 3.3 Evidence Weighting & Aggregation

When multiple independent facts support the same (subject, predicate, object) triple:

```
aggregate_confidence = 1 - Π(1 - confidence_i)
```

This is the noisy-OR model — each independent piece of evidence adds to aggregate certainty.

**Contradictory evidence** (e.g., two facts with opposite directionality on the same predicate):

```
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

```
[New paper ingested]
        │
        ▼
[NER extraction produces new entity mentions]
        │
        ▼
[Fact triple candidate: (subject, predicate, object)]
        │
   [Check existing kg_facts]
        │
   ┌────┴────┐
Exists?      New?
   │              │
   ▼              ▼
[Add evidence   [INSERT new
 row; recompute  kg_fact row]
 aggregate conf] │
   │              │
   └──────┬───────┘
          │
   [confidence_delta > 0.05?]
          │
         Yes
          │
          ▼
[Queue target re-scoring via
 IronClaw routines event:
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

```
Algorithm ConflictResolution(fact_a, fact_b):

1. DETECT: same (subject_id, predicate, object_id), opposite directionality
   OR confidence delta > 0.4 between two facts with confidence > 0.6

2. LOG: INSERT INTO kg_conflicts (fact_a_id, fact_b_id, conflict_type, net_confidence)

3. COMPUTE net_confidence:
   net = |confidence_a - confidence_b|   (for directional conflicts)
   OR = aggregated noisy-OR value        (for reinforcing evidence)

4. CLASSIFY:
   IF net_confidence < 0.30:
       → Mark relationship as DISPUTED
       → Exclude from default scoring queries (available via opt-in flag)
   IF net_confidence 0.30–0.60:
       → Include with DISPUTED flag in output
       → Confidence penalty: ×0.70
   IF net_confidence > 0.60:
       → Treat dominant direction as current, note minority evidence

5. HUMAN REVIEW QUEUE:
   IF both fact_a.confidence > 0.70 AND fact_b.confidence > 0.70:
       → INSERT into human_review_queue with priority = HIGH
       → Notify operator via IronClaw channel
```

## 3.7 Graph Traversal Query Patterns

### Pattern 1: Multi-hop — Genes inhibited by existing drugs in KRAS-pathway cancers

```sql
SELECT DISTINCT g.symbol, g.hgnc_id,
       COUNT(DISTINCT kf2.id) AS inhibitor_count
FROM ent_genes g
-- Join to mutations in this gene
JOIN kg_facts kf1 ON kf1.subject_id = g.id
                 AND kf1.predicate = 'mutated_in'
                 AND kf1.valid_until IS NULL
-- Cancer context: KRAS-associated
JOIN ent_cancer_types ct ON kf1.object_id = ct.id
                         AND ct.oncotree_code IN ('PAAD','LUAD','COAD')
-- Join to inhibitor relationships
JOIN kg_facts kf2 ON kf2.object_id = g.id
                 AND kf2.predicate = 'inhibits'
                 AND kf2.valid_until IS NULL
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

## 3.8 PostgreSQL-only vs PostgreSQL + Neo4j: Assessment

| Dimension | PostgreSQL-only | + Neo4j |
|---|---|---|
| MVP operational complexity | Low (1 DB) | High (2 DBs, sync layer) |
| Native graph traversal | Moderate (JOINs, CTEs) | Excellent (Cypher, native graph) |
| >3-hop path queries | Slow above 5M facts | Fast |
| Vector search | Native (pgvector) | Not supported natively |
| IronClaw integration | Already integrated | Requires new adapter |
| Licensing | Open source | Community edition free; Enterprise: $$ |
| Sync complexity | N/A | CDC pipeline required (Debezium) |

**Recommendation: PostgreSQL-only through Month 6.**

Rationale: For the MVP cancer domain (KRAS G12D PDAC), the knowledge graph will contain <500K facts. All required traversal patterns (up to 4-hop joins) are feasible in PostgreSQL with proper indexing. The operational cost of maintaining a second database and synchronisation layer is not justified at this scale.

**Trigger for Neo4j adoption:** If at Month 12, path traversal queries on >5M facts exceed 500ms P95 latency for common patterns. If adopted, Neo4j is a read-only analytical mirror updated via CDC; PostgreSQL remains the write-primary source of truth.

---

# Phase 4: Target Prioritization Engine

## 4.1 Composite Score Formula

The composite target priority score for gene *g* in cancer context *c* is:

```
S(g, c) = [ Σᵢ wᵢ × nᵢ(g, c) ] − P(g, c)

constrained to: S(g, c) ∈ [0, 1]

Confidence-adjusted:
S_adj(g, c) = S(g, c) × C(g, c)

where C(g, c) = mean confidence of all KG facts
               contributing to the 9 component scores
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

```
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

```
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

```
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

```
For component i, across all N candidate (gene, cancer) pairs:
  rank_i(g, c) = rank of (g, c) among all candidates by raw component value
  n_i(g, c) = rank_i(g, c) / N

So the top-ranked candidate gets n_i = 1.0, last-ranked gets n_i ≈ 0.
```

**Exception — CRISPR dependency (component 2):**
CERES scores have biological meaning at specific thresholds (< -1.0 = strongly essential). Apply min-max within [-2.0, 0.0] range before ranking:
```
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

```
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

---

# Phase 5: Structural Analysis & Molecule Design

## 5.1 Tool Inventory

| Tool | Function | Language | Deployment | License |
|---|---|---|---|---|
| PDB REST API client | Structure retrieval by PDB ID / UniProt | Rust | WASM | Open |
| AlphaFold DB API client | Predicted structure retrieval | Rust | WASM | CC BY 4.0 |
| fpocket | Binding pocket detection | C binary | Docker | BSD |
| AutoDock Vina | Molecular docking | C++ binary | Docker | Apache 2.0 |
| Gnina | CNN-scored docking (Vina-based) | C++ binary | Docker | MIT |
| RDKit | Molecule manipulation, SMILES, properties | Python | Docker | BSD |
| DeepPurpose | Binding affinity prediction (DTI) | Python | Docker | MIT |
| ADMET-AI | ADMET property prediction | Python | Docker | MIT |
| Reinvent4 | Generative molecule optimisation (optional, Month 7+) | Python | Docker | Apache 2.0 |

**WASM vs Docker rationale:**
- WASM: stateless HTTP API calls (PDB, AlphaFold); low compute; no binary dependencies
- Docker: compute-intensive binaries (fpocket, Vina) or large Python ML stacks (RDKit, DeepPurpose)
- All Docker containers: network-isolated, resource-limited (2 CPU / 4GB RAM cap), job-directory mounts only

## 5.2 IronClaw Tool Orchestration Sequence

```
[Target shortlist: (gene_id, cancer_id, composite_score)]
        â”‚
        â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  STEP 1: Structure Retrieval    â”‚
â”‚  Tool: pdb_fetch (WASM)         â”‚
â”‚  Input: UniProt / HGNC ID       â”‚
â”‚  Output: PDB IDs + .pdb files   â”‚
â”‚  Fallback: alphafold_fetch      â”‚
â”‚  (WASM) if no PDB available     â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
               â”‚
               â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  STEP 2: Structure Validation   â”‚
â”‚  PDB: resolution < 3.0 Ã…?       â”‚
â”‚  AlphaFold: pLDDT > 70?         â”‚
â”‚  Fail: FLAG + continue with     â”‚
â”‚  caveat; do not abort           â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
               â”‚
               â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  STEP 3: Pocket Detection       â”‚
â”‚  Tool: fpocket_run (Docker)     â”‚
â”‚  Input: .pdb file               â”‚
â”‚  Output: pocket files +         â”‚
â”‚          druggability scores    â”‚
â”‚  Select: best pocket by score   â”‚
â”‚  Fallback: DoGSiteScorer API    â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
               â”‚
               â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  STEP 4: Ligand Preparation     â”‚
â”‚  Tool: rdkit_ops (Docker)       â”‚
â”‚  a) Retrieve known binders from â”‚
â”‚     ChEMBL (seed ligands)       â”‚
â”‚  b) Generate scaffold variants  â”‚
â”‚     via RDKit fragment growing  â”‚
â”‚  c) Compute MW, LogP, HBD, HBA, â”‚
â”‚     TPSA, SA score              â”‚
â”‚  d) Filter: Lipinski Ro5 pass   â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
               â”‚
               â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  STEP 5: Molecular Docking      â”‚
â”‚  Tool: vina_dock (Docker)       â”‚
â”‚  Input: protein .pdbqt +        â”‚
â”‚         ligand .sdf batch       â”‚
â”‚  Output: docking poses + scores â”‚
â”‚  Alt: gnina_dock for CNN        â”‚
â”‚       rescoring of top poses    â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
               â”‚
               â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  STEP 6: ADMET Prediction       â”‚
â”‚  Tool: admet_predict (Docker)   â”‚
â”‚  Input: SMILES of top docked    â”‚
â”‚         molecules               â”‚
â”‚  Output: absorption, tox,       â”‚
â”‚          hERG, hepatotox flags  â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
               â”‚
               â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  STEP 7: Multi-Objective Score  â”‚
â”‚  Combine: vina_score + gnina +  â”‚
â”‚  admet_pass + sa_score +        â”‚
â”‚  novelty vs ChEMBL              â”‚
â”‚  Rank and select candidates     â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
               â”‚
               â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  STEP 8: Store Results          â”‚
â”‚  â†’ molecules + docking_results  â”‚
â”‚  tables (Phase 1 schema)        â”‚
â”‚  â†’ Trigger report generation    â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

## 5.3 Iterative Molecule Optimisation Loop

```
REPEAT (max 5 iterations OR convergence):

  1. Score current batch:
     multi_obj = 0.40 Ã— norm(vina_score)
               + 0.20 Ã— norm(gnina_score)
               + 0.20 Ã— admet_pass_ratio
               + 0.10 Ã— norm(1 / sa_score)
               + 0.10 Ã— novelty_vs_chembl

  2. Select top 20% by multi_obj_score

  3. Generate variants:
     a) Scaffold hopping via RDKit fragment replacement
     b) R-group enumeration on top scaffolds
     c) Reinvent4 (Month 7+): generative sampling
        conditioned on pocket pharmacophore

  4. Filter new batch:
     - Lipinski Ro5: MW < 500, LogP < 5, HBD â‰¤ 5, HBA â‰¤ 10
     - SA score < 6
     - No PAINS alerts (RDKit)
     - Not exact ChEMBL match (InChI key check)

  5. Dock filtered batch â†’ repeat from step 1

CONVERGENCE: mean(multi_obj) improvement < 0.02
             OR top score unchanged for 2 iterations
```

## 5.4 Multi-Objective Optimisation Targets

| Objective | Target | Hard Constraint? |
|---|---|---|
| Vina docking score | < âˆ’7.0 kcal/mol | No (soft) |
| Gnina CNN score | > 0.5 | No (soft) |
| Molecular weight | 300â€“500 Da | Yes |
| LogP | 1â€“5 | Yes |
| HBD | â‰¤ 5 | Yes |
| HBA | â‰¤ 10 | Yes |
| TPSA | < 140 Ã…Â² | Yes |
| Synthetic accessibility (SA) | < 6 | Yes |
| hERG toxicity | Not flagged | Soft (warning) |
| Hepatotoxicity | Not flagged | Soft (warning) |
| ChEMBL novelty | InChI not in ChEMBL | Soft (log only) |

## 5.5 Intermediate Result Storage

All intermediate files in IronClaw workspace:
```
/workspace/structural/{gene_symbol}/{job_id}/
  â”œâ”€â”€ structures/     # .pdb, .pdbqt
  â”œâ”€â”€ pockets/        # fpocket output
  â”œâ”€â”€ ligands/        # .sdf, .mol2
  â”œâ”€â”€ docking/        # Vina/Gnina poses
  â”œâ”€â”€ admet/          # prediction CSVs
  â””â”€â”€ report.json     # job summary
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

## 6.2 Intent Parsing

The NL query is parsed into a structured `ScientificQuery` object:

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
1. Run NER on query text â†’ gene (KRAS), mutation (G12D), cancer (pancreatic cancer), relationship (synthetic lethal)
2. Normalise: KRAS â†’ HGNC:6407; G12D â†’ p.Gly12Asp / rs121913529; pancreatic cancer â†’ PAAD
3. Map filter intent: "structural druggability" â†’ structural_tractability > 0.4; "low prior inhibitor" â†’ ChEMBL count < 20

## 6.3 Query Plan Generation

```
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
Step 6: Rank by composite_score Ã— SL evidence confidence
Step 7: Format output JSON with full citations
```

## 6.4 Tool Invocation Sequence

```
ferrumyx_query_handler.execute(query):

1. ner_extract(query_text)                  â†’ entities JSON
2. entity_normalise(entities)               â†’ canonical IDs
3. kg_query.synthetic_lethality(            â†’ SL partner list
       gene=KRAS, cancer=PAAD, conf>0.5)
4. FOR each SL partner:
   a. target_scores.get_current(gene, PAAD) â†’ score object
   b. structural_data.get(gene)             â†’ structure + druggability
   c. chembl.inhibitor_count(gene)          â†’ count
5. filter_and_rank(candidates, filters)     â†’ ranked list
6. evidence_bundle.assemble(top_N=10)       â†’ citations + KG facts
7. llm_backend.narrate(ranked, evidence)    â†’ human-readable summary
8. format_output(ranked, narrative)         â†’ final JSON
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

Every factual claim must link to â‰¥1 of: PMID, DOI, or database record ID (DepMap, ChEMBL, COSMIC, etc.).

Claims without traceable source â†’ labelled `"source": "INFERRED"`, confidence â‰¤ 0.3.

The LLM narration layer is **explicitly prohibited** from generating factual claims not grounded in KG evidence. The prompt template enforces this with a hard instruction: "Only assert facts that appear in the provided evidence bundle. Do not add information from training data."


---

# Phase 7: Self-Improvement Framework

No vague "learning." All improvement mechanisms are explicit, measurable, and human-gated.

## 7.1 Feedback Metrics

### Metric 1: Retrospective Recall@N
- **Definition:** Of Ferrumyx's top-N ranked targets for a cancer type, what fraction are in DrugBank's approved oncology drug target list?
- **Source:** DrugBank XML dump, filtered to cancer indication + approved status
- **Target baseline:** Recall@20 > 0.60 for KRAS G12D PAAD
- **Frequency:** Monthly

### Metric 2: Docking Score Predictive Value
- **Definition:** Pearson r between Ferrumyx Vina/Gnina scores and experimental IC50 values (ChEMBL) for the same target
- **Source:** ChEMBL `activities` endpoint, assay_type=B (binding)
- **Target baseline:** r > 0.45 (docking is noisy; >0.45 is informative)
- **Frequency:** Quarterly (tied to ChEMBL releases)

### Metric 3: Ranking Stability (Kendall-Ï„)
- **Definition:** Kendall-Ï„ between target ranking at T and Tâˆ’30 days
- **Ï„ > 0.80:** stable signal; weights are reliable
- **Ï„ < 0.50:** noisy; investigate which component is fluctuating
- **Frequency:** Weekly

### Metric 4: Literature Recall
- **Definition:** % of CIViC-validated targets in the domain appearing in Ferrumyx's top-50
- **Source:** CIViC API (clinicalgenome.org)
- **Target baseline:** > 70% in top-50
- **Frequency:** Monthly

### Metric 5: False Positive Accumulation Rate
- **Definition:** % of prior primary-shortlisted targets later clinically invalidated (Phase 3 failure with mechanistic confirmation)
- **Source:** ClinicalTrials.gov outcomes + PubMed failure reports
- **Frequency:** Quarterly

## 7.2 Feedback Data Collection

```
IronClaw Routine: "feedback_collection"
Schedule: Weekly (Sunday 02:00 local)

1. Pull DrugBank approved target list â†’ compute Recall@N
2. Pull ChEMBL activities for top-20 genes â†’ compute docking-IC50 Pearson r
3. Compute Kendall-Ï„ vs previous week's target_scores snapshot
4. Query CIViC API â†’ compute literature recall
5. Scan ClinicalTrials.gov for prior shortlisted target outcomes
6. INSERT all metrics into feedback_events table
7. Evaluate thresholds â†’ generate weight update PROPOSAL if triggered
```

## 7.3 Parameter Re-Weighting Algorithm

**Algorithm: Bayesian bounded update with expert prior**

W (initial expert weights from Â§4.1) is the prior. After each feedback cycle:

```
For each component i:
  compute corr_i = Pearson_r(component_i_scores, Recall@20_signal)

  if corr_i > 0.30:
    w_i_new = w_i Ã— (1 + 0.05 Ã— corr_i)   # reward predictive components
  elif corr_i < 0.10:
    w_i_new = w_i Ã— 0.95                    # penalise non-predictive components
  else:
    w_i_new = w_i                            # no change

Renormalise: W_new = W_new / sum(W_new)

Constraints:
  - No single weight changes by more than 0.05 per cycle
  - No weight drops below 0.01 or rises above 0.40
```

This is intentionally conservative. No gradient descent â€” insufficient data at MVP scale for reliable gradients.

## 7.4 Feedback Loop Triggers

| Trigger | Condition | Action |
|---|---|---|
| Scheduled | Weekly Sunday 02:00 | Full feedback collection run |
| Event: DrugBank release | Quarterly version diff detected | Immediate Recall@N run |
| Event: DepMap release | Quarterly | Re-pull CERES; re-score all targets |
| Event: Ranking volatility | Kendall-Ï„ < 0.50 for 2 consecutive weeks | Alert operator; pause weight proposals |
| Manual | Operator command via REPL | Force feedback run |

## 7.5 Human-in-the-Loop Checkpoints

**No weight update is ever applied automatically.**

```
PROPOSAL STAGE:
  - Old W vs new W diff table
  - Projected ranking changes (which targets move Â±5 positions)
  - Metric values that triggered proposal
  - Sample size of feedback data (confidence in proposal)

NOTIFICATION:
  - Sent via IronClaw channel (REPL / Web Gateway)
  - Includes: summary, projected impact, approve/reject action

APPROVAL:
  - Approved â†’ weights applied atomically; all target_scores re-queued
  - Rejected â†’ log reason; weights unchanged; threshold raised for next cycle

POST-APPLICATION:
  - weight_update_log INSERT (full diff + approver ID)
  - Re-score all current targets
  - Operator notification on completion
```

## 7.6 Audit Trail

All weight changes in `weight_update_log` table (Phase 1 schema):
- `previous_weights JSONB` â€” full W before
- `new_weights JSONB` â€” full W after
- `trigger_event TEXT` â€” what triggered it
- `algorithm TEXT` â€” `bayesian_bounded`
- `approved_by TEXT` â€” operator ID; never `auto`
- `delta_summary JSONB` â€” per-component changes


---

# Phase 8: Security & LLM Strategy

## 8.1 LLM Backend Abstraction (Rust Trait)

```rust
#[async_trait]
pub trait LlmBackend: Send + Sync {
    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError>;
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, LlmError>;
    fn model_id(&self) -> &str;
    fn is_local(&self) -> bool;
    fn max_context_tokens(&self) -> usize;
    fn max_output_tokens(&self) -> usize;
}

pub struct LlmRouter {
    backends: HashMap<String, Arc<dyn LlmBackend>>,
    policy: RoutingPolicy,
    classifier: DataClassifier,
    audit: AuditLogger,
}

impl LlmRouter {
    pub async fn route(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        let class = self.classifier.classify(&req.prompt);
        let backend = self.policy.select_backend(class, &self.backends)?;
        let response = backend.complete(req).await?;
        self.audit.log(&backend, &class, &response).await;
        Ok(response)
    }
}
```

**Concrete backends:**
- `OllamaBackend` â†’ `http://localhost:11434`; `is_local() = true`
- `OpenAIBackend` â†’ `api.openai.com`; `is_local() = false`
- `AnthropicBackend` â†’ `api.anthropic.com`; `is_local() = false`
- `CustomHttpBackend` â†’ configurable endpoint; `is_local()` = configurable

## 8.2 Data Classification

| Class | Definition | Examples |
|---|---|---|
| `PUBLIC` | Published literature, public database records | PubMed abstracts, COSMIC, PDB structures |
| `INTERNAL` | Ferrumyx-generated scores, hypotheses, KG facts | target_scores, molecule candidates, KG edges |
| `CONFIDENTIAL` | Proprietary or unpublished experimental data | Future wet-lab partner data |

Classification applied at **prompt construction** stage, before any LLM call. `DataClassifier` scans prompt content for INTERNAL/CONFIDENTIAL patterns (SMILES strings, composite score values, internal UUIDs).

## 8.3 Redaction Layer

```
Before any remote LLM API call:

classify(prompt) â†’
  CONFIDENTIAL â†’ HARD BLOCK + alert operator; throw error
  INTERNAL     â†’ IF allow_internal_remote == false:
                     route to local Ollama
                 ELSE (explicit override):
                     LOG WARNING in llm_audit_log
                     proceed with remote call
  PUBLIC       â†’ route per policy (prefer local if available)
```

**Redaction patterns (regex, configurable):**
- SMILES strings â†’ tagged INTERNAL by default
- Composite S_adj values â†’ tagged INTERNAL
- Internal job UUIDs â†’ stripped from prompts before sending

## 8.4 Local-Only Mode

```toml
[llm]
mode = "local_only"   # local_only | prefer_local | any
local_backend = "ollama"
local_model = "llama3:8b"
```

In `local_only` mode:
- Router refuses to construct any remote backend call
- If Ollama unavailable: return error (no remote fallback)
- Operator notified via IronClaw channel

## 8.5 Audit Logging

Cross-reference: `llm_audit_log` table (Phase 1 Â§1.4). Fields:
- `session_id` â€” IronClaw job/session ID
- `model` â€” model string
- `backend` â€” `ollama|openai|anthropic|custom`
- `prompt_tokens` / `completion_tokens`
- `data_class` â€” `PUBLIC|INTERNAL|CONFIDENTIAL`
- `output_hash` â€” SHA-256 of response (reproducibility)
- `latency_ms`
- `called_at`

Audit logs are **append-only**, never deleted. Compress rows older than 90 days to cold storage.

## 8.6 WASM Sandbox Enforcement

- **Capability model:** WASM tools declare required capabilities at registration (network: allowlist only; filesystem: none)
- **Endpoint allowlist:** e.g., `pubmed_search` allows only `eutils.ncbi.nlm.nih.gov`
- **Credential injection:** API keys never passed to WASM; injected at host boundary as scoped tokens
- **Leak detection:** IronClaw blocks + alerts on any non-allowlisted outbound request from WASM

Docker tools:
- Isolated Docker network (no internet except explicit allowlist)
- File I/O via mounted job directories only
- Hard resource caps: 2 CPU cores, 4GB RAM, PID limit 256

## 8.7 Secret Management

- All API keys in IronClaw AES-256-GCM keychain; never in plaintext config
- Keys referenced by name (`secret: pubmed_api_key`); resolved at host boundary only
- Docker secrets: passed as env vars at container start; never baked into image layers
- Rotation policy: external API keys rotated quarterly

## 8.8 Rate Limiting & Cost Controls

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

## 9.1 Three-Month MVP (Months 1â€“3)

**Focus:** KRAS G12D Pancreatic Ductal Adenocarcinoma (PDAC)

Chosen because: highest unmet clinical need, well-characterised mutation, rich public datasets (TCGA, COSMIC, DepMap), active ClinicalTrials landscape, and tractable scope for validation.

### Month 1: Foundation
- [ ] Cargo workspace: `ferrumyx-agent`, `ferrumyx-ingestion`, `ferrumyx-kg`, `ferrumyx-ranker` crates
- [ ] PostgreSQL 16 + pgvector deployed; Phase 1 schema migrations run
- [ ] PubMed E-utilities WASM tool (esearch + efetch XML)
- [ ] Europe PMC WASM tool
- [ ] PMC XML section-aware parser (Rust, `quick-xml`)
- [ ] Docling Docker tool wrapper
- [ ] Section-aware chunker
- [ ] BiomedBERT embedding Docker service (batch endpoint)
- [ ] pgvector IVFFlat index setup

**Deliverable:** Ingest a PubMed query result, parse full text, chunk, embed, store in PostgreSQL. Manual verification of 50 KRAS PDAC papers.

### Month 2: Knowledge Graph
- [ ] SciSpacy NER Docker tool (en_core_sci_lg + en_ner_bc5cdr_md)
- [ ] Entity normalisation (HGNC, HGVS, OLS)
- [ ] kg_facts population from NER output
- [ ] External DB pulls: COSMIC, DepMap CERES, ChEMBL, ClinicalTrials.gov
- [ ] Target score computation (all 9 components)
- [ ] Hybrid search (pgvector + tsvector RRF)
- [ ] Basic REPL query interface

**Deliverable:** Can answer "What are the top KRAS G12D targets in PAAD?" with ranked list and source citations.

### Month 3: Structural Pipeline + Query Handler
- [ ] PDB fetch + AlphaFold WASM tools
- [ ] fpocket Docker tool
- [ ] AutoDock Vina Docker tool
- [ ] RDKit Docker tool (SMILES â†’ properties + Lipinski filter)
- [ ] ADMET-AI Docker tool
- [ ] Molecule pipeline orchestration
- [ ] NL query handler (intent parsing â†’ structured plan â†’ tool calls)
- [ ] Output JSON schema (Â§6.5)
- [ ] LLM router with Ollama + OpenAI backends
- [ ] Audit logging populated

**Deliverable:** Full pipeline operational for KRAS G12D PAAD. Literature â†’ KG â†’ target scores â†’ structural analysis â†’ ranked output with citations. Retrospective validation: top-10 vs DrugBank known PDAC targets.

### MVP Explicit Limitations
- Single cancer subtype only (KRAS G12D PAAD)
- No self-improvement loop (metrics collected but weights not updated)
- No generative molecule design (Reinvent4 not integrated)
- PostgreSQL-only; no Neo4j
- LLM narration quality depends on Ollama model capability
- No web UI; REPL + Web Gateway only

---

## 9.2 Six-Month Expansion (Months 4â€“6)

**Expansion criteria:** MVP retrospective Recall@20 > 0.55 for PDAC domain.

**New capabilities:**
- [ ] bioRxiv/medRxiv ingestion tools
- [ ] ClinicalTrials.gov structured ingestion (trial outcomes â†’ KG)
- [ ] Semantic Scholar integration (citation graph + SPECTER2 embeddings)
- [ ] Expand to 3 cancer subtypes: KRAS G12D PDAC + EGFR-mutant NSCLC + BRCA1/2 ovarian
- [ ] BERN2 high-recall NER for high-citation papers
- [ ] Basic generative design (RDKit fragment growing)
- [ ] DeepPurpose binding affinity prediction
- [ ] Feedback metrics collection activated (weights NOT yet auto-updated)
- [ ] Deduplication pipeline hardened (preprintâ†’published pairing)
- [ ] Web Gateway basic query interface

**Validation strategy:** For each new cancer subtype, run retrospective Recall@20 vs DrugBank before declaring that domain operational.

---

## 9.3 Twelve-Month Autonomous Optimisation (Months 7â€“12)

**Self-improvement activation criteria:**
- â‰¥3 complete feedback cycles collected
- Docking-IC50 Pearson r > 0.40 on â‰¥2 target genes
- Recall@20 stable (Â±0.05) for 2 consecutive months
- Human operator approved â‰¥1 weight update proposal

**New capabilities:**
- [ ] Self-improvement loop fully active (weight proposals + human approval)
- [ ] Reinvent4 generative molecule design (CUDA GPU required)
- [ ] Expand to 10+ cancer subtypes; pan-cancer analysis
- [ ] Retrospective validation against all FDA oncology approvals (1990â€“present)
- [ ] Synthetic lethality network analysis (multi-hop Reactome traversal)
- [ ] External validation pipeline: submit top candidates to wet-lab partners
- [ ] Neo4j evaluation: benchmark if fact count > 2M and traversal latency > 500ms P95
- [ ] Full audit report generation (Markdown â†’ PDF)

---

# Deliverables

## Tool Inventory

| Tool / Library | Type | Language | Deployment | License | Notes |
|---|---|---|---|---|---|
| PubMed E-utilities client | Build | Rust | WASM | Open | esearch + efetch; API key optional |
| Europe PMC client | Build | Rust | WASM | Open | fullTextXML for OA papers |
| bioRxiv/medRxiv client | Build | Rust | WASM | Open | PDF metadata only |
| arXiv client | Build | Rust | WASM | Open | Atom XML; quick-xml |
| ClinicalTrials.gov v2 client | Build | Rust | WASM | Open | REST JSON |
| CrossRef client | Build | Rust | WASM | Open | DOI resolution |
| Semantic Scholar client | Build | Rust | WASM | Open | SPECTER2 embeddings |
| Unpaywall client | Build | Rust | WASM | Open | OA detection |
| PMC XML parser | Build | Rust | Native | Open | quick-xml; section-aware |
| Docling | Integrate | Python | Docker | Apache 2.0 | IBM Research; PDF â†’ structured JSON |
| BiomedBERT / PubMedBERT | Integrate | Python | Docker | Apache 2.0 | HuggingFace; 768-dim embeddings |
| SciSpacy en_core_sci_lg | Integrate | Python | Docker | MIT | General biomedical NER |
| SciSpacy en_ner_bc5cdr_md | Integrate | Python | Docker | MIT | Chemical + disease NER |
| BERN2 | Integrate | Python | Docker | MIT | Neural NER + entity linking |
| Gene entity normaliser | Build | Rust | Native | â€” | HGNC REST + hgvs crate |
| Mutation normaliser | Build | Rust | Native | â€” | HGVS regex; notation variants |
| Disease normaliser | Integrate | REST | WASM | Open | EBI OLS API |
| fpocket | Integrate | C | Docker | BSD | Pocket detection |
| AutoDock Vina | Integrate | C++ | Docker | Apache 2.0 | Molecular docking |
| Gnina | Integrate | C++ | Docker | MIT | CNN-scored docking |
| RDKit | Integrate | Python | Docker | BSD | Molecule ops; SMILES; SA score |
| DeepPurpose | Integrate | Python | Docker | MIT | DTI binding affinity |
| ADMET-AI | Integrate | Python | Docker | MIT | ADMET prediction |
| Reinvent4 | Integrate | Python | Docker | Apache 2.0 | Generative design (Month 7+) |
| pgvector | Integrate | C (PG ext) | Native | MIT | Vector similarity search |
| IronClaw | Extend | Rust | Native | Open | Agent loop, WASM sandbox, routines |

---

## Risk Analysis

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| Docking score â‰  binding affinity | H | H | Gnina CNN rescoring; validate against ChEMBL IC50; treat as hypothesis generator only |
| AlphaFold low confidence for disordered regions | H | M | Check pLDDT at active site; prefer PDB; flag structurally unresolved targets |
| NER accuracy on novel nomenclature | M | M | Gazetteer supplement; BERN2 for high-citation papers; manual curation queue |
| PostgreSQL scaling > 10M chunks | M | L (MVP) | IVFFlat â†’ HNSW migration path; pg_partman partitioning; read replicas |
| LLM hallucination in query narration | H | M | Strict grounding: narration only from KG-verified evidence bundle; no training-data facts allowed |
| WASM performance overhead on hot-path tools | L | M | Profile at Month 2; migrate to native Rust if >100ms overhead |
| Data licensing violation | M | L | COSMIC needs registration; ChEMBL CC BY-SA; full audit before any commercial use |
| Stochastic docking non-reproducibility | M | H | Fix random seed in Vina/Gnina; log all run params in docking_results |
| CRISPR data cell-line artefacts | H | H | Supplement DepMap with in vivo data; lower CRISPR weight for cell-line-only targets |
| Feedback loop overfitting to DrugBank | M | M | Multiple validation sources (CIViC + literature); enforce expert approval on all weight updates |

---

## Technical Bottlenecks

1. **Docking throughput at scale.** Vina: ~100 poses/min per CPU core. At 100 molecules Ã— 10 targets Ã— 3 pockets = 3,000 runs â†’ ~30 min on 8 cores. GPU-accelerated Gnina reduces to ~3 min but requires CUDA. A docking job queue (Docker batch) is necessary at Month 6+ scale.

2. **Embedding throughput.** BiomedBERT on CPU: ~50 chunks/sec. 100K papers Ã— 50 chunks avg = 5M chunks â†’ ~28 hours CPU. GPU (RTX 3080): ~800 chunks/sec â†’ ~1.7 hours. GPU is mandatory for production ingestion speed.

3. **KG aggregation correctness.** The noisy-OR aggregation and contradiction detection must be rigorously tested. Bugs propagate silently into target scores. Requires extensive unit tests with synthetic fact sets before production.

4. **NER precision on ambiguous gene symbols.** Symbols like "CAT", "SET", "MAX" are real English words. SciSpacy context window is limited. False positives create spurious KG edges. Requires precision-recall tuning and a periodic manual audit.

5. **Feedback loop data sparsity at MVP.** Underexplored targets (the most interesting ones) have sparse ChEMBL data â€” the exact targets Ferrumyx is designed to find. Docking-IC50 correlation is hardest to compute for novel targets. The self-improvement loop matures most at 12+ months with more coverage.

6. **LLM context window for complex queries.** Assembling a 10-target evidence bundle with full citations can exceed 32K tokens. Evidence prioritisation logic is needed to trim context without dropping key citations.

7. **IronClaw WASM toolchain constraints.** The wasm32-wasip1 target has no threading and limited system calls. Tools requiring parallelism or native crypto must be Docker containers. This limits how many Ferrumyx tools can run in the lightweight WASM sandbox.

---

## Scientific Validity Risks

1. **Docking is not binding.** AutoDock Vina scores correlate weakly with experimental IC50 (r â‰ˆ 0.4â€“0.6). A good docking score is a necessary but not sufficient condition for a real binder. All docking outputs are labelled "computational hypotheses."

2. **In silico â‰  in vivo.** DepMap CRISPR data is from cancer cell lines. Cell line models have altered metabolism, absent microenvironment, and unlimited passage artefacts. Essential targets in cell lines frequently fail in vivo.

3. **Publication bias.** Negative results are systematically underreported. Ferrumyx learns from published literature, which skews toward positive findings. Targets with known failures may appear more promising if failure papers are absent from the corpus.

4. **KRAS G12D cancer-type specificity.** G12D biology varies by tissue context. A synthetic lethal target identified in pancreatic CRISPR screens may not be lethal in KRAS G12D lung cancer. Cancer subtype context must be preserved in all KG queries â€” this is enforced by the (gene, cancer) pairing in target_scores but requires ongoing attention.

5. **AlphaFold disordered region inaccuracy.** Many oncology targets (c-Myc, p53 transactivation domain) are intrinsically disordered. AlphaFold pLDDT < 70 indicates low confidence. Docking against disordered regions is unreliable regardless of pocket detection scores.

6. **Synthetic lethality context dependency.** SL pairs validated in one genetic background may not hold in another co-occurring mutation context. All SL evidence must carry cancer context and genetic background metadata.

---

## Competitive Differentiation

| Competitor | Approach | Ferrumyx position |
|---|---|---|
| Insilico Medicine | Proprietary AI + wet-lab pipeline; clinical-stage assets | Different market; Ferrumyx is open research tooling, not a drug company |
| Recursion Pharmaceuticals | Image-based phenotypic screening + massive experimental data | Ferrumyx is literature/database-driven; complementary for hypothesis generation |
| BenchSci | AI for experimental design and reagent selection | Different problem; Ferrumyx targets target prioritisation, not experimental planning |
| SchrÃ¶dinger (FEP+) | High-accuracy physics-based simulations; expensive | Ferrumyx is high-throughput first-pass; SchrÃ¶dinger is downstream validation |
| Atomwise | Large-scale docking virtual screening; proprietary service | Ferrumyx is open-source, privacy-preserving, KG-grounded; not a screening service |

**Honest positioning:** At MVP, Ferrumyx is a research system â€” not a funded company with wet-lab infrastructure. It cannot compete on experimental validation throughput. The genuine differentiators are:

- **Full auditability:** every hypothesis traces to a specific PMID, DOI, or DB record. No black box decisions.
- **Privacy-preserving by default:** local LLM mode; no data leaves the machine unless explicitly configured.
- **Researcher-controlled:** scoring weights are transparent, inspectable, and modifiable. No vendor lock-in.
- **IronClaw-native:** inherits a production-grade agent loop, WASM sandbox, routines engine, and hybrid search â€” infrastructure that would take months to build from scratch.
- **Integrated hypothesis-to-molecule pipeline:** single autonomous system from literature ingestion to ranked docking candidates.

Target user: a computational biology research group wanting an auditable, privacy-respecting, continuously-learning literature mining and target prioritisation system.

---

## Next Engineering Steps

| # | Step | Complexity | Notes |
|---|---|---|---|
| 1 | Initialise Cargo workspace: ferrumyx-agent, ferrumyx-ingestion, ferrumyx-kg, ferrumyx-ranker | S | Standard Rust workspace; IronClaw as dependency |
| 2 | Deploy PostgreSQL 16 + pgvector; run Phase 1 schema migrations via sqlx | S | Docker Compose for local dev |
| 3 | Implement PubMed E-utilities WASM tool (esearch + efetch XML) | M | quick-xml + reqwest; test with 100 KRAS PDAC papers |
| 4 | Implement PMC XML section-aware parser | M | Map sec-type â†’ SectionType enum; extract tables separately |
| 5 | Implement Docling Docker tool wrapper | M | IronClaw DockerTool API; test with 10 PDF papers |
| 6 | Implement BiomedBERT embedding Docker service | M | Python FastAPI + HuggingFace; batch endpoint; pgvector INSERT |
| 7 | Implement SciSpacy NER Docker tool | M | en_core_sci_lg + en_ner_bc5cdr_md pipeline |
| 8 | Build entity normalisation (HGNC + HGVS) | L | HGNC REST bulk download; HGVS regex; edge cases numerous |
| 9 | Populate kg_facts from NER + external DB pulls (COSMIC, DepMap, ChEMBL) | L | Most complex ingestion step; each source needs separate parser |
| 10 | Implement target_scores computation (9 components + composite formula + versioning) | L | Rank normalisation; penalties; confidence adjustment; atomic versioning |
