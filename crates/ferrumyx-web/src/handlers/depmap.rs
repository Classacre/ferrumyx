//! DepMap integration page — CRISPR dependency scores and gene essentiality.

use crate::handlers::dashboard::NAV_HTML;
use crate::state::SharedState;
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Json},
};
use ferrumyx_ranker::depmap_provider::DepMapClientAdapter;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct DepMapFilter {
    pub gene: Option<String>,
    pub cancer_type: Option<String>,
}

#[derive(Serialize)]
pub struct DepMapGeneStats {
    pub gene: String,
    pub mean_ceres: f64,
    pub median_ceres: f64,
    pub min_ceres: f64,
    pub max_ceres: f64,
    pub cell_lines_count: i64,
    pub essential_count: i64,
    pub selective_count: i64,
    pub non_essential_count: i64,
}

#[derive(Serialize)]
pub struct DepMapCellLine {
    pub cell_line: String,
    pub cancer_type: String,
    pub ceres_score: f64,
    pub expression: Option<f64>,
    pub copy_number: Option<f64>,
}

/// GET /depmap — Show DepMap integration page
pub async fn depmap_page(State(_state): State<SharedState>) -> Html<String> {
    Html(render_depmap_page(None, None))
}

/// GET /api/depmap/gene/{gene} — Get DepMap stats for a gene
pub async fn api_depmap_gene(
    State(_state): State<SharedState>,
    Query(filter): Query<DepMapFilter>,
) -> impl IntoResponse {
    let gene = filter.gene.as_deref().unwrap_or("KRAS");
    let cancer_type = filter.cancer_type.as_deref().unwrap_or("PAAD");

    let mut stats = DepMapGeneStats {
        gene: gene.to_string(),
        mean_ceres: 0.0,
        median_ceres: 0.0,
        min_ceres: 0.0,
        max_ceres: 0.0,
        cell_lines_count: 0,
        essential_count: 0,
        selective_count: 0,
        non_essential_count: 0,
    };

    if let Ok(depmap) = DepMapClientAdapter::init().await {
        let scores = depmap.client().get_gene_scores(gene, cancer_type);
        if !scores.is_empty() {
            let mut sorted = scores.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mean = scores.iter().sum::<f64>() / scores.len() as f64;
            let mid = sorted.len() / 2;
            let median = if sorted.len() % 2 == 0 {
                (sorted[mid - 1] + sorted[mid]) / 2.0
            } else {
                sorted[mid]
            };
            let min = *sorted.first().unwrap_or(&0.0);
            let max = *sorted.last().unwrap_or(&0.0);

            let mut essential = 0i64;
            let mut selective = 0i64;
            let mut non_essential = 0i64;
            for s in &scores {
                if *s < -1.0 {
                    essential += 1;
                } else if *s < -0.5 {
                    selective += 1;
                } else {
                    non_essential += 1;
                }
            }

            stats.mean_ceres = mean;
            stats.median_ceres = median;
            stats.min_ceres = min;
            stats.max_ceres = max;
            stats.cell_lines_count = scores.len() as i64;
            stats.essential_count = essential;
            stats.selective_count = selective;
            stats.non_essential_count = non_essential;
        }
    }

    Json(stats)
}

/// GET /api/depmap/celllines — Get cell line data
pub async fn api_depmap_celllines(
    State(_state): State<SharedState>,
    Query(filter): Query<DepMapFilter>,
) -> impl IntoResponse {
    let _gene = filter.gene.as_deref().unwrap_or("KRAS");
    let _cancer = filter.cancer_type.as_deref().unwrap_or("PAAD");
    // Real-data only: no fabricated cell-line rows.
    let cell_lines: Vec<DepMapCellLine> = Vec::new();
    Json(cell_lines)
}

