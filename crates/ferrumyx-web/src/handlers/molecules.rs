//! Molecule pipeline viewer — docking results and candidate molecules.

use axum::{
    extract::{Query, State},
    response::Html,
    Json,
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

use crate::handlers::dashboard::NAV_HTML;
use crate::state::SharedState;
use ferrumyx_db::{entities::EntityRepository, kg_facts::KgFactRepository, schema::EntityType};
use ferrumyx_molecules::pipeline::MoleculesPipeline;

#[derive(Deserialize)]
pub struct MolRunParams {
    pub uniprot_id: String,
}

pub async fn api_molecules_run(
    State(_state): State<SharedState>,
    axum::extract::Json(payload): axum::extract::Json<MolRunParams>,
) -> Json<serde_json::Value> {
    let pipeline = MoleculesPipeline::new(".kilocode/cache");
    match pipeline.run(&payload.uniprot_id).await {
        Ok(results) => Json(serde_json::json!({ "status": "success", "results": results })),
        Err(e) => {
            let err: anyhow::Error = e.into();
            Json(serde_json::json!({ "status": "error", "error": err.to_string() }))
        }
    }
}

#[derive(Deserialize, Default)]
pub struct MolFilter {
    pub gene: Option<String>,
}

pub async fn molecules_page(
    State(state): State<SharedState>,
    Query(filter): Query<MolFilter>,
) -> Html<String> {
    let gene = filter
        .gene
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_string();

    let entity_repo = EntityRepository::new(state.db.clone());
    let kg_repo = KgFactRepository::new(state.db.clone());

    let compounds = entity_repo
        .find_by_type(EntityType::Chemical)
        .await
        .unwrap_or_default();

    let total_mols = compounds.len() as u64;

    let docking_facts = kg_repo
        .list_filtered(
            if gene.is_empty() { None } else { Some(&gene) },
            None,
            None,
            1200,
        )
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|f| is_dockingish_predicate(&f.predicate))
        .collect::<Vec<_>>();

    let total_docking = docking_facts.len() as u64;

    #[derive(Default)]
    struct DockAggregate {
        raw_conf_sum: f64,
        evidence_count: usize,
        paper_ids: HashSet<uuid::Uuid>,
        latest_at: Option<chrono::DateTime<chrono::Utc>>,
    }

    let mut grouped: HashMap<(String, String, String), DockAggregate> = HashMap::new();
    for fact in &docking_facts {
        let key = (
            fact.subject_name.clone(),
            fact.object_name.clone(),
            fact.predicate.clone(),
        );
        let entry = grouped.entry(key).or_default();
        entry.raw_conf_sum += fact.confidence as f64;
        entry.evidence_count += 1;
        if !fact.paper_id.is_nil() {
            entry.paper_ids.insert(fact.paper_id);
        }
        let is_newer = entry
            .latest_at
            .map(|ts| fact.created_at > ts)
            .unwrap_or(true);
        if is_newer {
            entry.latest_at = Some(fact.created_at);
        }
    }

    let mut rows: Vec<(String, String, String, f32, usize, usize, String)> = grouped
        .into_iter()
        .map(|((subject, object, predicate), agg)| {
            let mean_raw = if agg.evidence_count > 0 {
                (agg.raw_conf_sum / agg.evidence_count as f64) as f32
            } else {
                0.0
            };
            let calibrated = calibrated_confidence(
                &predicate,
                mean_raw,
                agg.evidence_count,
                agg.paper_ids.len(),
            );
            (
                subject,
                object,
                predicate,
                calibrated,
                agg.evidence_count,
                agg.paper_ids.len(),
                agg.latest_at
                    .map(|ts| ts.to_rfc3339())
                    .unwrap_or_else(|| "n/a".to_string()),
            )
        })
        .collect();
    rows.sort_by(|a, b| {
        b.3.partial_cmp(&a.3)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.4.cmp(&a.4))
            .then_with(|| b.5.cmp(&a.5))
    });
    rows.truncate(250);

    let result_rows: String = if rows.is_empty() {
        r#"<tr><td colspan="7" class="text-center text-muted py-4">
            No docking-style KG facts available yet. Populate molecule pipeline outputs to render this table.
        </td></tr>"#
            .to_string()
    } else {
        rows.iter()
            .map(
                |(subject, object, predicate, confidence, support, papers, ts)| {
                    let class = if *confidence >= 0.8 {
                        "success"
                    } else if *confidence >= 0.5 {
                        "warning"
                    } else {
                        "danger"
                    };
                    format!(
                        r#"<tr>
                <td style="font-family: monospace; font-size: 0.9rem;">{}</td>
                <td style="font-weight:700; color:var(--text-main);">{}</td>
                <td><span class="badge badge-outline">{}</span></td>
                <td style="color:var(--{});">{:.3}</td>
                <td class="text-muted">{}</td>
                <td class="text-muted">{}</td>
                <td class="text-muted small">{}</td>
            </tr>"#,
                        html_escape(subject),
                        html_escape(object),
                        html_escape(predicate),
                        class,
                        confidence,
                        support,
                        papers,
                        ts
                    )
                },
            )
            .collect()
    };

    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Molecules - Ferrumyx</title>
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@400;500;600;700&family=Inter:wght@400;500;600&display=swap" rel="stylesheet">
    <link rel="stylesheet" href="/static/css/main.css?v=1.0.3">
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">
                <svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" viewBox="0 0 24 24"><path d="M11 2v4.07C7.38 6.55 4.55 9.38 4.07 13H2v-2c0-3.86 3.14-7 7-7zm.3 6V2.3A9.975 9.975 0 0 1 20.3 11H16.3c-.45-1.92-2-3.47-3.92-3.92zM4.07 15C4.55 18.62 7.38 21.45 11 21.93V17.9c-1.92-.45-3.47-2-3.92-3.92H4.07zM15 11v2h5.7c-.42 3.86-3.42 6.86-7.28 7.28V15h-2v5.7C5.56 20.28 2 16.56 2 12V6.3c.42-3.86 3.42-6.86 7.28-7.28v2h2v-2C16.44 2.72 20 6.44 20 11h-5z"/></svg>
                Molecular Docking Engine
            </h1>
            <p class="text-muted">Docking and compound-readiness signals from persisted KG and entity outputs</p>
        </div>
    </div>

    <div class="grid-2 mb-4">
        <div class="stat-card card-hover">
            <div class="stat-value text-gradient">{}</div>
            <div class="stat-label">Chemical Entities</div>
        </div>
        <div class="stat-card card-hover">
            <div class="stat-value text-gradient">{}</div>
            <div class="stat-label">Docking-Related KG Facts</div>
        </div>
    </div>

    <form class="d-flex gap-3 mb-4 align-center" method="GET" action="/molecules">
        <input type="text" name="gene" class="form-control" style="max-width:300px"
               placeholder="Filter by target/entity term" value="{}">
        <button type="submit" class="btn btn-primary">Filter</button>
    </form>

    <div class="card">
        <div class="card-header">
            <div>Molecular Evidence Snapshot</div>
        </div>
        <details style="border-top:1px solid var(--border-glass);">
            <summary style="cursor:pointer; padding:0.8rem 1rem; color:var(--text-main); font-weight:600; font-family:'Outfit',sans-serif;">Evidence Rows (Expand)</summary>
            <div class="table-container">
                <table class="table">
                    <thead>
                        <tr>
                            <th>Source Entity</th>
                            <th>Target/Compound</th>
                            <th>Relation</th>
                            <th>Confidence</th>
                            <th>Support Facts</th>
                            <th>Evidence Papers</th>
                            <th>Recorded At</th>
                        </tr>
                    </thead>
                    <tbody>{}</tbody>
                </table>
            </div>
        </details>
    </div>
