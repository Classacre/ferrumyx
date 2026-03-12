//! Self-improvement metrics dashboard.

use std::collections::HashSet;

use axum::{extract::State, response::Html};

use crate::handlers::dashboard::NAV_HTML;
use crate::state::SharedState;
use ferrumyx_db::{kg_facts::KgFactRepository, target_scores::TargetScoreRepository};

pub async fn metrics_page(State(state): State<SharedState>) -> Html<String> {
    let fact_repo = KgFactRepository::new(state.db.clone());
    let score_repo = TargetScoreRepository::new(state.db.clone());

    let facts = fact_repo.list(0, 80_000).await.unwrap_or_default();
    let scores = score_repo.list(0, 50_000).await.unwrap_or_default();

    let fact_count = facts.len() as f64;
    let scored_count = scores.len() as f64;

    let mut gene_subjects = HashSet::new();
    for f in &facts {
        if is_gene_like(&f.subject_name) {
            gene_subjects.insert(f.subject_id);
        }
    }
    let gene_subject_count = gene_subjects.len() as f64;

    let primary_count = scores
        .iter()
        .filter(|s| s.shortlist_tier.eq_ignore_ascii_case("primary"))
        .count() as f64;

    let avg_conf_adj = if scored_count > 0.0 {
        scores
            .iter()
            .map(|s| s.confidence_adjusted_score)
            .sum::<f64>()
            / scored_count
    } else {
        0.0
    };

    let target_score_coverage = if gene_subject_count > 0.0 {
        (scored_count / gene_subject_count).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let primary_tier_rate = if scored_count > 0.0 {
        (primary_count / scored_count).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let evidence_per_target = if scored_count > 0.0 {
        (fact_count / scored_count / 100.0).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let conflicts = match state
        .db
        .connection()
        .open_table(ferrumyx_db::schema::TABLE_KG_CONFLICTS)
        .execute()
        .await
    {
        Ok(table) => table.count_rows(None).await.unwrap_or(0) as f64,
        Err(_) => 0.0,
    };
    let conflict_rate = if fact_count > 0.0 {
        (conflicts / fact_count).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let mut metrics: Vec<(String, f64, String)> = vec![
        (
            "target_score_coverage".to_string(),
            target_score_coverage,
            "target_scores + kg_facts".to_string(),
        ),
        (
            "avg_confidence_adjusted".to_string(),
            avg_conf_adj,
            "target_scores".to_string(),
        ),
        (
            "primary_tier_rate".to_string(),
            primary_tier_rate,
            "target_scores".to_string(),
        ),
        (
            "evidence_density".to_string(),
            evidence_per_target,
            "kg_facts / target_scores".to_string(),
        ),
        (
            "kg_conflict_rate".to_string(),
            conflict_rate,
            "kg_conflicts / kg_facts".to_string(),
        ),
    ];

    metrics.sort_by(|a, b| a.0.cmp(&b.0));

    let mut weight_history: Vec<(String, String)> = scores
        .iter()
        .map(|s| ("auto".to_string(), s.created_at.to_rfc3339()))
        .collect();
    weight_history.sort_by(|a, b| b.1.cmp(&a.1));
    weight_history.truncate(10);

    let metrics_cards = if metrics.is_empty() {
        r#"<div class="alert alert-info">
            No computed metrics yet. Metrics become available after KG facts and target scores are produced.
        </div>"#
            .to_string()
    } else {
        metrics
            .iter()
            .map(|(name, value, source)| {
                let (label, target, good) = metric_meta(name);
                let pct = (value * 100.0).min(100.0) as u32;
                let bar_class = if good { "bg-success" } else { "bg-danger" };
                format!(
                    r#"
            <div class="col-md-6 col-lg-4">
                <div class="metric-card">
                    <div class="metric-label">{}</div>
                    <div class="metric-value">{:.4}</div>
                    <div class="progress mt-2" style="height:6px">
                        <div class="progress-bar {}" style="width:{}%"></div>
                    </div>
                    <div class="d-flex justify-between mt-1">
                        <small class="text-muted">Target: {}</small>
                        <small class="text-muted">Source: {}</small>
                    </div>
                </div>
            </div>"#,
                    label, value, bar_class, pct, target, source
                )
            })
            .collect()
    };

    let weight_rows: String = if weight_history.is_empty() {
        r#"<tr><td colspan="2" class="text-center text-muted py-3">No autonomous score updates recorded yet.</td></tr>"#.to_string()
    } else {
        weight_history
            .iter()
            .map(|(approver, ts)| {
                format!(
                    r#"<tr><td class="text-muted small">{}</td><td><span class="badge bg-info text-dark">{}</span></td></tr>"#,
                    ts, approver
                )
            })
            .collect()
    };

    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Metrics — Ferrumyx</title>
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
    </style>
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">
                <svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" viewBox="0 0 24 24"><path d="M3.5 18.49l6-6.01 4 4L22 6.92l-1.41-1.41-7.09 7.97-4-4L2 16.99z"/></svg>
                Self-Improvement Metrics
            </h1>
            <p class="text-muted">Feedback loop performance — explicit, measurable signals only</p>
        </div>
    </div>

    <div class="card mb-4">
        <div class="card-body d-flex align-center gap-3">
            <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="var(--accent-blue)" stroke-width="2"><path d="M12 2l7 4v6c0 5-3.5 8-7 10-3.5-2-7-5-7-10V6l7-4z"/><path d="M9 12h6"/></svg>
            <div class="text-muted">
                Autonomous feedback mode enabled: ranking updates are derived from stored evidence and score outputs.
            </div>
        </div>
    </div>

    <h5 class="mb-3 mt-4" style="font-family:'Outfit'">Current Metric Values</h5>
    <div class="grid-2 mb-4">{}</div>

    <div class="grid-2 mt-4">
        <div class="card h-100">
            <div class="card-header d-flex align-center gap-2">
                <span>Current Metric Values</span>
                <span class="info-tip">i
                    <span class="tooltip-card">
                        <strong class="text-main">Metric Definitions</strong><br>
                        <strong>target_score_coverage</strong>: scored targets / gene-like KG subjects.<br>
                        <strong>avg_confidence_adjusted</strong>: mean confidence-adjusted score from target_scores.<br>
                        <strong>primary_tier_rate</strong>: share of targets in primary tier.<br>
                        <strong>evidence_density</strong>: KG evidence per scored target (normalized).<br>
                        <strong>kg_conflict_rate</strong>: conflict rows relative to KG fact volume.
                    </span>
                </span>
            </div>
            <div class="card-body text-muted" style="font-size:0.92rem;">
                Hover the info icon for definition details. This keeps the dashboard focused while preserving metric context.
            </div>
        </div>
        <div class="card h-100">
            <div class="card-header">Score Update History</div>
            <div class="table-container p-0">
                <table class="table mb-0">
                    <thead><tr><th>Timestamp</th><th>Update Mode</th></tr></thead>
                    <tbody>{}</tbody>
                </table>
            </div>
        </div>
    </div>
</main>
<script src="/static/js/main.js"></script>
</body>
</html>"#,
        NAV_HTML, metrics_cards, weight_rows
    ))
}

fn metric_meta(name: &str) -> (&'static str, &'static str, bool) {
    match name {
        "target_score_coverage" => ("Target Score Coverage", "> 0.50", true),
        "avg_confidence_adjusted" => ("Avg Confidence-Adjusted Score", "> 0.40", true),
        "primary_tier_rate" => ("Primary Tier Rate", "> 0.10", true),
        "evidence_density" => ("Evidence Density", "> 0.20", true),
        "kg_conflict_rate" => ("KG Conflict Rate", "< 0.20", false),
        _ => ("Unknown Metric", "—", true),
    }
}

fn is_gene_like(name: &str) -> bool {
    let n = name.trim();
    if n.is_empty() || n.len() > 16 || n.contains(' ') {
        return false;
    }
    n.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && n.chars().any(|c| c.is_ascii_uppercase())
}
