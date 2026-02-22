# Ferrumyx

**Open-Source Autonomous Oncology Drug Discovery Engine**

Ferrumyx is an autonomous R&D engine built on [IronClaw](https://github.com/nearai/ironclaw) (Rust AI agent framework). It is **not a chatbot** — it is an internal scientific system that:

- Continuously ingests oncology literature (PubMed, Europe PMC, bioRxiv, ClinicalTrials.gov)
- Maintains a structured, evolving cancer knowledge graph (PostgreSQL + pgvector)
- Identifies and ranks promising therapeutic targets using multi-factor scoring
- Evaluates structural druggability (fpocket, AlphaFold, PDB)
- Conducts in silico molecular docking and ADMET prediction
- **Learns from outcomes and improves target prioritisation over time**

## Why Ferrumyx?

| Feature | Ferrumyx | PandaOmics | BenevolentAI | Other Open Source |
|---------|----------|------------|--------------|-------------------|
| **Open Source** | ✅ Apache/MIT | ❌ Proprietary | ❌ Proprietary | ✅ Various |
| **Self-Hosted** | ✅ Your data stays yours | ❌ Cloud only | ❌ Cloud only | ✅ Varies |
| **Autonomous Agent** | ✅ Runs itself | ❌ Manual tool | ❌ Manual tool | ❌ None |
| **Self-Improving** | ✅ Learns from outcomes | ❌ | ❌ | ❌ |
| **Knowledge Graph** | ✅ Biological KG | ✅ Biological KG | ✅ Biological KG | ❌ Fragmented |
| **Literature Mining** | ✅ PubMed, Europe PMC, bioRxiv | ✅ 47M publications | ✅ Limited | ❌ |
| **Dynamic Targets** | ✅ User-configurable | ❌ Fixed workflow | ❌ Fixed workflow | ❌ |
| **Security-First** | ✅ Rust + IronClaw | ❌ | ❌ | ❌ |
| **Cost** | **Free** | $199/mo (academic) | Enterprise only | Free |
| **Clinical Validation** | ⏳ In development | ✅ Phase II drugs | ✅ Phase II drugs | ❌ |

### What Makes Us Different

1. **Autonomous Agent** — Ferrumyx runs itself. Define a target, and it continuously ingests, analyzes, and prioritizes without human intervention.

2. **Self-Improving** — The system learns from outcomes (clinical trial results, publication retractions, new evidence) and adjusts its scoring weights automatically.

3. **Dynamic Targets** — Users define targets via YAML config or natural language. No hardcoded assumptions.

4. **Security-First Rust** — Built on IronClaw for defense-in-depth against prompt injection, data exfiltration, and malicious tools.

5. **Open Source** — Free forever. Inspect the code, modify algorithms, self-host on your infrastructure.

### Comparison with PandaOmics

[PandaOmics](https://pharma.ai/pandaomics) by Insilico Medicine is the closest commercial platform to Ferrumyx:

| Aspect | PandaOmics | Ferrumyx |
|--------|------------|----------|
| **Data Sources** | 1.3M omics samples, 47M publications, 5.5M patents | PubMed, Europe PMC, bioRxiv, DepMap, COSMIC, ChEMBL (extensible) |
| **Target Scoring** | Multi-modal AI (omics + text) | Multi-factor weighted scoring (user-configurable) |
| **Knowledge Graph** | LLM-powered biological KG | PostgreSQL + pgvector KG |
| **Pathway Analysis** | iPanda algorithm | Planned (Phase 3) |
| **Autonomy** | Manual operation | Autonomous agent |
| **Learning** | Static models | Self-improving feedback loop |
| **Pricing** | $199/mo (academic), enterprise for pharma | Free (open source) |
| **Validation** | TNIK inhibitor (Phase II), aging targets | In development |

**We're building an open-source, autonomous, self-improving alternative.**

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
