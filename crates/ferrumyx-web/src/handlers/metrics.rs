//! Self-improvement metrics dashboard.

use axum::{extract::State, response::Html, Json};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

use crate::handlers::dashboard::NAV_HTML;
use crate::state::SharedState;
use ferrumyx_db::{
    entities::EntityRepository, kg_facts::KgFactRepository, papers::PaperRepository,
    schema::EntityType, target_scores::TargetScoreRepository,
};
use ferrumyx_ingestion::pipeline::load_recent_perf_snapshots;

#[derive(Debug, Serialize)]
pub struct PerfSummaryView {
    avg_duration_ms: u64,
    avg_papers_inserted_per_min: f64,
    avg_chunks_inserted_per_min: f64,
    avg_pdf_cache_hit_rate: f64,
    avg_unique_predicates: f64,
    avg_predicate_generic_share: f64,
    predicate_coverage_flagged_runs: usize,
}

#[derive(Debug, Serialize)]
pub struct PerfSnapshotView {
    recorded_at_epoch_secs: u64,
    query: String,
    duration_ms: u64,
    papers_found: usize,
    papers_inserted: usize,
    chunks_inserted: usize,
    quality_gate_skips: usize,
    pdf_cache_hits: usize,
    pdf_cache_misses: usize,
    relation_fact_count: usize,
    unique_predicate_count: usize,
    predicate_generic_share: f64,
    predicate_coverage_flagged: bool,
}

#[derive(Debug, Serialize)]
pub struct PerfResponse {
    summary: PerfSummaryView,
    recent: Vec<PerfSnapshotView>,
}

