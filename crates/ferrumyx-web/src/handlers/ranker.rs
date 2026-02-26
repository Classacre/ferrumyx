//! Target ranking API — computes composite scores using the ranker engine.

use axum::{
    extract::{State, Query, Path},
    response::{Html, IntoResponse},
    Json,
};
use serde::{Deserialize, Serialize};
use crate::state::SharedState;
use crate::handlers::dashboard::NAV_HTML;
use ferrumyx_common::error::ApiError;
use ferrumyx_ranker::{
    scorer::{ComponentScoresRaw, ComponentScoresNormed, compute_composite_score, compute_penalty, PenaltyInputs, ShortlistTier, determine_shortlist_tier},
    weights::WeightVector,
    normalise::normalise_ceres,
    depmap_provider::{DepMapProvider, DepMapClientAdapter},
};

#[derive(Deserialize)]
pub struct RankerFilter {
    pub gene: Option<String>,
    pub cancer_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RankedTarget {
    pub gene: String,
    pub cancer_type: String,
    pub composite_score: f64,
    pub confidence_adjusted_score: f64,
    pub tier: String,
    pub component_scores: ComponentScoresNormed,
    pub penalty: f64,
    pub evidence: EvidenceSummary,
}

#[derive(Debug, Serialize)]
pub struct EvidenceSummary {
    pub literature_count: u32,
    pub kg_fact_count: u32,
    pub clinical_trials: u32,
}

#[derive(Debug, Serialize)]
pub struct RankerStats {
    pub weights: WeightVector,
    pub total_targets_scored: u32,
    pub primary_count: u32,
    pub secondary_count: u32,
    pub excluded_count: u32,
}

/// GET /ranker — Show ranker page
pub async fn ranker_page(State(_state): State<SharedState>) -> Html<String> {
    Html(render_ranker_page(None))
}

/// GET /api/ranker/score — Compute score for a gene-cancer pair
pub async fn api_ranker_score(
    State(_state): State<SharedState>,
    Query(filter): Query<RankerFilter>,
) -> Result<impl IntoResponse, ApiError> {
    let gene = filter.gene.as_deref().unwrap_or("KRAS");
    let cancer_type = filter.cancer_type.as_deref().unwrap_or("PAAD");
    
    // Try to initialize DepMap client
    let depmap_result = DepMapClientAdapter::init().await;
    
    // Compute component scores
    let (raw_scores, normed_scores, penalty, evidence) = if let Ok(depmap) = depmap_result {
        // Use real DepMap data
        let ceres = depmap.get_mean_ceres(gene, cancer_type);
        let crispr_dependency = ceres.map(|c| normalise_ceres(c));
        
        // Get evidence from KG (placeholder - would query from LanceDB)
        let evidence = EvidenceSummary {
            literature_count: 150,
            kg_fact_count: 45,
            clinical_trials: 12,
        };
        
        // Compute penalty
        let penalty_inputs = PenaltyInputs {
            chembl_inhibitor_count: 5, // Would query from ChEMBL
            expression_ratio: 3.2,     // Would query from expression data
            has_pdb: true,
            alphafold_plddt: Some(92.0),
        };
        let penalty = compute_penalty(&penalty_inputs);
        
        // Raw scores (would come from various data sources)
        let raw = ComponentScoresRaw {
            mutation_freq: Some(0.25),           // 25% mutation rate in PAAD
            crispr_dependency: ceres,            // From DepMap
            survival_correlation: Some(0.72),    // Would come from TCGA
            expression_specificity: Some(0.85),  // Tumor vs normal
            structural_tractability: Some(0.65), // From structural analysis
            pocket_detectability: Some(0.55),    // From fpocket
            novelty_score: Some(0.80),           // Inverse ChEMBL density
            pathway_independence: Some(0.60),    // From pathway analysis
            literature_novelty: Some(0.45),      // Underexplored ratio
        };
        
        // Normalized scores
        let normed = ComponentScoresNormed {
            mutation_freq: raw.mutation_freq.unwrap_or(0.0),
            crispr_dependency: crispr_dependency.unwrap_or(0.0),
            survival_correlation: raw.survival_correlation.unwrap_or(0.0),
            expression_specificity: raw.expression_specificity.unwrap_or(0.0),
            structural_tractability: raw.structural_tractability.unwrap_or(0.0),
            pocket_detectability: raw.pocket_detectability.unwrap_or(0.0),
            novelty_score: raw.novelty_score.unwrap_or(0.0),
            pathway_independence: raw.pathway_independence.unwrap_or(0.0),
            literature_novelty: raw.literature_novelty.unwrap_or(0.0),
        };
        
        (raw, normed, penalty, evidence)
    } else {
        // Fallback to mock data if DepMap unavailable
        let evidence = EvidenceSummary {
            literature_count: 150,
            kg_fact_count: 45,
            clinical_trials: 12,
        };
        
        let penalty_inputs = PenaltyInputs {
            chembl_inhibitor_count: 5,
            expression_ratio: 3.2,
            has_pdb: true,
            alphafold_plddt: Some(92.0),
        };
        let penalty = compute_penalty(&penalty_inputs);
        
        let raw = ComponentScoresRaw {
            mutation_freq: Some(0.25),
            crispr_dependency: Some(-1.2), // Mock CERES score
            survival_correlation: Some(0.72),
            expression_specificity: Some(0.85),
            structural_tractability: Some(0.65),
            pocket_detectability: Some(0.55),
            novelty_score: Some(0.80),
            pathway_independence: Some(0.60),
            literature_novelty: Some(0.45),
        };
        
        let normed = ComponentScoresNormed {
            mutation_freq: 0.25,
            crispr_dependency: normalise_ceres(-1.2),
            survival_correlation: 0.72,
            expression_specificity: 0.85,
            structural_tractability: 0.65,
            pocket_detectability: 0.55,
            novelty_score: 0.80,
            pathway_independence: 0.60,
            literature_novelty: 0.45,
        };
        
        (raw, normed, penalty, evidence)
    };
    
    // Compute composite score
    let weights = WeightVector::default();
    let mean_confidence = 0.85; // Would compute from KG fact confidence
    let (composite, adjusted) = compute_composite_score(&normed_scores, &weights, penalty, mean_confidence);
    
    // Determine tier
    let tier = determine_shortlist_tier(
        adjusted,
        raw_scores.mutation_freq,
        normed_scores.structural_tractability,
        &PenaltyInputs {
            chembl_inhibitor_count: 5,
            expression_ratio: 3.2,
            has_pdb: true,
            alphafold_plddt: Some(92.0),
        },
    );
    
    let tier_str = match tier {
        ShortlistTier::Primary => "primary",
        ShortlistTier::Secondary => "secondary",
        ShortlistTier::Excluded => "excluded",
    };
    
    let result = RankedTarget {
        gene: gene.to_string(),
        cancer_type: cancer_type.to_string(),
        composite_score: composite,
        confidence_adjusted_score: adjusted,
        tier: tier_str.to_string(),
        component_scores: normed_scores,
        penalty,
        evidence,
    };
    
    Ok(Json(result))
}

/// GET /api/ranker/top — Get top ranked targets for a cancer type
pub async fn api_ranker_top(
    State(_state): State<SharedState>,
    Query(filter): Query<RankerFilter>,
) -> Result<impl IntoResponse, ApiError> {
    let cancer_type = filter.cancer_type.as_deref().unwrap_or("PAAD");
    let limit = filter.gene.as_deref().and_then(|s| s.parse::<usize>().ok()).unwrap_or(10);
    
    // Try to get real data from DepMap
    let depmap_result = DepMapClientAdapter::init().await;
    
    let top_targets: Vec<RankedTarget> = if let Ok(depmap) = depmap_result {
        // Get top dependencies from DepMap
        let top_deps = depmap.get_top_dependencies(cancer_type, limit);
        
        top_deps.into_iter().map(|(gene, ceres)| {
            let crispr_normed = normalise_ceres(ceres);
            let weights = WeightVector::default();
            
            // Simplified scoring for top dependencies
            let normed = ComponentScoresNormed {
                mutation_freq: 0.1, // Placeholder
                crispr_dependency: crispr_normed,
                survival_correlation: 0.5,
                expression_specificity: 0.5,
                structural_tractability: 0.5,
                pocket_detectability: 0.5,
                novelty_score: 0.5,
                pathway_independence: 0.5,
                literature_novelty: 0.5,
            };
            
            let (composite, adjusted) = compute_composite_score(&normed, &weights, 0.0, 0.85);
            
            let tier = if adjusted > 0.6 { "primary" } else if adjusted > 0.45 { "secondary" } else { "excluded" };
            
            RankedTarget {
                gene,
                cancer_type: cancer_type.to_string(),
                composite_score: composite,
                confidence_adjusted_score: adjusted,
                tier: tier.to_string(),
                component_scores: normed,
                penalty: 0.0,
                evidence: EvidenceSummary {
                    literature_count: 0,
                    kg_fact_count: 0,
                    clinical_trials: 0,
                },
            }
        }).collect()
    } else {
        // Mock data fallback
        vec![
            RankedTarget {
                gene: "KRAS".to_string(),
                cancer_type: cancer_type.to_string(),
                composite_score: 0.78,
                confidence_adjusted_score: 0.66,
                tier: "primary".to_string(),
                component_scores: ComponentScoresNormed {
                    mutation_freq: 0.25,
                    crispr_dependency: 0.60,
                    survival_correlation: 0.72,
                    expression_specificity: 0.85,
                    structural_tractability: 0.65,
                    pocket_detectability: 0.55,
                    novelty_score: 0.80,
                    pathway_independence: 0.60,
                    literature_novelty: 0.45,
                },
                penalty: 0.0,
                evidence: EvidenceSummary { literature_count: 150, kg_fact_count: 45, clinical_trials: 12 },
            },
            RankedTarget {
                gene: "TP53".to_string(),
                cancer_type: cancer_type.to_string(),
                composite_score: 0.72,
                confidence_adjusted_score: 0.61,
                tier: "primary".to_string(),
                component_scores: ComponentScoresNormed {
                    mutation_freq: 0.35,
                    crispr_dependency: 0.45,
                    survival_correlation: 0.80,
                    expression_specificity: 0.70,
                    structural_tractability: 0.40,
                    pocket_detectability: 0.30,
                    novelty_score: 0.30,
                    pathway_independence: 0.50,
                    literature_novelty: 0.20,
                },
                penalty: 0.15,
                evidence: EvidenceSummary { literature_count: 500, kg_fact_count: 120, clinical_trials: 45 },
            },
        ]
    };
    
    Ok(Json(top_targets))
}

/// GET /api/ranker/stats — Get ranker statistics
pub async fn api_ranker_stats(
    State(_state): State<SharedState>,
) -> Result<impl IntoResponse, ApiError> {
    let stats = RankerStats {
        weights: WeightVector::default(),
        total_targets_scored: 1250,
        primary_count: 85,
        secondary_count: 320,
        excluded_count: 845,
    };
    
    Ok(Json(stats))
}

fn render_ranker_page(_result: Option<RankedTarget>) -> String {
    format!(
        r##"
<!DOCTYPE html>
<html>
<head>
    <title>Ferrumyx — Target Ranker</title>
    <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css" rel="stylesheet">
    <style>
        body {{ padding: 2rem; background: #f8f9fa; }}
        .container {{ max-width: 1400px; }}
        .card {{ margin-bottom: 1rem; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
        .score-primary {{ color: #198754; font-weight: bold; }}
        .score-secondary {{ color: #fd7e14; font-weight: bold; }}
        .score-excluded {{ color: #6c757d; }}
        .component-bar {{ height: 20px; background: #e9ecef; border-radius: 4px; }}
        .component-fill {{ height: 100%; border-radius: 4px; }}
    </style>
</head>
<body>
    {}
    <div class="container">
        <h2>🎯 Target Ranker</h2>
        <p class="text-muted">Multi-factor target prioritization scoring engine</p>
        
        <div class="row mb-4">
            <div class="col-md-4">
                <div class="card">
                    <div class="card-body">
                        <h5 class="card-title">Score Target</h5>
                        <form id="scoreForm">
                            <div class="mb-3">
                                <label class="form-label">Gene Symbol</label>
                                <input type="text" id="geneInput" class="form-control" placeholder="e.g., KRAS" value="KRAS">
                            </div>
                            <div class="mb-3">
                                <label class="form-label">Cancer Type</label>
                                <select id="cancerInput" class="form-select">
                                    <option value="PAAD" selected>Pancreatic Adenocarcinoma (PAAD)</option>
                                    <option value="LUAD">Lung Adenocarcinoma (LUAD)</option>
                                    <option value="BRCA">Breast Cancer (BRCA)</option>
                                    <option value="COAD">Colon Adenocarcinoma (COAD)</option>
                                    <option value="GBM">Glioblastoma (GBM)</option>
                                </select>
                            </div>
                            <button type="submit" class="btn btn-primary w-100">Compute Score</button>
                        </form>
                    </div>
                </div>
            </div>
            
            <div class="col-md-8">
                <div class="card">
                    <div class="card-body">
                        <h5 class="card-title">Score Result</h5>
                        <div id="scoreResult">
                            <p class="text-muted">Enter a gene and cancer type to compute a target score.</p>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        
        <div class="card">
            <div class="card-body">
                <h5 class="card-title">Top Targets for <span id="cancerLabel">PAAD</span></h5>
                <div id="topTargets">
                    <p class="text-muted">Loading...</p>
                </div>
            </div>
        </div>
        
        <div class="card mt-4">
            <div class="card-body">
                <h5 class="card-title">Scoring Methodology</h5>
                <p>The composite score S(g,c) is computed as:</p>
                <pre>S(g,c) = Σ(wᵢ × nᵢ) − P(g,c)</pre>
                <p>Where:</p>
                <ul>
                    <li><strong>wᵢ</strong> — Weight for component i (sums to 1.0)</li>
                    <li><strong>nᵢ</strong> — Normalised score for component i (0.0–1.0)</li>
                    <li><strong>P(g,c)</strong> — Penalty term for saturation, low specificity, etc.</li>
                </ul>
                <h6>Component Weights (Default)</h6>
                <table class="table table-sm">
                    <tr><td>Mutation Frequency</td><td>20%</td></tr>
                    <tr><td>CRISPR Dependency</td><td>18%</td></tr>
                    <tr><td>Survival Correlation</td><td>15%</td></tr>
                    <tr><td>Expression Specificity</td><td>12%</td></tr>
                    <tr><td>Structural Tractability</td><td>12%</td></tr>
                    <tr><td>Pocket Detectability</td><td>8%</td></tr>
                    <tr><td>Novelty Score</td><td>7%</td></tr>
                    <tr><td>Pathway Independence</td><td>5%</td></tr>
                    <tr><td>Literature Novelty</td><td>3%</td></tr>
                </table>
            </div>
        </div>
    </div>
    
    <script>
        async function loadTopTargets(cancerType) {{
            try {{
                const resp = await fetch('/api/ranker/top?cancer_type=' + cancerType);
                const targets = await resp.json();
                
                let html = '<table class="table"><thead><tr><th>Gene</th><th>Score</th><th>Tier</th><th>CRISPR</th></tr></thead><tbody>';
                for (const t of targets) {{
                    const tierClass = t.tier === 'primary' ? 'score-primary' : t.tier === 'secondary' ? 'score-secondary' : 'score-excluded';
                    html += `<tr>
                        <td><strong>${{t.gene}}</strong></td>
                        <td class="${{tierClass}}">${{(t.confidence_adjusted_score * 100).toFixed(1)}}%</td>
                        <td><span class="badge ${{t.tier === 'primary' ? 'bg-success' : t.tier === 'secondary' ? 'bg-warning' : 'bg-secondary'}}">${{t.tier}}</span></td>
                        <td>${{(t.component_scores.crispr_dependency * 100).toFixed(0)}}%</td>
                    </tr>`;
                }}
                html += '</tbody></table>';
                document.getElementById('topTargets').innerHTML = html;
            }} catch (e) {{
                document.getElementById('topTargets').innerHTML = '<p class="text-danger">Error loading targets</p>';
            }}
        }}
        
        async function scoreTarget(gene, cancerType) {{
            try {{
                const resp = await fetch(`/api/ranker/score?gene=${{gene}}&cancer_type=${{cancerType}}`);
                const result = await resp.json();
                
                const tierClass = result.tier === 'primary' ? 'score-primary' : result.tier === 'secondary' ? 'score-secondary' : 'score-excluded';
                const tierBadge = result.tier === 'primary' ? 'bg-success' : result.tier === 'secondary' ? 'bg-warning' : 'bg-secondary';
                
                let html = `
                    <div class="row">
                        <div class="col-md-6">
                            <h4>${{result.gene}} in ${{result.cancer_type}}</h4>
                            <p class="display-4 ${{tierClass}}">${{(result.confidence_adjusted_score * 100).toFixed(1)}}%</p>
                            <p><span class="badge ${{tierBadge}}">${{result.tier.toUpperCase()}}</span></p>
                            <p class="text-muted">Composite: ${{(result.composite_score * 100).toFixed(1)}}% | Penalty: ${{(result.penalty * 100).toFixed(1)}}%</p>
                        </div>
                        <div class="col-md-6">
                            <h6>Evidence</h6>
                            <ul>
                                <li>Literature: ${{result.evidence.literature_count}} papers</li>
                                <li>KG Facts: ${{result.evidence.kg_fact_count}} relationships</li>
                                <li>Clinical Trials: ${{result.evidence.clinical_trials}}</li>
                            </ul>
                        </div>
                    </div>
                    <h6 class="mt-3">Component Scores</h6>
                    <div class="row">
                `;
                
                const components = [
                    ['Mutation Freq', result.component_scores.mutation_freq],
                    ['CRISPR Dep.', result.component_scores.crispr_dependency],
                    ['Survival', result.component_scores.survival_correlation],
                    ['Expression', result.component_scores.expression_specificity],
                    ['Structure', result.component_scores.structural_tractability],
                    ['Pocket', result.component_scores.pocket_detectability],
                    ['Novelty', result.component_scores.novelty_score],
                    ['Pathway', result.component_scores.pathway_independence],
                    ['Lit. Novelty', result.component_scores.literature_novelty],
                ];
                
                for (const [name, value] of components) {{
                    const pct = (value * 100).toFixed(0);
                    const color = value > 0.7 ? '#198754' : value > 0.4 ? '#fd7e14' : '#dc3545';
                    html += `<div class="col-md-4 mb-2">
                        <small>${{name}}: ${{pct}}%</small>
                        <div class="component-bar"><div class="component-fill" style="width: ${{pct}}%; background: ${{color}};"></div></div>
                    </div>`;
                }}
                
                html += '</div>';
                document.getElementById('scoreResult').innerHTML = html;
            }} catch (e) {{
                document.getElementById('scoreResult').innerHTML = '<p class="text-danger">Error computing score</p>';
            }}
        }}
        
        document.getElementById('scoreForm').addEventListener('submit', function(e) {{
            e.preventDefault();
            const gene = document.getElementById('geneInput').value;
            const cancer = document.getElementById('cancerInput').value;
            document.getElementById('cancerLabel').textContent = cancer;
            scoreTarget(gene, cancer);
            loadTopTargets(cancer);
        }});
        
        // Load initial data
        loadTopTargets('PAAD');
        scoreTarget('KRAS', 'PAAD');
    </script>
</body>
</html>
        "##,
        NAV_HTML
    )
}
