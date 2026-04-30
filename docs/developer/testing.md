# Ferrumyx Testing Guide

This document outlines the comprehensive testing strategy and infrastructure for Ferrumyx.

## Test Coverage Goals

- **95%+ code coverage** across all crates
- **Unit tests** for core functions and methods
- **Integration tests** for component interaction and data flow
- **End-to-end tests** for complete workflow validation
- **Performance tests** for load testing and regression detection
- **Security tests** for authentication, authorization, and data protection

## Test Structure

### Unit Tests (`#[test]`)
- Test individual functions and methods in isolation
- Use mocks for external dependencies
- Focus on business logic and edge cases

### Integration Tests (`#[tokio::test]`)
- Test component interactions
- Use test databases and real dependencies where appropriate
- Verify data flow between components

### End-to-End Tests (`#[tokio::test]` with `#[ignore]`)
- Mark with `#[ignore]` and run with `--ignored` flag
- Test complete workflows from start to finish
- Use dedicated test environments

### Performance Tests (`#[tokio::test]` with `#[ignore]`)
- Mark with `#[ignore]` and filter with `perf_` prefix
- Benchmark critical paths
- Detect performance regressions

### Security Tests (`#[tokio::test]` with `#[ignore]`)
- Mark with `#[ignore]` and filter with `security_` prefix
- Test for common vulnerabilities (SQL injection, XSS, etc.)
- Validate authentication and authorization

## Test Infrastructure

### ferrumyx-test-utils

Shared testing utilities available to all crates:

#### Fixtures (`fixtures.rs`)
- `TestFixtureManager`: Oncology test data generation
- `OncologyTestData`: Mock papers, targets, clinical trials
- `PaperFixture`, `TargetFixture`, `ClinicalTrialFixture`: Test data structures

#### Mocks (`mocks.rs`)
- `MockHttpClient`: HTTP client mocking
- `MockDatabase`: Database interface mocking

#### Assertions (`assertions.rs`)
- Enhanced assertion utilities
- Pretty diff output for test failures

#### Performance (`performance.rs`)
- Benchmarking utilities
- Performance regression detection

#### Chaos (`chaos.rs`)
- Fault injection testing
- Resilience testing utilities

#### Integration (`integration.rs`)
- `TestHttpClient`: API testing client
- `BenchmarkRunner`: Performance benchmarking
- `LoadTester`: Load testing utilities
- `SecurityTester`: Security vulnerability testing

## Running Tests

### All Tests
```bash
cargo test --workspace --features postgres,libsql
```

### Unit Tests Only
```bash
cargo test --workspace --features postgres,libsql --lib
```

### Integration Tests Only
```bash
cargo test --workspace --features postgres,libsql --test integration
```

### End-to-End Tests
```bash
cargo test --workspace --features postgres,libsql -- --ignored --nocapture e2e_
```

### Performance Tests
```bash
cargo test --workspace --features postgres,libsql -- --ignored --nocapture perf_
```

### Security Tests
```bash
cargo test --workspace --features postgres,libsql -- --ignored --nocapture security_
```

### Coverage Report
```bash
cargo tarpaulin --workspace --features postgres,libsql --out Html
```

## Test Organization by Crate

### ferrumyx-agent
**Coverage Areas:**
- Agent orchestration and IronClaw integration
- Workflow management and state handling
- Container orchestration for bioinformatics tools
- LLM routing and data classification

**Test Files:**
- `main.rs`: Core agent functionality, LLM routing
- `config/tests.rs`: Configuration validation
- `tools/`: Individual tool testing
- `container_orchestrator.rs`: Docker container management

### ferrumyx-web
**Coverage Areas:**
- API endpoints and request handling
- Authentication middleware
- CORS validation and security headers
- WebSocket/SSE communication

**Test Files:**
- `state.rs`: Application state management
- `router.rs`: Route configuration
- `handlers/`: Individual endpoint testing

### ferrumyx-ranker
**Coverage Areas:**
- Target scoring algorithms
- ML model validation
- Ranking logic and edge cases
- Provider signal processing

**Test Files:**
- `lib.rs`: Core ranking engine
- `scorer.rs`: Scoring algorithm validation
- `providers/`: Individual provider testing

### ferrumyx-molecules
**Coverage Areas:**
- Molecular structure analysis
- External tool integration (BLAST, PyMOL, etc.)
- Pipeline execution and error handling
- Scoring and ranking molecules

