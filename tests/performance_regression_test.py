#!/usr/bin/env python3
"""
Ferrumyx Performance Regression Detection Framework
Automated performance regression detection and historical tracking
"""

import json
import os
import time
import requests
import psutil
import pandas as pd
import numpy as np
from datetime import datetime, timedelta
from typing import Dict, List, Any, Optional
from pathlib import Path
import argparse
import subprocess
import logging
from dataclasses import dataclass, asdict
from statistics import mean, median, stdev
import matplotlib.pyplot as plt
import seaborn as sns
from scipy import stats

# Configure logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

@dataclass
class PerformanceMetric:
    """Represents a single performance metric measurement"""
    name: str
    value: float
    unit: str
    timestamp: datetime
    commit_sha: str
    branch: str
    environment: str
    metadata: Dict[str, Any] = None

@dataclass
class BenchmarkResult:
    """Represents a complete benchmark result"""
    scenario: str
    timestamp: datetime
    commit_sha: str
    branch: str
    environment: str

    # Response time metrics
    p50_latency: float
    p95_latency: float
    p99_latency: float
    avg_latency: float

    # Throughput metrics
    requests_per_second: float
    total_requests: int
    successful_requests: int
    success_rate: float

    # Resource metrics
    avg_cpu_percent: float
    max_cpu_percent: float
    avg_memory_percent: float
    max_memory_percent: float
    memory_growth_mb: float

    # Additional metadata
    metadata: Dict[str, Any] = None

class PerformanceDatabase:
    """Manages historical performance data storage and retrieval"""

    def __init__(self, db_path: str = "./performance_db"):
        self.db_path = Path(db_path)
        self.db_path.mkdir(exist_ok=True)
        self.metrics_file = self.db_path / "metrics.jsonl"
        self.benchmarks_file = self.db_path / "benchmarks.jsonl"
        self.baselines_file = self.db_path / "baselines.json"

    def save_metric(self, metric: PerformanceMetric):
        """Save a performance metric"""
        with open(self.metrics_file, 'a') as f:
            json.dump(asdict(metric), f, default=str)
            f.write('\n')

    def save_benchmark_result(self, result: BenchmarkResult):
        """Save a benchmark result"""
        with open(self.benchmarks_file, 'a') as f:
            json.dump(asdict(result), f, default=str)
            f.write('\n')

    def load_metrics(self, name: str = None, days: int = 30) -> pd.DataFrame:
        """Load historical metrics"""
        if not self.metrics_file.exists():
            return pd.DataFrame()

        data = []
        cutoff_date = datetime.now() - timedelta(days=days)

        with open(self.metrics_file, 'r') as f:
            for line in f:
                try:
                    record = json.loads(line.strip())
                    record['timestamp'] = pd.to_datetime(record['timestamp'])
                    if record['timestamp'] >= cutoff_date:
                        if name is None or record['name'] == name:
                            data.append(record)
                except json.JSONDecodeError:
                    continue

        return pd.DataFrame(data)

    def load_benchmarks(self, scenario: str = None, days: int = 90) -> pd.DataFrame:
        """Load historical benchmark results"""
        if not self.benchmarks_file.exists():
            return pd.DataFrame()

        data = []
        cutoff_date = datetime.now() - timedelta(days=days)

        with open(self.benchmarks_file, 'r') as f:
            for line in f:
                try:
                    record = json.loads(line.strip())
                    record['timestamp'] = pd.to_datetime(record['timestamp'])
                    if record['timestamp'] >= cutoff_date:
                        if scenario is None or record['scenario'] == scenario:
                            data.append(record)
                except json.JSONDecodeError:
                    continue

        return pd.DataFrame(data)

    def update_baselines(self, scenario: str, baseline_data: Dict[str, Any]):
        """Update baseline performance data"""
        baselines = {}
        if self.baselines_file.exists():
            with open(self.baselines_file, 'r') as f:
                baselines = json.load(f)

        baselines[scenario] = {
            **baseline_data,
            'last_updated': datetime.now().isoformat(),
            'sample_size': baseline_data.get('sample_size', 10)
        }

        with open(self.baselines_file, 'w') as f:
            json.dump(baselines, f, indent=2)

    def get_baseline(self, scenario: str) -> Optional[Dict[str, Any]]:
        """Get baseline performance data for a scenario"""
        if not self.baselines_file.exists():
            return None

        with open(self.baselines_file, 'r') as f:
            baselines = json.load(f)

        return baselines.get(scenario)

