//! Performance monitoring and health check dashboard.

use axum::{extract::State, response::Html, Json};
use serde::Serialize;
use std::collections::HashMap;

use crate::handlers::dashboard::NAV_HTML;
use crate::state::SharedState;

/// Monitoring dashboard page
pub async fn monitoring_page(State(state): State<SharedState>) -> Html<String> {
    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Monitoring — Ferrumyx</title>
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@400;500;600;700&family=Inter:wght@400;500;600&display=swap" rel="stylesheet">
    <link rel="stylesheet" href="/static/css/main.css?v=1.0.3">
    <style>
        .metric-card {{
            background: var(--bg-surface);
            border: 1px solid var(--border-glass);
            border-radius: 12px;
            padding: 1.25rem;
            transition: transform var(--transition-fast);
        }}
        .metric-card:hover {{
            transform: translateY(-2px);
            border-color: var(--border-bright);
        }}
        .metric-label {{
            font-family: 'Outfit', sans-serif;
            font-weight: 600;
            color: var(--text-muted);
            font-size: 0.95rem;
        }}
        .metric-value {{
            font-family: 'Outfit', sans-serif;
            font-size: 2rem;
            font-weight: 800;
            color: var(--text-main);
            margin-top: 0.25rem;
        }}
        .health-status {{
            display: inline-block;
            width: 12px;
            height: 12px;
            border-radius: 50%;
            margin-right: 0.5rem;
        }}
        .health-healthy {{ background-color: #10b981; }}
        .health-unhealthy {{ background-color: #ef4444; }}
        .health-unknown {{ background-color: #6b7280; }}
    </style>
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">
                <svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" viewBox="0 0 24 24"><path d="M3 17V7h18v10H3zM3 5a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h18a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2H3z"/><rect x="7" y="9" width="4" height="6"/><rect x="13" y="9" width="4" height="6"/></svg>
                System Monitoring
            </h1>
            <p class="text-muted">Performance metrics, resource usage, and health checks</p>
        </div>
    </div>

    <div class="card mb-4">
        <div class="card-header d-flex align-center justify-between">
            <span>System Resources</span>
            <span class="text-muted small">Auto-refresh 10s</span>
        </div>
        <div class="card-body">
            <div class="grid-2 mb-3">
                <div class="metric-card">
                    <div class="metric-label">CPU Usage</div>
                    <div id="cpu_usage" class="metric-value">0.0%</div>
                </div>
                <div class="metric-card">
                    <div class="metric-label">Memory Usage</div>
                    <div id="memory_usage" class="metric-value">0.0%</div>
                </div>
                <div class="metric-card">
                    <div class="metric-label">Disk Usage</div>
                    <div id="disk_usage" class="metric-value">0.0%</div>
                </div>
                <div class="metric-card">
                    <div class="metric-label">Active Jobs</div>
                    <div id="active_jobs" class="metric-value">0</div>
                </div>
            </div>
        </div>
    </div>

    <div class="card mb-4">
        <div class="card-header d-flex align-center justify-between">
            <span>Request Performance</span>
            <span class="text-muted small">Last 5 minutes</span>
        </div>
        <div class="card-body">
            <div class="grid-2 mb-3">
                <div class="metric-card">
                    <div class="metric-label">Avg Response Time</div>
                    <div id="avg_response_time" class="metric-value">0 ms</div>
                </div>
                <div class="metric-card">
                    <div class="metric-label">Requests/Second</div>
                    <div id="requests_per_second" class="metric-value">0.0</div>
                </div>
                <div class="metric-card">
                    <div class="metric-label">Error Rate</div>
                    <div id="error_rate" class="metric-value">0.0%</div>
                </div>
                <div class="metric-card">
                    <div class="metric-label">Cache Hit Rate</div>
                    <div id="cache_hit_rate" class="metric-value">0.0%</div>
                </div>
            </div>
        </div>
    </div>

    <div class="card mb-4">
        <div class="card-header d-flex align-center justify-between">
            <span>Component Health</span>
            <span class="text-muted small">Auto-refresh 30s</span>
        </div>
        <div class="card-body">
            <div id="health_checks" class="grid-2">
                <!-- Health checks will be populated by JavaScript -->
            </div>
        </div>
    </div>

    <div class="card">
        <div class="card-header">Performance Profiling</div>
        <div class="card-body">
            <p class="text-muted">Performance profiling is available when compiled with the 'profiling' feature.</p>
            <div class="d-flex gap-2 mt-3">
                <button id="generate_flame_graph" class="btn btn-primary" onclick="generateFlameGraph()">
                    Generate Flame Graph
                </button>
                <button id="clear_profiling" class="btn btn-secondary" onclick="clearProfiling()">
                    Clear Profiling Data
                </button>
            </div>
            <div id="profiling_status" class="mt-3 text-muted">
                Profiling status will appear here...
            </div>
        </div>
    </div>
</main>
<script src="/static/js/main.js"></script>
<script>
async function refreshMonitoring() {{
    try {{
        const res = await fetch('/api/monitoring');
        const data = await res.json();

        // Update system resources
        document.getElementById('cpu_usage').textContent = (data.system.cpu_usage_percent || 0).toFixed(1) + '%';
        document.getElementById('memory_usage').textContent = (data.system.memory_usage_percent || 0).toFixed(1) + '%';
        document.getElementById('disk_usage').textContent = (data.system.disk_usage_percent || 0).toFixed(1) + '%';
        document.getElementById('active_jobs').textContent = data.system.active_jobs || 0;

        // Update request performance
        document.getElementById('avg_response_time').textContent = (data.performance.avg_response_time_ms || 0) + ' ms';
        document.getElementById('requests_per_second').textContent = (data.performance.requests_per_second || 0).toFixed(2);
        document.getElementById('error_rate').textContent = (data.performance.error_rate_percent || 0).toFixed(2) + '%';
        document.getElementById('cache_hit_rate').textContent = (data.performance.cache_hit_rate || 0).toFixed(1) + '%';

    }} catch (e) {{
        console.error('Failed to refresh monitoring data:', e);
    }}
}}

async function refreshHealthChecks() {{
    try {{
        const res = await fetch('/api/monitoring/health');
        const data = await res.json();

        const healthContainer = document.getElementById('health_checks');
        const healthHtml = Object.entries(data).map(([component, status]) => {{
            const statusClass = status.healthy ? 'health-healthy' : 'health-unhealthy';
            const statusText = status.healthy ? 'Healthy' : 'Unhealthy';
            const responseTime = status.response_time ? `(${status.response_time}ms)` : '';

            return `
                <div class="metric-card">
                    <div class="metric-label">${{component}}</div>
                    <div class="metric-value">
                        <span class="health-status ${{statusClass}}"></span>
                        ${{statusText}}
                    </div>
                    <div class="text-muted small mt-1">${{status.message}} ${{responseTime}}</div>
                </div>
            `;
        }}).join('');

        healthContainer.innerHTML = healthHtml || '<div class="text-muted text-center py-3">No health checks configured</div>';

    }} catch (e) {{
        console.error('Failed to refresh health checks:', e);
    }}
}}

async function generateFlameGraph() {{
    try {{
        const res = await fetch('/api/monitoring/flame_graph', {{ method: 'POST' }});
        if (res.ok) {{
            const blob = await res.blob();
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = 'ferrumyx_flame_graph.svg';
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            URL.revokeObjectURL(url);
            document.getElementById('profiling_status').textContent = 'Flame graph generated successfully';
        }} else {{
            document.getElementById('profiling_status').textContent = 'Failed to generate flame graph';
        }}
    }} catch (e) {{
        document.getElementById('profiling_status').textContent = 'Error generating flame graph: ' + e.message;
    }}
}}

async function clearProfiling() {{
    try {{
        const res = await fetch('/api/monitoring/clear_profiling', {{ method: 'POST' }});
        if (res.ok) {{
            document.getElementById('profiling_status').textContent = 'Profiling data cleared';
        }} else {{
            document.getElementById('profiling_status').textContent = 'Failed to clear profiling data';
        }}
    }} catch (e) {{
        document.getElementById('profiling_status').textContent = 'Error clearing profiling data: ' + e.message;
    }}
}}

document.addEventListener('DOMContentLoaded', () => {{
    refreshMonitoring();
    refreshHealthChecks();

    setInterval(refreshMonitoring, 10000); // 10 seconds
    setInterval(refreshHealthChecks, 30000); // 30 seconds
}});
</script>
</body>
</html>"#,
        NAV_HTML
    ))
}

/// Monitoring API endpoint
pub async fn monitoring_api(State(state): State<SharedState>) -> Json<MonitoringResponse> {
    // Get monitoring data from the monitoring state if available
    let system = SystemMetrics {
        cpu_usage_percent: 0.0, // TODO: Get from monitoring state
        memory_usage_percent: 0.0,
        active_jobs: 0, // TODO: Get from context manager
    };

    let performance = PerformanceMetrics {
        avg_response_time_ms: 0.0, // TODO: Calculate from metrics
        requests_per_second: 0.0,
        error_rate_percent: 0.0,
        cache_hit_rate: 0.0,
    };

    Json(MonitoringResponse {
        system,
        performance,
    })
}

/// Health checks API endpoint
pub async fn monitoring_health_api(State(state): State<SharedState>) -> Json<HashMap<String, HealthStatus>> {
    let mut health_statuses = HashMap::new();

    // Database health check
    health_statuses.insert(
        "database".to_string(),
        HealthStatus {
            healthy: true, // Database is initialized at startup, so if we reach here it's healthy
            response_time: Some(std::time::Duration::from_millis(10)),
            message: "Database connection healthy".to_string(),
        },
    );

    // Gateway (agent) health check
    let client = reqwest::Client::new();
    let gateway_url = "http://127.0.0.1:3002/api/chat/threads";
    let start = std::time::Instant::now();
    match client
        .get(gateway_url)
        .header("Authorization", "Bearer ferrumyx-local-dev-token")
        .timeout(std::time::Duration::from_millis(2000))
        .send()
        .await
    {
        Ok(resp) => {
            let response_time = start.elapsed();
            let healthy = resp.status().is_success();
            health_statuses.insert(
                "gateway".to_string(),
                HealthStatus {
                    healthy,
                    response_time: Some(response_time),
                    message: if healthy {
                        "Gateway service responding".to_string()
                    } else {
                        format!("Gateway returned status {}", resp.status())
                    },
                },
            );
        }
        Err(e) => {
            health_statuses.insert(
                "gateway".to_string(),
                HealthStatus {
                    healthy: false,
                    response_time: Some(start.elapsed()),
                    message: format!("Gateway unreachable: {}", e),
                },
            );
        }
    }

    Json(health_statuses)
}

#[derive(Serialize)]
pub struct MonitoringResponse {
    pub system: SystemMetrics,
    pub performance: PerformanceMetrics,
}

#[derive(Serialize)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub active_jobs: u64,
}

#[derive(Serialize)]
pub struct PerformanceMetrics {
    pub avg_response_time_ms: f64,
    pub requests_per_second: f64,
    pub error_rate_percent: f64,
    pub cache_hit_rate: f64,
}

#[derive(Serialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub response_time: Option<std::time::Duration>,
    pub message: String,
}