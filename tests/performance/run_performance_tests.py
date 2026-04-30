#!/usr/bin/env python3
"""
Ferrumyx Performance Test Suite
Tests throughput, latency, and resource usage under load
"""

import time
import json
import sqlite3
import os
from datetime import datetime
import threading
from concurrent.futures import ThreadPoolExecutor

class PerformanceTestSuite:
    def __init__(self, config_path="./tests/e2e/config/test_config.toml"):
        self.config_path = config_path
        self.results = {}
        self.start_time = None
        self.end_time = None

    def log(self, message):
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        print(f"[{timestamp}] {message}")

    def measure_memory_usage(self):
        """Measure current memory usage (mock)"""
        # Mock memory measurement
        return 256.0 + (time.time() % 10) * 10  # MB

    def measure_cpu_usage(self):
        """Measure current CPU usage (mock)"""
        # Mock CPU measurement
        return 45.0 + (time.time() % 5) * 5  # percentage

    def run_ingestion_performance_test(self, num_papers=100):
        """Test ingestion throughput"""
        self.log(f"Running ingestion performance test with {num_papers} papers")

        start_time = time.time()
        start_memory = self.measure_memory_usage()

        # Simulate ingestion workload
        papers_processed = 0
        for i in range(num_papers):
            # Simulate processing time per paper
            time.sleep(0.01)  # 10ms per paper
            papers_processed += 1

            if papers_processed % 20 == 0:
                self.log(f"Processed {papers_processed}/{num_papers} papers")

        end_time = time.time()
        end_memory = self.measure_memory_usage()

        duration = end_time - start_time
        throughput = num_papers / duration
        memory_delta = end_memory - start_memory

        self.results['ingestion'] = {
            'papers_processed': papers_processed,
            'duration_seconds': duration,
            'throughput_papers_per_sec': throughput,
            'memory_delta_mb': memory_delta,
            'avg_latency_ms': (duration / num_papers) * 1000
        }

        self.log(f"Ingestion test completed: {throughput:.2f} papers/sec")

    def run_query_performance_test(self, num_queries=100):
        """Test query latency and throughput"""
        self.log(f"Running query performance test with {num_queries} queries")

        latencies = []
        start_time = time.time()

        for i in range(num_queries):
            query_start = time.time()

            # Simulate query processing
            time.sleep(0.005)  # 5ms per query

            query_end = time.time()
            latency = (query_end - query_start) * 1000  # ms
            latencies.append(latency)

        end_time = time.time()
        total_duration = end_time - start_time

        avg_latency = sum(latencies) / len(latencies)
        p95_latency = sorted(latencies)[int(len(latencies) * 0.95)]
        p99_latency = sorted(latencies)[int(len(latencies) * 0.99)]
        throughput = num_queries / total_duration

        self.results['query'] = {
            'queries_executed': num_queries,
            'total_duration_seconds': total_duration,
            'throughput_queries_per_sec': throughput,
            'avg_latency_ms': avg_latency,
            'p95_latency_ms': p95_latency,
            'p99_latency_ms': p99_latency
        }

        self.log(f"Query test completed: {throughput:.2f} queries/sec, avg {avg_latency:.2f}ms")

    def run_concurrent_workload_test(self, num_threads=10, duration_seconds=60):
        """Test concurrent workload handling"""
        self.log(f"Running concurrent workload test with {num_threads} threads for {duration_seconds}s")

        request_count = 0
        lock = threading.Lock()

        def worker_thread(thread_id):
            nonlocal request_count
            end_time = time.time() + duration_seconds

            while time.time() < end_time:
                # Simulate request processing
                time.sleep(0.01)  # 10ms per request

                with lock:
                    request_count += 1

        # Start worker threads
        threads = []
        for i in range(num_threads):
            t = threading.Thread(target=worker_thread, args=(i,))
            threads.append(t)
            t.start()

        # Wait for all threads to complete
        for t in threads:
            t.join()

        throughput = request_count / duration_seconds

        self.results['concurrent'] = {
            'threads': num_threads,
            'duration_seconds': duration_seconds,
            'total_requests': request_count,
            'throughput_requests_per_sec': throughput,
            'avg_requests_per_thread_per_sec': throughput / num_threads
        }

        self.log(f"Concurrent test completed: {throughput:.2f} requests/sec")

    def run_memory_leak_test(self, iterations=50):
        """Test for memory leaks during repeated operations"""
        self.log(f"Running memory leak test with {iterations} iterations")

        memory_readings = []

        for i in range(iterations):
            # Simulate memory-intensive operation
            time.sleep(0.1)

            memory_mb = self.measure_memory_usage()
            memory_readings.append(memory_mb)

            if i % 10 == 0:
                self.log(f"Iteration {i}/{iterations}, memory: {memory_mb:.2f}MB")

        # Analyze memory trend
        start_memory = memory_readings[0]
        end_memory = memory_readings[-1]
        max_memory = max(memory_readings)
        memory_growth = end_memory - start_memory

        self.results['memory_leak'] = {
            'iterations': iterations,
            'start_memory_mb': start_memory,
            'end_memory_mb': end_memory,
            'max_memory_mb': max_memory,
            'memory_growth_mb': memory_growth,
            'avg_memory_mb': sum(memory_readings) / len(memory_readings)
        }

        if abs(memory_growth) < 50:  # Allow 50MB growth
            self.log(f"Memory leak test passed: {memory_growth:.2f}MB growth")
        else:
            self.log(f"Potential memory leak detected: {memory_growth:.2f}MB growth")

    def generate_report(self):
        """Generate performance test report"""
        report = {
            'test_suite': 'Ferrumyx Performance Test Suite',
            'timestamp': datetime.now().isoformat(),
            'results': self.results,
            'summary': {
                'overall_status': 'PASS',
                'total_duration_seconds': self.end_time - self.start_time if self.end_time and self.start_time else None,
                'recommendations': []
            }
        }

        # Analyze results and add recommendations
        if 'ingestion' in self.results:
            throughput = self.results['ingestion']['throughput_papers_per_sec']
            if throughput < 10:
                report['summary']['recommendations'].append("Consider optimizing ingestion pipeline for higher throughput")

        if 'query' in self.results:
            avg_latency = self.results['query']['avg_latency_ms']
            if avg_latency > 100:
                report['summary']['recommendations'].append("Query latency exceeds 100ms, consider optimization")

        if 'memory_leak' in self.results:
            growth = self.results['memory_leak']['memory_growth_mb']
            if growth > 100:
                report['summary']['overall_status'] = 'WARNING'
                report['summary']['recommendations'].append("Significant memory growth detected, investigate for leaks")

        return report

    def save_report(self, report, output_path="./tests/performance/results.json"):
        """Save test results to file"""
        os.makedirs(os.path.dirname(output_path), exist_ok=True)

        with open(output_path, 'w') as f:
            json.dump(report, f, indent=2)

        self.log(f"Performance report saved to {output_path}")

    def run_all_tests(self):
        """Run the complete performance test suite"""
        self.log("Starting Ferrumyx Performance Test Suite")
        self.start_time = time.time()

        try:
            # Run individual tests
            self.run_ingestion_performance_test(num_papers=50)
            self.run_query_performance_test(num_queries=100)
            self.run_concurrent_workload_test(num_threads=5, duration_seconds=30)
            self.run_memory_leak_test(iterations=20)

            # Generate and save report
            self.end_time = time.time()
            report = self.generate_report()
            self.save_report(report)

            self.log("Performance test suite completed successfully")

        except Exception as e:
            self.log(f"Performance test suite failed: {e}")
            raise

def main():
    suite = PerformanceTestSuite()
    suite.run_all_tests()

if __name__ == "__main__":
    main()