class RegressionDetector:
    """Detects performance regressions using statistical analysis"""

    def __init__(self, db: PerformanceDatabase):
        self.db = db
        self.confidence_level = 0.95
        self.min_samples = 5

    def detect_regression(self, scenario: str, current_result: BenchmarkResult) -> Dict[str, Any]:
        """Detect performance regression for a scenario"""
        # Load historical data
        historical_data = self.db.load_benchmarks(scenario, days=90)

        if len(historical_data) < self.min_samples:
            return {
                'regression_detected': False,
                'reason': f'Insufficient historical data ({len(historical_data)} < {self.min_samples})',
                'confidence': 0.0
            }

        # Analyze each metric for regression
        regressions = {}
        metrics_to_check = [
            ('p95_latency', 'higher'),
            ('avg_latency', 'higher'),
            ('success_rate', 'lower'),
            ('requests_per_second', 'lower')
        ]

        for metric, direction in metrics_to_check:
            if metric not in historical_data.columns:
                continue

            current_value = getattr(current_result, metric)
            historical_values = historical_data[metric].dropna().values

            if len(historical_values) < self.min_samples:
                continue

            # Calculate baseline statistics
            baseline_mean = np.mean(historical_values)
            baseline_std = np.std(historical_values, ddof=1)

            if baseline_std == 0:
                # No variation in historical data
                deviation = abs(current_value - baseline_mean)
                threshold = baseline_mean * 0.1  # 10% deviation threshold
                is_regression = deviation > threshold
            else:
                # Use statistical test
                if direction == 'higher':
                    # Check if current value is significantly higher than baseline
                    z_score = (current_value - baseline_mean) / baseline_std
                    is_regression = z_score > stats.norm.ppf(self.confidence_level)
                else:
                    # Check if current value is significantly lower than baseline
                    z_score = (baseline_mean - current_value) / baseline_std
                    is_regression = z_score > stats.norm.ppf(self.confidence_level)

            if is_regression:
                regressions[metric] = {
                    'current': current_value,
                    'baseline_mean': baseline_mean,
                    'baseline_std': baseline_std,
                    'deviation': current_value - baseline_mean if direction == 'higher' else baseline_mean - current_value,
                    'direction': direction
                }

        return {
            'regression_detected': len(regressions) > 0,
            'regressed_metrics': regressions,
            'confidence': self.confidence_level,
            'analysis_timestamp': datetime.now().isoformat()
        }

