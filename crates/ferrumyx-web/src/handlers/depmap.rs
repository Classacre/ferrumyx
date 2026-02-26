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
        r##"
<!DOCTYPE html>
<html>
<head>
    <title>Ferrumyx — DepMap Integration</title>
    <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css" rel="stylesheet">
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        body {{ padding: 2rem; background: #f8f9fa; }}
        .container {{ max-width: 1400px; }}
        .card {{ margin-bottom: 1rem; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        .stat-value {{ font-size: 2rem; font-weight: bold; color: #0d6efd; }}
        .stat-label {{ color: #6c757d; }}
        .essential {{ color: #dc3545; }}
        .selective {{ color: #fd7e14; }}
        .non-essential {{ color: #198754; }}
    </style>
</head>
<body>
    {}
    <div class="container">
        <h2>🧬 DepMap Integration</h2>
        <p class="text-muted">CRISPR dependency scores and gene essentiality data</p>
        
        <div class="row mb-4">
            <div class="col-md-4">
                <div class="card">
                    <div class="card-body">
                        <h5 class="card-title">Search Gene</h5>
                        <form id="geneForm" class="mb-3">
                            <div class="input-group">
                                <input type="text" id="geneInput" class="form-control" placeholder="Enter gene symbol (e.g., KRAS)" value="KRAS">
                                <button class="btn btn-primary" type="submit">Search</button>
                            </div>
                        </form>
                        <div class="d-grid gap-2">
                            <button class="btn btn-sm btn-outline-secondary" onclick="loadGene('KRAS')">KRAS</button>
                            <button class="btn btn-sm btn-outline-secondary" onclick="loadGene('EGFR')">EGFR</button>
                            <button class="btn btn-sm btn-outline-secondary" onclick="loadGene('BRCA1')">BRCA1</button>
                            <button class="btn btn-sm btn-outline-secondary" onclick="loadGene('TP53')">TP53</button>
                        </div>
                    </div>
                </div>
            </div>
            
            <div class="col-md-8">
                <div class="card">
                    <div class="card-body">
                        <h5 class="card-title">Gene Essentiality: <span id="geneName">KRAS</span></h5>
                        <div class="row text-center mt-4">
                            <div class="col-md-3">
                                <div class="stat-value" id="meanCeres">-0.85</div>
                                <div class="stat-label">Mean CERES</div>
                            </div>
                            <div class="col-md-3">
                                <div class="stat-value" id="cellLines">42</div>
                                <div class="stat-label">Cell Lines</div>
                            </div>
                            <div class="col-md-3">
                                <div class="stat-value essential" id="essentialCount">38</div>
                                <div class="stat-label">Essential</div>
                            </div>
                            <div class="col-md-3">
                                <div class="stat-value selective" id="selectiveCount">3</div>
                                <div class="stat-label">Selective</div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        
        <div class="row">
            <div class="col-md-6">
                <div class="card">
                    <div class="card-body">
                        <h5 class="card-title">Dependency Distribution</h5>
                        <canvas id="distributionChart" height="200"></canvas>
                    </div>
                </div>
            </div>
            <div class="col-md-6">
                <div class="card">
                    <div class="card-body">
                        <h5 class="card-title">Cell Line Rankings</h5>
                        <div class="table-responsive">
                            <table class="table table-sm">
                                <thead>
                                    <tr>
                                        <th>Cell Line</th>
                                        <th>Cancer Type</th>
                                        <th>CERES Score</th>
                                        <th>Status</th>
                                    </tr>
                                </thead>
                                <tbody id="cellLineTable">
                                    <tr>
                                        <td>A549</td>
                                        <td>Lung Adenocarcinoma</td>
                                        <td class="essential">-1.23</td>
                                        <td><span class="badge bg-danger">Essential</span></td>
                                    </tr>
                                    <tr>
                                        <td>H358</td>
                                        <td>Lung Adenocarcinoma</td>
                                        <td class="essential">-1.15</td>
                                        <td><span class="badge bg-danger">Essential</span></td>
                                    </tr>
                                    <tr>
                                        <td>PANC1</td>
                                        <td>Pancreatic Adenocarcinoma</td>
                                        <td class="essential">-1.45</td>
                                        <td><span class="badge bg-danger">Essential</span></td>
                                    </tr>
                                </tbody>
                            </table>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        
        <div class="card mt-4">
            <div class="card-body">
                <h5 class="card-title">About DepMap Data</h5>
                <p class="text-muted">
                    DepMap (Dependency Map) provides CRISPR-Cas9 gene knockout screens across hundreds of cancer cell lines.
                    CERES scores indicate gene essentiality: more negative = more essential for cell survival.
                </p>
                <ul>
                    <li><strong>Essential (CERES < -1.0):</strong> Gene knockout strongly reduces cell viability</li>
                    <li><strong>Selective (-1.0 < CERES < -0.5):</strong> Context-dependent essentiality</li>
                    <li><strong>Non-essential (CERES > -0.5):</strong> Gene knockout has minimal effect</li>
                </ul>
            </div>
        </div>
    </div>
    
    <script>
        // Initialize distribution chart
        const ctx = document.getElementById('distributionChart').getContext('2d');
        const distributionChart = new Chart(ctx, {{
            type: 'bar',
            data: {{
                labels: ['<-1.5', '-1.5 to -1.0', '-1.0 to -0.5', '-0.5 to 0', '>0'],
                datasets: [{{
                    label: 'Cell Lines',
                    data: [12, 26, 3, 1, 0],
                    backgroundColor: ['#dc3545', '#dc3545', '#fd7e14', '#198754', '#198754'],
                }}]
            }},
            options: {{
                responsive: true,
                scales: {{
                    y: {{ beginAtZero: true }}
                }}
            }}
        }});
        
        function loadGene(selectedGene) {{
            document.getElementById('geneInput').value = selectedGene;
            document.getElementById('geneName').textContent = selectedGene;
            // In real implementation, fetch from /api/depmap/gene?gene=' + selectedGene
        }}
        
        document.getElementById('geneForm').addEventListener('submit', function(e) {{
            e.preventDefault();
            const gene = document.getElementById('geneInput').value;
            loadGene(gene);
        }});
    </script>
</body>
</html>
        "##,
        NAV_HTML
    )
}
