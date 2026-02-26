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
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Target Ranker Engine — Ferrumyx</title>
    <link rel="stylesheet" href="/static/css/main.css">
    <style>
        .score-primary {{ color: var(--success); font-weight: 700; font-family: 'Outfit'; }}
        .score-secondary {{ color: var(--warning); font-weight: 700; font-family: 'Outfit'; }}
        .score-excluded {{ color: var(--text-muted); font-weight: 700; font-family: 'Outfit'; }}
        .component-bar {{ height: 12px; background: var(--bg-surface); border: 1px solid var(--border-glass); border-radius: 6px; overflow: hidden; }}
        .component-fill {{ height: 100%; border-radius: 0; background: var(--brand-blue); transition: width 0.5s ease-out; }}
        .component-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 1rem; }}
        .method-table td:first-child {{ color: var(--text-main); font-weight: 500; border-bottom: 1px solid var(--border-glass); }}
        .method-table td:last-child {{ text-align: right; color: var(--brand-purple); font-weight: 700; border-bottom: 1px solid var(--border-glass); }}
    </style>
</head>
<body>
    {}
    <main class="main-content">
        <div class="page-header">
            <div>
                <h1 class="page-title">
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M16 11c1.66 0 2.99-1.34 2.99-3S17.66 5 16 5c-1.66 0-3 1.34-3 3s1.34 3 3 3zm-8 0c1.66 0 2.99-1.34 2.99-3S9.66 5 8 5C6.34 5 5 6.34 5 8s1.34 3 3 3zm0 2c-2.33 0-7 1.17-7 3.5V19h14v-2.5c0-2.33-4.67-3.5-7-3.5zm8 0c-.29 0-.62.02-.97.05 1.16.84 1.97 1.97 1.97 3.45V19h6v-2.5c0-2.33-4.67-3.5-7-3.5z"/></svg>
                    Target Prioritization Engine
                </h1>
                <p class="text-muted">Multi-factor composite scoring and algorithmic shortlisting matrix</p>
            </div>
        </div>

        <div class="grid-2 align-start" style="grid-template-columns: 350px 1fr; gap: 2rem;">
            <div class="card h-100">
                <div class="card-header border-bottom border-glass pb-3 mb-3">Execute Target Computation</div>
                <div class="card-body p-0">
                    <form id="scoreForm">
                        <div class="mb-3">
                            <label class="form-label text-muted small text-uppercase" style="letter-spacing:1px">Target Locus</label>
                            <input type="text" id="geneInput" class="form-control font-outfit" style="font-size:1.1rem; color:var(--text-main)" placeholder="e.g., KRAS" value="KRAS">
                        </div>
                        <div class="mb-4">
                            <label class="form-label text-muted small text-uppercase" style="letter-spacing:1px">Pathology Vector</label>
                            <select id="cancerInput" class="form-control font-outfit" style="font-size:1.05rem; color:var(--text-main)">
                                <option value="PAAD" selected>Pancreatic Adenocarcinoma (PAAD)</option>
                                <option value="LUAD">Lung Adenocarcinoma (LUAD)</option>
                                <option value="BRCA">Breast Cancer (BRCA)</option>
                                <option value="COAD">Colon Adenocarcinoma (COAD)</option>
                                <option value="GBM">Glioblastoma (GBM)</option>
                            </select>
                        </div>
                        <button type="submit" class="btn btn-primary w-100 py-3">Synthesize Matrix Score</button>
                    </form>
                </div>
            </div>
            
            <div class="card h-100">
                <div class="card-header border-bottom border-glass pb-3">Computation Matrix Topology</div>
                <div class="card-body p-0 pt-3" id="scoreResult">
                    <div class="d-flex align-center justify-center p-5 text-muted h-100 w-100" style="min-height:200px; border:1px dashed var(--border-glass); border-radius:8px;">
                        Provide a target locus and pathology vector to compute the synthesis score.
                    </div>
                </div>
            </div>
        </div>
        
        <div class="grid-2 gap-4 mt-4">
            <div class="card h-100">
                <div class="card-header border-bottom border-glass pb-3 mb-3 d-flex justify-between">
                    <div>Top Computed Candidates <span class="text-muted" id="cancerLabel">PAAD</span></div>
                    <span class="badge badge-outline">Algorithmic Rank</span>
                </div>
                <div class="table-container p-0">
                    <div id="topTargets">
                        <div class="p-4 text-center text-muted">Retrieving network data...</div>
                    </div>
                </div>
            </div>
            
            <div class="card h-100" style="border-left: 4px solid var(--brand-blue);">
                <div class="card-body p-4">
                    <h5 class="card-title font-outfit mb-3">Mathematical Synthesis Model</h5>
                    <p class="text-muted small">The composite rank function S(g,c) calculates weighted heuristic bounds:</p>
                    <div class="p-3 mb-3 font-outfit text-center" style="background:var(--bg-card); border:1px solid var(--border-glass); border-radius:6px; color:var(--text-main); font-size:1.1rem; letter-spacing:1px;">
                        S(g,c) = Σ(wᵢ × nᵢ) − P(g,c)
                    </div>
                    <p class="text-muted small mb-3">Parameters:</p>
                    <ul class="text-muted small mb-4" style="padding-left:1.5rem">
                        <li><strong style="color:var(--text-main)">wᵢ</strong> — Feature weight vector magnitude (normalized = 1.0)</li>
                        <li><strong style="color:var(--text-main)">nᵢ</strong> — Feature scalar projection (0.0–1.0 bounds)</li>
                        <li><strong style="color:var(--text-main)">P(g,c)</strong> — Penalty heuristic (saturation, structural constraints)</li>
                    </ul>
                    
                    <h6 class="font-outfit text-muted mb-2 text-uppercase" style="font-size:0.8rem; letter-spacing:1px">Configured Weights</h6>
                    <table class="table mb-0 method-table" style="font-size: 0.85rem">
                        <tbody>
                            <tr><td>Mutation Frequency</td><td>20%</td></tr>
                            <tr><td>CRISPR Dependency</td><td>18%</td></tr>
                            <tr><td>Survival Correlation</td><td>15%</td></tr>
                            <tr><td>Expression Specificity</td><td>12%</td></tr>
                            <tr><td>Structural Tractability</td><td>12%</td></tr>
                            <tr><td>Pocket Detectability</td><td>8%</td></tr>
                            <tr><td>Novelty Density</td><td>7%</td></tr>
                            <tr><td>Pathway Orthogonality</td><td>5%</td></tr>
                            <tr><td>Literature Deficit</td><td>3%</td></tr>
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    </main>
    <script src="/static/js/main.js"></script>
    <script>
        async function loadTopTargets(cancerType) {{
            try {{
                const resp = await fetch('/api/ranker/top?cancer_type=' + cancerType);
                const targets = await resp.json();
                
                let html = '<table class="table mb-0"><thead><tr><th>Locus</th><th>Confidence</th><th>Shortlist</th><th>CRISPR Dep</th></tr></thead><tbody>';
                for (const t of targets) {{
                    const tierClass = t.tier === 'primary' ? 'score-primary' : t.tier === 'secondary' ? 'score-secondary' : 'score-excluded';
                    const tierBadge = t.tier === 'primary' ? 'badge-success' : t.tier === 'secondary' ? 'badge-warning' : 'badge-outline';
                    html += `<tr>
                        <td class="font-outfit" style="color:var(--text-main); font-weight:500">${{t.gene}}</td>
                        <td class="${{tierClass}}">${{(t.confidence_adjusted_score * 100).toFixed(1)}}%</td>
                        <td><span class="badge ${{tierBadge}}">${{t.tier.toUpperCase()}}</span></td>
                        <td class="text-muted">${{(t.component_scores.crispr_dependency * 100).toFixed(0)}}%</td>
                    </tr>`;
                }}
                html += '</tbody></table>';
                document.getElementById('topTargets').innerHTML = html;
            }} catch (e) {{
                document.getElementById('topTargets').innerHTML = '<div class="p-4 text-center text-danger">Topology network retrieval error</div>';
            }}
        }}
        
        async function scoreTarget(gene, cancerType) {{
            try {{
                const resp = await fetch(`/api/ranker/score?gene=${{gene}}&cancer_type=${{cancerType}}`);
                const result = await resp.json();
                
                const tierClass = result.tier === 'primary' ? 'score-primary' : result.tier === 'secondary' ? 'score-secondary' : 'score-excluded';
                const tierBadge = result.tier === 'primary' ? 'badge-success' : result.tier === 'secondary' ? 'badge-warning' : 'badge-outline';
                
                let html = `
                    <div class="grid-2 gap-4 pb-4 border-bottom border-glass mb-4">
                        <div>
                            <div class="text-muted text-uppercase mb-1" style="font-size:0.8rem; letter-spacing:1px">Composite Output</div>
                            <div class="d-flex align-center gap-3">
                                <div class="font-outfit ${{tierClass}}" style="font-size:3.5rem; line-height:1">${{(result.confidence_adjusted_score * 100).toFixed(1)}}<span style="font-size:1.5rem">%</span></div>
                                <div class="d-flex flex-column gap-1">
                                    <span class="badge ${{tierBadge}}" style="align-self:flex-start">${{result.tier.toUpperCase()}} TIER</span>
                                    <span class="text-muted small">C: ${{(result.composite_score * 100).toFixed(1)}}% | P: ${{(result.penalty * 100).toFixed(1)}}%</span>
                                </div>
                            </div>
                        </div>
                        <div class="d-flex flex-column justify-center" style="border-left: 1px solid var(--border-glass); padding-left:1.5rem;">
                            <div class="text-muted text-uppercase mb-2" style="font-size:0.8rem; letter-spacing:1px">Evidence Support Topology</div>
                            <div class="d-flex flex-column gap-2 text-muted small">
                                <div class="d-flex justify-between"><span>Literature Base</span> <strong style="color:var(--text-main)">${{result.evidence.literature_count}} corpus artifacts</strong></div>
                                <div class="d-flex justify-between"><span>Knowledge Graph</span> <strong style="color:var(--text-main)">${{result.evidence.kg_fact_count}} edges</strong></div>
                                <div class="d-flex justify-between"><span>Clinical Network</span> <strong style="color:var(--text-main)">${{result.evidence.clinical_trials}} trials</strong></div>
                            </div>
                        </div>
                    </div>
                    <div class="text-muted text-uppercase mb-3" style="font-size:0.8rem; letter-spacing:1px">Scalar Constituents</div>
                    <div class="component-grid">
                `;
                
                const components = [
                    ['Mutation Freq', result.component_scores.mutation_freq],
                    ['CRISPR Dep.', result.component_scores.crispr_dependency],
                    ['Survival', result.component_scores.survival_correlation],
                    ['Expression', result.component_scores.expression_specificity],
                    ['Structure', result.component_scores.structural_tractability],
                    ['Pocket', result.component_scores.pocket_detectability],
                    ['Novelty', result.component_scores.novelty_score],
                    ['Pathway Ortho', result.component_scores.pathway_independence],
                    ['Lit. Deficit', result.component_scores.literature_novelty],
                ];
                
                for (const [name, value] of components) {{
                    const pct = (value * 100).toFixed(0);
                    const color = value > 0.7 ? 'var(--success)' : value > 0.4 ? 'var(--warning)' : 'var(--danger)';
                    html += `<div>
                        <div class="d-flex justify-between align-center mb-1">
                            <span class="text-muted small">${{name}}</span>
                            <strong style="color:var(--text-main); font-size:0.85rem">${{pct}}%</strong>
                        </div>
                        <div class="component-bar"><div class="component-fill" style="width: ${{pct}}%; background: ${{color}};"></div></div>
                    </div>`;
                }}
                
                html += '</div>';
                document.getElementById('scoreResult').innerHTML = html;
            }} catch (e) {{
                document.getElementById('scoreResult').innerHTML = '<div class="p-4 text-center text-danger">Network synthesis interference detected</div>';
            }}
        }}
        
        document.getElementById('scoreForm').addEventListener('submit', function(e) {{
            e.preventDefault();
            const gene = document.getElementById('geneInput').value;
            const cancer = document.getElementById('cancerInput').value;
            document.getElementById('cancerLabel').textContent = cancer;
            
            document.getElementById('scoreResult').innerHTML = '<div class="d-flex align-center justify-center p-5 text-brand-blue flex-column h-100"><div class="loading" style="width:24px; height:24px; border:3px solid rgba(59,130,246,0.3); border-radius:50%; border-top-color:var(--brand-blue); animation:spin 1s linear infinite;"></div><div class="mt-3 small">Synthesizing Network...</div></div>';
            
            setTimeout(() => {{
                scoreTarget(gene, cancer);
                loadTopTargets(cancer);
            }}, 400);
        }});
        
        loadTopTargets('PAAD');
        scoreTarget('KRAS', 'PAAD');
    </script>
    <style>@keyframes spin {{ 100% {{ transform: rotate(360deg); }} }}</style>
</body>
</html>"##,
        NAV_HTML
    )
}
