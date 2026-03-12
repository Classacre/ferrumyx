//! Target ranking API — computes composite scores using the ranker engine.

use crate::handlers::dashboard::NAV_HTML;
use crate::state::SharedState;
use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse},
    Json,
};
use ferrumyx_common::error::ApiError;
use ferrumyx_db::{
    entities::EntityRepository, kg_facts::KgFactRepository, target_scores::TargetScoreRepository,
};
use ferrumyx_ranker::{
    depmap_provider::{DepMapClientAdapter, DepMapProvider},
    normalise::normalise_ceres,
    scorer::ComponentScoresNormed,
    weights::WeightVector,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct RankerFilter {
    pub gene: Option<String>,
    pub cancer_type: Option<String>,
    pub limit: Option<usize>,
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
    State(state): State<SharedState>,
    Query(filter): Query<RankerFilter>,
) -> Result<impl IntoResponse, ApiError> {
    let gene = filter.gene.as_deref().unwrap_or("KRAS");
    let cancer_type = filter.cancer_type.as_deref().unwrap_or("PAAD");

    let all = load_ranked_targets(&state, Some(cancer_type), 50_000).await?;
    let row = all
        .into_iter()
        .find(|r| r.gene.eq_ignore_ascii_case(gene))
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "No persisted score found for {gene} in {cancer_type}"
            ))
        })?;

    Ok(Json(row))
}

/// GET /api/ranker/top — Get top ranked targets for a cancer type
pub async fn api_ranker_top(
    State(state): State<SharedState>,
    Query(filter): Query<RankerFilter>,
) -> Result<impl IntoResponse, ApiError> {
    let cancer_type = filter.cancer_type.as_deref().unwrap_or("PAAD");
    let limit = filter.limit.unwrap_or(10).clamp(1, 100);
    let mut top_targets = load_ranked_targets(&state, Some(cancer_type), 50_000).await?;
    top_targets.truncate(limit);
    Ok(Json(top_targets))
}

/// GET /api/ranker/stats — Get ranker statistics
pub async fn api_ranker_stats(
    State(state): State<SharedState>,
) -> Result<impl IntoResponse, ApiError> {
    let rows = load_ranked_targets(&state, None, 100_000).await?;
    let mut primary_count = 0u32;
    let mut secondary_count = 0u32;
    let mut excluded_count = 0u32;

    for row in &rows {
        match row.tier.as_str() {
            "primary" => primary_count += 1,
            "secondary" => secondary_count += 1,
            _ => excluded_count += 1,
        }
    }

    let stats = RankerStats {
        weights: WeightVector::default(),
        total_targets_scored: rows.len() as u32,
        primary_count,
        secondary_count,
        excluded_count,
    };

    Ok(Json(stats))
}

