# Ferrumyx

**Open-Source Autonomous Oncology Drug Discovery Engine**

Ferrumyx is an autonomous R&D engine built on [IronClaw](https://github.com/nearai/ironclaw) (Rust AI agent framework). It is **not a chatbot** — it is an internal scientific system that:

- Continuously ingests oncology literature (PubMed, Europe PMC, bioRxiv, ClinicalTrials.gov)
- Maintains a structured, evolving cancer knowledge graph (LanceDB embedded)
- Identifies and ranks promising therapeutic targets using multi-factor scoring
- Evaluates structural druggability (fpocket, AlphaFold, PDB)
- Conducts in silico molecular docking and ADMET prediction
- **Learns from outcomes and improves target prioritisation over time**

## Current Status (Phase 2)

| Component | Status | Notes |
|-----------|--------|-------|
| **Ingestion** | ✅ Working | PubMed API, PDF parsing, chunking |
| **Embedding** | ✅ Working | Rust-native BiomedBERT (768-dim, Candle) |
| **NER** | ✅ Working | Rust-native Candle token classification |
| **KG Building** | ✅ Working | Fact extraction, scoring computation |
| **Deduplication** | ✅ Working | SimHash + PMID conflict resolution |
| **Web GUI** | ✅ Working | Dashboard, ingestion form, API endpoints |
| **Target Ranker** | 🔧 Scaffold | Multi-factor scoring ready |
| **Molecules** | ⏳ Phase 3 | Structural analysis pipeline |

**No Python dependencies.** All components are Rust-native.

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

## Why Ferrumyx?

| Feature | Ferrumyx | PandaOmics | BenevolentAI | Other Open Source |
|---------|----------|------------|--------------|-------------------|
| **Open Source** | ✅ Apache/MIT | ❌ Proprietary | ❌ Proprietary | ✅ Various |
| **Self-Hosted** | ✅ Your data stays yours | ❌ Cloud only | ❌ Cloud only | ✅ Varies |
| **Autonomous Agent** | ✅ Runs itself | ❌ Manual tool | ❌ Manual tool | ❌ None |
| **Self-Improving** | ✅ Learns from outcomes | ❌ | ❌ | ❌ |
| **Knowledge Graph** | ✅ Biological KG | ✅ Biological KG | ✅ Biological KG | ❌ Fragmented |
| **Literature Mining** | ✅ PubMed, Europe PMC, bioRxiv | ✅ 47M publications | ✅ Limited | ❌ |
| **No Python** | ✅ 100% Rust | ❌ | ❌ | ❌ |
| **Security-First** | ✅ Rust + IronClaw | ❌ | ❌ | ❌ |
| **Cost** | **Free** | $199/mo (academic) | Enterprise only | Free |

### What Makes Us Different

1. **Autonomous Agent** — Ferrumyx runs itself. Define a target, and it continuously ingests, analyzes, and prioritizes without human intervention.

2. **Self-Improving** — The system learns from outcomes (clinical trial results, publication retractions, new evidence) and adjusts its scoring weights automatically.

3. **100% Rust** — No Python dependencies, no Docker containers for ML services. Single binary deployment possible.

4. **Security-First Rust** — Built on IronClaw for defense-in-depth against prompt injection, data exfiltration, and malicious tools.

5. **Open Source** — Free forever. Inspect the code, modify algorithms, self-host on your infrastructure.

## Crates

| Crate | Description | Status |
|-------|-------------|--------|
| `ferrumyx-embed` | BiomedBERT embeddings via Candle (768-dim) | ✅ Working |
| `ferrumyx-ner` | Biomedical NER via Candle token classification | ✅ Working |
| `ferrumyx-ingestion` | Literature pipeline (PubMed, chunking, dedup) | ✅ Working |
| `ferrumyx-kg` | Knowledge graph & target scoring | ✅ Working |
| `ferrumyx-ranker` | Target prioritization with DepMap CRISPR | 🔧 Scaffold |
| `ferrumyx-agent` | IronClaw agent with tools | ✅ Working |
| `ferrumyx-llm` | LLM abstraction layer | ✅ Scaffold |
| `ferrumyx-common` | Shared utilities | ✅ Working |
| `ferrumyx-web` | Web API & dashboard | ✅ Working |

## Quick Start

```bash
# Start database
cd docker && docker compose up -d postgres

# Run migrations
cargo sqlx migrate run

# Run tests
cargo test --workspace

# Start web server
cargo run --release
```

## MVP Scope

**Target:** KRAS G12D Pancreatic Ductal Adenocarcinoma (PDAC)
**Timeline:** 3-month MVP → 6-month expansion → 12-month autonomous optimisation

## Disclaimer

Ferrumyx is a research-grade computational hypothesis generation system. All outputs require expert wet-lab validation. Not intended for clinical use.

## License

Apache-2.0 OR MIT
