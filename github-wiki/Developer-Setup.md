# Development Setup

This guide covers setting up your development environment for contributing to Ferrumyx.

## Prerequisites

- **Rust 1.70+** (`rustup` recommended for toolchain management)
- **Docker and Docker Compose** (for containerized services)
- **PostgreSQL 15+ with pgvector extension** (for vector database functionality)
- **Node.js 18+** (for web UI development)
- **Git** (for version control)

## Automated Setup

For a quick start, use the automated setup script:

```bash
# Clone repository
git clone https://github.com/Classacre/ferrumyx.git
cd ferrumyx

# Run development setup
bash scripts/dev-setup.sh

# Verify installation
cargo check --workspace
npm test
```

## Manual Setup

If you prefer manual installation:

### Rust Toolchain
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Install development tools
cargo install cargo-watch
cargo install cargo-nextest
cargo install cargo-audit
```

### Database Setup
```bash
# PostgreSQL with pgvector
# See DEPLOYMENT.md for detailed database setup instructions
```

### Node.js Dependencies
```bash
# Install web UI dependencies
npm install
```

## Repository Structure

```
ferrumyx/
├── crates/                    # Rust workspace crates
│   ├── ferrumyx-agent/       # IronClaw agent orchestration
│   ├── ferrumyx-ingestion/   # Literature ingestion pipeline
│   ├── ferrumyx-kg/          # Knowledge graph construction
│   ├── ferrumyx-ranker/      # Target ranking and scoring
│   ├── ferrumyx-molecules/   # Molecular analysis tools
│   ├── ferrumyx-db/          # Database layer
│   ├── ferrumyx-web/         # Web interface and API
│   └── ferrumyx-common/      # Shared types and utilities
├── channels-src/             # WASM-based multi-channel implementations
├── data/skills/              # BioClaw-inspired bioinformatics skills
├── docker/                   # Container definitions
├── docs/                     # Documentation
├── scripts/                  # Development and deployment scripts
├── tests/                    # Integration and end-to-end tests
└── migrations/               # Database schema migrations
```

## Verification

After setup, verify your environment:

```bash
# Check Rust installation
cargo --version
rustc --version

# Check workspace compilation
cargo check --workspace

# Check web UI
npm test

# Run basic tests
cargo test --lib
```

## Getting Help

If you encounter issues during setup:
- Check the [troubleshooting guide](../operations/troubleshooting.md)
- Review the [contributing guide](./contributing.md) for detailed workflow
- Ask in the development chat or create an issue