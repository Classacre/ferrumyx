# Ferrumyx Developer Guide

## Contributing to Ferrumyx

Thank you for your interest in contributing to Ferrumyx! This guide provides comprehensive information for developers who want to contribute to the project.

### Code of Conduct

Ferrumyx follows a code of conduct that emphasizes:
- Respectful communication
- Collaborative development
- Security-first mindset
- Privacy protection
- Open source ethics

### Getting Started

#### Development Environment Setup

**Prerequisites:**
- Rust 1.70+ (`rustup` recommended)
- Docker and Docker Compose
- PostgreSQL 15+ with pgvector extension
- Node.js 18+ (for web UI development)
- Git

**Automated Setup:**
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

**Manual Setup:**
```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Install additional tools
cargo install cargo-watch
cargo install cargo-nextest
cargo install cargo-audit

# Setup PostgreSQL with pgvector
# (See DEPLOYMENT.md for detailed instructions)

# Install Node.js dependencies
npm install
```

#### Repository Structure

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

### Development Workflow

#### 1. Choose an Issue

- Check [GitHub Issues](https://github.com/Classacre/ferrumyx/issues) for open tasks
- Look for issues labeled `good-first-issue` or `help-wanted`
- Comment on the issue to indicate interest

#### 2. Create a Branch

```bash
# Create and switch to feature branch
git checkout -b feature/your-feature-name

# Or for bug fixes
git checkout -b fix/issue-number-description
```

#### 3. Development Process

```bash
# Run tests before starting
cargo test --workspace

# Make your changes
# ... edit code ...

# Run tests again
cargo test --workspace

# Format code
cargo fmt --all

# Run clippy
cargo clippy -- -D warnings

# Check for security issues
cargo audit
```

#### 4. Commit Guidelines

Follow conventional commit format:

```bash
# Feature commits
git commit -m "feat: add KRAS mutation analysis tool"

# Bug fixes
git commit -m "fix: resolve memory leak in ingestion pipeline"

# Documentation
git commit -m "docs: update API reference for ranking endpoints"

# Refactoring
git commit -m "refactor: simplify target scoring algorithm"
```

#### 5. Submit Pull Request

```bash
# Push your branch
git push origin feature/your-feature-name

# Create pull request on GitHub
# Include description of changes and link to issue
```

### Development Guidelines

#### Code Style

**Rust Code:**
- Follow `rustfmt` formatting
- Use `clippy` lints
- Prefer `async`/`await` for I/O operations
- Use strong typing and avoid `unwrap()` in production code
- Document public APIs with `///` comments

**Example:**
```rust
/// Represents a target with its scoring information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    /// Unique identifier for the target
    pub id: Uuid,
    /// Gene symbol (e.g., "KRAS")
    pub gene_symbol: String,
    /// Cancer type this target is relevant for
    pub cancer_type: String,
    /// Composite score from multiple evidence sources
    pub score: f64,
    /// Individual component scores
    pub component_scores: HashMap<String, f64>,
}
```

#### Security Considerations

**Never:**
- Log sensitive data (PHI, passwords, API keys)
- Store secrets in code or configuration files
- Expose internal system details in error messages
- Use insecure cryptographic primitives

**Always:**
- Use AES-256-GCM for encryption
- Implement proper access controls
- Validate all inputs
- Log security events appropriately

#### Testing

**Unit Tests:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_scoring() {
        let target = Target::new("KRAS", "PAAD");
        let score = score_target(&target).await;
        assert!(score > 0.0 && score <= 10.0);
    }
}
```

**Integration Tests:**
```rust
#[tokio::test]
async fn test_ingestion_pipeline() {
    let config = TestConfig::new();
    let ingestor = LiteratureIngestor::new(config).await;

    // Test full ingestion pipeline
    let result = ingestor.ingest_paper("test_paper.pdf").await;
    assert!(result.is_ok());
}
```

**End-to-End Tests:**
Located in `tests/e2e/` directory. Run with:
```bash
cargo test --test e2e
```

#### Documentation

**Code Documentation:**
- Document all public functions, structs, and modules
- Explain complex algorithms and data structures
- Provide usage examples in doc comments

**API Documentation:**
- Update API reference for new endpoints
- Include request/response examples
- Document error codes and handling

### Architecture Guidelines

#### Adding New Tools

1. **Create Tool Module:**
```rust
// crates/ferrumyx-agent/src/tools/new_tool.rs
use crate::tools::{Tool, ToolResult};

pub struct NewTool {
    config: NewToolConfig,
}

#[async_trait]
impl Tool for NewTool {
    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        // Implementation
    }
}
```

2. **Register Tool:**
```rust
// crates/ferrumyx-agent/src/tools/mod.rs
pub mod new_tool;

pub fn register_tools(registry: &mut ToolRegistry) {
    registry.register("new_tool", NewTool::new(config));
}
```

3. **Add Configuration:**
```rust
// crates/ferrumyx-agent/src/config/mod.rs
#[derive(Deserialize)]
pub struct AgentConfig {
    pub new_tool: NewToolConfig,
    // ... other configs
}
```

#### Adding New Skills

1. **Create Skill Definition:**
```markdown
# New Bioinformatics Skill