</main>
<script src="/static/js/main.js"></script>
</body>
</html>"#,
        NAV_HTML,
        total_mols,
        total_docking,
        html_escape(&gene),
        result_rows
    ))
}

fn is_dockingish_predicate(predicate: &str) -> bool {
    let p = predicate.to_ascii_lowercase();
    p.contains("bind")
        || p.contains("inhib")
        || p.contains("dock")
        || p.contains("affinity")
        || p.contains("ligand")
}

fn calibrated_confidence(
    predicate: &str,
    raw: f32,
    evidence_count: usize,
    paper_count: usize,
) -> f32 {
    let mut c = raw.clamp(0.0, 1.0);
    let p = predicate.to_ascii_lowercase();
    if p == "mentions" || p == "associated_with" {
        c *= 0.62;
    } else if p.contains("bind") || p.contains("dock") || p.contains("inhib") {
        c *= 1.04;
    } else {
        c *= 0.86;
    }
    let support = ((evidence_count as f32).ln_1p() / 12.0_f32.ln_1p()).clamp(0.0, 1.0);
    let paper_support = ((paper_count as f32).ln_1p() / 8.0_f32.ln_1p()).clamp(0.0, 1.0);
    let blended = (c * 0.68) + (support * 0.20) + (paper_support * 0.12);
    blended.clamp(0.05, 0.995)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