**Test Files:**
- `pipeline.rs`: End-to-end pipeline testing
- `scoring.rs`: Molecule scoring algorithms
- Component-specific test files

## CI/CD Integration

### GitHub Actions Workflows

#### CI Pipeline (`ci.yml`)
1. **Test Job**: Unit and integration tests with database services
2. **Security Job**: Vulnerability scanning and security tests
3. **Performance Job**: Benchmarking and performance regression detection
4. **E2E Job**: Full workflow testing with Docker Compose
5. **Coverage Job**: Code coverage reporting with Codecov

#### CD Pipeline (`cd.yml`)
- Automated deployment on successful tests
- Rollback on test failures

### Test Environments

#### Local Development
```bash
# Start test database
docker-compose -f docker-compose.test.yml up -d

# Run tests
DATABASE_URL=postgres://postgres:postgres@localhost:5432/ferrumyx_test cargo test
```

#### CI Environment
- PostgreSQL 15 and Redis 7 services
- Docker Compose for E2E testing
- Isolated test databases per run

## Performance Benchmarking

### Benchmark Categories
1. **Query Performance**: Target ranking and search operations
2. **Ingestion Performance**: Data processing and indexing
3. **API Performance**: REST endpoint response times
4. **Container Performance**: Bioinformatics tool execution

### Regression Detection
- Historical benchmark comparison
- Alerting on >10% performance degradation
- Automated performance profiling

## Security Testing

### Test Categories
1. **Input Validation**: SQL injection, XSS, command injection
2. **Authentication**: JWT validation, session management
3. **Authorization**: Role-based access control
4. **Data Protection**: Encryption, secure storage

### Security Tools
- **cargo-audit**: Rust dependency vulnerability scanning
- **Trivy**: Container and filesystem vulnerability scanning
- **Custom Security Tests**: Application-specific security validation

## Coverage Reporting

### Tools
- **cargo-tarpaulin**: Code coverage analysis
- **Codecov**: Coverage reporting and tracking
- **Coverage Badges**: Repository status indicators

### Coverage Targets
- **Overall**: 95%+ line coverage
- **Critical Paths**: 100% coverage for security and core business logic
- **New Code**: 100% coverage required for new features

## Test Data Management

### Test Datasets
- **Oncology Test Data**: Mock papers, targets, clinical trials
- **Molecular Data**: Test molecules and structures
- **Performance Data**: Large datasets for scalability testing

### Data Generation
- **Fixtures**: Predefined test data
- **Generators**: Dynamic test data creation
- **Factories**: Object creation helpers

## Best Practices

### Test Writing
1. **Descriptive Names**: `test_user_cannot_access_admin_endpoint`
2. **Arrange-Act-Assert**: Clear test structure
3. **Independent Tests**: No shared state between tests
4. **Fast Execution**: Keep unit tests under 1ms

### Mock Usage
1. **Interface Mocks**: Mock traits, not concrete types
2. **Behavior Verification**: Test interactions, not implementations
3. **Minimal Mocks**: Only mock what's necessary

### Async Testing
1. **tokio::test**: For async test functions
2. **Test Timeouts**: Prevent hanging tests
3. **Cleanup**: Ensure proper resource cleanup

### CI/CD Considerations
1. **Parallel Execution**: Run tests in parallel where possible
2. **Flaky Test Handling**: Retry mechanisms for transient failures
3. **Artifact Collection**: Collect logs and artifacts on failures

## Troubleshooting

### Common Issues
1. **Database Connection**: Ensure test database is running
2. **Port Conflicts**: Use unique ports for services
3. **Race Conditions**: Avoid shared state in async tests
4. **Memory Leaks**: Monitor memory usage in long-running tests

### Debugging Tests
```bash
# Run single test with output
cargo test test_name -- --nocapture

# Run tests with backtrace
RUST_BACKTRACE=1 cargo test

# Debug specific test
cargo test test_name -- --nocapture --ignored
```

## Contributing

When adding new features:
1. Add corresponding tests
2. Ensure 100% coverage for new code
3. Update this documentation if test patterns change
4. Run full test suite before submitting PR

## Future Enhancements

- **Property-based Testing**: Use proptest for edge case discovery
- **Mutation Testing**: Validate test quality with mutation analysis
- **Chaos Engineering**: Production resilience testing
- **Visual Testing**: UI component testing for web interface