## Purpose
Describe what this skill does and its use cases.

## Tools Used
- tool1: description
- tool2: description

## Input Parameters
- param1: type and description
- param2: type and description

## Output Format
Description of expected output structure.

## Example Usage
```
User: example query
Assistant: example response
```
```

2. **Implement Skill Logic:**
```rust
// In appropriate crate (e.g., ferrumyx-molecules/src/skills/)
pub async fn execute_new_skill(params: SkillParams) -> Result<SkillResult> {
    // Implementation using existing tools
}
```

#### Database Changes

1. **Create Migration:**
```sql
-- migrations/001_add_new_table.sql
CREATE TABLE new_entities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_new_entities_name ON new_entities(name);
```

2. **Update Schema:**
```rust
// crates/ferrumyx-db/src/schema.rs
pub struct NewEntity {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}
```

3. **Add Repository Methods:**
```rust
// crates/ferrumyx-db/src/repositories/mod.rs
impl Repository {
    pub async fn create_new_entity(&self, name: &str) -> Result<NewEntity> {
        // Implementation
    }
}
```

### Testing Strategy

#### Unit Testing
- Test individual functions and methods
- Mock external dependencies
- Focus on edge cases and error conditions

#### Integration Testing
- Test component interactions
- Use test database instances
- Verify data flow between modules

#### End-to-End Testing
- Test complete user workflows
- Use realistic data sets
- Verify performance requirements

#### Performance Testing
```bash
# Run benchmarks
cargo bench

# Profile specific functions
cargo flamegraph --bin ferrumyx-agent -- test_function
```

### CI/CD Pipeline

#### GitHub Actions Workflow

The project uses GitHub Actions for:
- Code formatting checks (`cargo fmt --check`)
- Linting (`cargo clippy`)
- Security scanning (`cargo audit`)
- Testing (`cargo test`)
- Integration tests with Docker
- Release builds

#### Pre-commit Hooks

```bash
# Install pre-commit hooks
pip install pre-commit
pre-commit install

# Run manually
pre-commit run --all-files
```

### Debugging and Troubleshooting

#### Logging

Ferrumyx uses structured logging. Set log levels:

```bash
# Debug level for development
export RUST_LOG=ferrumyx=debug,tokio=info

# Trace level for detailed debugging
export RUST_LOG=ferrumyx=trace
```

#### Common Debugging Techniques

**Database Issues:**
```bash
# Check database connections
docker-compose exec postgres pg_stat_activity;

# View slow queries
docker-compose logs postgres | grep "duration:"
```

**Memory Leaks:**
```bash
# Use heap profiling
cargo build --release --features heap-profiling
valgrind --tool=massif ./target/release/ferrumyx-agent
```

**Performance Issues:**
```bash
# Profile with flamegraph
cargo flamegraph --bin ferrumyx-agent

# Benchmark specific functions
cargo bench --bench my_benchmark
```

### Security Testing

#### Automated Security Checks

```bash
# Dependency vulnerability scanning
cargo audit

# Fuzz testing
cargo +nightly fuzz run fuzz_target

# Static analysis
cargo clippy -- -W clippy::pedantic
```

#### Manual Security Review

- Review code for sensitive data handling
- Check authentication and authorization
- Verify encryption implementation
- Test input validation

### Performance Optimization

#### Profiling

```bash
# CPU profiling
cargo flamegraph --bin ferrumyx-agent

# Memory profiling
cargo build --release
valgrind --tool=massif ./target/release/ferrumyx-agent
```

#### Optimization Techniques

- Use async I/O for network operations
- Implement connection pooling
- Cache frequently accessed data
- Optimize database queries
- Use streaming for large data processing

### Release Process

#### Version Management

Ferrumyx follows semantic versioning:
- **MAJOR**: Breaking changes
- **MINOR**: New features
- **PATCH**: Bug fixes

#### Release Checklist

- [ ] Update version in `Cargo.toml`
- [ ] Update changelog
- [ ] Run full test suite
- [ ] Create release branch
- [ ] Tag release
- [ ] Build and publish Docker images
- [ ] Update documentation

### Getting Help

#### Documentation Resources

- [Architecture Documentation](ARCHITECTURE.md)
- [API Reference](API-Reference.md)
- [Troubleshooting Guide](TROUBLESHOOTING.md)
- [Security Guidelines](docs/COMPLIANCE.md)

#### Communication Channels

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: General questions and discussions
- **Security Issues**: security@ferrumyx.org (for security-related issues)

#### Code Review Process

1. **Automated Checks**: CI must pass all checks
2. **Peer Review**: At least one maintainer review required
3. **Security Review**: Security team review for sensitive changes
4. **Testing**: Adequate test coverage required
5. **Documentation**: Code and API documentation updated

Thank you for contributing to Ferrumyx! Your work helps advance oncology research through better software tools.</content>
<parameter name="filePath">D:\AI\Ferrumyx\DEVELOPER_GUIDE.md