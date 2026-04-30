#!/usr/bin/env python3
"""
Ferrumyx Scalability Testing Framework
Load testing for different user concurrency levels and production-scale scenarios
"""

import asyncio
import aiohttp
import json
import time
import statistics
import psutil
import os
from datetime import datetime, timedelta
from typing import Dict, List, Any, Optional, Tuple
from dataclasses import dataclass, asdict
from pathlib import Path
import logging
import argparse
import matplotlib.pyplot as plt
import pandas as pd
import numpy as np
from concurrent.futures import ThreadPoolExecutor
import threading

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

@dataclass
class LoadTestResult:
    """Results from a scalability load test"""
    concurrency_level: int
    test_duration_seconds: float
    total_requests: int
    successful_requests: int
    failed_requests: int
    success_rate: float

    # Response time percentiles
    p50_response_time: float
    p95_response_time: float
    p99_response_time: float
    avg_response_time: float
    min_response_time: float
    max_response_time: float

    # Throughput
    requests_per_second: float

    # Resource usage
    avg_cpu_percent: float
    max_cpu_percent: float
    avg_memory_percent: float
    max_memory_percent: float
    memory_growth_mb: float

    # Error breakdown
    errors_by_type: Dict[str, int]

    timestamp: datetime

@dataclass
class ScalabilityReport:
    """Comprehensive scalability analysis report"""
    test_timestamp: datetime
    base_url: str
    environment: str

    # Test configuration
    concurrency_levels: List[int]
    test_duration_per_level: int
    total_test_duration: float

    # Results for each concurrency level
    results: List[LoadTestResult]

    # Scalability analysis
    scaling_efficiency: float  # How well performance scales with concurrency
    optimal_concurrency: int   # Recommended concurrency level
    breaking_point: Optional[int]  # Point where performance degrades significantly

    # Resource analysis
    resource_utilization_trend: str
    memory_leak_detected: bool
    cpu_saturation_point: Optional[int]

    # Recommendations
    recommendations: List[str]

