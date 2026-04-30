# Ferrumyx Application Metrics Implementation Guide

## Overview
This guide explains how to implement custom metrics in Ferrumyx services for comprehensive monitoring.

## Required Dependencies

### Rust (for ferrumyx-web and agents)
Add to `Cargo.toml`:
```toml
[dependencies]
prometheus = "0.13"
lazy_static = "1.4"
```

### Node.js (if applicable)
```json
{
  "dependencies": {
    "prom-client": "^14.0.1"
  }
}
```

## Metrics Implementation

### HTTP Request Metrics
```rust
use prometheus::{Encoder, TextEncoder, register_histogram_vec, HistogramVec};
use lazy_static::lazy_static;

lazy_static! {
    static ref HTTP_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "http_request_duration_seconds",
        "HTTP request duration in seconds",
        &["method", "route", "status"]
    ).unwrap();
}

// In your HTTP handler
let timer = HTTP_REQUEST_DURATION.with_label_values(&["GET", "/health", "200"]).start_timer();
// ... handler logic ...
timer.observe_duration();
```

### Custom Business Metrics
```rust
lazy_static! {
    static ref DISCOVERY_SUCCESS_TOTAL: IntCounter = register_int_counter!(
        "discovery_success_total",
        "Total successful discoveries"
    ).unwrap();

    static ref AGENT_JOBS_TOTAL: IntCounter = register_int_counter!(
        "agent_jobs_total",
        "Total agent jobs"
    ).unwrap();

    static ref FAILED_AUTH_TOTAL: IntCounter = register_int_counter!(
        "failed_auth_total",
        "Total failed authentication attempts"
    ).unwrap();
}

// Increment counters
DISCOVERY_SUCCESS_TOTAL.inc();
AGENT_JOBS_TOTAL.inc();
FAILED_AUTH_TOTAL.inc();
```

### Metrics Endpoint
Add a `/metrics` endpoint to expose Prometheus metrics:

```rust
use warp::Filter;

let metrics_route = warp::path("metrics")
    .map(|| {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    });
```

## Service-Specific Metrics

### IronClaw Agent
- `agent_jobs_total`: Counter for total jobs processed
- `agent_job_failures_total`: Counter for failed jobs
- `bioclaw_execution_duration_seconds`: Histogram for BioClaw execution times

### Gateway Service
- `message_sent_total`: Counter for messages sent
- `message_delivered_total`: Counter for messages delivered
- `webhook_requests_total`: Counter for webhook requests

### BioClaw WASM
- `wasm_execution_total`: Counter for WASM executions
- `wasm_violation_total`: Counter for sandbox violations

### Web Service
- `phi_access_total`: Counter for PHI data access (with proper logging)
- `active_sessions_total`: Gauge for active user sessions

## Structured Logging

Use structured logging with JSON format for better log aggregation:

```rust
use serde_json::json;
use log::{info, error};

info!("Request processed: {}", json!({
    "method": "GET",
    "path": "/api/search",
    "user_id": user_id,
    "duration_ms": duration,
    "status": 200
}));

error!("Authentication failed: {}", json!({
    "ip": client_ip,
    "user_agent": user_agent,
    "reason": "invalid_credentials"
}));
```

## Testing Metrics

Verify metrics are exposed correctly:
```bash
curl http://localhost:3000/metrics
```

Should return Prometheus-formatted metrics output.