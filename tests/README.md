# Ferrumyx IronClaw Integration E2E Test Suite

This directory contains comprehensive end-to-end tests for the IronClaw-integrated Ferrumyx system, focusing on oncology queries and workflows.

## Test Structure

```
tests/
├── e2e/                          # End-to-end test suite
│   ├── config/                   # Test configurations
│   ├── data/                     # Sample oncology data
│   ├── scripts/                  # Test execution scripts
│   ├── reports/                  # Validation reports
│   └── scenarios/                # Test scenarios
├── integration/                  # Component integration tests
├── performance/                  # Performance benchmarks
└── security/                     # Security validation tests
```

## Test Scenarios Covered

1. **Setup & Environment**
   - Database initialization
   - Service startup verification
   - Sample data ingestion

2. **Literature Ingestion**
   - PubMed/EuropePMC source integration
   - PDF parsing and chunking
   - Entity extraction and NER
   - Embedding generation

3. **Knowledge Graph Construction**
   - Entity normalization
   - Relation extraction
   - Fact validation
   - Graph integrity checks

4. **Target Discovery & Ranking**
   - Multi-signal scoring
   - Provider enrichment (DepMap, etc.)
   - Query processing
   - Result ranking

5. **Multi-Channel Interactions**
   - Web API endpoints
   - WhatsApp integration
   - Chat workflows

6. **WASM Sandboxing**
   - Tool isolation
   - Security boundaries
   - Container orchestration

7. **Autonomous Discovery**
   - Scheduled tasks
   - Cycle execution
   - State persistence

8. **Security Features**
   - Secrets management
   - Audit logging
   - Data classification

9. **Performance Testing**
   - Workload simulation
   - Throughput measurements
   - Resource monitoring

## Running Tests

```bash
# Run all E2E tests
./tests/e2e/scripts/run_all_tests.sh

# Run specific test category
./tests/e2e/scripts/test_ingestion.sh
./tests/e2e/scripts/test_ranking.sh

# Generate reports
./tests/e2e/scripts/generate_reports.sh
```

## Sample Data

Test data includes:
- Oncology papers (KRAS, pancreatic cancer)
- Mock clinical trials
- Sample entities and relations
- Performance test datasets

## Validation Reports

Tests generate detailed reports including:
- Test execution logs
- Performance metrics
- Coverage analysis
- Security audit results
- Integration verification