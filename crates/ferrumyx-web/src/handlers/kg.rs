//! Knowledge graph explorer.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::handlers::dashboard::NAV_HTML;
use crate::state::SharedState;
use ferrumyx_common::error::ApiError;
use ferrumyx_db::entities::EntityRepository;
use ferrumyx_db::kg_facts::KgFactRepository;
use ferrumyx_db::papers::PaperRepository;

struct CachedHtml {
    html: String,
    created_at: Instant,
}

static KG_PAGE_CACHE: OnceLock<Mutex<HashMap<String, CachedHtml>>> = OnceLock::new();

#[derive(Deserialize, Default)]
pub struct KgFilter {
    pub gene: Option<String>,
    pub q: Option<String>,
    pub predicate: Option<String>,
    pub max_papers: Option<usize>,
    pub expanded: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct EntitySuggestQuery {
    pub q: Option<String>,
    pub limit: Option<usize>,
}

// === API Types ===

#[derive(Debug, Serialize)]
pub struct ApiKgFact {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f64,
    pub source: String,
    pub evidence_count: i32,
}

#[derive(Debug, Serialize)]
pub struct ApiKgStats {
    pub entity_count: u64,
    pub fact_count: u64,
    pub gene_count: u64,
    pub cancer_count: u64,
}

#[derive(Debug, Serialize)]
pub struct ApiEntitySuggest {
    pub value: String,
}

/// GET /api/kg - List KG facts
pub async fn api_kg_facts(
    State(state): State<SharedState>,
    Query(filter): Query<KgFilter>,
) -> Result<impl IntoResponse, ApiError> {
    let _gene = filter.gene.as_deref().unwrap_or("");

    let fact_repo = KgFactRepository::new(state.db.clone());
    let facts = fact_repo.list(0, 100).await.unwrap_or_default();

    let api_facts: Vec<ApiKgFact> = facts
        .iter()
        .map(|f| ApiKgFact {
            subject: f.subject_name.clone(),
            predicate: f.predicate.clone(),
            object: f.object_name.clone(),
            confidence: f.confidence as f64,
            source: "unknown".to_string(),
            evidence_count: 1,
        })
        .collect();

    Ok(Json(api_facts))
}

/// GET /api/kg/stats - KG statistics
pub async fn api_kg_stats(State(state): State<SharedState>) -> Result<impl IntoResponse, ApiError> {
    let entity_repo = EntityRepository::new(state.db.clone());
    let fact_repo = KgFactRepository::new(state.db.clone());

    let mut entity_count = entity_repo.count().await.unwrap_or(0);
    let fact_count = fact_repo.count().await.unwrap_or(0);
    let mut gene_count = 0;
    let mut cancer_count = 0;

    // In some deployments we only persist facts and not denormalized entity rows.
    // Derive dashboard counts from KG facts so stats stay meaningful.
    if entity_count == 0 && fact_count > 0 {
        let facts = fact_repo.list(0, 20_000).await.unwrap_or_default();
        let (derived_entities, derived_genes, derived_cancers) =
            derive_entity_stats_from_facts(&facts);
        entity_count = derived_entities;
        gene_count = derived_genes;
        cancer_count = derived_cancers;
    }

    Ok(Json(ApiKgStats {
        entity_count,
        fact_count,
        gene_count,
        cancer_count,
    }))
}

/// GET /api/entities/suggest?q=...&limit=...
pub async fn api_entity_suggest(
    State(state): State<SharedState>,
    Query(query): Query<EntitySuggestQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let q = query.q.unwrap_or_default().trim().to_string();
    let limit = query.limit.unwrap_or(12).clamp(5, 30);
    let entity_repo = EntityRepository::new(state.db.clone());
    let fact_repo = KgFactRepository::new(state.db.clone());

    let mut values: Vec<String> = if q.len() >= 2 {
        entity_repo
            .search(&q, limit.saturating_mul(2))
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|e| e.name.trim().to_string())
            .filter(|v| !v.is_empty())
            .collect()
    } else {
        entity_repo
            .list(0, limit.saturating_mul(2))
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|e| e.name.trim().to_string())
            .filter(|v| !v.is_empty())
            .collect()
    };

    // Fallback for deployments where entities table is not yet populated:
    // derive top suggestions from KG fact subjects/objects.
    if values.is_empty() {
        let q_lc = q.to_lowercase();
        let facts = fact_repo.list(0, 2000).await.unwrap_or_default();
        for f in facts {
            let candidates = [f.subject_name, f.object_name];
            for c in candidates {
                let trimmed = c.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if !q_lc.is_empty() && !trimmed.to_lowercase().contains(&q_lc) {
                    continue;
                }
                values.push(trimmed.to_string());
            }
        }
    }

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for name in values {
        if !is_suggestable_name(&name) {
            continue;
        }
        if seen.insert(name.to_lowercase()) {
            out.push(ApiEntitySuggest { value: name });
            if out.len() >= limit {
                break;
            }
        }
    }

    Ok(Json(out))
}