class AsyncLoadTester:
    """Asynchronous load testing for high concurrency"""

    def __init__(self, base_url: str, concurrency: int):
        self.base_url = base_url.rstrip('/')
        self.concurrency = concurrency
        self.results = []
        self.errors = {}
        self.start_time = None
        self.end_time = None

        # Test scenarios with different request patterns
        self.scenarios = {
            'health_check': {
                'endpoint': '/api/monitoring/health',
                'method': 'GET',
                'weight': 0.1  # 10% of requests
            },
            'literature_search': {
                'endpoint': '/api/search',
                'method': 'GET',
                'weight': 0.25  # 25% of requests
            },
            'target_discovery': {
                'endpoint': '/api/targets',
                'method': 'GET',
                'weight': 0.25  # 25% of requests
            },
            'knowledge_graph': {
                'endpoint': '/api/kg',
                'method': 'GET',
                'weight': 0.15  # 15% of requests
            },
            'chat_query': {
                'endpoint': '/api/chat',
                'method': 'POST',
                'payload': {'message': 'Analyze oncology targets for breast cancer therapy', 'thread_id': 'load_test'},
                'weight': 0.15  # 15% of requests
            },
            'ner_extraction': {
                'endpoint': '/api/ner/extract',
                'method': 'POST',
                'payload': {'text': 'BRCA1 and BRCA2 mutations are associated with increased risk of breast and ovarian cancer'},
                'weight': 0.1  # 10% of requests
            }
        }

    def select_scenario(self) -> Dict[str, Any]:
        """Select a test scenario based on weights"""
        import random
        scenarios = list(self.scenarios.keys())
        weights = [self.scenarios[s]['weight'] for s in scenarios]
        selected = random.choices(scenarios, weights=weights)[0]
        return self.scenarios[selected]

    async def make_request(self, session: aiohttp.ClientSession, request_id: int) -> Tuple[bool, float, Optional[str]]:
        """Make a single HTTP request"""
        scenario = self.select_scenario()
        url = f"{self.base_url}{scenario['endpoint']}"

        start_time = time.time()

        try:
            if scenario['method'] == 'GET':
                async with session.get(url, timeout=aiohttp.ClientTimeout(total=30)) as response:
                    await response.text()  # Consume response
                    success = response.status < 400
                    error_type = f"http_{response.status}" if not success else None
            else:
                payload = scenario.get('payload', {})
                headers = {'Content-Type': 'application/json'}
                async with session.post(url, json=payload, headers=headers,
                                      timeout=aiohttp.ClientTimeout(total=30)) as response:
                    await response.text()  # Consume response
                    success = response.status < 400
                    error_type = f"http_{response.status}" if not success else None

        except asyncio.TimeoutError:
            success = False
            error_type = "timeout"
        except aiohttp.ClientError as e:
            success = False
            error_type = f"client_error_{type(e).__name__}"
        except Exception as e:
            success = False
            error_type = f"unknown_error_{type(e).__name__}"

        response_time = (time.time() - start_time) * 1000  # ms

        return success, response_time, error_type

    async def worker(self, worker_id: int, duration: int):
        """Worker coroutine that makes requests for the specified duration"""
        async with aiohttp.ClientSession() as session:
            request_count = 0
            end_time = time.time() + duration

            while time.time() < end_time:
                success, response_time, error_type = await self.make_request(session, request_count)

                self.results.append({
                    'success': success,
                    'response_time': response_time,
                    'error_type': error_type,
                    'worker_id': worker_id,
                    'timestamp': time.time()
                })

                if error_type:
                    self.errors[error_type] = self.errors.get(error_type, 0) + 1

                request_count += 1

                # Small delay to prevent overwhelming (adjust based on needs)
                await asyncio.sleep(0.01)

    async def run_load_test(self, duration: int) -> Dict[str, Any]:
        """Run the load test with specified concurrency and duration"""
        logger.info(f"Starting async load test with {self.concurrency} concurrent users for {duration}s")

        self.start_time = time.time()

        # Start monitoring in background
        monitor_task = asyncio.create_task(self.monitor_resources(duration))

        # Create worker tasks
        tasks = []
        for i in range(self.concurrency):
            task = asyncio.create_task(self.worker(i, duration))
            tasks.append(task)

        # Wait for all workers to complete
        await asyncio.gather(*tasks, return_exceptions=True)

        # Stop monitoring
        await monitor_task

        self.end_time = time.time()
        actual_duration = self.end_time - self.start_time

        return self.analyze_results(actual_duration)

    async def monitor_resources(self, duration: int):
        """Monitor system resources during the test"""
        self.resource_samples = []
        end_time = time.time() + duration

        while time.time() < end_time:
            self.resource_samples.append({
                'timestamp': time.time(),
                'cpu_percent': psutil.cpu_percent(interval=1),
                'memory_percent': psutil.virtual_memory().percent,
                'memory_used_mb': psutil.virtual_memory().used / 1024 / 1024
            })
            await asyncio.sleep(1)

    def analyze_results(self, actual_duration: float) -> Dict[str, Any]:
        """Analyze the test results"""
        if not self.results:
            return {'error': 'No requests completed'}

        # Calculate response times
        response_times = [r['response_time'] for r in self.results]
        successful_requests = sum(1 for r in self.results if r['success'])

        # Calculate percentiles
        sorted_times = sorted(response_times)
        p50 = sorted_times[int(len(sorted_times) * 0.5)]
        p95 = sorted_times[int(len(sorted_times) * 0.95)]
        p99 = sorted_times[int(len(sorted_times) * 0.99)]

        # Resource analysis
        if hasattr(self, 'resource_samples') and self.resource_samples:
            cpu_samples = [s['cpu_percent'] for s in self.resource_samples]
            memory_samples = [s['memory_percent'] for s in self.resource_samples]
            memory_used = [s['memory_used_mb'] for s in self.resource_samples]

            avg_cpu = statistics.mean(cpu_samples)
            max_cpu = max(cpu_samples)
            avg_memory = statistics.mean(memory_samples)
            max_memory = max(memory_samples)
            memory_growth = memory_used[-1] - memory_used[0] if len(memory_used) > 1 else 0
        else:
            avg_cpu = max_cpu = avg_memory = max_memory = memory_growth = 0

        return {
            'concurrency_level': self.concurrency,
            'test_duration_seconds': actual_duration,
            'total_requests': len(self.results),
            'successful_requests': successful_requests,
            'failed_requests': len(self.results) - successful_requests,
            'success_rate': successful_requests / len(self.results),

            'p50_response_time': p50,
            'p95_response_time': p95,
            'p99_response_time': p99,
            'avg_response_time': statistics.mean(response_times),
            'min_response_time': min(response_times),
            'max_response_time': max(response_times),

            'requests_per_second': len(self.results) / actual_duration,

            'avg_cpu_percent': avg_cpu,
            'max_cpu_percent': max_cpu,
            'avg_memory_percent': avg_memory,
            'max_memory_percent': max_memory,
            'memory_growth_mb': memory_growth,

            'errors_by_type': self.errors.copy(),

            'timestamp': datetime.now()
        }

