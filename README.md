# Ferrumyx

<div align="center">
  <img src="crates/ferrumyx-web/static/logo.svg" alt="Ferrumyx Logo" width="200"/>
</div>

<div align="center">
  <a href="https://colab.research.google.com/github/Classacre/ferrumyx/blob/main/ferrumyx_colab.ipynb">
    <img src="https://colab.research.google.com/assets/colab-badge.svg" alt="Open In Colab"/>
  </a>
</div>

**Open-Source Autonomous Oncology Drug Discovery Engine**

Ferrumyx is an autonomous R&D engine built natively in Rust on the [IronClaw](https://github.com/nearai/ironclaw) autonomous agent framework. Designed as a fully self-improving scientific system, Ferrumyx orchestrates end-to-end therapeutic target discovery and molecular design without human intervention. 

By leveraging IronClaw's robust event loop, reasoning capabilities, and Tool Registry, Ferrumyx operates as a persistent agent. It autonomously queries the latest biomedical literature, constructs and updates a dense Knowledge Graph within a local embedded LanceDB, and iteratively refines its multi-parametric scoring heuristics based on continuous evaluation of generated targets. This closed-loop learning architecture ensures that the system's predictive accuracy scales with its ingestion volume.

For a detailed technical breakdown of the engine's layers, reasoning loop, and state management, please refer directly to the [Architecture Document (ARCHITECTURE.md)](ARCHITECTURE.md).

## Current Status (Phase 2)

| Component | Status | Notes |
|-----------|--------|-------|
| **Ingestion** | ✅ Working | PubMed API, PDF parsing, chunking |
| **Embedding** | ✅ Working | Rust-native BiomedBERT (768-dim, Candle) |
| **NER** | ✅ Working | Rust-native Aho-Corasick trie dictionary matching |
| **KG Building** | ✅ Working | Fact extraction, scoring computation |
| **Deduplication** | ✅ Working | SimHash + PMID conflict resolution |
| **Web GUI** | ✅ Working | Dashboard, ingestion form, API endpoints |
| **Target Ranker** | ✅ Working | Multi-factor scoring with DepMap |
| **Molecules** | ✅ Working | Structural analysis pipeline, ADMET, Ligand generation |

**No Python dependencies.** All components are Rust-native. No external database required (LanceDB embedded).

## Architecture

```
Ferrumyx (100% Rust)
├── ferrumyx-db         — LanceDB embedded vector database
├── ferrumyx-ingestion  — PDF parsing, chunking, PubMed API
├── ferrumyx-embed      — Candle + BiomedBERT embeddings
├── ferrumyx-ner        — Candle NER (biomedical entities)
├── ferrumyx-kg         — Knowledge graph building & scoring
├── ferrumyx-ranker     — Target prioritization
├── ferrumyx-llm        — LLM abstraction layer
├── ferrumyx-agent      — IronClaw agent with tools
└── ferrumyx-web        — Web API & dashboard
```

## Computational Methodology and Framework

Ferrumyx leverages a defense-in-depth, 100% Rust architecture to mitigate performance bottlenecks typically associated with large-scale scientific computation pipelines. By operating independently of external data services, we ensure computational reproducibility and data security.

### Core Algorithmic Components

1. **Information Extraction Engine**
   Employs highly optimized biomedical named entity recognition (NER) via Aho-Corasick dictionary matching across multiple taxonomic classes (Genes, Proteins, Drugs, Diseases). High-throughput ingestion queues process dense literature efficiently.

2. **Graph-Theoretic Knowledge Representation**
   Extracts semantic triplets from unstructured text and constructs a local graph topology. The system uses SimHash-based deduplication algorithms to merge conflicting factual nodes and scales linearly via embedded LanceDB vector storage.

3. **Composite Target Prioritization Matrix**
   Implements a multi-parametric heuristic function `S(g,c)` merging independent component scalars:
   - Structural variants and mutation frequencies
   - CRISPR dependency models (DepMap)
   - Survival correlates
   - Proteomic pocket detectability

4. **In Silico Pipeline Orchestration**
   Autonomously invokes structural parsing logic, Lipinski's Rule of 5 evaluations for druglikeness (ADMET metrics), and schedules downstream molecular interactions.

## Crates

| Crate | Description | Status |
|-------|-------------|--------|
| `ferrumyx-embed` | BiomedBERT embeddings via Candle (768-dim) | ✅ Working |
| `ferrumyx-ner` | Fast biomedical NER via Aho-Corasick dictionary matching | ✅ Working |
| `ferrumyx-ingestion` | Literature pipeline (PubMed, chunking, dedup) | ✅ Working |
| `ferrumyx-kg` | Knowledge graph & target scoring | ✅ Working |
| `ferrumyx-ranker` | Target prioritization with DepMap CRISPR | ✅ Working |
| `ferrumyx-agent` | IronClaw agent with tools (Primary Event Loop) | ✅ Working |
| `ferrumyx-llm` | LLM abstraction layer (Ollama) | ✅ Working |
| `ferrumyx-common` | Shared utilities | ✅ Working |
| `ferrumyx-web` | Web API & dashboard | ✅ Working |

## Quick Start

```bash
# Set Protobuf compiler path (required for LanceDB)
# Windows:
set PROTOC=C:\protoc\bin\protoc.exe
# Linux/macOS:
# export PROTOC=/usr/bin/protoc

# Windows easy start (Installs Rust, Ollama, selects model, and runs)
.\start.ps1

# Linux/macOS easy start
./start.sh

# Manual run tests
cargo test --workspace

# Manual start agent / web server
cargo run --release --bin ferrumyx
```

## License

Apache-2.0 OR MIT
