//! Self-improvement metrics dashboard.

use axum::{extract::State, response::Html};
use crate::state::SharedState;
use crate::handlers::dashboard::NAV_HTML;

pub async fn metrics_page(State(_state): State<SharedState>) -> Html<String> {
    // Placeholder metrics - would need feedback_events table implementation
    let metrics: Vec<(String, f64, String)> = Vec::new();
    let weight_history: Vec<(String, String)> = Vec::new();

    let metrics_cards = if metrics.is_empty() {
        r#"<div class="alert alert-info">
            No feedback metrics collected yet. Metrics are computed weekly after the feedback_collection routine runs.
            The first metrics will be available after ~1 week of operation with scored targets.
        </div>"#.to_string()
    } else {
        metrics.iter().map(|(name, value, source)| {
            let (label, target, good) = metric_meta(name);
            let pct = (value * 100.0).min(100.0) as u32;
            let bar_class = if good { "bg-success" } else { "bg-danger" };
            format!(r#"
            <div class="col-md-6 col-lg-4">
                <div class="metric-card">
                    <div class="metric-label">{}</div>
                    <div class="metric-value">{:.4}</div>
                    <div class="progress mt-2" style="height:6px">
                        <div class="progress-bar {}" style="width:{}%"></div>
                    </div>
                    <div class="d-flex justify-content-between mt-1">
                        <small class="text-muted">Target: {}</small>
                        <small class="text-muted">Source: {}</small>
                    </div>
                </div>
            </div>"#, label, value, bar_class, pct, target, source)
        }).collect()
    };

    let weight_rows: String = if weight_history.is_empty() {
        r#"<tr><td colspan="2" class="text-center text-muted py-3">No weight updates yet. Requires human approval.</td></tr>"#.to_string()
    } else {
        weight_history.iter().map(|(approver, ts)| format!(
            r#"<tr><td class="text-muted small">{}</td><td><span class="badge bg-info text-dark">{}</span></td></tr>"#,
            ts, approver
        )).collect()
    };

    Html(format!(r#"<!DOCTYPE html>
<html lang="en" data-bs-theme="dark">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Ferrumyx — Self-Improvement Metrics</title>
    <link rel="stylesheet" href="/static/css/main.css">
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">📊 Self-Improvement Metrics</h1>
            <p class="text-muted">Feedback loop performance — explicit, measurable signals only</p>
        </div>
    </div>

    <div class="alert alert-secondary mb-4">
        <strong>🔒 Human-gated:</strong> Weight updates require operator approval before application.
        No automatic parameter changes. See <a href="/audit" class="alert-link">Audit Log</a> for full history.
    </div>

    <h5 class="mb-3">Current Metric Values</h5>
    <div class="row g-3">{}</div>

    <div class="row g-4 mt-2">
        <div class="col-md-6">
            <div class="card h-100">
                <div class="card-header"><h6 class="mb-0">📋 Metric Definitions</h6></div>
                <div class="card-body">
                    <ul class="list-unstyled metric-definitions">
                        <li><strong>recall_at_n:</strong> Top-N targets vs DrugBank approved drugs. Target: > 0.60</li>
                        <li><strong>docking_ic50_pearson_r:</strong> Correlation between docking scores and ChEMBL IC50. Target: > 0.45</li>
                        <li><strong>ranking_kendall_tau:</strong> Ranking stability week-over-week. Target: > 0.80</li>
                        <li><strong>literature_recall:</strong> % of CIViC-validated targets in top-50. Target: > 0.70</li>
                        <li><strong>false_positive_rate:</strong> Clinically invalidated targets in shortlist. Target: < 0.20</li>
                    </ul>
                </div>
            </div>
        </div>
        <div class="col-md-6">
            <div class="card h-100">
                <div class="card-header"><h6 class="mb-0">⚖️ Weight Update History</h6></div>
                <div class="card-body p-0">
                    <table class="table table-dark table-sm mb-0">
                        <thead><tr><th>Timestamp</th><th>Approved By</th></tr></thead>
                        <tbody>{}</tbody>
                    </table>
                </div>
            </div>
        </div>
    </div>
</main>
<script src="/static/js/main.js"></script>
</body>
</html>"#, NAV_HTML, metrics_cards, weight_rows))
}

fn metric_meta(name: &str) -> (&'static str, &'static str, bool) {
    match name {
        "recall_at_n"            => ("Recall@N (DrugBank)", "> 0.60", true),
        "docking_ic50_pearson_r" => ("Docking–IC50 Pearson r", "> 0.45", true),
        "ranking_kendall_tau"    => ("Ranking Stability (τ)", "> 0.80", true),
        "literature_recall"      => ("Literature Recall", "> 0.70", true),
        "false_positive_rate"    => ("False Positive Rate", "< 0.20", false),
        _                        => ("Unknown Metric", "—", true),
    }
}