class ScalabilityAnalyzer:
    """Analyzes scalability patterns and provides optimization recommendations"""

    def __init__(self, results: List[LoadTestResult]):
        self.results = sorted(results, key=lambda x: x.concurrency_level)

    def analyze_scaling_efficiency(self) -> float:
        """Calculate scaling efficiency (0-1, higher is better)"""
        if len(self.results) < 2:
            return 1.0

        # Calculate how throughput scales with concurrency
        base_result = self.results[0]
        max_result = self.results[-1]

        actual_scaling = max_result.requests_per_second / base_result.requests_per_second
        ideal_scaling = max_result.concurrency_level / base_result.concurrency_level

        efficiency = min(actual_scaling / ideal_scaling, 1.0)
        return efficiency

    def find_optimal_concurrency(self) -> int:
        """Find the optimal concurrency level based on efficiency"""
        if len(self.results) < 2:
            return self.results[0].concurrency_level if self.results else 1

        # Find the "elbow point" where performance starts degrading
        throughputs = [r.requests_per_second for r in self.results]
        concurrencies = [r.concurrency_level for r in self.results]

        # Calculate efficiency at each point
        efficiencies = []
        for i in range(1, len(throughputs)):
            actual = throughputs[i] / throughputs[0]
            ideal = concurrencies[i] / concurrencies[0]
            efficiencies.append(min(actual / ideal, 1.0))

        # Find where efficiency drops significantly
        optimal_idx = 0
        for i, eff in enumerate(efficiencies):
            if eff < 0.8:  # 80% efficiency threshold
                break
            optimal_idx = i + 1

        return concurrencies[optimal_idx]

    def find_breaking_point(self) -> Optional[int]:
        """Find the concurrency level where performance breaks"""
        for i in range(1, len(self.results)):
            current = self.results[i]
            previous = self.results[i-1]

            # Check for significant degradation
            throughput_drop = (previous.requests_per_second - current.requests_per_second) / previous.requests_per_second
            success_drop = (previous.success_rate - current.success_rate)

            if throughput_drop > 0.3 or success_drop > 0.2:  # 30% throughput drop or 20% success drop
                return current.concurrency_level

        return None

    def analyze_resource_trends(self) -> str:
        """Analyze resource utilization trends"""
        if len(self.results) < 2:
            return "insufficient_data"

        cpu_trend = self.results[-1].avg_cpu_percent - self.results[0].avg_cpu_percent
        memory_trend = self.results[-1].avg_memory_percent - self.results[0].avg_memory_percent

        if cpu_trend > 50 or memory_trend > 30:
            return "high_resource_pressure"
        elif cpu_trend > 20 or memory_trend > 15:
            return "moderate_resource_pressure"
        else:
            return "stable_resource_usage"

    def detect_memory_leaks(self) -> bool:
        """Detect potential memory leaks"""
        memory_growths = [r.memory_growth_mb for r in self.results]
        avg_growth = statistics.mean(memory_growths)

        # If memory grows significantly with each concurrency increase, likely leak
        return avg_growth > 50  # 50MB growth threshold

    def find_cpu_saturation_point(self) -> Optional[int]:
        """Find concurrency level where CPU becomes saturated"""
        for result in self.results:
            if result.max_cpu_percent > 90:
                return result.concurrency_level
        return None

    def generate_recommendations(self) -> List[str]:
        """Generate scalability optimization recommendations"""
        recommendations = []

        efficiency = self.analyze_scaling_efficiency()
        optimal = self.find_optimal_concurrency()
        breaking = self.find_breaking_point()
        resource_trend = self.analyze_resource_trends()
        memory_leak = self.detect_memory_leaks()
        cpu_saturation = self.find_cpu_saturation_point()

        if efficiency < 0.7:
            recommendations.append("Poor scaling efficiency detected. Consider implementing horizontal scaling with load balancer.")
        elif efficiency < 0.9:
            recommendations.append("Moderate scaling efficiency. Optimize bottlenecks for better performance.")

        if breaking:
            recommendations.append(f"Performance breaking point detected at {breaking} concurrent users. Implement request queuing above this level.")

        if resource_trend == "high_resource_pressure":
            recommendations.append("High resource pressure detected. Consider vertical scaling (more CPU/memory) or horizontal scaling.")
        elif resource_trend == "moderate_resource_pressure":
            recommendations.append("Moderate resource pressure. Monitor closely and consider optimization.")

        if memory_leak:
            recommendations.append("Memory leak detected. Investigate memory allocation patterns and implement proper cleanup.")

        if cpu_saturation:
            recommendations.append(f"CPU saturation occurs at {cpu_saturation} concurrent users. Consider CPU optimization or horizontal scaling.")

        recommendations.append(f"Optimal concurrency level appears to be around {optimal} users for current configuration.")

        # Add specific technical recommendations
        recommendations.extend([
            "Implement Redis caching for frequently accessed data to reduce database load",
            "Consider using connection pooling for external API calls",
            "Implement circuit breakers for external service dependencies",
            "Add request/response compression to reduce network overhead",
            "Consider implementing request prioritization for different user types"
        ])

        return recommendations

