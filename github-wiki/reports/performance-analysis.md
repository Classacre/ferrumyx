# Ferrumyx Performance Testing Infrastructure

This document outlines the comprehensive performance testing and optimization framework implemented for Ferrumyx.

## Performance Testing Tools

### 1. Performance Regression Test (`performance_regression_test.py`)
Automated performance regression detection and historical tracking.

**Purpose:** Detect performance regressions in CI/CD pipeline and track performance over time.

**Key Features:**
- Statistical regression detection using historical baselines
- Multi-scenario API endpoint testing (literature search, target discovery, KG queries, chat)
- Resource usage monitoring (CPU, memory, network)
- Automated alerting for performance degradation
- Historical performance data storage and trend analysis

**Usage:**
```bash
# Run regression tests against local server
python performance_regression_test.py --url http://localhost:3001

# CI mode (exits with error on regression)
python performance_regression_test.py --url http://localhost:3001 --ci-mode

# Test specific scenarios
python performance_regression_test.py --scenarios api_health literature_search target_discovery
```

**Output:**
- JSON results in `performance_db/`
- Markdown reports in `performance_reports/`
- Regression alerts and recommendations

### 2. Database Performance Analyzer (`database_performance_analyzer.py`)
Comprehensive database performance monitoring and optimization recommendations.

**Purpose:** Analyze PostgreSQL performance, identify bottlenecks, and provide optimization recommendations.

**Key Features:**
- Query performance analysis (slow queries, execution plans)
- Index usage statistics and recommendations
- Connection pool monitoring
- Cache hit ratio analysis
- Table bloat detection
- Automatic EXPLAIN ANALYZE for query optimization

**Usage:**
```bash
# Analyze local database
python database_performance_analyzer.py --connection-string "postgresql://user:pass@localhost/ferrumyx"

# CI mode (exits with error on critical issues)
python database_performance_analyzer.py --connection-string "$DATABASE_URL" --ci-mode
```

**Output:**
- Database health metrics and analysis
- Optimization recommendations by priority
- Performance score (0-100) with improvement suggestions

### 3. Scalability Test Framework (`scalability_test.py`)
Load testing for different user concurrency levels and production-scale scenarios.

**Purpose:** Test system scalability under various load conditions and identify breaking points.

**Key Features:**
- Asynchronous load testing with configurable concurrency
- Realistic request patterns based on actual usage scenarios
- Resource monitoring during tests
- Scaling efficiency analysis
- Memory leak detection
- Optimal concurrency determination

**Usage:**
```bash
# Run scalability test with default concurrency levels
python scalability_test.py --url http://localhost:3001

# Custom concurrency levels and duration
python scalability_test.py --url http://localhost:3001 \
  --concurrency-levels 1 5 10 25 50 100 200 \
  --duration 120
```

**Output:**
- Scalability analysis reports
- Performance charts (throughput, latency, resource usage)
- Breaking point identification
- Capacity planning recommendations

### 4. Performance Optimization Advisor (`performance_optimization_advisor.py`)
Automated performance analysis and optimization plan generation.

**Purpose:** Aggregate all performance data and generate comprehensive optimization recommendations.

**Key Features:**
- Combines data from all performance tests
- Prioritizes issues by impact and effort
- Creates implementation roadmap with phases
- Projects performance improvements
- Cost-benefit analysis

**Usage:**
```bash
# Generate optimization plan from all available data
python performance_optimization_advisor.py

# CI mode (exits with error on critical issues)
python performance_optimization_advisor.py --ci-mode

# Custom data directory
python performance_optimization_advisor.py --data-dir ./performance_data
```

**Output:**
- Comprehensive optimization plan with priorities
- Implementation phases (immediate, short-term, long-term)
- Quick wins identification
- Projected performance improvements

## CI/CD Integration

### GitHub Actions Performance Job

The performance testing is integrated into the CI/CD pipeline via `.github/workflows/ci.yml`:

```yaml
performance:
  name: Performance Testing & Regression Detection
  runs-on: ubuntu-latest
  services:
    postgres:
      image: postgres:15
      # ... database setup
    redis:
      image: redis:7-alpine
      # ... redis setup

  steps:
  - name: Checkout code
    uses: actions/checkout@v4
    with:
      fetch-depth: 0  # Full history for regression analysis

  - name: Setup Python
    uses: actions/setup-python@v4
    with:
      python-version: '3.11'

  - name: Install Python dependencies
    run: |
      pip install requests psutil matplotlib pandas numpy scipy seaborn

  - name: Run database performance analysis
    run: python database_performance_analyzer.py --connection-string "$DATABASE_URL" --ci-mode

  - name: Run performance regression tests
    run: |
      python performance_regression_test.py --url http://localhost:3001 --ci-mode \
        --scenarios api_health literature_search target_discovery kg_query chat_query

  # ... additional steps for Criterion benchmarks and custom tests
```

## Performance Metrics Tracked

### Application Metrics
- Response times (P50, P95, P99)
- Throughput (requests per second)
- Success rates and error patterns
- Resource usage (CPU, memory, disk, network)

### Database Metrics
- Query execution times
- Connection pool utilization
- Cache hit ratios
- Index usage statistics
- Table bloat and vacuum requirements

### Scalability Metrics
- Scaling efficiency (how well performance scales with concurrency)
- Breaking points (when performance degrades significantly)
- Optimal concurrency levels
- Memory leak detection

### Historical Tracking
- Performance baselines and trends
- Regression detection with statistical significance
- Long-term performance monitoring
- Seasonal and load-based pattern analysis

## Success Criteria

### Automated Regression Detection ✅
- Performance regressions detected in CI/CD
- Statistical significance testing
- Automated alerts and notifications

### Benchmarking Framework ✅
- Comprehensive API endpoint testing
- Resource usage monitoring
- Historical data storage and comparison

### Load Testing ✅
- Multi-concurrency level testing
- Realistic request patterns
- Scalability analysis and breaking point detection

### Resource Monitoring ✅
- Continuous resource usage tracking
- Memory leak detection
- CPU saturation monitoring

### Database Performance ✅
- Query performance monitoring
- Index optimization recommendations
- Connection pool monitoring

### CI/CD Integration ✅
- Performance tests in GitHub Actions
- Automated failure on critical issues
- Performance report artifacts

### Historical Tracking ✅
- Performance metrics storage
- Trend analysis and comparison
- Baseline management

### Automated Alerts ✅
- Regression notifications
- Performance degradation alerts
- Critical issue detection

### Optimization Recommendations ✅
- Prioritized action items
- Implementation phases
- Projected improvements

### Scalability Testing ✅
- Load testing for various concurrency levels
- Production capacity planning
- Scaling bottleneck identification

## Output Directories

- `performance_db/` - Historical performance metrics and baselines
- `performance_reports/` - Performance regression test reports
- `db_performance_reports/` - Database analysis reports
- `scalability_reports/` - Scalability test results and charts
- `optimization_reports/` - Comprehensive optimization plans

## Dependencies

```bash
pip install requests psutil matplotlib pandas numpy scipy seaborn aiohttp
```

## Database Requirements

For full database analysis, ensure pg_stat_statements extension is available:

```sql
CREATE EXTENSION IF NOT EXISTS pg_stat_statements;
```

## Future Enhancements

- Distributed tracing integration (Jaeger/OpenTelemetry)
- Real-time performance dashboards
- ML-based anomaly detection
- Automated performance optimization suggestions
- Multi-region performance testing