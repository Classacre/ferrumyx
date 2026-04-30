#!/usr/bin/env python3
"""
Memory Leak Validation Script for Ferrumyx
Tests memory stability under extended load
"""

import time
import psutil
import os
import subprocess
import signal
import sys
from datetime import datetime
from statistics import mean, stdev

class MemoryLeakValidator:
    def __init__(self):
        self.process = None
        self.memory_readings = []
        self.start_time = None

    def start_server(self):
        """Start the Ferrumyx server"""
        print("Starting Ferrumyx server...")
        try:
            self.process = subprocess.Popen(
                ["cargo", "run", "-p", "ferrumyx-web"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                cwd="/d/AI/Ferrumyx"
            )
            self.start_time = time.time()
            # Wait for server to start
            time.sleep(10)
            return True
        except Exception as e:
            print(f"Failed to start server: {e}")
            return False

    def monitor_memory(self, duration_seconds=3600):  # 1 hour
        """Monitor memory usage for specified duration"""
        print(f"Monitoring memory for {duration_seconds} seconds...")

        end_time = time.time() + duration_seconds
        while time.time() < end_time:
            if self.process.poll() is not None:
                print("Server process died")
                return False

            try:
                # Get memory info
                mem = psutil.virtual_memory()
                self.memory_readings.append({
                    'timestamp': time.time() - self.start_time,
                    'used_mb': mem.used / 1024 / 1024,
                    'percent': mem.percent
                })
            except Exception as e:
                print(f"Error monitoring memory: {e}")

            time.sleep(30)  # Sample every 30 seconds

        return True

    def analyze_results(self):
        """Analyze memory usage results"""
        if not self.memory_readings:
            return {"error": "No memory readings collected"}

        used_mb_values = [r['used_mb'] for r in self.memory_readings]

        if len(used_mb_values) < 2:
            return {"error": "Insufficient data"}

        initial_memory = used_mb_values[0]
        final_memory = used_mb_values[-1]
        growth = final_memory - initial_memory

        # Calculate trend
        timestamps = [r['timestamp'] for r in self.memory_readings]
        slope = (final_memory - initial_memory) / (timestamps[-1] - timestamps[0]) if timestamps[-1] > timestamps[0] else 0

        return {
            'initial_memory_mb': initial_memory,
            'final_memory_mb': final_memory,
            'memory_growth_mb': growth,
            'growth_rate_mb_per_hour': slope * 3600,  # Convert to per hour
            'max_memory_mb': max(used_mb_values),
            'avg_memory_mb': mean(used_mb_values),
            'memory_stdev_mb': stdev(used_mb_values) if len(used_mb_values) > 1 else 0,
            'duration_seconds': timestamps[-1] - timestamps[0],
            'readings_count': len(self.memory_readings),
            'pass_criteria': growth < 10,  # Less than 10MB growth
        }

    def cleanup(self):
        """Clean up the server process"""
        if self.process:
            try:
                self.process.terminate()
                self.process.wait(timeout=10)
            except:
                self.process.kill()
                self.process.wait()

def main():
    validator = MemoryLeakValidator()

    try:
        if not validator.start_server():
            print("Failed to start validation - server wouldn't start")
            sys.exit(1)

        if not validator.monitor_memory(duration_seconds=1800):  # 30 minutes for testing
            print("Memory monitoring failed")
            sys.exit(1)

        results = validator.analyze_results()

        print("\n" + "="*60)
        print("MEMORY LEAK VALIDATION RESULTS")
        print("="*60)

        if 'error' in results:
            print(f"ERROR: {results['error']}")
            sys.exit(1)

        print(f"Duration: {results['duration_seconds']:.1f} seconds")
        print(f"Memory readings: {results['readings_count']}")
        print(f"Initial memory: {results['initial_memory_mb']:.1f} MB")
        print(f"Final memory: {results['final_memory_mb']:.1f} MB")
        print(f"Memory growth: {results['memory_growth_mb']:.1f} MB")
        print(f"Growth rate: {results['growth_rate_mb_per_hour']:.1f} MB/hour")
        print(f"Max memory: {results['max_memory_mb']:.1f} MB")
        print(f"Average memory: {results['avg_memory_mb']:.1f} MB")

        print("\n" + "-"*40)
        if results['pass_criteria']:
            print("✅ SUCCESS: Memory growth < 10MB")
        else:
            print("❌ FAIL: Memory growth >= 10MB")

        # Save results
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        with open(f"memory_validation_{timestamp}.json", 'w') as f:
            import json
            json.dump(results, f, indent=2)

        print(f"\nDetailed results saved to memory_validation_{timestamp}.json")

    finally:
        validator.cleanup()

if __name__ == "__main__":
    main()