class ScalabilityTestRunner:
    """Main scalability testing framework"""

    def __init__(self, base_url: str, output_dir: str = "./scalability_reports"):
        self.base_url = base_url
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(exist_ok=True)

    async def run_scalability_test(self, concurrency_levels: List[int] = None,
                                 duration_per_level: int = 60) -> ScalabilityReport:
        """Run complete scalability test suite"""
        if concurrency_levels is None:
            concurrency_levels = [1, 5, 10, 25, 50, 100, 200]

        logger.info(f"Starting scalability test with concurrency levels: {concurrency_levels}")

        start_time = time.time()
        results = []

        for concurrency in concurrency_levels:
            logger.info(f"Testing concurrency level: {concurrency}")

            tester = AsyncLoadTester(self.base_url, concurrency)
            test_result = await tester.run_load_test(duration_per_level)

            # Convert to LoadTestResult
            result = LoadTestResult(**test_result)
            results.append(result)

            # Brief pause between tests
            await asyncio.sleep(2)

        total_duration = time.time() - start_time

        # Analyze scalability
        analyzer = ScalabilityAnalyzer(results)

        report = ScalabilityReport(
            test_timestamp=datetime.now(),
            base_url=self.base_url,
            environment=os.getenv('ENVIRONMENT', 'development'),

            concurrency_levels=concurrency_levels,
            test_duration_per_level=duration_per_level,
            total_test_duration=total_duration,

            results=results,

            scaling_efficiency=analyzer.analyze_scaling_efficiency(),
            optimal_concurrency=analyzer.find_optimal_concurrency(),
            breaking_point=analyzer.find_breaking_point(),

            resource_utilization_trend=analyzer.analyze_resource_trends(),
            memory_leak_detected=analyzer.detect_memory_leaks(),
            cpu_saturation_point=analyzer.find_cpu_saturation_point(),

            recommendations=analyzer.generate_recommendations()
        )

        return report

    def generate_report(self, report: ScalabilityReport) -> str:
        """Generate human-readable scalability report"""
        lines = []
        lines.append("# Ferrumyx Scalability Test Report")
        lines.append(f"**Test Date:** {report.test_timestamp.strftime('%Y-%m-%d %H:%M:%S')}")
        lines.append(f"**Base URL:** {report.base_url}")
        lines.append(f"**Environment:** {report.environment}")
        lines.append("")

        lines.append("## Test Configuration")
        lines.append(f"- **Concurrency Levels:** {', '.join(map(str, report.concurrency_levels))}")
        lines.append(f"- **Duration per Level:** {report.test_duration_per_level}s")
        lines.append(f"- **Total Test Duration:** {report.total_test_duration:.1f}s")
        lines.append("")

        lines.append("## Scalability Analysis")
        lines.append(f"- **Scaling Efficiency:** {report.scaling_efficiency:.2%}")
        lines.append(f"- **Optimal Concurrency:** {report.optimal_concurrency} users")
        if report.breaking_point:
            lines.append(f"- **Breaking Point:** {report.breaking_point} users")
        else:
            lines.append("- **Breaking Point:** Not reached")
        lines.append("")

        lines.append("## Resource Analysis")
        lines.append(f"- **Resource Trend:** {report.resource_utilization_trend.replace('_', ' ').title()}")
        lines.append(f"- **Memory Leak Detected:** {'Yes' if report.memory_leak_detected else 'No'}")
        if report.cpu_saturation_point:
            lines.append(f"- **CPU Saturation Point:** {report.cpu_saturation_point} users")
        lines.append("")

        lines.append("## Performance Results")
        lines.append("| Concurrency | Requests/sec | Success Rate | P95 Latency | CPU % | Memory % |")
        lines.append("|-------------|--------------|--------------|-------------|-------|----------|")

        for result in report.results:
            lines.append("|02d")

        lines.append("")

        lines.append("## Recommendations")
        for i, rec in enumerate(report.recommendations, 1):
            lines.append(f"{i}. {rec}")
        lines.append("")

        # Add performance insights
        lines.append("## Key Insights")
        if report.scaling_efficiency > 0.9:
            lines.append("✅ Excellent scaling efficiency - system handles concurrency well")
        elif report.scaling_efficiency > 0.7:
            lines.append("⚠️ Good scaling efficiency - minor optimizations recommended")
        else:
            lines.append("❌ Poor scaling efficiency - significant improvements needed")

        if report.memory_leak_detected:
            lines.append("🚨 Memory leak detected - investigate immediately")
        else:
            lines.append("✅ No memory leaks detected")

        return "\n".join(lines)

    def create_charts(self, report: ScalabilityReport):
        """Create performance charts"""
        results_df = pd.DataFrame([asdict(r) for r in report.results])

        fig, ((ax1, ax2), (ax3, ax4)) = plt.subplots(2, 2, figsize=(15, 10))

        # Throughput scaling
        ax1.plot(results_df['concurrency_level'], results_df['requests_per_second'],
                marker='o', linewidth=2, markersize=6)
        ax1.set_title('Throughput Scaling')
        ax1.set_xlabel('Concurrent Users')
        ax1.set_ylabel('Requests per Second')
        ax1.grid(True, alpha=0.3)

        # Response time scaling
        ax2.plot(results_df['concurrency_level'], results_df['p95_response_time'],
                marker='s', color='orange', linewidth=2, markersize=6, label='P95')
        ax2.plot(results_df['concurrency_level'], results_df['avg_response_time'],
                marker='^', color='red', linewidth=2, markersize=6, label='Average')
        ax2.set_title('Response Time Scaling')
        ax2.set_xlabel('Concurrent Users')
        ax2.set_ylabel('Response Time (ms)')
        ax2.legend()
        ax2.grid(True, alpha=0.3)

        # Success rate
        ax3.plot(results_df['concurrency_level'], results_df['success_rate'] * 100,
                marker='d', color='green', linewidth=2, markersize=6)
        ax3.set_title('Success Rate vs Concurrency')
        ax3.set_xlabel('Concurrent Users')
        ax3.set_ylabel('Success Rate (%)')
        ax3.grid(True, alpha=0.3)

        # Resource usage
        ax4.plot(results_df['concurrency_level'], results_df['avg_cpu_percent'],
                marker='o', color='purple', linewidth=2, markersize=6, label='CPU %')
        ax4.plot(results_df['concurrency_level'], results_df['avg_memory_percent'],
                marker='s', color='brown', linewidth=2, markersize=6, label='Memory %')
        ax4.set_title('Resource Utilization')
        ax4.set_xlabel('Concurrent Users')
        ax4.set_ylabel('Utilization (%)')
        ax4.legend()
        ax4.grid(True, alpha=0.3)

        plt.tight_layout()
        chart_file = self.output_dir / f"scalability_analysis_{report.test_timestamp.strftime('%Y%m%d_%H%M%S')}.png"
        plt.savefig(chart_file, dpi=150, bbox_inches='tight')
        plt.close()

        logger.info(f"Scalability charts saved to: {chart_file}")
        return chart_file

    def save_report(self, report: ScalabilityReport):
        """Save scalability report and data"""
        timestamp = report.test_timestamp.strftime("%Y%m%d_%H%M%S")

        # Save JSON data
        json_file = self.output_dir / f"scalability_report_{timestamp}.json"
        with open(json_file, 'w') as f:
            # Convert dataclasses to dicts for JSON serialization
            report_dict = asdict(report)
            json.dump(report_dict, f, indent=2, default=str)

        # Save markdown report
        md_report = self.generate_report(report)
        md_file = self.output_dir / f"scalability_report_{timestamp}.md"
        with open(md_file, 'w') as f:
            f.write(md_report)

        # Create charts
        try:
            chart_file = self.create_charts(report)
            logger.info(f"Scalability analysis saved to: {md_file}")
            return md_file, chart_file
        except Exception as e:
            logger.warning(f"Could not create charts: {e}")
            return md_file, None

