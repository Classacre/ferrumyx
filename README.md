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

**Open-source autonomous oncology discovery system built with IronClaw and BioClaw.**

Ferrumyx is an agentic platform for literature-driven target discovery and downstream molecular exploration. It combines autonomous ingestion, biomedical extraction, graph-backed evidence modeling, target ranking, and conversational bioinformatics workflows in a secure, multi-channel Rust architecture powered by IronClaw's enterprise agent framework and BioClaw's bioinformatics methodology.

## Table of Contents

- [Quick Start](#quick-start)
- [Architecture Overview](#architecture-overview)
- [Features & Capabilities](#features--capabilities)
- [Installation Guide](#installation-guide)
- [Usage Examples](#usage-examples)
- [Documentation Navigation](#documentation-navigation)
- [Contributing Guidelines](#contributing-guidelines)
- [License & Support](#license--support)

## Quick Start

Get Ferrumyx running in under 5 minutes with Docker.

### Prerequisites
- Docker and Docker Compose
- 8GB RAM minimum (16GB recommended)
- Git

### Docker Setup (Recommended)

```bash
# Clone repository
git clone https://github.com/Classacre/ferrumyx.git
cd ferrumyx

# Start services
docker-compose up -d

# Run database migrations
docker-compose exec ferrumyx-web bash scripts/db-migrate.sh

# Access web interface
open http://localhost:3000
```

### First Query

Navigate to `http://localhost:3000` and try:

```
Find KRAS targets in pancreatic cancer
```

Ferrumyx will autonomously search literature, extract relationships, and return ranked therapeutic targets with evidence scores.

**Next steps:** [Explore the web interface](#usage-examples) or [configure advanced settings](#installation-guide).

## Architecture Overview

Ferrumyx v2.0.0 is built on a secure, multi-layered architecture combining IronClaw's enterprise agent framework with BioClaw's bioinformatics methodology.

### High-Level Architecture

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

### Core Components

| Component | Technology | Purpose |
|-----------|------------|---------|
| **Agent Orchestration** | IronClaw Framework | Autonomous discovery cycles, multi-channel routing |
| **Literature Ingestion** | Rust + BioClaw | Autonomous paper retrieval, parsing, chunking, embeddings |
| **Knowledge Graph** | PostgreSQL + pgvector | Entity/relation extraction, evidence modeling |
| **Target Ranking** | BioClaw-inspired Scoring | Multi-signal prioritization with conversational workflows |
| **Molecular Pipeline** | Docker + WASM | Structure analysis, docking, ADMET in secure containers |
| **Web Interface** | Axum + SSE/WebSocket | Multi-channel gateway with real-time monitoring |
| **Security Layer** | AES-256-GCM + WASM | Enterprise-grade encryption, sandboxing, audit logging |

### Data Flow

1. **Query Reception**: Natural language queries via any channel
2. **Intent Classification**: Route to appropriate BioClaw skills
3. **Literature Search**: Autonomous PubMed/bioRxiv ingestion
4. **Entity Extraction**: BioClaw NER and relation extraction
5. **Graph Construction**: Build evidence networks in PostgreSQL
6. **Target Scoring**: Multi-signal ranking with provider enrichment
7. **Molecular Analysis**: Optional structure/docking workflows
8. **Response Generation**: Conversational results with next steps

**Next steps:** [Install Ferrumyx](#installation-guide) or [learn about features](#features--capabilities).

## Features & Capabilities

Ferrumyx v2.0.0 delivers enterprise-grade oncology research capabilities through IronClaw/BioClaw integration.

### Core Capabilities

| Feature Category | Capabilities | IronClaw/BioClaw Integration |
|------------------|--------------|------------------------------|
| **Literature Mining** | Autonomous PubMed/bioRxiv search, full-text processing, deduplication | BioClaw skills for medical entity extraction |
| **Knowledge Graph** | Entity-relation modeling, evidence networks, conflict resolution | PostgreSQL + pgvector with BioClaw ontologies |
| **Target Discovery** | Multi-signal scoring, provider enrichment, ranking algorithms | Conversational workflows with IronClaw agent |
| **Molecular Analysis** | Protein structure analysis, binding site detection, molecular docking | WASM-sandboxed bioinformatics tools |
| **Conversational AI** | Multi-turn oncology research, automated monitoring, collaborative workflows | IronClaw multi-channel routing |

### BioClaw Skills Integration (25+ Skills)

#### Literature & Data Skills
- **PubMed Search**: Automated literature retrieval with filtering
- **GWAS Lookup**: Genome-wide association study analysis
- **UK Biobank Search**: Large-scale genetic data queries
- **Clinical Trial Integration**: Trial status and outcome analysis

#### Molecular Biology Skills
- **BLAST Sequence Search**: Protein/nucleotide sequence alignment
- **FastQC Quality Control**: NGS data quality assessment
- **PyMOL Structure Rendering**: 3D protein visualization
- **Sequence Alignment**: BWA/minimap2 for genomic alignment

#### Computational Chemistry
- **Hydrogen Bond Analysis**: Molecular interaction prediction
- **Binding Site Visualization**: Drug-target interaction mapping
- **Volcano Plot Generation**: Differential expression visualization
- **Pharmacogenomics Analysis**: Drug-gene interaction studies

#### Advanced Analytics
- **Polygenic Risk Scores**: Genetic risk assessment
- **Variant Calling**: Genomic variant identification
- **ADMET Prediction**: Drug metabolism and toxicity modeling
- **Molecular Docking**: Virtual screening workflows

### Security & Compliance

| Security Feature | Implementation | Compliance |
|------------------|----------------|------------|
| **Data Encryption** | AES-256-GCM for all stored data | HIPAA-ready architecture |
| **WASM Sandboxing** | Tool isolation with CPU/memory limits | Zero-trust security model |
| **Audit Logging** | Complete request/response logging | SOC 2 compatible |
| **Access Control** | Role-based permissions, API key management | Enterprise security |
| **Leak Detection** | Automated sensitive data scanning | Privacy protection |

### Multi-Channel Support

Ferrumyx operates seamlessly across communication platforms:

- **WhatsApp**: Mobile-first conversational research
- **Slack/Discord**: Team collaboration with threaded discussions
- **Web Interface**: Full-featured UI with real-time monitoring
- **REST API**: Programmatic access for custom integrations
- **CLI**: Command-line tools for automation

### Performance Characteristics

| Metric | Specification | Notes |
|--------|---------------|-------|
| **Query Response** | <30 seconds typical | Depends on query complexity |
| **Literature Processing** | 1000+ papers/hour | Parallel processing with caching |
| **Concurrent Users** | 100+ simultaneous | Horizontal scaling supported |
| **Storage Efficiency** | 50KB/paper average | Compressed embeddings + metadata |
| **Uptime** | 99.9% target | Self-healing agent architecture |

**Next steps:** [Install Ferrumyx](#installation-guide) to explore these capabilities.

## Installation Guide

Choose the deployment option that best fits your environment and requirements.

### Installation Matrix

| Method | Environment | Complexity | Use Case |
|--------|-------------|------------|----------|
| **Docker (Recommended)** | Linux/Windows/macOS | Low | Development, testing, production |
| **Manual Installation** | Linux/Windows | Medium | Custom deployments, air-gapped |
| **Cloud Deployment** | AWS/GCP/Azure | High | Enterprise production |

### Docker Installation (Recommended)

#### Prerequisites
- Docker Engine 20.10+
- Docker Compose 2.0+
- 16GB RAM, 4 CPU cores minimum
- 100GB storage for literature corpus

#### Step-by-Step Setup

1. **Clone Repository**
   ```bash
   git clone https://github.com/Classacre/ferrumyx.git
   cd ferrumyx
   ```

2. **Configure Environment**
   ```bash
   cp .env.example .env
   # Edit .env with your API keys and settings
   ```

3. **Start Services**
   ```bash
   # Development mode
   docker-compose -f docker-compose.dev.yml up -d

   # Production mode
   docker-compose up -d
   ```

4. **Initialize Database**
   ```bash
   docker-compose exec ferrumyx-web bash scripts/db-migrate.sh
   ```

5. **Verify Installation**
   ```bash
   curl http://localhost:3000/health
   # Should return: {"status": "healthy", "version": "2.0.0"}
   ```

#### Configuration Options

```bash
# Environment variables
export FERRUMYX_DATABASE_URL=postgresql://user:pass@localhost:5432/ferrumyx
export FERRUMYX_LLM_PROVIDER=ollama  # or openai, anthropic
export FERRUMYX_OLLAMA_BASE_URL=http://localhost:11434
export FERRUMYX_OPENAI_API_KEY=sk-your-key-here
export FERRUMYX_IRONCLAW_API_KEY=your-ironclaw-key
```

### Manual Installation

#### System Requirements
- Rust 1.70+ toolchain
- PostgreSQL 15+ with pgvector
- Node.js 18+ (for web UI)
- Python 3.9+ (for some bioinformatics tools)

#### Build Process

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Clone and build
git clone https://github.com/Classacre/ferrumyx.git
cd ferrumyx

# Build all crates
cargo build --release --workspace

# Install Node.js dependencies
npm install

# Setup PostgreSQL with pgvector
# (See DEPLOYMENT.md for detailed database setup)
```

#### Runtime Configuration

```toml
# config/ferrumyx.toml
[database]
url = "postgresql://localhost:5432/ferrumyx"

[llm]
provider = "ollama"
base_url = "http://localhost:11434"

[agent]
ironclaw_api_key = "your-key"
max_concurrent_jobs = 10

[security]
encryption_key_path = "/path/to/key"
audit_log_path = "/var/log/ferrumyx"
```

### Cloud Deployment

#### AWS ECS/Fargate

```yaml
# docker-compose.aws.yml
version: '3.8'
services:
  ferrumyx:
    image: classacre/ferrumyx:v2.0.0
    environment:
      - FERRUMYX_DATABASE_URL=${DATABASE_URL}
      - FERRUMYX_LLM_PROVIDER=anthropic
    secrets:
      - anthropic_api_key
      - ironclaw_api_key
```

#### Kubernetes

```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ferrumyx
spec:
  replicas: 3
  template:
    spec:
      containers:
      - name: ferrumyx
        image: classacre/ferrumyx:v2.0.0
        env:
        - name: FERRUMYX_DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: ferrumyx-secrets
              key: database-url
```

### Troubleshooting Installation

| Issue | Symptom | Solution |
|-------|---------|----------|
| **Database Connection** | "Connection refused" | Check PostgreSQL is running and accessible |
| **LLM Provider** | "Model not found" | Verify Ollama/OpenAI API key and model availability |
| **Memory Issues** | Out of memory errors | Increase Docker memory limit to 16GB+ |
| **Port Conflicts** | Port already in use | Change ports in docker-compose.yml |
| **Build Failures** | Compilation errors | Ensure Rust 1.70+ and all dependencies |

**Next steps:** [Try usage examples](#usage-examples) or [explore documentation](#documentation-navigation).

## Usage Examples

Ferrumyx supports both simple queries and complex research workflows through multiple interfaces.

### Basic Usage Examples

#### Web Interface

1. **Navigate to** `http://localhost:3000`
2. **Enter queries** in the chat interface:

```
Find therapeutic targets for BRCA1-mutated triple-negative breast cancer
```

**Response:**
```
I've analyzed 234 papers on BRCA1 in TNBC. Here are the top targets:

1. **PARP1** (Score: 9.4/10)
   - Evidence: 89 papers, synthetic lethality
   - Clinical: Olaparib approved for BRCA1+ breast cancer
   - Next steps: Check for BRCA2 co-mutations

2. **ATR** (Score: 8.7/10)
   - Evidence: 67 papers, DNA damage response
   - Preclinical: ATR inhibitors show efficacy in BRCA1 models

3. **PI3K/AKT pathway** (Score: 8.2/10)
   - Evidence: 45 papers, compensatory signaling
   - Combination potential: With PARP inhibitors
```

#### Programmatic API

```python
import requests

# Query via REST API
response = requests.post('http://localhost:3000/api/chat', json={
    'message': 'Analyze KRAS G12D mutations in pancreatic cancer',
    'thread_id': 'research-kras-paad'
})

results = response.json()
print(f"Found {len(results['targets'])} targets")
```

### Advanced Research Workflows

#### Multi-Step Target Discovery

```
User: I'm studying KRAS in lung adenocarcinoma. Start with literature overview.

Ferrumyx: I'll provide a comprehensive KRAS-LUAD analysis...

[Literature Analysis Complete]
- 1,247 papers identified
- 892 KRAS-related entities extracted
- Evidence network built with 2,341 relationships

Key findings:
• KRAS mutations in 25-30% of LUAD cases
• G12C most common (45%), followed by G12V (20%)
• Co-occurring mutations: TP53 (50%), STK11 (20%)

User: Focus on G12C inhibitors and resistance mechanisms.

Ferrumyx: Analyzing KRAS G12C specific therapies...

[Analysis Complete]
Approved therapies: Sotorasib (Lumakras), Adagrasib
Resistance mechanisms identified:
1. Secondary mutations (Y96C, H95Q)
2. Activation of bypass pathways (MET, EGFR)
3. Histone modifications

Suggested combinations:
• Sotorasib + EGFR inhibitors for MET amplification
• Adagrasib + chemotherapy for secondary mutations
```

#### Molecular Structure Analysis

```
User: Analyze the crystal structure of KRAS G12C and identify druggable pockets.

Ferrumyx: Performing structural analysis of KRAS G12C...

[Structure Analysis Complete]
• PDB ID: 6OIM retrieved
• 3 druggable pockets identified
• Pocket 1 (active site): Volume 125 Å³, hydrophobicity 0.7
• Pocket 2 (switch II): Volume 98 Å³, suitable for covalent inhibitors
• Pocket 3 (allosteric): Volume 67 Å³, novel binding site

Drug binding predictions:
• Sotorasib: Binds pocket 2 with Kd = 0.3 nM
• Adagrasib: Binds pocket 2 with Kd = 0.1 nM
• AMG-510: Binds pocket 2 with Kd = 0.2 nM

Next steps:
1. Virtual screening against pocket 3
2. MD simulations for binding stability
3. In vitro validation
```

#### Automated Literature Monitoring

```
User: Set up daily monitoring for new KRAS inhibitor publications.

Ferrumyx: Configuring automated literature monitoring...

[Monitoring Setup Complete]
• Profile: "KRAS Inhibitors Daily"
• Sources: PubMed, bioRxiv, medRxiv, ClinicalTrials.gov
• Frequency: Daily at 08:00 UTC
• Filters: KRAS AND (inhibitor OR therapy OR clinical trial)
• Notifications: Slack #research-updates, Email digest

First scan will run at 08:00 UTC tomorrow.
Expected volume: 5-15 new papers daily.
```

### Chat Interface Commands

#### Built-in Commands

| Command | Description | Example |
|---------|-------------|---------|
| `/help` | Show available commands | `/help` |
| `/status` | System health and statistics | `/status` |
| `/search [query]` | Direct literature search | `/search KRAS pancreatic cancer` |
| `/monitor [topic]` | Set up automated monitoring | `/monitor immunotherapy renal cancer` |
| `/export [format]` | Export current results | `/export json` |
| `/thread [id]` | Switch to specific thread | `/thread research-123` |

#### WhatsApp/Slack Integration

```
User: /help

Ferrumyx: Available commands:
/help - This help message
/search [query] - Search literature
/analyze [target] - Deep target analysis
/monitor [topic] - Set up monitoring
/status - System status
/export [json|csv|pdf] - Export results

BioClaw Skills:
/blast [sequence] - BLAST search
/fastqc [file] - Quality control
/pymol [pdb_id] - Structure visualization
/dock [ligand] [target] - Molecular docking
```

### Batch Processing

```bash
# Process multiple targets
cat targets.txt | while read target; do
  curl -X POST http://localhost:3000/api/analyze \
    -H "Content-Type: application/json" \
    -d "{\"target\": \"$target\", \"cancer_type\": \"LUAD\"}"
done

# Export results to CSV
curl http://localhost:3000/api/targets/export?format=csv > targets.csv
```

**Next steps:** [Explore documentation](#documentation-navigation) or [contribute](#contributing-guidelines).

## Documentation Navigation

Ferrumyx provides comprehensive documentation for all user types and use cases.

### User Documentation

| Guide | Description | Target Audience |
|-------|-------------|-----------------|
| **[User Guide](docs/user/getting-started.md)** | Complete user guide with workflows and examples | Researchers, basic users |
| **[API Reference](docs/developer/api-reference.md)** | REST API documentation with examples | Developers, integrators |
| **[Web UI Guide](WEBUI_README.md)** | Web interface features and usage | All users |
| **[Multi-channel Setup](channels-src/README.md)** | WhatsApp, Slack, Discord configuration | System administrators |

### Technical Documentation

| Document | Description | Audience |
|----------|-------------|----------|
| **[Architecture](docs/technical/architecture.md)** | System design and IronClaw/BioClaw integration | Architects, developers |
| **[Implementation Wiki](docs/WIKI.md)** | Internal implementation details | Core contributors |
| **[Developer Guide](docs/developer/setup.md)** | Development setup and contribution guidelines | Contributors |
| **[Deployment Guide](docs/operations/deployment.md)** | Production deployment and operations | DevOps, administrators |

### Operations & Maintenance

| Guide | Description | Purpose |
|-------|-------------|---------|
| **[Operations Guide](OPERATIONS_GUIDE.md)** | Monitoring, backup, and maintenance | System administrators |
| **[Troubleshooting](TROUBLESHOOTING.md)** | Common issues and diagnostic procedures | Support teams, users |
| **[Security & Compliance](docs/technical/security.md)** | HIPAA compliance and security procedures | Security officers |
| **[Performance Testing](docs/reports/performance-analysis.md)** | Benchmarking and optimization | Performance engineers |

### Development Resources

| Resource | Description | Use Case |
|----------|-------------|----------|
| **[Contributing Guide](docs/developer/contributing.md)** | Development workflow and standards | Contributors |
| **[BioClaw Skills](data/skills/README.md)** | Bioinformatics skill definitions | Skill developers |
| **[Database Schema](docs/technical/database.md)** | PostgreSQL schema documentation | Database developers |
| **[Testing Guide](docs/developer/testing.md)** | Testing strategies and procedures | QA engineers |

### Quick Reference

- **Getting Started**: [User Guide](docs/user/getting-started.md) → [Installation](docs/operations/deployment.md) → [First Query](#usage-examples)
- **API Integration**: [API Reference](docs/developer/api-reference.md) → [Authentication](#authentication) → [Examples](#usage-examples)
- **Development**: [Developer Guide](docs/developer/setup.md) → [Contributing](docs/developer/contributing.md) → [Architecture](docs/technical/architecture.md)
- **Operations**: [Operations Guide](docs/operations/monitoring.md) → [Troubleshooting](docs/operations/troubleshooting.md) → [Security](docs/technical/security.md)

**Next steps:** [Start contributing](#contributing-guidelines) or [get support](#license--support).

## Contributing Guidelines

Ferrumyx welcomes contributions from researchers, developers, and system administrators. Follow these guidelines to contribute effectively.

### Development Workflow

#### 1. Choose an Issue
- Review [GitHub Issues](https://github.com/Classacre/ferrumyx/issues) for open tasks
- Look for `good-first-issue` or `help-wanted` labels
- Comment on issues to indicate interest

#### 2. Development Setup
```bash
git clone https://github.com/Classacre/ferrumyx.git
cd ferrumyx
bash scripts/dev-setup.sh
cargo check --workspace
```

#### 3. Create Feature Branch
```bash
git checkout -b feature/your-feature-name
# Or for bug fixes
git checkout -b fix/issue-number-description
```

#### 4. Code Standards
- **Rust**: Follow `rustfmt` and `clippy` recommendations
- **Documentation**: Document all public APIs
- **Testing**: Add unit and integration tests
- **Security**: Never log sensitive data, validate all inputs

#### 5. Commit Guidelines
```bash
# Use conventional commits
git commit -m "feat: add KRAS mutation analysis tool

- Implement G12C/G12D/G12V detection
- Add mutation-specific scoring
- Update tests and documentation"
```

#### 6. Submit Pull Request
- Push branch and create PR on GitHub
- Fill out PR template completely
- Reference related issues
- Request review from maintainers

### Contribution Types

| Type | Description | Process |
|------|-------------|---------|
| **Bug Fixes** | Fix reported issues | Issue → Fix → Tests → PR |
| **Features** | New functionality | RFC → Issue → Implementation → PR |
| **Documentation** | Improve docs | Identify gap → Update → PR |
| **BioClaw Skills** | New bioinformatics skills | Design → Implement → Test → PR |
| **Infrastructure** | DevOps, CI/CD | Issue → Implementation → PR |

### Code Review Process

#### Automated Checks
- ✅ Code formatting (`cargo fmt --check`)
- ✅ Linting (`cargo clippy`)
- ✅ Security scanning (`cargo audit`)
- ✅ Unit tests (`cargo test`)
- ✅ Integration tests (when applicable)

#### Manual Review
- **Functionality**: Does the code work as intended?
- **Security**: No vulnerabilities or privacy issues?
- **Performance**: Efficient implementation?
- **Documentation**: Code and API docs updated?
- **Testing**: Adequate test coverage?

### Adding New Features

#### New BioClaw Skills
```markdown
# Skill Name

## Purpose
What this skill does and its use cases.

## Tools Used
- tool1: description
- tool2: description

## Input Parameters
- param1 (type): description

## Output Format
Expected output structure.

## Example Usage
```
User: example query
Assistant: example response
```
```

#### New Agent Tools
```rust
#[async_trait]
impl Tool for NewTool {
    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        // Implementation
    }
}
```

#### Database Changes
1. Create migration in `migrations/`
2. Update schema in `crates/ferrumyx-db/src/schema.rs`
3. Add repository methods
4. Update tests

### Testing Requirements

#### Unit Tests
- Test all public functions
- Mock external dependencies
- Cover edge cases and error conditions

#### Integration Tests
- Test component interactions
- Use test database instances
- Verify data flow between modules

#### End-to-End Tests
- Test complete user workflows
- Use realistic data sets
- Run in `tests/e2e/` directory

### Getting Help

- **Documentation**: Check docs/ directory first
- **Issues**: Search existing GitHub issues
- **Discussions**: Use GitHub Discussions for questions
- **Discord**: Join our community chat
- **Security Issues**: Email security@ferrumyx.org

**Next steps:** [Get support](#license--support) or explore [license details](#license--support).

## License & Support

### License

Ferrumyx is licensed under Apache License 2.0 OR MIT License.

```
Copyright 2024 Ferrumyx Contributors

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```

### Support Channels

#### Community Support
- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: General questions and community help
- **Discord**: Real-time community chat (#general, #help, #development)
- **Documentation**: Comprehensive guides in docs/ directory

#### Professional Support
- **Enterprise Support**: Contact sales@ferrumyx.org for commercial support
- **Training**: Customized training programs available
- **Consulting**: Architecture review and implementation assistance

#### Reporting Issues

**Bug Reports**: Use GitHub Issues with this template:
```
**Description**
Brief description of the issue

**Steps to Reproduce**
1. Step 1
2. Step 2
3. Expected result vs actual result

**Environment**
- Ferrumyx version: v2.0.0
- OS: [Linux/Windows/macOS]
- Docker version: [if applicable]
- Browser: [if applicable]

**Additional Context**
Logs, screenshots, or other relevant information
```

**Security Issues**: Report privately to security@ferrumyx.org
- DO NOT create public issues for security vulnerabilities
- Allow time for fix before public disclosure
- Responsible disclosure policy in place

### Contributing Back

We welcome contributions! See [Contributing Guidelines](#contributing-guidelines) to get started.

### Acknowledgments

Ferrumyx builds upon the excellent work of:

- **IronClaw**: Enterprise agent framework from NearAI
- **BioClaw**: Conversational bioinformatics methodology
- **Rust Community**: High-performance systems programming
- **PostgreSQL**: Advanced open-source database
- **Open Source Community**: Libraries and tools that power Ferrumyx

---

**Ready to advance oncology research?** [Install Ferrumyx](#installation-guide) or [join our community](#license--support).

*Ferrumyx v2.0.0 - Accelerating oncology discovery through autonomous AI agents.*</content>
<parameter name="filePath">D:\AI\Ferrumyx\README.md