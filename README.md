# Ferrumyx

**Autonomous Oncology Drug Discovery Engine**

Ferrumyx is an autonomous R&D engine built on [IronClaw](https://github.com/nearai/ironclaw) (Rust AI agent framework). It is **not a chatbot** — it is an internal scientific system that:

- Continuously ingests oncology literature (PubMed, Europe PMC, bioRxiv, ClinicalTrials.gov)
- Maintains a structured, evolving cancer knowledge graph (PostgreSQL + pgvector)
- Identifies and ranks promising therapeutic targets using multi-factor scoring
- Evaluates structural druggability (fpocket, AlphaFold, PDB)
- Conducts in silico molecular docking and ADMET prediction
- Learns from outcomes and improves target prioritisation over time

## Status

> **Phase 1 Complete.** Core infrastructure implemented. Phase 2 (Literature Ingestion) in progress.

### Implemented Crates

| Crate | Description | Status |
|-------|-------------|--------|
| `ferrumyx-embed` | PubMedBERT embeddings (768-dim, ~130ms inference) | ✅ Working |
| `ferrumyx-ingestion` | Literature pipeline (PubMed, chunking, dedup) | ✅ Scaffold |
| `ferrumyx-ranker` | Target scoring with DepMap CRISPR integration | ✅ Scaffold |
| `ferrumyx-kg` | Knowledge graph repository | ✅ Scaffold |
| `ferrumyx-agent` | IronClaw agent with tools (NER, KG, ranker) | ✅ Scaffold |
| `ferrumyx-llm` | LLM abstraction layer | ✅ Scaffold |
| `ferrumyx-common` | Shared utilities | ✅ Working |
| `ferrumyx-web` | Web API | ✅ Scaffold |

**Tests:** 27 passing across workspace

## Architecture

See [`ARCHITECTURE.md`](./ARCHITECTURE.md) for the full system design (all 9 phases).

## Phases

| Phase | Description | Status |
|---|---|---|
| 1 | System Architecture & IronClaw scaffold | ✅ Complete |
| 2 | Literature Ingestion Pipeline | 🔧 In Progress |
| 3 | Knowledge Graph & Target Intelligence | ⏳ Planned |
| 4 | Target Prioritization Engine | 🔧 Scaffold Ready |
| 5 | Structural Analysis & Molecule Design | ⏳ Planned |
| 6 | Autonomous Scientific Query Handling | ⏳ Planned |
| 7 | Self-Improvement Framework | ⏳ Planned |
| 8 | Security & LLM Strategy | ⏳ Planned |
| 9 | Roadmap | ⏳ Planned |

### Phase 2 Remaining Tasks

- [ ] PostgreSQL + pgvector database setup
- [ ] Database migrations (papers, chunks, embeddings, entities, kg_facts)
- [ ] Wire PubMed API → chunker → embedder → database
- [ ] Add Europe PMC, bioRxiv sources
- [ ] NER extraction pipeline

### Phase 4 Remaining Tasks

- [ ] Download DepMap CRISPR data
- [ ] Integrate COSMIC mutation data
- [ ] Add TCGA expression data
- [ ] Full weighted scoring pipeline

## MVP Scope

**Target:** KRAS G12D Pancreatic Ductal Adenocarcinoma (PDAC)
**Timeline:** 3-month MVP → 6-month expansion → 12-month autonomous optimisation

## Quick Start

```bash
# Run tests
cargo test --workspace

# Test embeddings
cargo run --package ferrumyx-embed --example test_embed --release
```

## Disclaimer

Ferrumyx is a research-grade computational hypothesis generation system. All outputs require expert wet-lab validation. Not intended for clinical use.

## License

TBD
