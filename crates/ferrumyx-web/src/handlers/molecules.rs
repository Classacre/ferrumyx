//! Molecule pipeline viewer — docking results and candidate molecules.

use axum::{extract::{State, Query}, response::Html};
use serde::Deserialize;
use crate::state::SharedState;
use crate::handlers::dashboard::nav_html;

#[derive(Deserialize, Default)]
pub struct MolFilter { pub gene: Option<String> }

pub async fn molecules_page(
    State(state): State<SharedState>,
    Query(filter): Query<MolFilter>,
) -> Html<String> {
    let gene = filter.gene.as_deref().unwrap_or_default();

    let total_mols: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM molecules")
        .fetch_one(&state.db).await.unwrap_or(0);
    let total_docking: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM docking_results")
        .fetch_one(&state.db).await.unwrap_or(0);

    let docking_results: Vec<(String, String, Option<f64>, Option<f64>, String)> = sqlx::query_as(
        "SELECT m.smiles, COALESCE(eg.symbol, e.name) AS gene,
                dr.vina_score, dr.gnina_score,
                dr.docked_at::TEXT
         FROM docking_results dr
         JOIN molecules m ON dr.molecule_id = m.id
         JOIN entities e ON dr.target_gene_id = e.id
         LEFT JOIN ent_genes eg ON eg.id = e.id
         WHERE ($1 = '' OR eg.symbol = $1)
         ORDER BY dr.vina_score ASC NULLS LAST
         LIMIT 50"
    ).bind(gene)
     .fetch_all(&state.db)
     .await
     .unwrap_or_default();

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
        <a href="/molecules" class="btn btn-outline-secondary">Clear</a>
    </form>

    <div class="card">
        <div class="card-header">
            <h6 class="mb-0">Docking Results — ranked by Vina score (lower = better binding)</h6>
        </div>
        <div class="card-body p-0">
            <table class="table table-dark table-hover mb-0">
                <thead>
                    <tr>
                        <th>SMILES</th><th>Target Gene</th>
                        <th>Vina Score</th><th>Gnina Score</th><th>Docked At</th>
                    </tr>
                </thead>
                <tbody>{}</tbody>
            </table>
        </div>
    </div>

    <div class="alert alert-secondary mt-4">
        <strong>Note:</strong> Docking scores are computational hypotheses only.
        Vina scores &lt; −7.0 kcal/mol suggest potential binding; require wet-lab validation.
        See <a href="/system" class="alert-link">System</a> for pipeline configuration.
    </div>
</main>
<script src="/static/js/main.js"></script>
</body>
</html>"#, nav_html(), total_mols, total_docking, gene, result_rows))
}
