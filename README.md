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

> **Pre-implementation.** Architecture design complete. Implementation beginning Phase 1.

## Architecture

See [`ARCHITECTURE.md`](./ARCHITECTURE.md) for the full system design (all 9 phases).

## Phases

| Phase | Description | Status |
|---|---|---|
| 1 | System Architecture & IronClaw scaffold | 🔧 In Progress |
| 2 | Literature Ingestion Pipeline | ⏳ Planned |
| 3 | Knowledge Graph & Target Intelligence | ⏳ Planned |
| 4 | Target Prioritization Engine | ⏳ Planned |
| 5 | Structural Analysis & Molecule Design | ⏳ Planned |
| 6 | Autonomous Scientific Query Handling | ⏳ Planned |
| 7 | Self-Improvement Framework | ⏳ Planned |
| 8 | Security & LLM Strategy | ⏳ Planned |
| 9 | Roadmap | ⏳ Planned |

## MVP Scope

**Target:** KRAS G12D Pancreatic Ductal Adenocarcinoma (PDAC)
**Timeline:** 3-month MVP → 6-month expansion → 12-month autonomous optimisation

## Disclaimer

Ferrumyx is a research-grade computational hypothesis generation system. All outputs require expert wet-lab validation. Not intended for clinical use.

## License

TBD
