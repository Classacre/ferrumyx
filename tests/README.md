# Tests Directory

This directory contains comprehensive testing infrastructure for Ferrumyx.

## Files

### Test Scripts
- final_validation_test.py - End-to-end validation testing
- multi_channel_test.py - Multi-channel interface testing
- ci_test.yml - CI/CD pipeline testing

### Performance Testing
- database_performance_analyzer.py - Database performance analysis
- deep_performance_analyzer.py - Advanced performance metrics
- gpu_acceleration_benchmark.py - GPU performance benchmarking
- performance_optimization_advisor.py - Performance optimization recommendations
- performance_regression_test.py - Regression detection
- run_performance_benchmark.py - Benchmark execution
- scalability_test.py - Load testing and scaling analysis
- test_benchmark.py - Basic performance testing
- validate_memory_leaks.py - Memory leak detection

## Test Categories

- Unit tests in crates/*/tests/
- Integration tests in tests/integration/
- Performance tests in tests/performance/
- End-to-end tests in tests/e2e/

## Usage

`ash
# Run all tests
cargo test --workspace

# Run performance tests
python tests/performance_regression_test.py

# Run E2E tests
python tests/final_validation_test.py
`

See TESTING.md for comprehensive testing documentation.
