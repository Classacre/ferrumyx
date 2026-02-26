//! Molecule pipeline viewer — docking results and candidate molecules.

use axum::{extract::{State, Query}, response::Html, Json};
use serde::Deserialize;
use crate::state::SharedState;
use crate::handlers::dashboard::NAV_HTML;
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
pub struct MolFilter { pub gene: Option<String> }

pub async fn molecules_page(
    State(_state): State<SharedState>,
    Query(filter): Query<MolFilter>,
) -> Html<String> {
    let gene = filter.gene.as_deref().unwrap_or_default();

    // Placeholder values - would need molecules/docking tables
    let total_mols: u64 = 0;
    let total_docking: u64 = 0;
    let docking_results: Vec<(String, String, Option<f64>, Option<f64>, String)> = Vec::new();

    let result_rows: String = if docking_results.is_empty() {
        r#"<tr><td colspan="5" class="text-center text-muted py-4">
            No docking results yet. Run the structural analysis pipeline on a target.
        </td></tr>"#.to_string()
    } else {
        docking_results.iter().map(|(smiles, gene_sym, vina, gnina, ts)| {
            let vina_fmt = vina.map(|v| format!("{:.2}", v)).unwrap_or("—".to_string());
            let gnina_fmt = gnina.map(|v| format!("{:.3}", v)).unwrap_or("—".to_string());
            let vina_class = match vina {
                Some(v) if *v < -8.0 => "text-success fw-bold",
                Some(v) if *v < -7.0 => "text-warning",
                _ => "text-muted",
            };
            let smiles_short = if smiles.len() > 40 {
                format!("{}…", &smiles[..40])
            } else {
                smiles.clone()
            };
            format!(r#"<tr>
                <td class="font-monospace small" title="{}">{}</td>
                <td class="fw-bold">{}</td>
                <td class="{}">{} kcal/mol</td>
                <td>{}</td>
                <td class="text-muted small">{}</td>
            </tr>"#, smiles, smiles_short, gene_sym, vina_class, vina_fmt, gnina_fmt, ts)
        }).collect()
    };

    Html(format!(r#"<!DOCTYPE html>
<html lang="en" data-bs-theme="dark">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Ferrumyx — Molecules</title>
    <link rel="stylesheet" href="/static/css/main.css">
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">⚗️ Molecule Pipeline</h1>
            <p class="text-muted">Docking results, ADMET scores, and candidate molecules</p>
        </div>
    </div>

    <div class="row g-3 mb-4">
        <div class="col-md-6">
            <div class="stat-card">
                <div class="stat-icon">🧪</div>
                <div class="stat-value">{}</div>
                <div class="stat-label">Total Molecules</div>
            </div>
        </div>
        <div class="col-md-6">
            <div class="stat-card">
                <div class="stat-icon">🔬</div>
                <div class="stat-value">{}</div>
                <div class="stat-label">Docking Results</div>
            </div>
        </div>
    </div>

    <form class="d-flex gap-2 mb-4" method="GET" action="/molecules">
        <input type="text" name="gene" class="form-control" style="max-width:200px"
               placeholder="Filter by gene..." value="{}">
        <button type="submit" class="btn btn-primary">Filter</button>
    </form>

    <div class="card">
        <div class="card-header">
            <h6 class="mb-0">Docking Results</h6>
        </div>
        <div class="card-body p-0">
            <table class="table table-dark table-hover mb-0">
                <thead>
                    <tr>
                        <th>SMILES</th>
                        <th>Target Gene</th>
                        <th>Vina Score</th>
                        <th>Gnina Score</th>
                        <th>Docked At</th>
                    </tr>
                </thead>
                <tbody>{}</tbody>
            </table>
        </div>
    </div>
</main>
<script src="/static/js/main.js"></script>
</body>
</html>"#, NAV_HTML, total_mols, total_docking, gene, result_rows))
}
