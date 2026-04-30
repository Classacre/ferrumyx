# End-to-End Testing Suite

This directory contains comprehensive end-to-end tests for Ferrumyx's oncology discovery workflows, validating the complete pipeline from literature ingestion to target ranking across all system components.

## Purpose and Overview

The E2E test suite validates Ferrumyx's ability to perform complete oncology discovery workflows, from biomedical literature ingestion through target identification and ranking. Tests cover:

- **Full pipeline execution**: Literature → KG → Ranking → Multi-channel output
- **Integration testing**: Component interactions and data flow
- **Performance validation**: System behavior under load
- **Security verification**: Data protection and access controls
- **Multi-channel testing**: Interface consistency across platforms

Tests are designed to run against a complete Ferrumyx deployment, simulating real-world oncology research scenarios.

## Test Scenarios

### Oncology Discovery Workflows

1. **KRAS G12D Pancreatic Cancer**
   - Literature search and ingestion
   - Entity extraction and KG construction
   - Target ranking with multi-signal scoring
   - Pathway analysis and drug target identification

2. **Breast Cancer BRCA1/2 Mutations**
   - Clinical trial data integration
   - Expression analysis workflows
   - Variant calling validation
   - Therapeutic target prioritization

3. **Lung Cancer EGFR Inhibitors**
   - Drug-target interaction modeling
   - Resistance mechanism identification
   - Biomarker discovery
   - Treatment response prediction

### System Integration Tests

4. **Multi-Channel Interactions**
   - Web API endpoints
   - WhatsApp/Slack/Discord integration
   - Conversational workflow completion
   - Data sensitivity filtering

5. **Autonomous Discovery Cycles**
   - Scheduled task execution
   - Self-healing workflow recovery
   - State persistence across restarts
   - Background monitoring validation

6. **Security and Compliance**
   - Data classification enforcement
   - Audit logging verification
   - Access control validation
   - HIPAA compliance checks

## Installation/Setup Instructions

### Prerequisites

1. **Complete Ferrumyx Installation**
   ```bash
   # Follow main README setup
   cargo build --release
   ./start.ps1  # Windows
   ```

2. **Test Data Setup**
   ```bash
   # Initialize test database
   cd tests/e2e
   ./scripts/setup_test_data.sh
   ```

3. **External Dependencies**
   - PostgreSQL with test database
   - Docker for bioinformatics containers
   - Platform API credentials (optional for channel tests)

### Test Environment Setup

```bash
# Create test configuration
cp config/test_config.toml config/local_config.toml
# Edit local_config.toml with your settings

# Build test containers
cd docker && ./build.sh

# Initialize test database
./tests/e2e/scripts/init_test_db.sh
```

## Usage Examples

### Running All E2E Tests

```bash
# From project root
./tests/e2e/scripts/run_e2e_tests.sh

# Or on Windows
./tests/e2e/scripts/run_e2e_tests.ps1
```

### Running Specific Test Scenarios

```bash
# Test KRAS discovery workflow
./tests/e2e/scripts/test_kras_workflow.sh

# Test multi-channel integration
./tests/e2e/scripts/test_channels.sh

# Test security features
./tests/e2e/scripts/test_security.sh
```

### Running Individual Tests

```bash
# Run with specific scenario
cargo test --test e2e -- --scenario oncology_kras_g12d

# Run performance tests
./tests/performance/run_performance_tests.py
```

### Generating Reports

```bash
# Generate test reports
./tests/e2e/scripts/generate_reports.sh

# View latest report
cat tests/e2e/reports/$(ls -t reports/ | head -1)
```

## Configuration Options

### Test Configuration

```toml
[test_config]
# Test environment settings
database_url = "postgresql://test:test@localhost/ferrumyx_test"
timeout_secs = 300
parallel_execution = true

# Test data settings
sample_data_dir = "tests/e2e/data"
scenarios_dir = "tests/e2e/scenarios"

# Performance thresholds
max_response_time_secs = 30
min_throughput_reqs_per_sec = 10

# Security settings
enable_audit_checks = true
validate_data_sensitivity = true
```