fn render_depmap_page(_stats: Option<DepMapGeneStats>, _error: Option<String>) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <title>DepMap Integration — Ferrumyx</title>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@400;500;600;700&family=Inter:wght@400;500;600&display=swap" rel="stylesheet">
    <link rel="stylesheet" href="/static/css/main.css?v=1.0.3">
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        .essential {{ color: var(--danger); font-weight: bold; }}
        .selective {{ color: var(--warning); font-weight: bold; }}
        .non-essential {{ color: var(--success); font-weight: bold; }}
        .search-container {{ display: flex; gap: 0.55rem; flex-wrap: wrap; margin-bottom: 0.9rem; }}
        .ceres-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(130px, 1fr)); gap: 1rem; text-align: center; margin-top: 1rem; padding: 0.35rem 0.1rem 0.2rem; }}
        .ceres-val {{ font-size: 2.2rem; font-weight: 800; font-family: 'Outfit'; margin-bottom: 0.25rem; }}
        .ceres-label {{ font-size: 0.85rem; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; }}
        .method-note {{ border:1px solid var(--border-color); border-radius:12px; overflow:hidden; }}
        .method-note summary {{ cursor:pointer; list-style:none; padding: 12px 16px; font-weight:600; }}
        .method-note summary::-webkit-details-marker {{ display:none; }}
        .method-note[open] summary {{ border-bottom:1px solid var(--border-color); }}
        @media (max-width: 1180px) {{
            .search-container {{ flex-direction: column; }}
            .search-container .btn {{ width: 100%; justify-content: center; }}
        }}
    </style>
