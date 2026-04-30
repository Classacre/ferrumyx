# Ferrumyx v2.0.0 Documentation Home

<div align="center">
  <img src="https://raw.githubusercontent.com/Classacre/ferrumyx/main/crates/ferrumyx-web/static/logo.svg" alt="Ferrumyx Logo" width="120"/>
</div>

**Ferrumyx v2.0.0** is an autonomous oncology discovery platform built with IronClaw and BioClaw frameworks. This comprehensive wiki provides complete documentation for users, developers, and administrators.

## 🚀 Quick Start

Get Ferrumyx running in under 5 minutes:

```bash
git clone https://github.com/Classacre/ferrumyx.git
cd ferrumyx
docker-compose up -d
open http://localhost:3000
```

Try: *"Find KRAS targets in pancreatic cancer"*

## 📚 Documentation Sections

### User Documentation
- **[Getting Started](Getting-Started)** - Installation, setup, and first queries
- **[User Guides](User-Guides)** - Research workflows, data analysis, and result interpretation
- **[API Reference](API-Reference)** - REST API documentation and examples
- **[CLI Reference](CLI-Reference)** - Command-line tools and usage

### Technical Documentation
- **[Architecture & Design](Architecture-&-Design)** - System design, components, and data flows
- **[Developer Documentation](Developer-Documentation)** - Development setup, APIs, and contribution guidelines
- **[Security & Compliance](Security-&-Compliance)** - HIPAA compliance, PHI handling, and audit procedures
- **[Performance & Scaling](Performance-&-Scaling)** - Optimization, GPU acceleration, and capacity planning

### Operations & Maintenance
- **[Operations Guide](Operations-Guide)** - Monitoring, deployment, and maintenance procedures
- **[Troubleshooting](Troubleshooting)** - Common issues, diagnostics, and support resources

## 🎯 Key Features v2.0.0

| Feature Category | Capabilities |
|------------------|--------------|
| **Literature Mining** | Autonomous PubMed/bioRxiv search, full-text processing, deduplication |
| **Knowledge Graph** | Entity-relation modeling, evidence networks, conflict resolution |
| **Target Discovery** | Multi-signal scoring, provider enrichment, ranking algorithms |
| **Molecular Analysis** | Structure analysis, binding site detection, molecular docking |
| **Conversational AI** | Multi-turn oncology research, automated monitoring, collaborative workflows |
| **Multi-Channel Support** | WhatsApp, Slack, Discord, Web Chat, REST API, CLI |

## 🔧 System Architecture

Ferrumyx v2.0.0 combines IronClaw's enterprise agent framework with BioClaw's bioinformatics methodology:

```
┌─────────────────────────────────────────────────────────────┐
│                    Multi-Channel Interface                   │
│  WhatsApp • Slack • Discord • Web Chat • REST API • CLI     │
├─────────────────────────────────────────────────────────────┤
│                   IronClaw Agent Core                       │
│  Agent Loop • Intent Router • Job Scheduler • Tool Registry │
├─────────────────────────────────────────────────────────────┤
│                 BioClaw Skills & Tools                      │
│  Literature Search • BLAST • PyMOL • FastQC • 25+ Skills    │
├─────────────────────────────────────────────────────────────┤
│                   Storage & Security                        │
│  PostgreSQL + pgvector • Encrypted Secrets • WASM Sandbox   │
├─────────────────────────────────────────────────────────────┤
│                 LLM Abstraction Layer                       │
│  Ollama • OpenAI • Anthropic • Data Classification Gates    │
└─────────────────────────────────────────────────────────────┘
```

## 📊 Performance Characteristics

- **Query Response**: <30 seconds typical
- **Literature Processing**: 1000+ papers/hour
- **Concurrent Users**: 100+ simultaneous
- **Storage Efficiency**: 50KB/paper average
- **Uptime**: 99.9% target

## 🔗 Quick Links

- **GitHub Repository**: [Classacre/ferrumyx](https://github.com/Classacre/ferrumyx)
- **Live Demo**: [Colab Notebook](https://colab.research.google.com/github/Classacre/ferrumyx/blob/main/ferrumyx_colab.ipynb)
- **API Documentation**: [API Reference](API-Reference)
- **Community**: [Discord](https://discord.gg/ferrumyx)

## 🆘 Need Help?

- **First Time?** → [Getting Started](Getting-Started)
- **API Integration?** → [API Reference](API-Reference)
- **Having Issues?** → [Troubleshooting](Troubleshooting)
- **Contributing?** → [Developer Documentation](Developer-Documentation)

## 📝 Recent Updates (v2.0.0)

- ✅ **IronClaw Integration**: Enterprise-grade agent orchestration
- ✅ **BioClaw Skills**: 25+ bioinformatics tools and workflows
- ✅ **Enhanced Security**: WASM sandboxing and PHI protection
- ✅ **Multi-Channel Support**: WhatsApp, Slack, Discord interfaces
- ✅ **GPU Acceleration**: CUDA/ROCm support for performance
- ✅ **Production Ready**: Comprehensive monitoring and alerting
- ✅ **Enhanced Documentation**: Advanced implementation details, code examples, and developer workflows

---

**Ready to advance oncology research?** Start with [Getting Started](Getting-Started) or explore the [API Reference](API-Reference).

*Ferrumyx v2.0.0 - Accelerating oncology discovery through autonomous AI agents.*