class PerformanceAnalyzer:
    """Analyzes performance data and provides optimization recommendations"""

    def __init__(self, db: PerformanceDatabase):
        self.db = db

    def analyze_bottlenecks(self, benchmark_result: BenchmarkResult) -> List[str]:
        """Analyze benchmark result for bottlenecks"""
        bottlenecks = []

        # CPU bottlenecks
        if benchmark_result.max_cpu_percent > 90:
            bottlenecks.append("Critical: CPU utilization exceeds 90% - optimize compute-intensive operations")
        elif benchmark_result.avg_cpu_percent > 70:
            bottlenecks.append("Warning: High CPU utilization (>70%) - consider optimization")

        # Memory bottlenecks
        if benchmark_result.max_memory_percent > 90:
            bottlenecks.append("Critical: Memory utilization exceeds 90% - check for memory leaks")
        elif benchmark_result.memory_growth_mb > 100:
            bottlenecks.append("Warning: Significant memory growth detected - investigate potential leaks")

        # Performance bottlenecks
        if benchmark_result.success_rate < 0.95:
            bottlenecks.append(f"Warning: Low success rate ({benchmark_result.success_rate:.1%}) - investigate failures")

        if benchmark_result.p95_latency > 5000:  # 5 seconds
            bottlenecks.append(f"Warning: High P95 latency ({benchmark_result.p95_latency:.0f}ms) - optimize response times")

        if benchmark_result.requests_per_second < 10:  # Very low throughput
            bottlenecks.append("Warning: Low throughput - check system capacity or bottlenecks")

        return bottlenecks

    def generate_recommendations(self, benchmark_result: BenchmarkResult,
                               historical_data: pd.DataFrame) -> List[str]:
        """Generate optimization recommendations"""
        recommendations = []

        # Analyze scaling patterns
        if len(historical_data) > 3:
            throughput_trend = historical_data['requests_per_second'].pct_change().mean()
            if throughput_trend < -0.1:
                recommendations.append("Performance degrading over time - investigate root cause")
            elif throughput_trend > 0.1:
                recommendations.append("Performance improving - continue current optimization efforts")

        # Resource-based recommendations
        if benchmark_result.avg_cpu_percent > 70:
            recommendations.extend([
                "Profile CPU-intensive code paths using perf or flame graphs",
                "Consider GPU acceleration for ML operations (embeddings, NER)",
                "Implement request queuing for peak loads",
                "Optimize database queries and add proper indexing"
            ])

        if benchmark_result.avg_memory_percent > 80:
            recommendations.extend([
                "Implement memory-bounded caches and queues",
                "Profile memory allocations and optimize data structures",
                "Add garbage collection tuning for long-running processes",
                "Implement memory leak detection in CI/CD"
            ])

        # Database performance recommendations
        if benchmark_result.p95_latency > 2000:
            recommendations.extend([
                "Add database query result caching (Redis)",
                "Optimize vector similarity searches with proper indexing",
                "Implement connection pooling tuning",
                "Consider read replicas for query-heavy workloads"
            ])

        # Scalability recommendations
        if benchmark_result.success_rate < 0.95:
            recommendations.extend([
                "Implement circuit breakers for external API calls",
                "Add retry logic with exponential backoff",
                "Set up proper error handling and monitoring",
                "Consider horizontal scaling with load balancer"
            ])

        return list(set(recommendations))  # Remove duplicates