pub async fn kg_page(
    State(state): State<SharedState>,
    Query(filter): Query<KgFilter>,
) -> Html<String> {
    const FACT_SCAN_LIMIT: usize = 4_000;
    const FILTERED_FACT_CAP: usize = 4_000;
    const MAX_GRAPH_NODES: usize = 260;
    const MAX_GRAPH_LINKS: usize = 1_000;
    const MAX_ROWS_PER_PAPER: usize = 40;

    let gene = filter.gene.clone().unwrap_or_default().trim().to_string();
    let q = filter.q.clone().unwrap_or_default();
    let predicate_filter = filter
        .predicate
        .clone()
        .unwrap_or_else(|| "all".to_string());
    let max_papers = filter.max_papers.unwrap_or(50).clamp(10, 200);
    let expanded_paper = filter.expanded.clone().unwrap_or_default();
    let cache_key = format!(
        "g={}|q={}|p={}|m={}",
        gene.trim().to_lowercase(),
        q.trim().to_lowercase(),
        predicate_filter.trim().to_lowercase(),
        max_papers
    );
    let is_snapshot_scope = gene.trim().is_empty()
        && q.trim().is_empty()
        && predicate_filter.trim().eq_ignore_ascii_case("all");
    let cache_ttl = if is_snapshot_scope {
        Duration::from_secs(300)
    } else {
        Duration::from_secs(45)
    };
    if let Some(cached) = kg_cache_get(&cache_key, cache_ttl) {
        return Html(cached);
    }

    let gene_lc = gene.to_lowercase();
    let q_lc = q.to_lowercase();
    let pred_lc = predicate_filter.to_lowercase();
    let fact_repo = KgFactRepository::new(state.db.clone());
    let paper_repo = PaperRepository::new(state.db.clone());
    let total_papers = paper_repo.count().await.unwrap_or(0);

    let scanned = fact_repo
        .list_filtered(
            if gene.trim().is_empty() {
                None
            } else {
                Some(gene.trim())
            },
            if q.trim().is_empty() {
                None
            } else {
                Some(q.trim())
            },
            Some(predicate_filter.trim()),
            FACT_SCAN_LIMIT,
        )
        .await
        .unwrap_or_default();
    let scanned_facts = scanned.len();
    let mut filtered_facts = Vec::new();

    for f in scanned.into_iter() {
        if filtered_facts.len() >= FILTERED_FACT_CAP {
            break;
        }

        let subject_lc = f.subject_name.to_lowercase();
        let object_lc = f.object_name.to_lowercase();
        let predicate_lc = f.predicate.to_lowercase();

        let gene_match = gene_lc.is_empty()
            || subject_lc.contains(&gene_lc)
            || object_lc.contains(&gene_lc)
            || predicate_lc.contains(&gene_lc);

        let q_match = q_lc.is_empty()
            || subject_lc.contains(&q_lc)
            || object_lc.contains(&q_lc)
            || predicate_lc.contains(&q_lc)
            || f.paper_id.to_string().contains(&q_lc);

        let predicate_match = pred_lc == "all" || predicate_lc == pred_lc;

        if gene_match && q_match && predicate_match {
            filtered_facts.push(f);
        }
    }

    let focus_label = if gene.is_empty() {
        "All matched entities".to_string()
    } else {
        gene.clone()
    };

    // Build a compact, entity-centric graph for performance: aggregate duplicate edges
    let mut edge_counts: HashMap<(String, String, String), usize> = HashMap::new();
    let mut degree: HashMap<String, usize> = HashMap::new();

    for f in &filtered_facts {
        if f.predicate == "mentions" {
            continue;
        }

        let source = f.subject_name.trim();
        let target = f.object_name.trim();

        if source.is_empty() || target.is_empty() || source == target {
            continue;
        }

        *edge_counts
            .entry((source.to_string(), target.to_string(), f.predicate.clone()))
            .or_insert(0) += 1;

        *degree.entry(source.to_string()).or_insert(0) += 1;
        *degree.entry(target.to_string()).or_insert(0) += 1;
    }

    // Keep only top connected nodes to stay interactive at large scale.
    let mut degree_pairs: Vec<(String, usize)> = degree.into_iter().collect();
    degree_pairs.sort_by(|a, b| b.1.cmp(&a.1));

    let selected_nodes: HashSet<String> = degree_pairs
        .iter()
        .take(MAX_GRAPH_NODES)
        .map(|(name, _)| name.clone())
        .collect();

    let mut graph_links = Vec::new();
    for ((source, target, predicate), weight) in edge_counts {
        if selected_nodes.contains(&source) && selected_nodes.contains(&target) {
            graph_links.push(serde_json::json!({
                "source": source,
                "target": target,
                "label": predicate,
                "weight": weight,
            }));
        }
    }

    graph_links.sort_by(|a, b| {
        let aw = a.get("weight").and_then(|w| w.as_u64()).unwrap_or(0);
        let bw = b.get("weight").and_then(|w| w.as_u64()).unwrap_or(0);
        bw.cmp(&aw)
    });
    graph_links.truncate(MAX_GRAPH_LINKS);
    let graph_link_count = graph_links.len();

    let mut final_node_degree: HashMap<String, usize> = HashMap::new();
    for link in &graph_links {
        if let Some(src) = link.get("source").and_then(|v| v.as_str()) {
            *final_node_degree.entry(src.to_string()).or_insert(0) += 1;
        }
        if let Some(dst) = link.get("target").and_then(|v| v.as_str()) {
            *final_node_degree.entry(dst.to_string()).or_insert(0) += 1;
        }
    }

    let graph_nodes: Vec<_> = final_node_degree
        .iter()
        .map(|(name, deg)| {
            let group = if name.eq_ignore_ascii_case(&gene) {
                1
            } else {
                2
            };
            let size = 3.2 + (*deg as f64).ln_1p() * 1.9;
            let short = truncate(name, 42);

            serde_json::json!({
                "id": name,
                "name": name,
                "short": short,
                "group": group,
                "degree": deg,
                "size": size,
            })
        })
        .collect();

    let graph_json = serde_json::to_string(&serde_json::json!({
        "nodes": graph_nodes,
        "links": graph_links,
    }))
    .unwrap_or_else(|_| "{}".to_string());

    // Group evidence by paper; default collapsed to avoid huge page heights.
    let mut paper_groups: HashMap<String, Vec<(String, String, String, Option<String>)>> =
        HashMap::new();
    for f in &filtered_facts {
        let paper_key = if f.paper_id.is_nil() {
            "unknown-paper".to_string()
        } else {
            format!("paper-{}", f.paper_id)
        };

        paper_groups.entry(paper_key).or_default().push((
            f.subject_name.clone(),
            f.predicate.clone(),
            f.object_name.clone(),
            f.evidence.clone(),
        ));
    }

    let mut paper_entries: Vec<(String, Vec<(String, String, String, Option<String>)>)> =
        paper_groups.into_iter().collect();
    paper_entries.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    let matched_paper_count = paper_entries.len();

    let selected_paper_ids: Vec<uuid::Uuid> = paper_entries
        .iter()
        .take(max_papers)
        .filter_map(|(paper, _)| {
            paper
                .strip_prefix("paper-")
                .and_then(|id| uuid::Uuid::parse_str(id).ok())
        })
        .collect();

    let paper_titles = paper_repo
        .find_titles_by_ids(&selected_paper_ids)
        .await
        .unwrap_or_default();

    let paper_html = if paper_entries.is_empty() {
        format!(
            r#"<div class="card-body text-muted">No KG evidence found for <strong class="text-main">{}</strong>.</div>"#,
            gene
        )
    } else {
        paper_entries
            .into_iter()
            .take(max_papers)
            .map(|(paper, rows)| {
                let open_attr = if !expanded_paper.is_empty() && expanded_paper == paper {
                    "open"
                } else {
                    ""
                };

                let title = paper
                    .strip_prefix("paper-")
                    .and_then(|id| uuid::Uuid::parse_str(id).ok())
                    .and_then(|id| paper_titles.get(&id).cloned())
                    .unwrap_or_else(|| truncate(&paper, 56));

                let chunk = rows
                    .iter()
                    .find_map(|(_, _, _, ev)| ev.clone())
                    .unwrap_or_else(|| "No chunk preview available.".to_string());
                let chunk_tooltip = html_escape(&truncate(&chunk, 360));

                let non_mention_rows: Vec<_> = rows
                    .iter()
                    .filter(|(_, predicate, _, _)| predicate.to_lowercase() != "mentions")
                    .collect();
                let shown = non_mention_rows.len().min(MAX_ROWS_PER_PAPER);

                let mut rows_html = if non_mention_rows.is_empty() {
                    r#"<tr><td colspan="3" class="text-muted">No non-mention relation edges extracted yet for this paper.</td></tr>"#.to_string()
                } else {
                    non_mention_rows
                        .iter()
                        .take(MAX_ROWS_PER_PAPER)
                        .map(|(subject, predicate, object, _)| {
                            format!(
                                r#"<tr>
                                    <td class="text-main">{}</td>
                                    <td><span class="badge badge-outline">{}</span></td>
                                    <td class="text-main">{}</td>
                                </tr>"#,
                                subject, predicate, object
                            )
                        })
                        .collect()
                };
                if non_mention_rows.len() > MAX_ROWS_PER_PAPER {
                    rows_html.push_str(&format!(
                        r#"<tr><td colspan="3" class="text-muted">Showing first {} of {} relations for performance.</td></tr>"#,
                        shown,
                        non_mention_rows.len()
                    ));
                }

                format!(
                    r#"<details class="paper-group" {}>
                        <summary>
                            <div class="d-flex align-center gap-2">
                                <span class="paper-title">{}</span>
                                <span class="info-tip">i
                                    <span class="tooltip-card">{}</span>
                                </span>
                            </div>
                            <span class="badge badge-primary">{} relations</span>
                        </summary>
                        <div class="table-container">
                            <table class="table">
                                <thead>
                                    <tr>
                                        <th>Subject</th>
                                        <th>Predicate</th>
                                        <th>Object</th>
                                    </tr>
                                </thead>
                                <tbody>{}</tbody>
                            </table>
                        </div>
                    </details>"#,
                    open_attr,
                    title,
                    chunk_tooltip,
                    rows.len(),
                    rows_html
                )
            })
            .collect()
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Knowledge Graph — Ferrumyx</title>
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@400;500;600;700&family=Inter:wght@400;500;600&display=swap" rel="stylesheet">
    <link rel="stylesheet" href="/static/css/main.css?v=1.0.3">
    <style>
        .kg-filters {{
            display: grid;
            grid-template-columns: minmax(180px, 260px) 1fr minmax(150px, 220px) minmax(120px, 140px) auto;
            gap: 12px;
            align-items: end;
            margin-bottom: 16px;
        }}

        .paper-group {{
            border-top: 1px solid var(--border-color);
            content-visibility: auto;
            contain-intrinsic-size: 320px;
        }}

        .paper-group summary {{
            list-style: none;
            display: flex;
            justify-content: space-between;
            align-items: center;
            gap: 10px;
            cursor: pointer;
            padding: 10px 14px;
            background: rgba(16, 23, 36, 0.72);
        }}

        .paper-group summary::-webkit-details-marker {{
            display: none;
        }}

        .paper-group[open] summary {{
            border-bottom: 1px solid rgba(122, 144, 179, 0.2);
            background: rgba(22, 31, 47, 0.78);
        }}

        .paper-title {{
            color: var(--text-main);
            font-weight: 600;
            font-size: 0.92rem;
            line-height: 1.25;
            max-width: 840px;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
        }}

        @media (max-width: 1100px) {{
            .kg-filters {{
                grid-template-columns: 1fr 1fr;
            }}
        }}
    </style>
</head>
<body>
{}
<main class="main-content">
    <div class="page-header">
        <div>
            <h1 class="page-title">
                <svg xmlns="http://www.w3.org/2000/svg" width="28" height="28" viewBox="0 0 24 24"><path d="M16 11c1.66 0 2.99-1.34 2.99-3S17.66 5 16 5c-1.66 0-3 1.34-3 3v.15l-3.32 1.62A2.97 2.97 0 0 0 8 9c-1.66 0-3 1.34-3 3s1.34 3 3 3c.6 0 1.15-.18 1.61-.48l3.36 1.64c-.01.12-.04.24-.04.37 0 1.66 1.34 3 3 3s3-1.34 3-3-1.34-3-3-3c-.62 0-1.18.19-1.64.5l-3.32-1.62C10.96 12.15 11 12.04 11 11.91V11.9z"/></svg>
                Knowledge Graph
            </h1>
            <p class="text-muted">Entity-focused graph view for speed, with paper evidence grouped in collapsible sections.</p>
        </div>
    </div>

    <form class="kg-filters" method="GET" action="/kg">
        <div>
            <label class="form-label">Entity Focus</label>
            <input type="text" id="entity-focus-input" name="gene" class="form-control" list="entity-options" autocomplete="off" placeholder="Start typing an extracted entity..." value="{}">
            <datalist id="entity-options"></datalist>
        </div>
        <div>
            <label class="form-label">Search Relations</label>
            <input type="text" name="q" class="form-control" placeholder="filter by text / paper id" value="{}">
        </div>
        <div>
            <label class="form-label">Predicate</label>
            <select name="predicate" class="form-control">
                <option value="all" {}>All</option>
                <option value="mentions" {}>mentions</option>
                <option value="interacts_with" {}>interacts_with</option>
                <option value="inhibits" {}>inhibits</option>
                <option value="activates" {}>activates</option>
            </select>
        </div>
        <div>
            <label class="form-label">Papers</label>
            <input type="number" name="max_papers" class="form-control" min="10" max="200" value="{}">
        </div>
        <button type="submit" class="btn btn-primary">Apply</button>
    </form>

    <div class="card mb-4" style="padding:0; overflow:hidden;">
        <div class="card-header">
            <div>Entity Graph around <span class="text-gradient">{}</span></div>
            <div class="d-flex gap-2">
                <span class="badge badge-outline">{} papers in DB</span>
                <span class="badge badge-outline">{} facts scanned</span>
                <span class="badge badge-outline">{} papers matched</span>
                <span class="badge badge-outline">{} nodes</span>
                <span class="badge badge-outline">{} links</span>
            </div>
        </div>
        <div id="graph-container" style="width:100%; height:560px;"></div>
    </div>

    <div class="card">
        <div class="card-header">
            <div>Paper Evidence (Collapsed by Default)</div>
            <span class="badge badge-outline">showing up to {} papers</span>
        </div>
        <div>{}</div>
    </div>
</main>
<script src="/static/js/main.js"></script>
<script>
    const graphData = {};
    const focusInput = document.getElementById('entity-focus-input');
    const entityOptions = document.getElementById('entity-options');
    const suggestCache = new Map();
    let suggestTimer = null;
    let inFlight = null;

    function setEntityOptions(items) {{
        entityOptions.innerHTML = '';
        (items || []).forEach(item => {{
            const opt = document.createElement('option');
            opt.value = (item && item.value) ? item.value : '';
            entityOptions.appendChild(opt);
        }});
    }}

    async function fetchEntitySuggestions(term) {{
        const q = (term || '').trim();
        if (!q) {{
            setEntityOptions([]);
            return;
        }}

        if (suggestCache.has(q)) {{
            setEntityOptions(suggestCache.get(q));
            return;
        }}

        if (inFlight) {{
            inFlight.abort();
        }}

        inFlight = new AbortController();
        try {{
            const resp = await fetch(`/api/entities/suggest?q=${{encodeURIComponent(q)}}&limit=12`, {{
                signal: inFlight.signal,
                headers: {{ 'Accept': 'application/json' }},
            }});
            if (!resp.ok) return;
            const data = await resp.json();
            suggestCache.set(q, data);
            setEntityOptions(data);
        }} catch (_err) {{
            // ignore abort/network noise
        }}
    }}

    if (focusInput) {{
        focusInput.addEventListener('input', () => {{
            clearTimeout(suggestTimer);
            suggestTimer = setTimeout(() => fetchEntitySuggestions(focusInput.value), 180);
        }});
        focusInput.addEventListener('focus', () => {{
            if ((focusInput.value || '').trim().length >= 2) {{
                fetchEntitySuggestions(focusInput.value);
            }}
        }});
    }}

    const elem = document.getElementById('graph-container');
    let graphBooted = false;

    function bootGraph() {{
        if (graphBooted) return;
        graphBooted = true;
        const script = document.createElement('script');
        script.src = 'https://unpkg.com/force-graph';
        script.onload = () => startGraph();
        document.body.appendChild(script);
    }}

    function startGraph() {{
        if (!graphData.nodes || graphData.nodes.length === 0) {{
            elem.innerHTML = '<div style="display:flex; height:100%; align-items:center; justify-content:center; color: var(--text-muted);">No graph edges matched your filters.</div>';
            return;
        }}

        const Graph = ForceGraph()(elem)
            .graphData(graphData)
            .backgroundColor('transparent')
            .nodeLabel(node => `${{node.name}} (${{node.degree}} links)`)
            .nodeVal('size')
            .linkColor(link => {{
                const l = (link.label || '').toLowerCase();
                if (l.includes('inhibit')) return 'rgba(255,125,144,0.55)';
                if (l.includes('activate')) return 'rgba(57,211,158,0.55)';
                return 'rgba(109,168,255,0.40)';
            }})
            .linkWidth(link => Math.min(2.4, 0.65 + Math.log2((link.weight || 1) + 1)))
            .cooldownTime(1300)
            .d3AlphaDecay(0.12)
            .d3VelocityDecay(0.5)
            .onNodeClick(node => {{
                Graph.centerAt(node.x, node.y, 800);
                Graph.zoom(4.2, 600);
            }})
            .nodeCanvasObject((node, ctx, globalScale) => {{
                const r = node.size || 3;
                ctx.beginPath();
                ctx.arc(node.x, node.y, r, 0, 2 * Math.PI, false);

                if (node.group === 1) {{
                    ctx.fillStyle = 'rgba(193, 142, 255, 0.95)';
                }} else if (node.degree >= 8) {{
                    ctx.fillStyle = 'rgba(110, 202, 255, 0.90)';
                }} else {{
                    ctx.fillStyle = 'rgba(95, 146, 235, 0.78)';
                }}

                ctx.fill();

                const showLabel = globalScale > 2.4 && (node.group === 1 || node.degree >= 14);
                if (showLabel) {{
                    ctx.font = `${{Math.max(9, 11 / globalScale)}}px Inter`;
                    ctx.fillStyle = 'rgba(235, 242, 255, 0.72)';
                    ctx.textAlign = 'center';
                    ctx.fillText(node.short || node.name, node.x, node.y + r + 8);
                }}
            }})
            .onEngineStop(() => {{
                Graph.zoomToFit(320, 40);
                Graph.pauseAnimation();
            }});

        window.addEventListener('resize', () => {{
            if (elem.clientWidth > 0 && elem.clientHeight > 0) {{
                Graph.width(elem.clientWidth).height(elem.clientHeight);
            }}
        }});
    }}

    if ('IntersectionObserver' in window) {{
        const io = new IntersectionObserver((entries) => {{
            for (const entry of entries) {{
                if (entry.isIntersecting) {{
                    bootGraph();
                    io.disconnect();
                    break;
                }}
            }}
        }}, {{ rootMargin: '240px' }});
        io.observe(elem);
    }} else {{
        setTimeout(bootGraph, 120);
    }}
</script>
</body>
</html>"#,
        NAV_HTML,
        html_escape(&gene),
        q,
        if pred_lc == "all" { "selected" } else { "" },
        if pred_lc == "mentions" {
            "selected"
        } else {
            ""
        },
        if pred_lc == "interacts_with" {
            "selected"
        } else {
            ""
        },
        if pred_lc == "inhibits" {
            "selected"
        } else {
            ""
        },
        if pred_lc == "activates" {
            "selected"
        } else {
            ""
        },
        max_papers,
        html_escape(&focus_label),
        total_papers,
        scanned_facts,
        matched_paper_count,
        final_node_degree.len(),
        graph_link_count,
        max_papers,
        paper_html,
        graph_json,
    );
    kg_cache_put(cache_key, html.clone());
    Html(html)
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }

    let mut out = String::with_capacity(max_chars + 1);
    for (idx, ch) in s.chars().enumerate() {
        if idx >= max_chars.saturating_sub(1) {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn kg_cache_get(key: &str, ttl: Duration) -> Option<String> {
    let cache = KG_PAGE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = cache.lock().ok()?;
    if let Some(entry) = guard.get(key) {
        if entry.created_at.elapsed() <= ttl {
            return Some(entry.html.clone());
        }
    }
    guard.remove(key);
    None
}

fn kg_cache_put(key: String, html: String) {
    let cache = KG_PAGE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut guard) = cache.lock() {
        if guard.len() > 64 {
            guard.clear();
        }
        guard.insert(
            key,
            CachedHtml {
                html,
                created_at: Instant::now(),
            },
        );
    }
}

fn is_suggestable_name(name: &str) -> bool {
    let trimmed = name.trim();
    let len = trimmed.chars().count();
    if len < 2 || len > 80 {
        return false;
    }
    if trimmed.contains('\n') || trimmed.contains('\r') {
        return false;
    }
    let words = trimmed.split_whitespace().count();
    if words > 7 {
        return false;
    }
    true
}

fn derive_entity_stats_from_facts(facts: &[ferrumyx_db::schema::KgFact]) -> (u64, u64, u64) {
    let mut entities = HashSet::new();
    let mut genes = HashSet::new();
    let mut cancers = HashSet::new();

    for fact in facts {
        for raw in [&fact.subject_name, &fact.object_name] {
            let name = raw.trim();
            if name.is_empty() {
                continue;
            }

            let lc = name.to_lowercase();
            entities.insert(lc.clone());

            let is_gene_like = name.len() <= 10
                && name.chars().any(|c| c.is_ascii_uppercase())
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
            if is_gene_like {
                genes.insert(lc.clone());
            }

            let is_cancer_like = lc.contains("cancer")
                || lc.contains("carcinoma")
                || lc.contains("tumor")
                || lc.contains("sarcoma")
                || lc.contains("lymphoma")
                || lc.contains("leukemia");
            if is_cancer_like {
                cancers.insert(lc);
            }
        }
    }

    (
        entities.len() as u64,
        genes.len() as u64,
        cancers.len() as u64,
    )
}
