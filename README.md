# Ferrumyx v2.0.0

<div align="center">
  <img src="crates/ferrumyx-web/static/logo.svg" alt="Ferrumyx Logo" width="200"/>
</div>

<div align="center">
  <a href="https://colab.research.google.com/github/Classacre/ferrumyx/blob/main/ferrumyx_colab.ipynb">
    <img src="https://colab.research.google.com/assets/colab-badge.svg" alt="Open In Colab"/>
  </a>
  <a href="https://github.com/Classacre/ferrumyx/actions/workflows/ci.yml">
    <img src="https://github.com/Classacre/ferrumyx/actions/workflows/ci.yml/badge.svg" alt="CI Status"/>
  </a>
  <a href="https://crates.io/crates/ferrumyx">
    <img src="https://img.shields.io/crates/v/ferrumyx.svg" alt="Crates.io"/>
  </a>
  <a href="https://github.com/Classacre/ferrumyx/blob/main/LICENSE">
    <img src="https://img.shields.io/github/license/Classacre/ferrumyx.svg" alt="License"/>
  </a>
  <a href="https://discord.gg/ferrumyx">
    <img src="https://img.shields.io/discord/123456789.svg?label=Discord&logo=discord" alt="Discord"/>
  </a>
</div>

## 📖 Wiki as Source of Truth