</head>
<body>
    {}
    <main class="main-content">
        <div class="page-header">
            <div>
                <h1 class="page-title">
                    <svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"/></svg>
                    DepMap CRISPR Integration
                </h1>
                <p class="text-muted">CRISPR dependency signals and cell-line essentiality from DepMap.</p>
            </div>
        </div>
        
        <div class="grid-2 align-start" style="grid-template-columns: 350px 1fr; gap: 2rem; margin-bottom: 2rem;">
            <div class="card">
                <div class="card-header border-bottom border-glass pb-3 mb-3">Gene Lookup</div>
                <div class="card-body p-1">
                    <form id="geneForm" class="mb-4">
                        <div class="search-container">
                            <input type="text" id="geneInput" class="form-control" style="flex:1" placeholder="Enter gene symbol">
                            <button class="btn btn-primary" type="submit">Run</button>
                        </div>
                    </form>
                </div>
            </div>
            
            <div class="card h-100">
                <div class="card-header border-bottom border-glass pb-3">Essentiality Snapshot: <span id="geneName" class="text-gradient" style="font-weight:700">—</span></div>
                <div class="card-body d-flex align-center justify-center p-1">
                    <div class="ceres-grid w-100">
                        <div>
                            <div class="ceres-val" style="color:var(--brand-blue)" id="meanCeres">—</div>
                            <div class="ceres-label">Mean CERES</div>
                        </div>
                        <div style="border-left: 1px solid var(--border-glass); padding-left: 1rem;">
                            <div class="ceres-val text-muted" style="color:var(--text-main) !important" id="cellLines">—</div>
                            <div class="ceres-label">Cell Lines Processed</div>
                        </div>
                        <div style="border-left: 1px solid var(--border-glass); padding-left: 1rem;">
                            <div class="ceres-val essential" id="essentialCount">—</div>
                            <div class="ceres-label">Globally Essential</div>
                        </div>
                        <div style="border-left: 1px solid var(--border-glass); padding-left: 1rem;">
                            <div class="ceres-val selective" id="selectiveCount">—</div>
                            <div class="ceres-label">Context Selective</div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        
        <div class="grid-2 gap-4">
            <div class="card h-100">
                <div class="card-header border-bottom border-glass pb-3 mb-3">CERES Distribution</div>
                <div class="card-body p-0">
                    <canvas id="distributionChart" height="200"></canvas>
                </div>
            </div>
            <div class="card h-100">
                <div class="card-header border-bottom border-glass pb-3 mb-3 d-flex justify-between">
                    <div>Cell-Line Dependencies</div>
                    <span class="badge badge-outline">Ranked</span>
                </div>
                <div class="table-container p-0">
                    <table class="table mb-0">
                        <thead>
                            <tr>
                                <th>Cell Line Origin</th>
                                <th>Pathology Classification</th>
                                <th>CERES Score</th>
                                <th>Status Marker</th>
                            </tr>
                        </thead>
                        <tbody id="cellLineTable">
                            <tr><td colspan="4" class="text-center text-muted py-4">No DepMap metrics ingested yet. Please populate downstream cell line data.</td></tr>
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
        
        <details class="method-note mt-4 mb-4">
            <summary>Method Notes</summary>
            <div class="card-body p-4">
                <p class="text-muted" style="font-size:0.95rem;">
                    DepMap (Dependency Map) provides CRISPR-Cas9 gene knockout screens across hundreds of cancer cell lines.
                    CERES scores indicate gene essentiality: more negative values correlate to higher essentiality for cellular proliferation and survival.
                </p>
                <div class="d-flex gap-4 mt-3 flex-wrap text-muted" style="font-size:0.9rem;">
                    <div style="display:flex; align-items:center; gap:0.5rem;"><div style="width:12px; height:12px; border-radius:50%; background:var(--danger)"></div> <strong>Essential (CERES < -1.0):</strong> Strong viability reduction</div>
                    <div style="display:flex; align-items:center; gap:0.5rem;"><div style="width:12px; height:12px; border-radius:50%; background:var(--warning)"></div> <strong>Selective (-1.0 < CERES < -0.5):</strong> Context-dependent</div>
                    <div style="display:flex; align-items:center; gap:0.5rem;"><div style="width:12px; height:12px; border-radius:50%; background:var(--success)"></div> <strong>Non-essential (CERES > -0.5):</strong> Minimal knockout effect</div>
                </div>
            </div>
        </details>
    </main>
    <script src="/static/js/main.js"></script>
    <script>
        // Initialize distribution chart with glass/dark theme
        Chart.defaults.color = '#9ca3af';
        Chart.defaults.borderColor = 'rgba(255, 255, 255, 0.1)';
        const ctx = document.getElementById('distributionChart').getContext('2d');
        const distributionChart = new Chart(ctx, {{
            type: 'bar',
            data: {{
                labels: ['<-1.5', '-1.5 to -1.0', '-1.0 to -0.5', '-0.5 to 0', '>0'],
                datasets: [{{
                    label: 'Cell Line Volume',
                    data: [0, 0, 0, 0, 0],
                    backgroundColor: [
                        'rgba(239, 68, 68, 0.8)', 
                        'rgba(239, 68, 68, 0.6)', 
                        'rgba(245, 158, 11, 0.8)', 
                        'rgba(16, 185, 129, 0.6)', 
                        'rgba(16, 185, 129, 0.4)'
                    ],
                    borderWidth: 0,
                    borderRadius: 4
                }}]
            }},
            options: {{
                responsive: true,
                maintainAspectRatio: false,
                plugins: {{
                    legend: {{ display: false }}
                }},
                scales: {{
                    y: {{ beginAtZero: true, grid: {{ color: 'rgba(255,255,255,0.05)' }} }},
                    x: {{ grid: {{ display: false }} }}
                }}
            }}
        }});
        
        function loadGene(selectedGene) {{
            document.getElementById('geneInput').value = selectedGene;
            document.getElementById('geneName').textContent = selectedGene;
        }}
        
        document.getElementById('geneForm').addEventListener('submit', function(e) {{
            e.preventDefault();
            const gene = document.getElementById('geneInput').value;
            loadGene(gene);
        }});
    </script>
</body>
</html>"##,
        NAV_HTML
    )
}