pub async fn metrics_page(State(state): State<SharedState>) -> Html<String> {
    let fact_repo = KgFactRepository::new(state.db.clone());
    let score_repo = TargetScoreRepository::new(state.db.clone());

    let entity_repo = EntityRepository::new(state.db.clone());
    let fact_count = fact_repo.count().await.unwrap_or(0) as f64;
    let scored_count = score_repo.count().await.unwrap_or(0) as f64;
    let gene_subject_count = entity_repo
        .count_by_type(EntityType::Gene)
        .await
        .unwrap_or(0) as f64;
    let scores = score_repo.list(0, 5_000).await.unwrap_or_default();

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

    // Use a small Bayesian prior so tiny sample sizes don't render as extreme 0/1.
    let primary_tier_rate = if scored_count > 0.0 {
        let prior_strength = 8.0;
        let prior_mean = 0.25;
        ((primary_count + prior_mean * prior_strength) / (scored_count + prior_strength))
            .clamp(0.0, 1.0)
    } else {
        0.0
    };

    let evidence_per_target = if scored_count > 0.0 {
        let avg_facts_per_target = fact_count / scored_count;
        (avg_facts_per_target.ln_1p() / 1000.0_f64.ln_1p()).clamp(0.0, 1.0)
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
    let heuristic_conflict_rate = if conflicts == 0.0 && fact_count > 0.0 {
        estimate_conflict_rate_from_facts(&fact_repo).await
    } else {
        0.0
    };
    let conflict_rate = if fact_count > 0.0 {
        if conflicts > 0.0 {
            (conflicts / fact_count).clamp(0.0, 1.0)
        } else {
            heuristic_conflict_rate
        }
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
        .metrics-disclosure {{
            border: 1px solid var(--border-glass);
            border-radius: 12px;
            overflow: hidden;
            background: rgba(14, 21, 33, 0.58);
        }}
        .metrics-disclosure summary {{
            cursor: pointer;
            list-style: none;
            padding: 0.72rem 0.95rem;
            color: var(--text-main);
            font-family: 'Outfit', sans-serif;
            font-weight: 600;
        }}
        .metrics-disclosure-body {{
            border-top: 1px solid var(--border-glass);
            padding: 0.55rem 0.85rem 0.8rem;
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

    <div class="card mb-4">
        <div class="card-header d-flex align-center justify-between">
            <span>Live Ingestion Performance</span>
            <span class="text-muted small">Auto-refresh 5s</span>
        </div>
        <div class="card-body">
            <div class="grid-2 mb-3">
                <div class="metric-card"><div class="metric-label">Avg Run Duration</div><div id="perf_avg_duration" class="metric-value">0 ms</div></div>
                <div class="metric-card"><div class="metric-label">Avg Papers / Min</div><div id="perf_avg_papers" class="metric-value">0.00</div></div>
                <div class="metric-card"><div class="metric-label">Avg Chunks / Min</div><div id="perf_avg_chunks" class="metric-value">0.00</div></div>
                <div class="metric-card"><div class="metric-label">PDF Parse Hit Rate</div><div id="perf_cache_hit_rate" class="metric-value">0.0%</div></div>
            </div>
            <div class="metrics-disclosure">
                <details>
                    <summary>Recent Run Breakdown (Expand)</summary>
                    <div class="metrics-disclosure-body">
                        <div class="table-container p-0">
                            <table class="table mb-0">
                                <thead><tr><th>When</th><th>Query</th><th>Duration</th><th>Papers</th><th>Inserted</th><th>Chunks</th></tr></thead>
                                <tbody id="perf_recent_rows"><tr><td colspan="6" class="text-muted text-center py-3">No ingestion telemetry yet.</td></tr></tbody>
                            </table>
                        </div>
                    </div>
                </details>
            </div>
        </div>
    </div>

    <h5 class="mb-3 mt-4" style="font-family:'Outfit'">Current Metric Values</h5>
    <div class="grid-2 mb-4">{}</div>

    <div class="grid-2 mt-4">
        <div class="card h-100">
            <div class="card-header d-flex align-center gap-2">
                <span>Metric Definitions</span>
                <span class="info-tip">i
                    <span class="tooltip-card">
                        <strong class="text-main">Metric Definitions</strong><br>
                        <strong>target_score_coverage</strong>: scored targets / gene-like KG subjects.<br>
                        <strong>avg_confidence_adjusted</strong>: mean confidence-adjusted score from target_scores.<br>
                        <strong>primary_tier_rate</strong>: share of targets in primary tier.<br>
                        <strong>evidence_density</strong>: log-scaled KG evidence per scored target.<br>
                        <strong>kg_conflict_rate</strong>: explicit conflicts or inferred contradiction rate from predicates.
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
<script>
async function refreshPerfPanel() {{
  try {{
    const res = await fetch('/api/metrics/perf');
    const data = await res.json();
    const s = data.summary || {{}};
    const byId = (id) => document.getElementById(id);
    byId('perf_avg_duration').textContent = `${{Number(s.avg_duration_ms || 0)}} ms`;
    byId('perf_avg_papers').textContent = Number(s.avg_papers_inserted_per_min || 0).toFixed(2);
    byId('perf_avg_chunks').textContent = Number(s.avg_chunks_inserted_per_min || 0).toFixed(2);
    byId('perf_cache_hit_rate').textContent = `${{(Number(s.avg_pdf_cache_hit_rate || 0) * 100).toFixed(1)}}%`;

    const rows = (data.recent || []).map((r) => {{
      const ts = new Date((Number(r.recorded_at_epoch_secs || 0)) * 1000).toLocaleString();
      const q = String(r.query || '').slice(0, 90);
      return `<tr><td class="text-muted small">${{ts}}</td><td>${{q}}</td><td>${{r.duration_ms}} ms</td><td>${{r.papers_found}}</td><td>${{r.papers_inserted}}</td><td>${{r.chunks_inserted}}</td></tr>`;
    }}).join('');
    byId('perf_recent_rows').innerHTML = rows || '<tr><td colspan="6" class="text-muted text-center py-3">No ingestion telemetry yet.</td></tr>';
  }} catch (_) {{}}
}}
document.addEventListener('DOMContentLoaded', () => {{
  refreshPerfPanel();
  setInterval(refreshPerfPanel, 5000);
}});
</script>
</body>
</html>"#,
        NAV_HTML, metrics_cards, weight_rows
    ))
}

pub async fn metrics_perf_api(State(state): State<SharedState>) -> Json<PerfResponse> {
    let recent_raw = load_recent_perf_snapshots(24);
    let mut duration_total = 0u64;
    let mut papers_per_min_total = 0.0f64;
    let mut chunks_per_min_total = 0.0f64;
    let mut hit_total = 0usize;
    let mut miss_total = 0usize;
    let mut unique_pred_total = 0usize;
    let mut generic_share_total = 0.0f64;
    let mut coverage_flagged = 0usize;

    let recent: Vec<PerfSnapshotView> = recent_raw
        .iter()
        .map(|s| {
            duration_total += s.duration_ms;
            let minutes = (s.duration_ms as f64 / 1000.0 / 60.0).max(1.0 / 60.0);
            papers_per_min_total += s.papers_inserted as f64 / minutes;
            chunks_per_min_total += s.chunks_inserted as f64 / minutes;
            hit_total += s.pdf_cache_hits;
            miss_total += s.pdf_cache_misses;
            unique_pred_total += s.unique_predicate_count;
            generic_share_total += s.predicate_generic_share;
            if s.predicate_coverage_flagged {
                coverage_flagged += 1;
            }
            PerfSnapshotView {
                recorded_at_epoch_secs: s.recorded_at_epoch_secs,
                query: s.query.clone(),
                duration_ms: s.duration_ms,
                papers_found: s.papers_found,
                papers_inserted: s.papers_inserted,
                chunks_inserted: s.chunks_inserted,
                quality_gate_skips: s.quality_gate_skips,
                pdf_cache_hits: s.pdf_cache_hits,
                pdf_cache_misses: s.pdf_cache_misses,
                relation_fact_count: s.relation_fact_count,
                unique_predicate_count: s.unique_predicate_count,
                predicate_generic_share: s.predicate_generic_share,
                predicate_coverage_flagged: s.predicate_coverage_flagged,
            }
        })
        .collect();

    let n = recent.len().max(1) as f64;
    let fallback_parse_hit_rate = estimate_parse_success_rate(&state).await;
    let summary = PerfSummaryView {
        avg_duration_ms: (duration_total as f64 / n).round() as u64,
        avg_papers_inserted_per_min: papers_per_min_total / n,
        avg_chunks_inserted_per_min: chunks_per_min_total / n,
        avg_pdf_cache_hit_rate: if hit_total + miss_total > 0 {
            hit_total as f64 / (hit_total + miss_total) as f64
        } else {
            fallback_parse_hit_rate
        },
        avg_unique_predicates: unique_pred_total as f64 / n,
        avg_predicate_generic_share: generic_share_total / n,
        predicate_coverage_flagged_runs: coverage_flagged,
    };

    Json(PerfResponse { summary, recent })
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

async fn estimate_parse_success_rate(state: &SharedState) -> f64 {
    let repo = PaperRepository::new(state.db.clone());
    let parsed = repo.count_by_parse_status("parsed").await.unwrap_or(0) as f64;
    let parsed_fast = repo.count_by_parse_status("parsed_fast").await.unwrap_or(0) as f64;
    let parsed_light = repo
        .count_by_parse_status("parsed_light")
        .await
        .unwrap_or(0) as f64;
    let failed = repo.count_by_parse_status("failed").await.unwrap_or(0) as f64;
    let total = parsed + parsed_fast + parsed_light + failed;
    if total <= 0.0 {
        0.0
    } else {
        ((parsed + parsed_fast + parsed_light) / total).clamp(0.0, 1.0)
    }
}

async fn estimate_conflict_rate_from_facts(fact_repo: &KgFactRepository) -> f64 {
    let facts = fact_repo.list(0, 15_000).await.unwrap_or_default();
    if facts.is_empty() {
        return 0.0;
    }

    let mut predicates_by_pair: HashMap<(String, String), HashSet<String>> = HashMap::new();
    for fact in facts {
        let subject = fact.subject_name.trim().to_ascii_lowercase();
        let object = fact.object_name.trim().to_ascii_lowercase();
        let predicate = fact.predicate.trim().to_ascii_lowercase();
        if subject.is_empty() || object.is_empty() || predicate.is_empty() {
            continue;
        }
        predicates_by_pair
            .entry((subject, object))
            .or_default()
            .insert(predicate);
    }

    let contradictions = [
        ("upregulated_in", "downregulated_in"),
        ("activates", "inhibits"),
        ("confers_resistance", "sensitizes_to"),
        (
            "prognostic_for_poor_outcome",
            "prognostic_for_better_outcome",
        ),
        ("promotes_proliferation", "inhibits"),
    ];

    let mut conflicting_pairs = 0usize;
    for predicates in predicates_by_pair.values() {
        if contradictions
            .iter()
            .any(|(a, b)| predicates.contains(*a) && predicates.contains(*b))
        {
            conflicting_pairs += 1;
        }
    }

    if predicates_by_pair.is_empty() {
        0.0
    } else {
        (conflicting_pairs as f64 / predicates_by_pair.len() as f64).clamp(0.0, 1.0)
    }
}