async def main():
    parser = argparse.ArgumentParser(description="Ferrumyx Scalability Testing Framework")
    parser.add_argument("--url", default="http://localhost:3001", help="Base URL of Ferrumyx server")
    parser.add_argument("--concurrency-levels", nargs="+", type=int,
                       default=[1, 5, 10, 25, 50, 100],
                       help="Concurrency levels to test")
    parser.add_argument("--duration", type=int, default=60,
                       help="Duration in seconds for each concurrency level")
    parser.add_argument("--output-dir", default="./scalability_reports",
                       help="Output directory for reports")

    args = parser.parse_args()

    # Create output directory
    Path(args.output_dir).mkdir(exist_ok=True)

    # Initialize test runner
    runner = ScalabilityTestRunner(args.url, args.output_dir)

    try:
        logger.info("Starting Ferrumyx scalability testing...")

        # Run scalability test
        report = await runner.run_scalability_test(
            concurrency_levels=args.concurrency_levels,
            duration_per_level=args.duration
        )

        # Save report
        md_file, chart_file = runner.save_report(report)

        # Print summary
        print("\n" + "="*60)
        print("FERRUMYX SCALABILITY TEST SUMMARY")
        print("="*60)
        print(f"Scaling Efficiency: {report.scaling_efficiency:.1%}")
        print(f"Optimal Concurrency: {report.optimal_concurrency} users")
        print(f"Breaking Point: {report.breaking_point or 'Not reached'}")
        print(f"Memory Leak Detected: {'Yes' if report.memory_leak_detected else 'No'}")
        print(f"Resource Trend: {report.resource_utilization_trend.replace('_', ' ')}")
        print("\nTop Recommendations:")
        for i, rec in enumerate(report.recommendations[:5], 1):
            print(f"{i}. {rec}")
        print(f"\nDetailed report: {md_file}")
        if chart_file:
            print(f"Performance charts: {chart_file}")

    except Exception as e:
        logger.error(f"Scalability testing failed: {e}")
        raise

if __name__ == "__main__":
    asyncio.run(main())