class PerformanceRegressionTest:
    """Main performance regression testing framework"""

    def __init__(self, base_url: str = "http://localhost:3001", db_path: str = "./performance_db"):
        self.base_url = base_url
        self.db = PerformanceDatabase(db_path)
        self.detector = RegressionDetector(self.db)
        self.analyzer = PerformanceAnalyzer(self.db)

        # Test scenarios
        self.scenarios = {
            'api_health': {
                'endpoint': '/api/monitoring/health',
                'method': 'GET',
                'expected_status': 200,
                'timeout': 5
            },
            'literature_search': {
                'endpoint': '/api/search',
                'method': 'GET',
                'timeout': 30
            },
            'target_discovery': {
                'endpoint': '/api/targets',
                'method': 'GET',
                'timeout': 60
            },
            'kg_query': {
                'endpoint': '/api/kg',
                'method': 'GET',
                'timeout': 10
            },
            'chat_query': {
                'endpoint': '/api/chat',
                'method': 'POST',
                'payload': {'message': 'Find oncology targets', 'thread_id': 'perf_test'},
                'timeout': 30
            }
        }

    def get_git_info(self) -> Dict[str, str]:
        """Get current git commit and branch information"""
        try:
            commit_sha = subprocess.check_output(['git', 'rev-parse', 'HEAD']).decode().strip()
            branch = subprocess.check_output(['git', 'rev-parse', '--abbrev-ref', 'HEAD']).decode().strip()
            return {'commit_sha': commit_sha, 'branch': branch}
        except subprocess.CalledProcessError:
            return {'commit_sha': 'unknown', 'branch': 'unknown'}

    def run_single_test(self, scenario_name: str) -> Dict[str, Any]:
        """Run a single performance test scenario"""
        scenario = self.scenarios[scenario_name]
        start_time = time.time()

        try:
            if scenario['method'] == 'GET':
                response = requests.get(f"{self.base_url}{scenario['endpoint']}",
                                      timeout=scenario['timeout'])
            else:
                response = requests.post(f"{self.base_url}{scenario['endpoint']}",
                                       json=scenario.get('payload', {}),
                                       timeout=scenario['timeout'])

            latency = (time.time() - start_time) * 1000  # ms

            return {
                'success': response.status_code == scenario.get('expected_status', 200),
                'status_code': response.status_code,
                'latency_ms': latency,
                'response_size': len(response.content),
                'error': None
            }

        except Exception as e:
            latency = (time.time() - start_time) * 1000

            return {
                'success': False,
                'status_code': None,
                'latency_ms': latency,
                'response_size': 0,
                'error': str(e)
            }

    def run_load_test(self, scenario_name: str, concurrent_users: int = 10,
                     duration_seconds: int = 60) -> BenchmarkResult:
        """Run a load test for a scenario"""
        logger.info(f"Running load test: {scenario_name} with {concurrent_users} users for {duration_seconds}s")

        git_info = self.get_git_info()
        start_time = time.time()

        # Monitor system resources
        cpu_samples = []
        memory_samples = []
        initial_memory = psutil.virtual_memory().used / 1024 / 1024

        results = []

        # Simple load test implementation
        for i in range(concurrent_users * 10):  # 10 requests per user equivalent
            result = self.run_single_test(scenario_name)
            results.append(result)

            # Sample system resources
            cpu_samples.append(psutil.cpu_percent(interval=0.1))
            memory_samples.append(psutil.virtual_memory().percent)

            time.sleep(duration_seconds / (concurrent_users * 10))

        end_time = time.time()
        final_memory = psutil.virtual_memory().used / 1024 / 1024
        memory_growth = final_memory - initial_memory

        # Calculate metrics
        latencies = [r['latency_ms'] for r in results]
        successful_requests = sum(1 for r in results if r['success'])

        return BenchmarkResult(
            scenario=scenario_name,
            timestamp=datetime.now(),
            commit_sha=git_info['commit_sha'],
            branch=git_info['branch'],
            environment=os.getenv('ENVIRONMENT', 'development'),

            p50_latency=median(latencies),
            p95_latency=sorted(latencies)[int(len(latencies) * 0.95)],
            p99_latency=sorted(latencies)[int(len(latencies) * 0.99)] if len(latencies) >= 100 else max(latencies),
            avg_latency=mean(latencies),

            requests_per_second=len(results) / (end_time - start_time),
            total_requests=len(results),
            successful_requests=successful_requests,
            success_rate=successful_requests / len(results),

            avg_cpu_percent=mean(cpu_samples),
            max_cpu_percent=max(cpu_samples),
            avg_memory_percent=mean(memory_samples),
            max_memory_percent=max(memory_samples),
            memory_growth_mb=memory_growth
        )

    def run_regression_test(self, scenarios: List[str] = None) -> Dict[str, Any]:
        """Run complete regression test suite"""
        if scenarios is None:
            scenarios = list(self.scenarios.keys())

        logger.info(f"Starting performance regression test for scenarios: {scenarios}")

        results = {}
        regressions = {}
        recommendations = []

        for scenario in scenarios:
            logger.info(f"Testing scenario: {scenario}")

            # Run benchmark
            benchmark_result = self.run_load_test(scenario, concurrent_users=5, duration_seconds=30)
            self.db.save_benchmark_result(benchmark_result)

            # Detect regressions
            regression_analysis = self.detector.detect_regression(scenario, benchmark_result)

            # Analyze bottlenecks and generate recommendations
            bottlenecks = self.analyzer.analyze_bottlenecks(benchmark_result)
            historical_data = self.db.load_benchmarks(scenario, days=30)
            scenario_recommendations = self.analyzer.generate_recommendations(benchmark_result, historical_data)

            results[scenario] = {
                'benchmark_result': asdict(benchmark_result),
                'regression_analysis': regression_analysis,
                'bottlenecks': bottlenecks,
                'recommendations': scenario_recommendations
            }

            if regression_analysis['regression_detected']:
                regressions[scenario] = regression_analysis

            recommendations.extend(scenario_recommendations)

        # Overall analysis
        overall_regressions = len(regressions)
        overall_success = overall_regressions == 0

        summary = {
            'timestamp': datetime.now().isoformat(),
            'git_info': self.get_git_info(),
            'overall_success': overall_success,
            'total_scenarios': len(scenarios),
            'regressed_scenarios': overall_regressions,
            'results': results,
            'recommendations': list(set(recommendations))
        }

        # Save summary
        summary_file = self.db.db_path / f"regression_test_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
        with open(summary_file, 'w') as f:
            json.dump(summary, f, indent=2, default=str)

        logger.info(f"Regression test completed. Success: {overall_success}")
        if not overall_success:
            logger.warning(f"Performance regressions detected in {overall_regressions} scenarios")

        return summary

    def generate_report(self, test_results: Dict[str, Any]) -> str:
        """Generate a human-readable performance report"""
        report = []
        report.append("# Performance Regression Test Report")
        report.append(f"**Date:** {test_results['timestamp']}")
        report.append(f"**Commit:** {test_results['git_info']['commit_sha'][:8]}")
        report.append(f"**Branch:** {test_results['git_info']['branch']}")
        report.append("")

        report.append("## Overall Status")
        status = "✅ PASS" if test_results['overall_success'] else "❌ FAIL"
        report.append(f"**Result:** {status}")
        report.append(f"**Scenarios Tested:** {test_results['total_scenarios']}")
        report.append(f"**Regressions Detected:** {test_results['regressed_scenarios']}")
        report.append("")

        # Detailed results
        report.append("## Detailed Results")
        for scenario, data in test_results['results'].items():
            benchmark = data['benchmark_result']
            regression = data['regression_analysis']

            report.append(f"### {scenario}")
            report.append("**Performance Metrics:**")
            report.append(f"- Requests/sec: {benchmark.get('requests_per_second', 0):.1f}")
            report.append(f"- P95 Latency: {benchmark.get('p95_latency', 0):.1f}ms")
            report.append(f"- Success Rate: {benchmark.get('success_rate', 0):.1%}")
            report.append(f"- CPU Usage: {benchmark.get('avg_cpu_percent', 0):.1f}%")
            report.append("")

            if regression['regression_detected']:
                report.append("**🚨 REGRESSION DETECTED**")
                for metric, details in regression['regressed_metrics'].items():
                    report.append(f"- {metric}: {details['current']:.2f} (baseline: {details['baseline_mean']:.2f})")
                report.append("")

            if data['bottlenecks']:
                report.append("**Bottlenecks:**")
                for bottleneck in data['bottlenecks']:
                    report.append(f"- {bottleneck}")
                report.append("")

        # Recommendations
        if test_results['recommendations']:
            report.append("## Optimization Recommendations")
            for rec in test_results['recommendations']:
                report.append(f"- {rec}")
            report.append("")

        return "\n".join(report)

