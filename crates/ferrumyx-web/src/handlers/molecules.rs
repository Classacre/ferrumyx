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
                Some(v) if *v < -8.0 => "success fw-bold",
                Some(v) if *v < -7.0 => "warning",
                _ => "muted",
            };
            let smiles_short = if smiles.len() > 40 {
                format!("{}…", &smiles[..40])
            } else {
                smiles.clone()
            };
            format!(r#"<tr>
                <td style="font-family: monospace; font-size: 0.9rem;" title="{}">{}</td>
                <td style="font-weight:700; color:var(--text-main);">{}</td>
                <td style="color:var(--{});">{} kcal/mol</td>
                <td>{}</td>
                <td class="text-muted small">{}</td>
            </tr>"#, smiles, smiles_short, gene_sym, vina_class, vina_fmt, gnina_fmt, ts)
        }).collect()
    };

    Html(format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Molecules — Ferrumyx</title>
    <link rel="stylesheet" href="/static/css/main.css?v=1.0.1">
</head>
<body>
<div class="app-container">
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">
                <svg width="36" height="36" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M13 11.14V4h1c.55 0 1-.45 1-1s-.45-1-1-1H10c-.55 0-1 .45-1 1s.45 1 1 1h1v7.14l-4.75 6.42A2.003 2.003 0 0 0 7.85 21h8.3a2.003 2.003 0 0 0 1.6-3.44L13 11.14z"/></svg>
                Molecular Docking Engine
            </h1>
            <p class="text-muted">Docking results, ADMET scores, and generated ligand configurations</p>
        </div>
    </div>

    <div class="grid-2 mb-4">
        <div class="stat-card card-hover">
            <div class="stat-icon">
                <svg width="20" height="20" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M7 14c-1.66 0-3 1.34-3 3 0 1.31.84 2.41 2 2.83V21h2v-1.17c1.16-.42 2-1.52 2-2.83 0-1.66-1.34-3-3-3zm0 4c-.55 0-1-.45-1-1s.45-1 1-1 1 .45 1 1-.45 1-1 1zm10-4c-1.66 0-3 1.34-3 3 0 1.31.84 2.41 2 2.83V21h2v-1.17c1.16-.42 2-1.52 2-2.83 0-1.66-1.34-3-3-3zm0 4c-.55 0-1-.45-1-1s.45-1 1-1 1 .45 1 1-.45 1-1 1zM7 3C5.34 3 4 4.34 4 6c0 1.31.84 2.41 2 2.83V11h2V8.83C9.16 8.41 10 7.31 10 6c0-1.66-1.34-3-3-3zm0 4c-.55 0-1-.45-1-1s.45-1 1-1 1 .45 1 1-.45 1-1 1zm10-4c-1.66 0-3 1.34-3 3 0 1.31.84 2.41 2 2.83V11h2V8.83c1.16-.42 2-1.52 2-2.83 0-1.66-1.34-3-3-3zm0 4c-.55 0-1-.45-1-1s.45-1 1-1 1 .45 1 1-.45 1-1 1zM11 11h2v2h-2z"/></svg>
            </div>
            <div class="stat-value text-gradient">{}</div>
            <div class="stat-label">Total Candidate Ligands</div>
        </div>
        <div class="stat-card card-hover">
            <div class="stat-icon">
                <svg width="20" height="20" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"/></svg>
            </div>
            <div class="stat-value text-gradient">{}</div>
            <div class="stat-label">Verified Protein-Ligand Pockets</div>
        </div>
    </div>

    <form class="d-flex gap-3 mb-4 align-center" method="GET" action="/molecules">
        <input type="text" name="gene" class="form-control" style="max-width:300px"
               placeholder="Filter pipeline by target gene..." value="{}">
        <button type="submit" class="btn btn-primary">Filter Run Log</button>
    </form>

    <div class="card">
        <div class="card-header">
            <div>Molecular Docking Simulations</div>
        </div>
        <div class="table-container">
            <table class="table">
                <thead>
                    <tr>
                        <th>SMILES Reference</th>
                        <th>Target Pocket</th>
                        <th>Binding Affinity (Vina)</th>
                        <th>CNN Score (Gnina)</th>
                        <th>Timestamp</th>
                    </tr>
                </thead>
                <tbody>{}</tbody>
            </table>
        </div>
    </div>
</main>
<script src="/static/js/main.js"></script>
</div>
</body>
</html>"#, NAV_HTML, total_mols, total_docking, gene, result_rows))
}
