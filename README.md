# Ferrumyx

<div align="center">
  <img src="crates/ferrumyx-web/static/logo.svg" alt="Ferrumyx Logo" width="200"/>
</div>

<div align="center">
  <a href="https://colab.research.google.com/github/Classacre/ferrumyx/blob/main/ferrumyx_colab.ipynb">
    <img src="https://colab.research.google.com/assets/colab-badge.svg" alt="Open In Colab"/>
  </a>
</div>

**Open-source autonomous oncology discovery system built with IronClaw and BioClaw.**

Ferrumyx is an agentic platform for literature-driven target discovery and downstream molecular exploration. It combines autonomous ingestion, biomedical extraction, graph-backed evidence modeling, target ranking, and conversational bioinformatics workflows in a secure, multi-channel Rust architecture powered by IronClaw's enterprise agent framework and BioClaw's bioinformatics methodology.

## What Ferrumyx Does

- Ingests and deduplicates biomedical literature from multiple sources with autonomous scheduling.
- Extracts entities and evidence relations from text using BioClaw-inspired skills.
- Builds a queryable evidence graph backed by PostgreSQL + pgvector storage.
- Produces ranked target outputs using multi-signal scoring and conversational workflows.
- Supports downstream molecular steps (structure, pockets, ligand/docking flow) in secure containers.
- Exposes interactive workflows through multi-channel interfaces (WhatsApp, Slack, web, Discord).
- Implements enterprise-grade security with WASM sandboxing, encrypted secrets, and audit logging.
- Supports federated package export/validation/signing/sync for shared knowledge distribution.

## Implementation Highlights

- **IronClaw-powered architecture:** Built on NearAI's enterprise agent framework with defense-in-depth security, WASM sandboxing, and multi-channel support.
- **BioClaw-inspired workflows:** 25+ pre-built bioinformatics skills for conversational oncology research (PubMed search, BLAST, PyMOL, FastQC, etc.).
- **Multi-channel orchestration:** Natural language interaction via WhatsApp, Slack, Discord, web chat, and programmatic APIs.
- **Enterprise-grade security:** AES-256-GCM encrypted secrets, capability-based permissions, leak detection, and comprehensive audit logging.
- **PostgreSQL + pgvector storage:** Production-ready vector database with pgvector for embeddings and JSONB for flexible metadata.
- **Container orchestration:** Docker-based execution for bioinformatics tools with resource limits and isolation.
- **Autonomous discovery cycles:** Self-repairing agent loops with scheduled routines, event triggers, and background monitoring.
- **Performance monitoring:** Real-time dashboards, Prometheus metrics, and bottleneck identification.

## Repository Layout

| Crate | Purpose |
|---|---|
| `crates/ferrumyx-agent` | IronClaw agent orchestration, tool registration, autonomous discovery cycles |
| `crates/ferrumyx-ingestion` | Literature ingestion, chunking, full-text flow, embeddings with job scheduling |
| `crates/ferrumyx-kg` | Entity/relation extraction, KG update and scoring with BioClaw skills |
| `crates/ferrumyx-ranker` | Target ranking and provider-backed enrichment logic |
| `crates/ferrumyx-molecules` | Structure/pocket/ligand/docking pipeline in secure containers |
| `crates/ferrumyx-db` | PostgreSQL + pgvector schema, repositories, and federation persistence |
| `crates/ferrumyx-web` | Axum web UI with multi-channel gateway and monitoring dashboard |
| `crates/ferrumyx-runtime` | IronClaw runtime adapter layer with WASM tool support |
| `crates/ferrumyx-runtime-core` | Shared runtime-core infrastructure with Docker orchestration |
| `crates/ferrumyx-common` | Shared schema/types and cross-crate contracts |
| `crates/ferrumyx-monitoring` | Performance metrics, health checks, and Prometheus integration |
| `data/skills/` | BioClaw-inspired bioinformatics skill definitions |
| `docker/` | Container orchestration for bioinformatics tools |
| `channels-src/` | WASM-based multi-channel implementations |
| `tests/e2e/` | End-to-end testing suite with oncology workflows |

## Interfaces

- **Agent runtime:** `cargo run --release --bin ferrumyx`
- **Web app/API:** `cargo run -p ferrumyx-web`
- **Multi-channel chat:** WhatsApp, Slack, Discord, Telegram (via WASM plugins)
- **Monitoring dashboard:** Real-time performance metrics at `/monitoring`
- **REST API:** Full programmatic access to all workflows

## Quick Start

### Prerequisites
- Docker and Docker Compose (recommended for full functionality)
- PostgreSQL with pgvector extension
- For Windows: Visual Studio Build Tools 2022 with C++ workload

### Docker Setup (Recommended)
```bash
# Clone repository
git clone https://github.com/Classacre/ferrumyx.git
cd ferrumyx

# Start services with Docker Compose
docker-compose up -d

# Run database migrations
docker-compose exec ferrumyx-web bash scripts/db-migrate.sh

# Access web interface
open http://localhost:3000
```

### Manual Setup
```powershell
# Prerequisites: PostgreSQL with pgvector extension, Docker for bioinformatics tool containers (optional)

# Windows-specific build requirements:
# - Visual Studio Build Tools 2022 with C++ workload (for SQLite C dependencies)
#   Download: https://aka.ms/vs/17/release/vs_BuildTools.exe
#   Install with: .\vs_BuildTools.exe --nocache --wait --noUpdateInstaller --noWeb --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --includeOptional --quiet --norestart

# Optional (Windows): ensure protoc is available for build dependencies
$env:PROTOC = "C:\protoc\bin\protoc.exe"
```

## Documentation

### User Guides
- [Getting Started Guide](USER_GUIDE.md) - Complete user guide for researchers and basic usage
- [Developer Guide](DEVELOPER_GUIDE.md) - Contributing, development setup, and coding guidelines
- [Operations Guide](OPERATIONS_GUIDE.md) - Monitoring, maintenance, and backup procedures

### Technical Documentation
- [API Reference](API_REFERENCE.md) - Complete REST API documentation with examples
- [Configuration Reference](github-wiki/Configuration-and-Tunable-Parameters.md) - All settings and parameters
- [High-level Architecture](ARCHITECTURE.md) - IronClaw/BioClaw implementation details
- [Implementation Wiki](docs/WIKI.md) - In-depth technical implementation details

### Deployment & Operations
- [Deployment Guide](DEPLOYMENT.md) - Step-by-step deployment for all environments
- [Troubleshooting Guide](TROUBLESHOOTING.md) - Common issues, error codes, and diagnostic procedures
- [Security & Compliance](docs/COMPLIANCE.md) - HIPAA compliance and security procedures

### Development Resources
- [BioClaw Skills](data/skills/README.md) - Bioinformatics skill definitions
- [Multi-channel Setup](channels-src/README.md) - WhatsApp, Slack, Discord integration
- [Web UI Guide](WEBUI_README.md) - Web interface usage and features

## License

Apache-2.0 OR MIT