async fn load_ranked_targets(
    state: &SharedState,
    cancer_filter: Option<&str>,
    limit: usize,
) -> Result<Vec<RankedTarget>, ApiError> {
    let score_repo = TargetScoreRepository::new(state.db.clone());
    let entity_repo = EntityRepository::new(state.db.clone());
    let kg_repo = KgFactRepository::new(state.db.clone());
    let depmap = DepMapClientAdapter::init().await.ok();

    let mut rows = score_repo
        .list(0, limit.min(10_000))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    rows.sort_by(|a, b| {
        b.confidence_adjusted_score
            .partial_cmp(&a.confidence_adjusted_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let gene_ids: Vec<uuid::Uuid> = rows.iter().map(|s| s.gene_id).collect();
    let fact_count_by_gene: HashMap<uuid::Uuid, u32> = kg_repo
        .count_by_subject_ids(&gene_ids, 300)
        .await
        .unwrap_or_default();

    let mut name_cache: HashMap<uuid::Uuid, String> = HashMap::new();
    let mut out = Vec::new();
    for s in rows {
        let raw_json: serde_json::Value =
            serde_json::from_str(&s.components_raw).unwrap_or_default();
        let norm_json: serde_json::Value =
            serde_json::from_str(&s.components_normed).unwrap_or_default();
        let mut gene = raw_json
            .get("gene")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());
        let mut cancer_type = raw_json
            .get("cancer_code")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());

        if gene.is_none() {
            if let Some(cached) = name_cache.get(&s.gene_id) {
                gene = Some(cached.clone());
            } else if let Ok(Some(ent)) = entity_repo.find_by_id(s.gene_id).await {
                name_cache.insert(s.gene_id, ent.name.clone());
                gene = Some(ent.name);
            }
        }
        if cancer_type.is_none() {
            if let Some(cached) = name_cache.get(&s.cancer_id) {
                cancer_type = Some(cached.clone());
            } else if s.cancer_id != uuid::Uuid::nil() {
                if let Ok(Some(ent)) = entity_repo.find_by_id(s.cancer_id).await {
                    name_cache.insert(s.cancer_id, ent.name.clone());
                    cancer_type = Some(ent.name);
                }
            }
        }

        let gene = gene.unwrap_or_else(|| s.gene_id.to_string());
        let cancer_type = cancer_type.unwrap_or_else(|| "UNSPECIFIED".to_string());
        if let Some(filter_code) = cancer_filter {
            if !cancer_type.eq_ignore_ascii_case(filter_code) {
                continue;
            }
        }

        let mut component_scores = ComponentScoresNormed {
            mutation_freq: norm_json
                .get("mutation_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            crispr_dependency: 0.0,
            survival_correlation: 0.0,
            expression_specificity: 0.0,
            structural_tractability: 0.0,
            pocket_detectability: 0.0,
            novelty_score: 0.0,
            pathway_independence: 0.0,
            literature_novelty: norm_json
                .get("literature_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
        };
        if let Some(depmap) = &depmap {
            if let Some(ceres) = depmap.get_mean_ceres(&gene, &cancer_type) {
                component_scores.crispr_dependency = normalise_ceres(ceres);
            }
        }

        out.push(RankedTarget {
            gene: gene.clone(),
            cancer_type: cancer_type.clone(),
            composite_score: s.composite_score,
            confidence_adjusted_score: s.confidence_adjusted_score,
            tier: s.shortlist_tier.clone(),
            component_scores,
            penalty: s.penalty_score,
            evidence: EvidenceSummary {
                literature_count: fact_count_by_gene.get(&s.gene_id).copied().unwrap_or(0),
                kg_fact_count: fact_count_by_gene.get(&s.gene_id).copied().unwrap_or(0),
                clinical_trials: 0,
            },
        });
    }

    out.sort_by(|a, b| {
        b.confidence_adjusted_score
            .partial_cmp(&a.confidence_adjusted_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(out)
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
        
        <div class="mt-4">
            <div class="card h-100">
                <div class="card-header border-bottom border-glass pb-3 mb-3 d-flex justify-between align-center">
                    <div class="d-flex align-center gap-2">
                        <span>Top Computed Candidates <span class="text-muted" id="cancerLabel">PAAD</span></span>
                        <span class="info-tip">i
                            <span class="tooltip-card">
                                <strong class="text-main">Mathematical Synthesis Model</strong><br>
                                S(g,c) = Σ(wᵢ × nᵢ) − P(g,c)<br><br>
                                <strong>wᵢ</strong>: configured feature weights.<br>
                                <strong>nᵢ</strong>: normalized feature values (0.0-1.0).<br>
                                <strong>P(g,c)</strong>: penalty term for saturation and structural constraints.<br><br>
                                Weight vector: mutation 20%, CRISPR 18%, survival 15%, expression 12%, structure 12%, pocket 8%, novelty 7%, pathway 5%, literature deficit 3%.
                            </span>
                        </span>
                    </div>
                    <span class="badge badge-outline">Algorithmic Rank</span>
                </div>
                <div class="table-container p-0">
                    <div id="topTargets">
                        <div class="p-4 text-center text-muted">Retrieving network data...</div>
                    </div>
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
