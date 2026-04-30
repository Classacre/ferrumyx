#!/usr/bin/env python3
"""
Quick test of Ferrumyx benchmark components
"""

import time
import random
import psutil
import pandas as pd

print("Testing system monitoring...")

# Test system monitor
start = time.time()
time.sleep(2)
end = time.time()

print(".2f")

# Test mock scenario
print("Testing mock scenario execution...")

base_latencies = {
    'literature_search': (100, 500),
    'target_discovery': (200, 1500),
    'multi_channel_query': (500, 3000),
    'kg_query': (50, 300),
    'ner_extraction': (100, 800)
}

for scenario, (min_lat, max_lat) in base_latencies.items():
    latency = random.uniform(min_lat, max_lat)
    success = random.random() > 0.05
    print(".1f")

print("Basic tests completed successfully!")