**For comprehensive documentation, visit our [GitHub Wiki](https://github.com/Classacre/ferrumyx/wiki)** - the definitive source for all Ferrumyx guides, tutorials, and technical details.

**Open-source autonomous oncology discovery system built with IronClaw and BioClaw.**

Ferrumyx is an agentic platform for literature-driven target discovery and downstream molecular exploration. It combines autonomous ingestion, biomedical extraction, graph-backed evidence modeling, target ranking, and conversational bioinformatics workflows in a secure, multi-channel Rust architecture powered by IronClaw's enterprise agent framework and BioClaw's bioinformatics methodology.

## Table of Contents

- [Quick Start](#quick-start)
- [Navigation Hub](#navigation-hub)
- [Developer Resources](#developer-resources)
- [Operations](#operations)
- [Community](#community)

## Quick Start

Get Ferrumyx running in under 5 minutes with Docker.

### Prerequisites
- Docker and Docker Compose
- 8GB RAM minimum (16GB recommended)
- Git

### Docker Setup

```bash
# Clone repository
git clone https://github.com/Classacre/ferrumyx.git
cd ferrumyx

# Run complete setup (configuration + infrastructure)
ferrumyx-setup setup --environment development

# Access web interface
open http://localhost:3000
```

### First Query

Navigate to `http://localhost:3000` and try:

```
Find KRAS targets in pancreatic cancer
```

Ferrumyx will autonomously search literature, extract relationships, and return ranked therapeutic targets with evidence scores.

**📖 For detailed setup guides:** [Installation Wiki](https://github.com/Classacre/ferrumyx/wiki/Operations-Deployment) | [User Getting Started](https://github.com/Classacre/ferrumyx/wiki/User-Getting-Started)

## Navigation Hub

Explore Ferrumyx's comprehensive capabilities through our organized wiki sections:

### 🚀 Getting Started Pathways

| User Type | Quick Path | Wiki Entry Point |
|-----------|------------|------------------|
| **Researchers** | Literature search → Target discovery → Molecular analysis | [User Getting Started](https://github.com/Classacre/ferrumyx/wiki/User-Getting-Started) |
| **Developers** | API integration → Web UI → Multi-channel setup | [Developer API Reference](https://github.com/Classacre/ferrumyx/wiki/Developer-API-Reference) |
| **Administrators** | Installation → Configuration → Security setup | [Operations Deployment](https://github.com/Classacre/ferrumyx/wiki/Operations-Deployment) |
| **Contributors** | Development setup → Contributing guidelines → Architecture | [Developer Setup](https://github.com/Classacre/ferrumyx/wiki/Developer-Setup) |

### 📚 Core Documentation Sections

| Section | Description | Key Wiki Pages |
|---------|-------------|---------------|
| **User Guides** | Research workflows, interface usage, examples | [Web UI Guide](https://github.com/Classacre/ferrumyx/wiki/WebUI-Readme), [Multi-Channel Usage](https://github.com/Classacre/ferrumyx/wiki/User-Multi-Channel-Usage) |
| **Technical Architecture** | System design, components, data flow | [Technical Architecture](https://github.com/Classacre/ferrumyx/wiki/Technical-Architecture), [Implementation Details](https://github.com/Classacre/ferrumyx/wiki/Wiki-Implementation-Details) |
| **Bioinformatics Features** | Skills, tools, molecular analysis capabilities | [BioClaw Skills](https://github.com/Classacre/ferrumyx/wiki/BioClaw-Skills), [Molecular Analysis](https://github.com/Classacre/ferrumyx/wiki/Molecular-Analysis) |
| **Integration & APIs** | REST API, CLI, programmatic access | [Developer API Reference](https://github.com/Classacre/ferrumyx/wiki/Developer-API-Reference), [CLI Reference](https://github.com/Classacre/ferrumyx/wiki/CLI-Reference) |

### 🔍 Feature Overview

Ferrumyx delivers enterprise-grade oncology research through autonomous AI agents:

- **Literature Mining**: Autonomous PubMed/bioRxiv search with entity extraction
- **Knowledge Graph**: Evidence networks with multi-signal target ranking
- **Molecular Analysis**: Structure analysis, docking, and bioinformatics tools
- **Multi-Channel Access**: WhatsApp, Slack, Discord, Web UI, REST API, CLI
- **Security & Compliance**: HIPAA-ready with WASM sandboxing and encryption

**📖 Dive deeper:** [Features & Capabilities Wiki](https://github.com/Classacre/ferrumyx/wiki/Features-and-Capabilities)

## Developer Resources

### 🛠️ Development & Integration

| Resource | Description | Wiki Link |
|----------|-------------|-----------|
| **Developer Setup** | Environment setup, build process, dependencies | [Developer Setup](https://github.com/Classacre/ferrumyx/wiki/Developer-Setup) |
| **API Reference** | REST API endpoints, authentication, examples | [Developer API Reference](https://github.com/Classacre/ferrumyx/wiki/Developer-API-Reference) |
| **BioClaw Skills** | Bioinformatics skill development and integration | [BioClaw Skills](https://github.com/Classacre/ferrumyx/wiki/BioClaw-Skills) |
| **Database Schema** | PostgreSQL schema, migrations, data models | [Technical Database](https://github.com/Classacre/ferrumyx/wiki/Technical-Database) |
| **Testing Guide** | Unit tests, integration tests, QA processes | [Developer Testing](https://github.com/Classacre/ferrumyx/wiki/Developer-Testing) |

### 🚀 Contribution Pathways

- **New Contributors**: Start with [Developer Setup](https://github.com/Classacre/ferrumyx/wiki/Developer-Setup) → [Contributing Guide](https://github.com/Classacre/ferrumyx/wiki/Developer-Contributing)
- **Skill Developers**: Learn about [BioClaw Skills](https://github.com/Classacre/ferrumyx/wiki/BioClaw-Skills) and [Tool Development](https://github.com/Classacre/ferrumyx/wiki/Tool-Development)
- **Infrastructure**: See [Operations Deployment](https://github.com/Classacre/ferrumyx/wiki/Operations-Deployment) and [CI/CD](https://github.com/Classacre/ferrumyx/wiki/CI-CD)

## Operations

### ⚙️ Deployment & Maintenance

| Area | Description | Wiki Resources |
|------|-------------|----------------|
| **Installation** | Docker, manual, cloud deployment options | [Operations Deployment](https://github.com/Classacre/ferrumyx/wiki/Operations-Deployment) |
| **Configuration** | Environment variables, security settings | [Configuration Guide](https://github.com/Classacre/ferrumyx/wiki/Configuration-Guide) |
| **Monitoring** | Health checks, logging, performance metrics | [Operations Guide](https://github.com/Classacre/ferrumyx/wiki/Operations-Guide) |
| **Security** | HIPAA compliance, access control, encryption | [Technical Security](https://github.com/Classacre/ferrumyx/wiki/Technical-Security) |
| **Troubleshooting** | Common issues, diagnostics, support | [Troubleshooting](https://github.com/Classacre/ferrumyx/wiki/Troubleshooting) |

### 📊 Performance & Scaling

- **Benchmarking**: Performance testing and optimization guides
- **Scaling**: Horizontal scaling, load balancing, high availability
- **Backup & Recovery**: Data backup, disaster recovery procedures

**📖 All operations details:** [Operations Wiki](https://github.com/Classacre/ferrumyx/wiki/Operations-Guide)

## Community

### 🤝 Contributing & Support

| Channel | Purpose | Link/Details |
|---------|---------|--------------|
| **GitHub Issues** | Bug reports, feature requests | [GitHub Issues](https://github.com/Classacre/ferrumyx/issues) |
| **GitHub Discussions** | Community questions, general help | [GitHub Discussions](https://github.com/Classacre/ferrumyx/discussions) |
| **Discord** | Real-time chat, development updates | [Discord Server](https://discord.gg/ferrumyx) |
| **Contributing Guide** | Development workflow, standards | [Developer Contributing](https://github.com/Classacre/ferrumyx/wiki/Developer-Contributing) |
| **Security Issues** | Private vulnerability reporting | security@ferrumyx.org |

### 📚 Learning Resources

- **User Tutorials**: Step-by-step research workflows and examples
- **Video Guides**: Screencasts for common tasks and features
- **Case Studies**: Real-world oncology research applications
- **API Tutorials**: Integration examples and best practices

**📖 Join the community:** [Contributing Wiki](https://github.com/Classacre/ferrumyx/wiki/Developer-Contributing) | [Community Guidelines](https://github.com/Classacre/ferrumyx/wiki/Community-Guidelines)

### 📄 License

Ferrumyx is licensed under Apache License 2.0 OR MIT License. See [LICENSE](https://github.com/Classacre/ferrumyx/blob/main/LICENSE) for details.

---

**Ready to advance oncology research?** Explore the [Wiki](https://github.com/Classacre/ferrumyx/wiki) for comprehensive documentation or join our [Discord community](https://discord.gg/ferrumyx).

*Ferrumyx v2.0.0 - Accelerating oncology discovery through autonomous AI agents.*