### Scenario Configuration

```toml
# oncology_kras_g12d.toml
[scenario]
name = "KRAS G12D Pancreatic Cancer Discovery"
description = "Complete workflow from literature to targets"

[scenario.inputs]
queries = ["KRAS G12D pancreatic cancer"]
sources = ["pubmed", "europepmc"]

[scenario.validations]
min_papers = 100
min_entities = 50
min_relations = 200
expected_targets = ["KRAS", "TP53", "CDKN2A"]
```

### Performance Configuration

```toml
[performance]
# Load testing settings
concurrent_users = 10
test_duration_mins = 5
ramp_up_secs = 30

# Resource monitoring
enable_resource_monitoring = true
memory_threshold_mb = 2048
cpu_threshold_percent = 80
```

## Troubleshooting Guide

### Common Issues

**Database Connection Failed**
```
Error: Cannot connect to test database
```
**Solution:** Ensure PostgreSQL is running and test database exists
```bash
createdb ferrumyx_test
psql ferrumyx_test < schema.sql
```

**Container Not Available**
```
Error: Required bioinformatics container not found
```
**Solution:** Build containers
```bash
cd docker && ./build.sh
```

**Test Timeout**
```
Error: Test execution timed out
```
**Solution:** Increase timeout in config or check system performance

**Scenario Data Missing**
```
Error: Test scenario data not found
```
**Solution:** Run data setup script
```bash
./tests/e2e/scripts/setup_test_data.sh
```

### Debugging Failed Tests

Enable verbose logging:

```bash
export FERRUMYX_TEST_LOG_LEVEL=debug
export FERRUMYX_LOG_LEVEL=trace
./tests/e2e/scripts/run_e2e_tests.sh --verbose
```

Check test artifacts:

```bash
# View test workspace
ls -la tests/e2e/workspace/

# Check logs
tail -f tests/e2e/workspace/test.log

# Inspect database state
psql ferrumyx_test -c "SELECT COUNT(*) FROM papers;"
```

### Performance Issues

**Slow Test Execution**
- Check system resources (CPU, memory, disk I/O)
- Reduce concurrent test execution
- Optimize database queries

**Memory Exhaustion**
- Increase system memory allocation
- Reduce test data size
- Enable memory profiling

**Database Bottlenecks**
- Check PostgreSQL configuration
- Ensure pgvector extension is optimized
- Monitor query performance

### Test Data Issues

**Incomplete Test Data**
- Re-run data setup script
- Verify data integrity
- Check data source availability

**Outdated Sample Data**
- Update test data from live sources
- Refresh cached embeddings
- Validate entity annotations

## Test Reports and Validation

### Report Structure

Tests generate comprehensive reports including:

```
tests/e2e/reports/
├── e2e_test_report_YYYYMMDD_HHMMSS.md    # Detailed execution report
├── final_test_summary_YYYYMMDD.md         # Summary across all runs
├── performance_metrics.json               # Performance data
└── security_audit.log                     # Security validation
```

### Validation Metrics

- **Coverage**: Percentage of code and workflows tested
- **Success Rate**: Pass/fail ratio for test scenarios
- **Performance**: Response times, throughput, resource usage
- **Security**: Audit findings, compliance violations
- **Data Quality**: Entity extraction accuracy, KG completeness

### Continuous Integration

Tests are designed for CI/CD integration:

```yaml
# .github/workflows/e2e-tests.yml
- name: Run E2E Tests
  run: |
    ./tests/e2e/scripts/run_e2e_tests.sh
    ./tests/e2e/scripts/generate_reports.sh
```

## Links to Related Documentation

- [Main README](../../README.md) - Project overview
- [Test README](../README.md) - General testing information
- [Architecture](../../ARCHITECTURE.md) - System design
- [Security Guide](../../docs/SECURITY.md) - Security testing
- [Performance Guide](../../docs/PERFORMANCE.md) - Performance testing
- [API Documentation](../../crates/ferrumyx-web/src/api/) - Endpoint testing