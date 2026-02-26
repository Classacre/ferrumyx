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
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Metrics — Ferrumyx</title>
    <link rel="stylesheet" href="/static/css/main.css">
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
    </style>
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M3.5 18.49l6-6.01 4 4L22 6.92l-1.41-1.41-7.09 7.97-4-4L2 16.99z"/></svg>
                Self-Improvement Metrics
            </h1>
            <p class="text-muted">Feedback loop performance — explicit, measurable signals only</p>
        </div>
    </div>

    <div class="card mb-4" style="border-left: 4px solid var(--warning);">
        <div class="d-flex align-center gap-3">
            <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="var(--warning)" stroke-width="2"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
            <div>
                <strong style="color:var(--text-main);">Human-gated constraint active:</strong> Weight updates require operator approval before application.
                No automatic parameter changes. See full history for audit events.
            </div>
        </div>
    </div>

    <h5 class="mb-3 mt-4" style="font-family:'Outfit'">Current Metric Values</h5>
    <div class="grid-2 mb-4">{}</div>

    <div class="grid-2 mt-4">
        <div class="card h-100">
            <div class="card-header">Metric Definitions</div>
            <div class="card-body" style="padding: 0;">
                <ul style="list-style: none; padding: 0; margin: 0; display:flex; flex-direction:column; gap: 1rem;">
                    <li><strong style="color:var(--brand-blue)">recall_at_n:</strong> Top-N targets vs DrugBank approved drugs. Target: > 0.60</li>
                    <li><strong style="color:var(--brand-blue)">docking_ic50_pearson_r:</strong> Correlation between docking scores and ChEMBL IC50. Target: > 0.45</li>
                    <li><strong style="color:var(--brand-blue)">ranking_kendall_tau:</strong> Ranking stability week-over-week. Target: > 0.80</li>
                    <li><strong style="color:var(--brand-blue)">literature_recall:</strong> % of CIViC-validated targets in top-50. Target: > 0.70</li>
                    <li><strong style="color:var(--brand-blue)">false_positive_rate:</strong> Clinically invalidated targets in shortlist. Target: < 0.20</li>
                </ul>
            </div>
        </div>
        <div class="card h-100">
            <div class="card-header">Weight Update History</div>
            <div class="table-container p-0">
                <table class="table mb-0">
                    <thead><tr><th>Timestamp</th><th>Approved By Entity</th></tr></thead>
                    <tbody>{}</tbody>
                </table>
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
