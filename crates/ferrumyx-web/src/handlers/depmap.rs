//! DepMap integration page — CRISPR dependency scores and gene essentiality.

use axum::{
    extract::{State, Query},
    response::{Html, IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use crate::state::SharedState;
use crate::handlers::dashboard::NAV_HTML;

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
    
    // Query from database if available, otherwise return mock data
    let stats = DepMapGeneStats {
        gene: gene.to_string(),
        mean_ceres: -0.85,
        median_ceres: -0.92,
        min_ceres: -1.45,
        max_ceres: 0.23,
        cell_lines_count: 42,
        essential_count: 38,
        selective_count: 3,
        non_essential_count: 1,
    };
    
    Json(stats)
}

/// GET /api/depmap/celllines — Get cell line data
pub async fn api_depmap_celllines(
    State(_state): State<SharedState>,
    Query(filter): Query<DepMapFilter>,
) -> impl IntoResponse {
    let _gene = filter.gene.as_deref().unwrap_or("KRAS");
    
    // Mock data for now - would query from database
    let cell_lines = vec![
        DepMapCellLine {
            cell_line: "A549".to_string(),
            cancer_type: "Lung Adenocarcinoma".to_string(),
            ceres_score: -1.23,
            expression: Some(8.5),
            copy_number: Some(1.2),
        },
        DepMapCellLine {
            cell_line: "H358".to_string(),
            cancer_type: "Lung Adenocarcinoma".to_string(),
            ceres_score: -1.15,
            expression: Some(7.8),
            copy_number: Some(1.0),
        },
        DepMapCellLine {
            cell_line: "PANC1".to_string(),
            cancer_type: "Pancreatic Adenocarcinoma".to_string(),
            ceres_score: -1.45,
            expression: Some(9.2),
            copy_number: Some(1.8),
        },
    ];
    
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
    <link rel="stylesheet" href="/static/css/main.css?v=1.0.1">
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        .essential {{ color: var(--danger); font-weight: bold; }}
        .selective {{ color: var(--warning); font-weight: bold; }}
        .non-essential {{ color: var(--success); font-weight: bold; }}
        .search-container {{ display: flex; gap: 0.5rem; flex-wrap: wrap; margin-bottom: 0.5rem; }}
        .ceres-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(130px, 1fr)); gap: 1rem; text-align: center; margin-top: 1rem; }}
        .ceres-val {{ font-size: 2.2rem; font-weight: 800; font-family: 'Outfit'; margin-bottom: 0.25rem; }}
        .ceres-label {{ font-size: 0.85rem; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; }}
    </style>
</head>
<body>
<div class="app-container">
    {}
    <main class="main-content">
        <div class="page-header">
            <div>
                <h1 class="page-title">
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z"/></svg>
                    DepMap CRISPR Integration
                </h1>
                <p class="text-muted">CRISPR-Cas9 dependency score analytics and cell line viability profiles via Broad Institute DepMap</p>
            </div>
        </div>
        
        <div class="grid-2 align-start" style="grid-template-columns: 350px 1fr; gap: 2rem; margin-bottom: 2rem;">
            <div class="card">
                <div class="card-header border-bottom border-glass pb-3 mb-3">Execute Target Search</div>
                <div class="card-body p-0">
                    <form id="geneForm" class="mb-4">
                        <div class="search-container">
                            <input type="text" id="geneInput" class="form-control" style="flex:1" placeholder="Gene symbol (e.g., KRAS)" value="KRAS">
                            <button class="btn btn-primary" type="submit">Analyze</button>
                        </div>
                    </form>
                    <div class="text-muted mb-2" style="font-size:0.85rem;">Reference Corpi:</div>
                    <div class="d-grid gap-2" style="grid-template-columns: 1fr 1fr;">
                        <button class="btn btn-sm btn-outline w-100" onclick="loadGene('KRAS')">KRAS</button>
                        <button class="btn btn-sm btn-outline w-100" onclick="loadGene('EGFR')">EGFR</button>
                        <button class="btn btn-sm btn-outline w-100" onclick="loadGene('BRCA1')">BRCA1</button>
                        <button class="btn btn-sm btn-outline w-100" onclick="loadGene('TP53')">TP53</button>
                    </div>
                </div>
            </div>
            
            <div class="card h-100">
                <div class="card-header border-bottom border-glass pb-3">Essentiality Matrix: <span id="geneName" class="text-gradient" style="font-weight:700">KRAS</span></div>
                <div class="card-body d-flex align-center justify-center p-0">
                    <div class="ceres-grid w-100">
                        <div>
                            <div class="ceres-val" style="color:var(--brand-blue)" id="meanCeres">-0.85</div>
                            <div class="ceres-label">Mean CERES</div>
                        </div>
                        <div style="border-left: 1px solid var(--border-glass); padding-left: 1rem;">
                            <div class="ceres-val text-muted" style="color:var(--text-main) !important" id="cellLines">42</div>
                            <div class="ceres-label">Cell Lines Processed</div>
                        </div>
                        <div style="border-left: 1px solid var(--border-glass); padding-left: 1rem;">
                            <div class="ceres-val essential" id="essentialCount">38</div>
                            <div class="ceres-label">Globally Essential</div>
                        </div>
                        <div style="border-left: 1px solid var(--border-glass); padding-left: 1rem;">
                            <div class="ceres-val selective" id="selectiveCount">3</div>
                            <div class="ceres-label">Context Selective</div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        
        <div class="grid-2 gap-4">
            <div class="card h-100">
                <div class="card-header border-bottom border-glass pb-3 mb-3">CERES Score Distribution Topology</div>
                <div class="card-body p-0">
                    <canvas id="distributionChart" height="200"></canvas>
                </div>
            </div>
            <div class="card h-100">
                <div class="card-header border-bottom border-glass pb-3 mb-3 d-flex justify-between">
                    <div>Ranked Cell Line Dependency</div>
                    <span class="badge badge-outline">Top Dependencies</span>
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
                            <tr>
                                <td>A549</td>
                                <td>Lung Adenocarcinoma</td>
                                <td class="essential">-1.23</td>
                                <td><span class="badge badge-danger">Essential Threshold</span></td>
                            </tr>
                            <tr>
                                <td>H358</td>
                                <td>Lung Adenocarcinoma</td>
                                <td class="essential">-1.15</td>
                                <td><span class="badge badge-danger">Essential Threshold</span></td>
                            </tr>
                            <tr>
                                <td>PANC1</td>
                                <td>Pancreatic Adenocarcinoma</td>
                                <td class="essential">-1.45</td>
                                <td><span class="badge badge-danger">Essential Threshold</span></td>
                            </tr>
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
        
        <div class="card mt-4 mb-4" style="border-left: 4px solid var(--brand-purple);">
            <div class="card-body p-4">
                <h5 class="mb-2" style="font-family:'Outfit'">DepMap Methodology Notes</h5>
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
        </div>
    </main>
</div>
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
                    data: [12, 26, 3, 1, 0],
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
