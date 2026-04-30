# Developer Documentation

This comprehensive guide provides technical documentation for developers working with Ferrumyx v2.0.0. It covers development setup, architecture guidelines, contribution workflows, and best practices for extending the platform.

## Table of Contents

- [Getting Started](#getting-started)
- [Architecture Overview](#architecture-overview)
- [Development Workflow](#development-workflow)
- [Code Guidelines](#code-guidelines)
- [API Documentation](#api-documentation)
- [Testing Strategy](#testing-strategy)
- [Security Guidelines](#security-guidelines)
- [Performance Optimization](#performance-optimization)
- [Deployment](#deployment)

## Getting Started

### Prerequisites

- **Rust**: 1.70+ toolchain (`rustup` recommended)
- **Docker**: For containerized development and testing
- **PostgreSQL**: 15+ with pgvector extension
- **Node.js**: 18+ for web UI development
- **Git**: For version control

### Development Environment Setup

#### Automated Setup
```bash
# Clone repository
git clone https://github.com/Classacre/ferrumyx.git
cd ferrumyx

# Run development setup script
bash scripts/dev-setup.sh

# Verify installation
cargo check --workspace
npm test
```

#### Manual Setup
```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Install development tools
cargo install cargo-watch
cargo install cargo-nextest
cargo install cargo-audit

# Setup PostgreSQL with pgvector
# (See deployment documentation)

# Install Node.js dependencies
npm install
```

### Repository Structure

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

## Architecture Overview

### Core Components

Ferrumyx v2.0.0 is built on a modular architecture combining IronClaw's agent framework with BioClaw's bioinformatics capabilities.

#### Agent Orchestration (ferrumyx-agent)
- **Purpose**: Central coordination of autonomous research workflows
- **Key Features**: Multi-channel routing, job scheduling, context management
- **Integration**: IronClaw framework for enterprise-grade agent orchestration

#### Literature Ingestion (ferrumyx-ingestion)
- **Purpose**: Automated literature retrieval and processing
- **Capabilities**: PubMed, EuropePMC, bioRxiv integration
- **Features**: Full-text processing, chunking, embeddings generation

#### Knowledge Graph (ferrumyx-kg)
- **Purpose**: Entity-relation modeling and evidence networks
- **Technology**: PostgreSQL + pgvector for vector operations
- **Features**: Named entity recognition, confidence scoring, conflict resolution

#### Target Ranking (ferrumyx-ranker)
- **Purpose**: Multi-signal prioritization of therapeutic targets
- **Algorithms**: Composite scoring, evidence weighting, ranking pipelines
- **Output**: Ranked targets with confidence scores and evidence summaries

#### Molecular Analysis (ferrumyx-molecules)
- **Purpose**: Structure analysis and computational chemistry
- **Tools**: PyMOL, molecular docking, binding site detection
- **Security**: WASM sandboxing for tool execution

#### Database Layer (ferrumyx-db)
- **Purpose**: Data persistence and vector operations
- **Schema**: Optimized for literature storage and KG queries
- **Features**: Connection pooling, migrations, query optimization

#### Web Interface (ferrumyx-web)
- **Purpose**: Multi-channel API gateway and user interface
- **Technology**: Axum framework with WebSocket support
- **Features**: REST API, real-time updates, authentication

### Data Flow Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   User Query    │───▶│  Agent Router   │───▶│  Tool Execution │
│                 │    │  (IronClaw)     │    │  (BioClaw)      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
        │                        │                        │
        ▼                        ▼                        ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Literature     │───▶│  Knowledge      │───▶│  Target Scoring │
│  Ingestion      │    │  Graph          │    │  & Ranking      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
        │                        │                        │
        ▼                        ▼                        ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  PostgreSQL     │    │  Response       │    │  Multi-Channel  │
│  + pgvector     │    │  Generation     │    │  Output         │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Development Workflow

### Contributing Process

1. **Choose an Issue**: Review GitHub issues for open tasks
2. **Create Branch**: Use descriptive branch names (`feature/` or `fix/`)
3. **Development**: Follow coding guidelines and testing requirements
4. **Code Review**: Submit PR with comprehensive description
5. **Merge**: After approval and CI checks

### Branch Naming Convention
```bash
# Feature branches
git checkout -b feature/kras-mutation-analysis

# Bug fix branches
git checkout -b fix/memory-leak-ingestion

# Documentation branches
git checkout -b docs/api-reference-update
```

### Commit Guidelines

Follow conventional commit format:
```bash
# Feature commits
git commit -m "feat: add KRAS mutation analysis tool

- Implement G12C/G12D/G12V detection
- Add mutation-specific scoring
- Update tests and documentation"

# Bug fixes
git commit -m "fix: resolve memory leak in ingestion pipeline

- Fix connection pool exhaustion
- Add proper resource cleanup
- Update error handling"

# Documentation
git commit -m "docs: update API reference for ranking endpoints"
```

## Code Guidelines

### Rust Standards

#### Formatting and Linting
```bash
# Format code
cargo fmt --all

# Run linter
cargo clippy -- -D warnings

# Check for security issues
cargo audit
```

#### Code Style Principles
- Use `async`/`await` for I/O operations
- Prefer strong typing over `unwrap()`
- Document public APIs with `///` comments
- Implement proper error handling

#### Example Code Structure
```rust
/// Represents a therapeutic target with scoring information
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
    /// Supporting evidence references
    pub evidence: Vec<Evidence>,
}
```

### Security Guidelines

#### Data Protection
- **Never log**: PHI, passwords, API keys, or sensitive data
- **Always encrypt**: Use AES-256-GCM for data at rest
- **Validate inputs**: Implement comprehensive input validation
- **Access control**: Use role-based permissions

#### Secure Coding Practices
```rust
// ✅ Good: Proper error handling without data leakage
pub async fn process_user_data(user_id: Uuid) -> Result<UserData, AppError> {
    let data = self.db.get_user_data(user_id).await?;
    debug!("Processing data for user {}", user_id); // Safe: no sensitive data
    Ok(data)
}

// ❌ Bad: Logging sensitive information
error!("Failed to process data: {:?}", sensitive_data);
```

## API Documentation

### REST API Structure

Ferrumyx provides a comprehensive REST API for programmatic access.

#### Base URL
```
http://localhost:3000/api/v1
```

#### Authentication
```bash
# API key authentication
curl -H "X-API-Key: your-api-key" http://localhost:3000/api/v1/targets
```

#### Common Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/chat` | POST | Send conversational queries |
| `/targets` | GET | Retrieve ranked targets |
| `/literature` | GET | Search literature database |
| `/molecules` | POST | Molecular analysis requests |
| `/export` | GET | Export results in various formats |

#### Example API Usage
```python
import requests

# Chat query
response = requests.post('http://localhost:3000/api/v1/chat', json={
    'message': 'Find KRAS targets in pancreatic cancer',
    'thread_id': 'research-kras-paad'
})

results = response.json()
print(f"Found {len(results['targets'])} targets")
```

### WebSocket API

Real-time communication for live updates:
```javascript
const ws = new WebSocket('ws://localhost:3000/ws');

ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    console.log('Received:', data);
};

ws.send(JSON.stringify({
    type: 'chat',
    message: 'Monitor KRAS literature'
}));
```

## Testing Strategy

### Test Categories

#### Unit Tests
- Test individual functions and methods
- Mock external dependencies
- Focus on edge cases and error conditions

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_scoring() {
        let target = Target::new("KRAS", "PAAD");
        let score = score_target(&target);
        assert!(score > 0.0 && score <= 10.0);
    }

    #[tokio::test]
    async fn test_database_operations() {
        let repo = TestRepository::new().await;
        let target = repo.create_target("KRAS").await.unwrap();
        assert_eq!(target.gene_symbol, "KRAS");
    }
}
```

#### Integration Tests
- Test component interactions
- Use test database instances
- Verify data flow between modules

```rust
#[tokio::test]
async fn test_ingestion_pipeline() {
    let config = TestConfig::new();
    let ingestor = LiteratureIngestor::new(config).await;

    let result = ingestor.ingest_paper("test_paper.pdf").await;
    assert!(result.is_ok());
    assert!(result.unwrap().entities.len() > 0);
}
```

#### End-to-End Tests
- Test complete user workflows
- Use realistic data sets
- Verify performance requirements

```bash
# Run E2E tests
cargo test --test e2e

# Performance benchmarks
cargo bench
```

### Testing Tools

#### Development Testing
```bash
# Run all tests
cargo test --workspace

# Run specific test
cargo test test_target_scoring

# Run with coverage
cargo tarpaulin --workspace
```

#### CI/CD Testing
- GitHub Actions workflow includes:
  - Code formatting checks
  - Linting with clippy
  - Security scanning with cargo audit
  - Full test suite execution
  - Integration tests with Docker

## Security Guidelines

### Authentication & Authorization

#### API Key Management
```rust
// Secure API key validation
pub async fn validate_api_key(api_key: &str) -> Result<User, AuthError> {
    let hashed_key = hash_api_key(api_key)?;
    let user = self.db.get_user_by_api_key(&hashed_key).await?;
    Ok(user)
}
```

#### Role-Based Access Control
- **Admin**: Full system access
- **Researcher**: Query and analysis access
- **Viewer**: Read-only access to results

### Data Protection

#### Encryption Standards
- **At Rest**: AES-256-GCM for all stored data
- **In Transit**: TLS 1.3 for network communications
- **Secrets**: Encrypted keychain storage

#### PHI Protection
- Automatic detection of sensitive data
- Data classification and access controls
- Comprehensive audit logging
- Leak detection and alerting

### Secure Development Practices

#### Input Validation
```rust
pub fn validate_gene_symbol(symbol: &str) -> Result<(), ValidationError> {
    if symbol.is_empty() || symbol.len() > 50 {
        return Err(ValidationError::InvalidLength);
    }

    if !symbol.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err(ValidationError::InvalidCharacters);
    }

    Ok(())
}
```

#### Error Handling
```rust
// Safe error messages without data leakage
pub async fn process_request(req: Request) -> Result<Response, AppError> {
    match self.validate_request(&req).await {
        Ok(_) => self.process_valid_request(req).await,
        Err(e) => {
            error!("Request validation failed: {}", e); // Log details
            Err(AppError::BadRequest("Invalid request parameters")) // Safe message
        }
    }
}
```

## Performance Optimization

### Profiling Tools

#### CPU Profiling
```bash
# Generate flame graph
cargo flamegraph --bin ferrumyx-agent

# Profile specific function
cargo flamegraph --bin ferrumyx-agent -- test_target_scoring

# Profile with custom sampling rate
cargo flamegraph --bin ferrumyx-agent --rate 1000 -- test_integration
```

#### Memory Profiling
```bash
# Build with profiling
cargo build --release --features heap-profiling

# Run with valgrind
valgrind --tool=massif ./target/release/ferrumyx-agent

# Memory leak detection
valgrind --tool=memcheck --leak-check=full ./target/release/ferrumyx-agent

# Cache profiling
valgrind --tool=cachegrind ./target/release/ferrumyx-agent
```

#### Async Profiling
```rust
use tokio::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::Mutex;

// Async operation profiling
pub struct Profiler {
    start_times: Arc<Mutex<HashMap<String, Instant>>>,
}

impl Profiler {
    pub async fn start_operation(&self, operation: &str) {
        let mut times = self.start_times.lock().await;
        times.insert(operation.to_string(), Instant::now());
    }

    pub async fn end_operation(&self, operation: &str) -> Duration {
        let mut times = self.start_times.lock().await;
        let start = times.remove(operation).unwrap_or_else(|| Instant::now());
        start.elapsed()
    }
}
```

### Advanced Optimization Techniques

#### SIMD Vectorization
```rust
use std::arch::x86_64::*;

// SIMD-accelerated vector similarity calculation
#[target_feature(enable = "avx2")]
unsafe fn cosine_similarity_avx2(a: &[f32], b: &[f32]) -> f32 {
    let mut sum_ab = _mm256_setzero_ps();
    let mut sum_aa = _mm256_setzero_ps();
    let mut sum_bb = _mm256_setzero_ps();

    for i in (0..a.len()).step_by(8) {
        let va = _mm256_loadu_ps(a.as_ptr().add(i));
        let vb = _mm256_loadu_ps(b.as_ptr().add(i));

        sum_ab = _mm256_fmadd_ps(va, vb, sum_ab);
        sum_aa = _mm256_fmadd_ps(va, va, sum_aa);
        sum_bb = _mm256_fmadd_ps(vb, vb, sum_bb);
    }

    let ab = _mm256_reduce_add_ps(sum_ab);
    let aa = _mm256_reduce_add_ps(sum_aa);
    let bb = _mm256_reduce_add_ps(sum_bb);

    ab / (aa.sqrt() * bb.sqrt())
}
```

#### Lock-Free Data Structures
```rust
use crossbeam::queue::SegQueue;
use std::sync::atomic::{AtomicUsize, Ordering};

// Lock-free job queue for high-throughput processing
pub struct JobQueue<T> {
    queue: SegQueue<T>,
    size: AtomicUsize,
}

impl<T> JobQueue<T> {
    pub fn push(&self, item: T) {
        self.queue.push(item);
        self.size.fetch_add(1, Ordering::Relaxed);
    }

    pub fn pop(&self) -> Option<T> {
        let item = self.queue.pop()?;
        self.size.fetch_sub(1, Ordering::Relaxed);
        Some(item)
    }

    pub fn len(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }
}
```

#### Memory Pool Allocation
```rust
use std::alloc::{alloc, dealloc, Layout};
use std::ptr::NonNull;

// Custom memory pool for frequent allocations
pub struct MemoryPool {
    blocks: Vec<NonNull<u8>>,
    free_list: Vec<NonNull<u8>>,
    block_size: usize,
    layout: Layout,
}

impl MemoryPool {
    pub fn new(block_size: usize, initial_blocks: usize) -> Self {
        let layout = Layout::from_size_align(block_size, 8).unwrap();

        let mut blocks = Vec::with_capacity(initial_blocks);
        let mut free_list = Vec::with_capacity(initial_blocks);

        for _ in 0..initial_blocks {
            unsafe {
                let ptr = alloc(layout);
                if !ptr.is_null() {
                    let block = NonNull::new_unchecked(ptr);
                    blocks.push(block);
                    free_list.push(block);
                }
            }
        }

        Self {
            blocks,
            free_list,
            block_size,
            layout,
        }
    }

    pub fn allocate(&mut self) -> Option<NonNull<u8>> {
        if let Some(block) = self.free_list.pop() {
            Some(block)
        } else {
            // Allocate new block
            unsafe {
                let ptr = alloc(self.layout);
                if !ptr.is_null() {
                    let block = NonNull::new_unchecked(ptr);
                    self.blocks.push(block);
                    Some(block)
                } else {
                    None
                }
            }
        }
    }

    pub fn deallocate(&mut self, block: NonNull<u8>) {
        self.free_list.push(block);
    }
}
```

### Optimization Techniques

#### Database Optimization
```sql
-- Add performance indexes
CREATE INDEX CONCURRENTLY idx_papers_pub_date ON papers(pub_date);
CREATE INDEX CONCURRENTLY idx_entities_gene_symbol ON entities(entity_text) WHERE entity_type = 'gene';

-- Optimize queries
EXPLAIN ANALYZE SELECT * FROM papers WHERE doi = '10.1234/example';
```

#### Async Programming
```rust
// Efficient concurrent processing
pub async fn process_targets(targets: Vec<String>) -> Result<Vec<TargetResult>> {
    let tasks: Vec<_> = targets.into_iter()
        .map(|target| tokio::spawn(async move {
            self.analyze_target(&target).await
        }))
        .collect();

    let results = futures::future::join_all(tasks).await;
    results.into_iter().collect::<Result<Vec<_>>>()
}
```

#### Caching Strategy
```rust
// Implement multi-level caching
pub struct CacheManager {
    memory_cache: Arc<RwLock<HashMap<String, CachedResult>>>,
    redis_cache: Arc<RedisCache>,
}

impl CacheManager {
    pub async fn get_or_compute<F, Fut>(&self, key: &str, compute: F) -> Result<CachedResult>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<CachedResult>>,
    {
        // Check memory cache first
        if let Some(result) = self.memory_cache.read().await.get(key) {
            return Ok(result.clone());
        }

        // Check Redis cache
        if let Some(result) = self.redis_cache.get(key).await? {
            // Update memory cache
            self.memory_cache.write().await.insert(key.to_string(), result.clone());
            return Ok(result);
        }

        // Compute and cache
        let result = compute().await?;
        self.set(key, result.clone()).await?;
        Ok(result)
    }
}
```

## Custom Tool Development

### Tool Architecture

Ferrumyx's extensible tool system allows developers to create custom bioinformatics tools that integrate seamlessly with the agent framework.

#### Tool Interface

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool metadata
    fn metadata(&self) -> ToolMetadata;

    /// Execute the tool with given parameters
    async fn execute(&self, params: ToolParams) -> Result<ToolResult, ToolError>;

    /// Validate tool parameters
    fn validate_params(&self, params: &ToolParams) -> Result<(), ToolError>;

    /// Tool capabilities and requirements
    fn capabilities(&self) -> ToolCapabilities;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub security_domain: SecurityDomain,
}

#[derive(Debug, Clone)]
pub enum SecurityDomain {
    Orchestrator,  // Safe for main process
    Container,     // Requires Docker sandbox
    BioClaw,       // Specialized bioinformatics
}
```

#### Example Custom Tool

```rust
use ferrumyx_runtime_core::tools::{Tool, ToolParams, ToolResult, ToolError};

pub struct CustomGeneAnalyzer {
    metadata: ToolMetadata,
    llm_client: Arc<dyn LlmBackend>,
}

#[async_trait]
impl Tool for CustomGeneAnalyzer {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "custom_gene_analyzer".to_string(),
            version: "1.0.0".to_string(),
            description: "Advanced gene expression analysis tool".to_string(),
            author: "Research Team".to_string(),
            security_domain: SecurityDomain::BioClaw,
        }
    }

    async fn execute(&self, params: ToolParams) -> Result<ToolResult, ToolError> {
        // Extract parameters
        let gene_symbol = params.get_string("gene_symbol")?;
        let cancer_type = params.get_string("cancer_type")?;

        // Perform analysis
        let expression_data = self.analyze_expression(&gene_symbol, &cancer_type).await?;
        let pathways = self.identify_pathways(&expression_data).await?;

        // Generate LLM-powered insights
        let insights = self.llm_client.complete(LlmRequest {
            prompt: format!("Analyze gene expression data for {} in {} cancer: {}", gene_symbol, cancer_type, expression_data),
            max_tokens: 500,
            temperature: 0.7,
        }).await?;

        Ok(ToolResult::Json(serde_json::json!({
            "gene_symbol": gene_symbol,
            "cancer_type": cancer_type,
            "expression_levels": expression_data,
            "affected_pathways": pathways,
            "insights": insights.content
        })))
    }

    fn validate_params(&self, params: &ToolParams) -> Result<(), ToolError> {
        if !params.has_key("gene_symbol") {
            return Err(ToolError::Validation("gene_symbol parameter required".to_string()));
        }
        if !params.has_key("cancer_type") {
            return Err(ToolError::Validation("cancer_type parameter required".to_string()));
        }
        Ok(())
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities {
            requires_gpu: false,
            max_execution_time: Duration::from_secs(300),
            memory_limit_mb: 1024,
        }
    }
}
```

#### Tool Registration

```rust
// Register custom tool with the agent
pub async fn register_custom_tools(registry: &mut ToolRegistry) -> Result<(), ToolError> {
    // Register bioinformatics tools
    registry.register(Box::new(CustomGeneAnalyzer::new(llm_client.clone()))).await?;
    registry.register(Box::new(ProteinStructurePredictor::new())).await?;

    // Register orchestrator tools
    registry.register(Box::new(DataExportTool::new())).await?;

    Ok(())
}
```

### Plugin Architecture

#### Plugin Interface

```rust
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Plugin initialization
    async fn initialize(&self, config: &PluginConfig) -> Result<(), PluginError>;

    /// Plugin hooks
    fn hooks(&self) -> Vec<PluginHook>;

    /// Plugin metadata
    fn metadata(&self) -> PluginMetadata;
}

pub enum PluginHook {
    PreToolExecution(Box<dyn Fn(&ToolParams) -> Result<(), PluginError> + Send + Sync>),
    PostToolExecution(Box<dyn Fn(&ToolResult) -> Result<(), PluginError> + Send + Sync>),
    PreAgentLoop(Box<dyn Fn(&AgentContext) -> Result<(), PluginError> + Send + Sync>),
    Custom(String, Box<dyn Fn(&serde_json::Value) -> Result<(), PluginError> + Send + Sync>),
}
```

#### Example Plugin

```rust
pub struct AuditPlugin {
    audit_logger: Arc<AuditLogger>,
}

#[async_trait]
impl Plugin for AuditPlugin {
    async fn initialize(&self, config: &PluginConfig) -> Result<(), PluginError> {
        // Initialize audit logging
        self.audit_logger.initialize(config.audit_config.clone()).await?;
        Ok(())
    }

    fn hooks(&self) -> Vec<PluginHook> {
        vec![
            PluginHook::PreToolExecution(Box::new(|params| {
                self.audit_logger.log_tool_execution_start(params).await?;
                Ok(())
            })),
            PluginHook::PostToolExecution(Box::new(|result| {
                self.audit_logger.log_tool_execution_end(result).await?;
                Ok(())
            })),
        ]
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "audit_plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "Comprehensive audit logging plugin".to_string(),
        }
    }
}
```

### Federated Learning Integration

#### Federated Training Protocol

```rust
#[async_trait]
pub trait FederatedLearner: Send + Sync {
    /// Initialize federated learning
    async fn initialize_federation(&self, config: FederationConfig) -> Result<(), FederationError>;

    /// Train on local data
    async fn train_local(&self, data: &[TrainingSample]) -> Result<ModelUpdate, TrainingError>;

    /// Aggregate updates from peers
    async fn aggregate_updates(&self, updates: Vec<ModelUpdate>) -> Result<ModelUpdate, AggregationError>;

    /// Update local model
    async fn update_model(&self, update: ModelUpdate) -> Result<(), UpdateError>;
}

pub struct SecureAggregation {
    crypto: Arc<CryptoProvider>,
    peers: Vec<PeerInfo>,
}

impl SecureAggregation {
    /// Secure multi-party computation for model aggregation
    pub async fn aggregate_secure(&self, updates: Vec<ModelUpdate>) -> Result<ModelUpdate, AggregationError> {
        // Use MPC to aggregate without revealing individual updates
        let masked_updates = self.mask_updates(updates).await?;
        let aggregated = self.compute_sum(masked_updates).await?;
        let unmasked = self.unmask_result(aggregated).await?;

        Ok(unmasked)
    }
}
```

## Deployment

### Development Deployment

```bash
# Start development environment
docker-compose -f docker-compose.dev.yml up -d

# Run with live reload
cargo watch -x 'run --bin ferrumyx-web'

# Development with custom tools
cargo run --bin ferrumyx-agent -- --config dev-config.toml --enable-custom-tools
```

### Production Deployment

#### Docker Deployment
```yaml
# docker-compose.prod.yml
version: '3.8'
services:
  ferrumyx-web:
    image: classacre/ferrumyx:v2.0.0
    environment:
      - RUST_LOG=warn
      - DATABASE_URL=${DATABASE_URL}
      - FERRUMYX_PLUGINS=audit_plugin,metrics_plugin
    secrets:
      - ironclaw_api_key
      - encryption_key
      - custom_tool_keys
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
    volumes:
      - ./plugins:/app/plugins:ro
```

#### Kubernetes Deployment
```yaml
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
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: ferrumyx-secrets
              key: database-url
        resources:
          limits:
            cpu: "2"
            memory: 4Gi
```

### Configuration Management

#### Environment Variables
```bash
# Database configuration
export DATABASE_URL=postgresql://user:pass@localhost:5432/ferrumyx

# LLM provider settings
export IRONCLAW_API_KEY=your-key-here
export OLLAMA_BASE_URL=http://localhost:11434

# Security settings
export ENCRYPTION_KEY_PATH=/etc/ferrumyx/keys
export AUDIT_LOG_PATH=/var/log/ferrumyx
```

#### Configuration Files
```toml
# config/ferrumyx.toml
[database]
url = "postgresql://localhost:5432/ferrumyx"
max_connections = 20

[llm]
provider = "ollama"
base_url = "http://localhost:11434"

[agent]
max_concurrent_jobs = 10
job_timeout_seconds = 300

[security]
encryption_key_path = "/etc/ferrumyx/keys"
audit_log_path = "/var/log/ferrumyx"
```

### Monitoring and Observability

#### Application Metrics
- Request latency and throughput
- Error rates and success rates
- Database connection pool usage
- Memory and CPU utilization

#### Logging Configuration
```bash
# Structured logging
export RUST_LOG=ferrumyx=info,tokio=warn,sqlx=warn

# Debug specific components
export RUST_LOG=ferrumyx=debug,ferrumyx_agent=trace
```

### Scaling Considerations

#### Horizontal Scaling
- Stateless application design
- Load balancer configuration
- Database read replicas
- Redis cluster for caching

#### Vertical Scaling
- Resource limit configuration
- GPU acceleration for compute-intensive tasks
- Memory optimization techniques

## Getting Help

### Documentation Resources

- **[Architecture Guide](Architecture-&-Design)** - System design and components
- **[API Reference](API-Reference)** - Complete API documentation
- **[Troubleshooting](Troubleshooting)** - Common issues and solutions
- **[Operations Guide](Operations-Guide)** - Deployment and maintenance

### Community Support

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: Technical questions and discussions
- **Discord**: Real-time community chat
- **Security Issues**: security@ferrumyx.org

### Code Review Process

1. **Automated Checks**: CI pipeline must pass all tests
2. **Peer Review**: Minimum one maintainer review required
3. **Security Review**: Sensitive changes require security team review
4. **Documentation**: Update relevant documentation
5. **Testing**: Maintain adequate test coverage

Thank you for contributing to Ferrumyx! Your work advances oncology research through better software tools.