def main():
    parser = argparse.ArgumentParser(description="Ferrumyx Performance Regression Testing")
    parser.add_argument("--url", default="http://localhost:3001", help="Base URL of Ferrumyx server")
    parser.add_argument("--db-path", default="./performance_db", help="Performance database path")
    parser.add_argument("--scenarios", nargs="*", help="Specific scenarios to test")
    parser.add_argument("--output-dir", default="./performance_reports", help="Output directory")
    parser.add_argument("--ci-mode", action="store_true", help="CI mode - exit with error code on regression")

    args = parser.parse_args()

    # Ensure output directory exists
    Path(args.output_dir).mkdir(exist_ok=True)

    # Initialize regression tester
    tester = PerformanceRegressionTest(args.url, args.db_path)

    # Run regression test
    try:
        results = tester.run_regression_test(args.scenarios)

        # Generate and save report
        report = tester.generate_report(results)
        report_file = Path(args.output_dir) / f"performance_report_{datetime.now().strftime('%Y%m%d_%H%M%S')}.md"
        with open(report_file, 'w') as f:
            f.write(report)

        print(f"Performance report saved to: {report_file}")

        # Print summary to console
        print("\n" + "="*60)
        print("PERFORMANCE REGRESSION TEST SUMMARY")
        print("="*60)
        print(f"Result: {'PASS' if results['overall_success'] else 'FAIL'}")
        print(f"Scenarios tested: {results['total_scenarios']}")
        print(f"Regressions detected: {results['regressed_scenarios']}")

        if not results['overall_success']:
            print("\n❌ Performance regressions detected!")
            for scenario, data in results['results'].items():
                if data['regression_analysis']['regression_detected']:
                    print(f"  - {scenario}: Regression in {list(data['regression_analysis']['regressed_metrics'].keys())}")
        else:
            print("\n✅ No performance regressions detected")

        # Exit with error code in CI mode if regressions found
        if args.ci_mode and not results['overall_success']:
            exit(1)

    except Exception as e:
        logger.error(f"Regression test failed: {e}")
        if args.ci_mode:
            exit(1)
        raise

if __name__ == "__main__":
    main()