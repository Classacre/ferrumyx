//! Knowledge graph explorer.

use std::collections::{HashMap, HashSet, VecDeque};
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
use ferrumyx_db::papers::{PaperReference, PaperRepository};

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
    pub confidence_tier: Option<String>,
    pub max_papers: Option<usize>,
    pub view: Option<String>,
    pub lens: Option<String>,
    pub preset: Option<String>,
    pub source: Option<String>,
    pub target: Option<String>,
    pub hops: Option<usize>,
    pub expanded: Option<String>,
}

#[derive(Clone, Debug)]
struct ScoredEdge {
    source: String,
    target: String,
    predicate: String,
    weight: usize,
    avg_confidence: f64,
    specificity: f64,
    confidence_tier: String,
    provenance: String,
    score: f64,
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
    pub confidence_tier: String,
    pub provenance: String,
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
            confidence_tier: classify_confidence_tier(f).to_string(),
            provenance: classify_fact_provenance(f).to_string(),
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
    const MAX_ROWS_PER_PAPER: usize = 40;

    let gene = filter.gene.clone().unwrap_or_default().trim().to_string();
    let q = filter.q.clone().unwrap_or_default();
    let requested_lens = filter.lens.clone().unwrap_or_else(kg_default_lens_mode);
    let lens_mode = normalize_lens_mode(&requested_lens);
    let requested_preset = filter
        .preset
        .clone()
        .unwrap_or_else(kg_default_render_preset);
    let render_preset = normalize_render_preset(&requested_preset);
    let is_dense_preset = render_preset == "dense";
    let path_source = filter.source.clone().unwrap_or_default().trim().to_string();
    let path_target = filter.target.clone().unwrap_or_default().trim().to_string();
    let local_hops = filter.hops.unwrap_or(1).clamp(1, 3);
    let requested_view = filter.view.clone().unwrap_or_else(kg_default_mode);
    let view_mode = if requested_view.trim().eq_ignore_ascii_case("3d") {
        "3d".to_string()
    } else {
        "2d".to_string()
    };
    let use_3d = view_mode == "3d";
    let base_max_graph_nodes = if use_3d {
        kg_3d_max_nodes()
    } else {
        kg_2d_max_nodes()
    };
    let base_max_graph_links = if use_3d {
        kg_3d_max_links()
    } else {
        kg_2d_max_links()
    };
    let max_graph_nodes = if is_dense_preset {
        ((base_max_graph_nodes as f64) * 1.30).round() as usize
    } else {
        base_max_graph_nodes
    };
    let max_graph_links = if is_dense_preset {
        ((base_max_graph_links as f64) * 1.45).round() as usize
    } else {
        base_max_graph_links
    };
    let predicate_filter = filter
        .predicate
        .clone()
        .unwrap_or_else(|| "specific".to_string());
    let confidence_filter = normalize_confidence_filter(
        &filter
            .confidence_tier
            .clone()
            .unwrap_or_else(|| "all".to_string()),
    );
    let max_papers = filter.max_papers.unwrap_or(50).clamp(10, 200);
    let expanded_paper = filter.expanded.clone().unwrap_or_default();
    let cache_key = format!(
        "g={}|q={}|p={}|c={}|m={}|v={}|l={}|r={}|s={}|t={}|h={}",
        gene.trim().to_lowercase(),
        q.trim().to_lowercase(),
        predicate_filter.trim().to_lowercase(),
        confidence_filter,
        max_papers,
        view_mode,
        lens_mode,
        render_preset,
        path_source.trim().to_lowercase(),
        path_target.trim().to_lowercase(),
        local_hops
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
    let confidence_filter_lc = confidence_filter.to_lowercase();
    let repo_predicate_filter = if is_virtual_predicate_filter(&pred_lc) {
        "all"
    } else {
        predicate_filter.trim()
    };
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
            Some(repo_predicate_filter),
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

        let predicate_match = predicate_matches_filter(&predicate_lc, &pred_lc);
        let confidence_match = confidence_filter_matches(
            classify_confidence_tier(&f),
            classify_fact_provenance(&f),
            &confidence_filter_lc,
        );

        if gene_match && q_match && predicate_match && confidence_match {
            filtered_facts.push(f);
        }
    }

    let focus_label = match lens_mode.as_str() {
        "atlas" => "Community Atlas".to_string(),
        "path" => {
            if !path_source.is_empty() && !path_target.is_empty() {
                format!("{} → {}", path_source, path_target)
            } else {
                "Path Explorer".to_string()
            }
        }
        _ => {
            if gene.is_empty() {
                "All matched entities".to_string()
            } else {
                gene.clone()
            }
        }
    };

    // Pass 1 anti-hairball: relation specificity + evidence/confidence scoring + hub dampening.
    #[derive(Default, Clone)]
    struct EdgeAccumulator {
        evidence_count: usize,
        confidence_sum: f64,
        high_count: usize,
        medium_count: usize,
        low_count: usize,
        provider_count: usize,
        extracted_count: usize,
        generic_count: usize,
    }

    let hub_suppression = if is_dense_preset {
        (kg_hub_suppression_strength() * 0.72).clamp(0.0, 3.0)
    } else {
        kg_hub_suppression_strength()
    };
    let per_node_edge_cap = if is_dense_preset {
        kg_per_node_edge_cap().saturating_mul(2).min(400)
    } else {
        kg_per_node_edge_cap()
    };
    let mut edge_acc: HashMap<(String, String, String), EdgeAccumulator> = HashMap::new();
    let mut raw_degree: HashMap<String, usize> = HashMap::new();

    for f in &filtered_facts {
        let source = f.subject_name.trim();
        let target = f.object_name.trim();

        if source.is_empty() || target.is_empty() || source == target {
            continue;
        }

        let entry = edge_acc
            .entry((source.to_string(), target.to_string(), f.predicate.clone()))
            .or_default();
        entry.evidence_count += 1;
        entry.confidence_sum += (f.confidence as f64).clamp(0.01, 1.0);
        match classify_confidence_tier(f) {
            "high" => entry.high_count += 1,
            "medium" => entry.medium_count += 1,
            _ => entry.low_count += 1,
        }
        match classify_fact_provenance(f) {
            "provider" => entry.provider_count += 1,
            "generic" => entry.generic_count += 1,
            _ => entry.extracted_count += 1,
        }

        *raw_degree.entry(source.to_string()).or_insert(0) += 1;
        *raw_degree.entry(target.to_string()).or_insert(0) += 1;
    }

    let mut candidate_edges: Vec<ScoredEdge> = edge_acc
        .into_iter()
        .map(|((source, target, predicate), acc)| {
            let src_deg = raw_degree.get(&source).copied().unwrap_or(1);
            let dst_deg = raw_degree.get(&target).copied().unwrap_or(1);
            let avg_confidence = if acc.evidence_count > 0 {
                acc.confidence_sum / acc.evidence_count as f64
            } else {
                0.5
            };
            let confidence_tier = dominant_label(
                &[
                    ("high", acc.high_count),
                    ("medium", acc.medium_count),
                    ("low", acc.low_count),
                ],
                "medium",
            )
            .to_string();
            let provenance = dominant_label(
                &[
                    ("provider", acc.provider_count),
                    ("extracted", acc.extracted_count),
                    ("generic", acc.generic_count),
                ],
                "extracted",
            )
            .to_string();
            let specificity = predicate_specificity(&predicate);
            let tier_multiplier = match confidence_tier.as_str() {
                "high" => 1.16,
                "low" => 0.82,
                _ => 1.0,
            };
            let score = edge_quality_score(
                acc.evidence_count,
                avg_confidence,
                specificity,
                src_deg,
                dst_deg,
                hub_suppression,
            ) * tier_multiplier;
            ScoredEdge {
                source,
                target,
                predicate,
                weight: acc.evidence_count,
                avg_confidence,
                specificity,
                confidence_tier,
                provenance,
                score,
            }
        })
        .collect();

    candidate_edges.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| b.weight.cmp(&a.weight))
            .then_with(|| a.predicate.cmp(&b.predicate))
    });

    // Cap links per node so hubs cannot dominate the entire visualization.
    let mut per_node_kept: HashMap<String, usize> = HashMap::new();
    let mut capped_edges = Vec::new();
    for edge in candidate_edges {
        let src_used = per_node_kept.get(&edge.source).copied().unwrap_or(0);
        let dst_used = per_node_kept.get(&edge.target).copied().unwrap_or(0);
        if src_used >= per_node_edge_cap || dst_used >= per_node_edge_cap {
            continue;
        }
        *per_node_kept.entry(edge.source.clone()).or_insert(0) += 1;
        *per_node_kept.entry(edge.target.clone()).or_insert(0) += 1;
        capped_edges.push(edge);
        if capped_edges.len() >= max_graph_links.saturating_mul(4) {
            break;
        }
    }

    let mut lens_status = String::new();
    let mut working_edges = capped_edges;

    let collect_nodes_within_hops =
        |edges: &[ScoredEdge], seed: &str, hops: usize, max_nodes: usize| -> HashSet<String> {
            if seed.trim().is_empty() {
                return HashSet::new();
            }
            let seed_lc = seed.trim().to_lowercase();
            let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
            for e in edges {
                adjacency
                    .entry(e.source.clone())
                    .or_default()
                    .push(e.target.clone());
                adjacency
                    .entry(e.target.clone())
                    .or_default()
                    .push(e.source.clone());
            }
            let start = adjacency
                .keys()
                .find(|name| name.to_lowercase() == seed_lc)
                .cloned()
                .unwrap_or_else(|| seed.trim().to_string());
            if !adjacency.contains_key(&start) {
                return HashSet::new();
            }
            let mut out = HashSet::new();
            let mut queue = VecDeque::new();
            queue.push_back((start.clone(), 0usize));
            out.insert(start);
            while let Some((node, depth)) = queue.pop_front() {
                if depth >= hops {
                    continue;
                }
                if let Some(neighbors) = adjacency.get(&node) {
                    for next in neighbors {
                        if out.len() >= max_nodes {
                            break;
                        }
                        if out.insert(next.clone()) {
                            queue.push_back((next.clone(), depth + 1));
                        }
                    }
                }
                if out.len() >= max_nodes {
                    break;
                }
            }
            out
        };

    let find_path_edges =
        |edges: &[ScoredEdge], source: &str, target: &str, max_edges: usize| -> Vec<ScoredEdge> {
            if source.trim().is_empty() || target.trim().is_empty() {
                return Vec::new();
            }
            let source_lc = source.trim().to_lowercase();
            let target_lc = target.trim().to_lowercase();
            let mut node_keys = HashSet::new();
            for e in edges {
                node_keys.insert(e.source.clone());
                node_keys.insert(e.target.clone());
            }
            let src = match node_keys
                .iter()
                .find(|n| n.to_lowercase() == source_lc)
                .cloned()
            {
                Some(v) => v,
                None => return Vec::new(),
            };
            let dst = match node_keys
                .iter()
                .find(|n| n.to_lowercase() == target_lc)
                .cloned()
            {
                Some(v) => v,
                None => return Vec::new(),
            };

            let mut adjacency_idx: HashMap<String, Vec<usize>> = HashMap::new();
            for (idx, e) in edges.iter().enumerate() {
                adjacency_idx.entry(e.source.clone()).or_default().push(idx);
                adjacency_idx.entry(e.target.clone()).or_default().push(idx);
            }

            let mut visited = HashSet::new();
            let mut prev: HashMap<String, (String, usize)> = HashMap::new();
            let mut queue = VecDeque::new();
            queue.push_back(src.clone());
            visited.insert(src.clone());

            let mut found = false;
            while let Some(curr) = queue.pop_front() {
                if curr == dst {
                    found = true;
                    break;
                }
                if let Some(edge_ids) = adjacency_idx.get(&curr) {
                    for edge_idx in edge_ids {
                        let e = &edges[*edge_idx];
                        let next = if e.source == curr {
                            e.target.clone()
                        } else {
                            e.source.clone()
                        };
                        if visited.insert(next.clone()) {
                            prev.insert(next.clone(), (curr.clone(), *edge_idx));
                            queue.push_back(next);
                        }
                    }
                }
            }
            if !found {
                return Vec::new();
            }

            let mut edge_ids: HashSet<usize> = HashSet::new();
            let mut path_nodes: Vec<String> = Vec::new();
            let mut cursor = dst.clone();
            path_nodes.push(cursor.clone());
            while cursor != src {
                let Some((from, edge_idx)) = prev.get(&cursor).cloned() else {
                    break;
                };
                edge_ids.insert(edge_idx);
                cursor = from;
                path_nodes.push(cursor.clone());
            }

            // Include a small amount of local context around the path.
            for node in &path_nodes {
                if let Some(ids) = adjacency_idx.get(node) {
                    let mut sorted = ids.clone();
                    sorted.sort_by(|a, b| edges[*b].score.total_cmp(&edges[*a].score));
                    for edge_idx in sorted.into_iter().take(2) {
                        edge_ids.insert(edge_idx);
                    }
                }
            }

            let mut out: Vec<ScoredEdge> =
                edge_ids.into_iter().map(|idx| edges[idx].clone()).collect();
            out.sort_by(|a, b| {
                b.score
                    .total_cmp(&a.score)
                    .then_with(|| b.weight.cmp(&a.weight))
            });
            out.truncate(max_edges.max(8));
            out
        };

    if lens_mode == "path" {
        let path_edges =
            find_path_edges(&working_edges, &path_source, &path_target, max_graph_links);
        if path_edges.is_empty() {
            lens_status = if path_source.is_empty() || path_target.is_empty() {
                "Path mode requires both source and target entities. Showing analysis graph."
                    .to_string()
            } else {
                format!(
                    "No path found between '{}' and '{}' in current filtered scope. Showing analysis graph.",
                    path_source, path_target
                )
            };
        } else {
            working_edges = path_edges;
            lens_status = format!(
                "Path lens active: focused subgraph between '{}' and '{}'.",
                path_source, path_target
            );
        }
    }

    let mut node_strength: HashMap<String, f64> = HashMap::new();
    for e in &working_edges {
        *node_strength.entry(e.source.clone()).or_insert(0.0) += e.score;
        *node_strength.entry(e.target.clone()).or_insert(0.0) += e.score;
    }

    let mut node_rank: Vec<(String, f64)> = node_strength.into_iter().collect();
    node_rank.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut selected_nodes: HashSet<String> = node_rank
        .iter()
        .take(max_graph_nodes)
        .map(|(name, _)| name.clone())
        .collect();

    if lens_mode == "analysis" && !gene.trim().is_empty() {
        let hop_nodes =
            collect_nodes_within_hops(&working_edges, gene.trim(), local_hops, max_graph_nodes);
        if hop_nodes.len() >= 2 {
            selected_nodes = hop_nodes;
            lens_status = format!(
                "Analysis lens: {}-hop neighborhood around '{}'.",
                local_hops, gene
            );
        }
    }

    let mut selected_edges: Vec<ScoredEdge> = working_edges
        .into_iter()
        .filter(|e| selected_nodes.contains(&e.source) && selected_nodes.contains(&e.target))
        .collect();
    selected_edges.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| b.weight.cmp(&a.weight))
    });

    let mut atlas_node_meta: HashMap<String, (String, i32, f64, String)> = HashMap::new();
    let mut graph_links = Vec::new();
    if lens_mode == "atlas" {
        let atlas_cluster_cap = kg_atlas_max_clusters();
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
        for e in &selected_edges {
            adjacency
                .entry(e.source.clone())
                .or_default()
                .push(e.target.clone());
            adjacency
                .entry(e.target.clone())
                .or_default()
                .push(e.source.clone());
        }

        let mut visited = HashSet::new();
        let mut components: Vec<Vec<String>> = Vec::new();
        for node in adjacency.keys() {
            if visited.contains(node) {
                continue;
            }
            let mut queue = VecDeque::new();
            let mut component = Vec::new();
            queue.push_back(node.clone());
            visited.insert(node.clone());
            while let Some(curr) = queue.pop_front() {
                component.push(curr.clone());
                if let Some(neighbors) = adjacency.get(&curr) {
                    for next in neighbors {
                        if visited.insert(next.clone()) {
                            queue.push_back(next.clone());
                        }
                    }
                }
            }
            components.push(component);
        }

        components.sort_by(|a, b| b.len().cmp(&a.len()));
        components.truncate(atlas_cluster_cap);
        let mut node_cluster: HashMap<String, String> = HashMap::new();
        for (idx, component) in components.iter().enumerate() {
            let cluster_id = format!("cluster-{}", idx + 1);
            let mut members = component.clone();
            members.sort();
            let preview = members
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            let label = if preview.is_empty() {
                format!("Cluster {}", idx + 1)
            } else {
                format!("Cluster {}: {}", idx + 1, preview)
            };
            let short = truncate(&label, 42);
            let group = if !gene.is_empty()
                && members.iter().any(|m| m.eq_ignore_ascii_case(gene.trim()))
            {
                1
            } else {
                2
            };
            let size = 6.2 + (members.len() as f64).ln_1p() * 2.8;
            atlas_node_meta.insert(cluster_id.clone(), (label, group, size, short));
            for member in component {
                node_cluster.insert(member.clone(), cluster_id.clone());
            }
        }

        let mut cluster_links: HashMap<(String, String), (usize, f64, f64)> = HashMap::new();
        for e in &selected_edges {
            let Some(src_cluster) = node_cluster.get(&e.source).cloned() else {
                continue;
            };
            let Some(dst_cluster) = node_cluster.get(&e.target).cloned() else {
                continue;
            };
            if src_cluster == dst_cluster {
                continue;
            }
            let key = if src_cluster <= dst_cluster {
                (src_cluster, dst_cluster)
            } else {
                (dst_cluster, src_cluster)
            };
            let entry = cluster_links.entry(key).or_insert((0, 0.0, 0.0));
            entry.0 += e.weight;
            entry.1 += e.score;
            entry.2 += e.avg_confidence;
        }

        for ((source, target), (weight, score_sum, confidence_sum)) in cluster_links {
            graph_links.push(serde_json::json!({
                "source": source,
                "target": target,
                "label": "cluster_link",
                "weight": weight,
                "score": (score_sum * 1000.0).round() / 1000.0,
                "confidence": (confidence_sum * 1000.0).round() / 1000.0,
                "specificity": 1.0,
                "confidence_tier": "medium",
                "provenance": "aggregated",
            }));
        }
        lens_status = format!(
            "Atlas lens active: {} community clusters from {} entities.",
            atlas_node_meta.len(),
            node_cluster.len()
        );
    } else {
        for edge in selected_edges {
            graph_links.push(serde_json::json!({
                "source": edge.source,
                "target": edge.target,
                "label": edge.predicate,
                "weight": edge.weight,
                "score": (edge.score * 1000.0).round() / 1000.0,
                "confidence": (edge.avg_confidence * 1000.0).round() / 1000.0,
                "specificity": (edge.specificity * 1000.0).round() / 1000.0,
                "confidence_tier": edge.confidence_tier,
                "provenance": edge.provenance,
            }));
        }
    }

    graph_links.sort_by(|a, b| {
        let ascore = a.get("score").and_then(|s| s.as_f64()).unwrap_or(0.0);
        let bscore = b.get("score").and_then(|s| s.as_f64()).unwrap_or(0.0);
        bscore.total_cmp(&ascore).then_with(|| {
            let aw = a.get("weight").and_then(|w| w.as_u64()).unwrap_or(0);
            let bw = b.get("weight").and_then(|w| w.as_u64()).unwrap_or(0);
            bw.cmp(&aw)
        })
    });
    graph_links.truncate(max_graph_links);
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
    if lens_mode == "atlas" {
        for cluster_id in atlas_node_meta.keys() {
            final_node_degree.entry(cluster_id.clone()).or_insert(0);
        }
    }

    let mut node_pairs: Vec<(String, usize)> = final_node_degree
        .iter()
        .map(|(name, deg)| (name.clone(), *deg))
        .collect();
    node_pairs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let rust_3d_layout = compute_rust_3d_layout(&node_pairs, &graph_links, &gene);

    let graph_nodes: Vec<_> = node_pairs
        .iter()
        .map(|(name, deg)| {
            let (display_name, short, group, size) =
                if let Some((label, atlas_group, atlas_size, atlas_short)) =
                    atlas_node_meta.get(name)
                {
                    (
                        label.clone(),
                        atlas_short.clone(),
                        *atlas_group,
                        *atlas_size,
                    )
                } else {
                    let group = if name.eq_ignore_ascii_case(&gene) {
                        1
                    } else {
                        2
                    };
                    let size = 3.2 + (*deg as f64).ln_1p() * 1.9;
                    (name.clone(), truncate(name, 42), group, size)
                };
            let (x3d, y3d, z3d) = rust_3d_layout.get(name).copied().unwrap_or((0.0, 0.0, 0.0));
            let (x2d, y2d) = project_layout_to_2d(name, x3d, y3d, z3d, group == 1);

            serde_json::json!({
                "id": name,
                "name": display_name,
                "short": short,
                "group": group,
                "degree": deg,
                "size": size,
                "x": x2d,
                "y": y2d,
                "x3d": x3d,
                "y3d": y3d,
                "z3d": z3d,
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
    let raw_matched_paper_count = paper_entries.len();

    let selected_paper_ids: Vec<uuid::Uuid> = paper_entries
        .iter()
        .filter_map(|(paper, _)| {
            paper
                .strip_prefix("paper-")
                .and_then(|id| uuid::Uuid::parse_str(id).ok())
        })
        .collect();

    let paper_refs = paper_repo
        .find_references_by_ids(&selected_paper_ids)
        .await
        .unwrap_or_default();

    #[derive(Default)]
    struct GroupedPaper {
        title: String,
        url: Option<String>,
        duplicate_count: usize,
        rows: Vec<(String, String, String, Option<String>)>,
    }

    let mut title_grouped: HashMap<String, GroupedPaper> = HashMap::new();
    for (paper, rows) in paper_entries.into_iter() {
        let paper_id = paper
            .strip_prefix("paper-")
            .and_then(|id| uuid::Uuid::parse_str(id).ok())
            .unwrap_or(uuid::Uuid::nil());
        let paper_ref = if paper_id.is_nil() {
            None
        } else {
            paper_refs.get(&paper_id)
        };
        let title = derive_paper_title(&paper, paper_ref);
        let url = paper_ref.and_then(build_paper_external_url);
        let bucket = canonical_text_bucket(&title);
        let entry = title_grouped.entry(bucket).or_insert_with(|| GroupedPaper {
            title: title.clone(),
            url: None,
            duplicate_count: 0usize,
            rows: Vec::new(),
        });
        if entry.title.is_empty() {
            entry.title = title;
        }
        if entry.url.is_none() {
            entry.url = url;
        }
        entry.duplicate_count += 1;
        entry.rows.extend(rows);
    }
    let mut grouped_entries: Vec<(
        String,
        Option<String>,
        usize,
        Vec<(String, String, String, Option<String>)>,
    )> = title_grouped
        .into_values()
        .map(|group| (group.title, group.url, group.duplicate_count, group.rows))
        .collect();
    grouped_entries.sort_by(|a, b| b.3.len().cmp(&a.3.len()));
    let matched_paper_count = grouped_entries.len();

    let paper_html = if grouped_entries.is_empty() {
        format!(
            r#"<div class="card-body text-muted">No KG evidence found for <strong class="text-main">{}</strong>.</div>"#,
            gene
        )
    } else {
        grouped_entries
            .into_iter()
            .take(max_papers)
            .map(|(title, paper_url, duplicate_count, rows)| {
                let expanded_key = canonical_text_bucket(&title);
                let open_attr = if !expanded_paper.is_empty() && expanded_paper == expanded_key {
                    "open"
                } else {
                    ""
                };
                let title_html = if let Some(url) = paper_url {
                    format!(
                        r#"<a class="paper-title-link" href="{}" target="_blank" rel="noopener noreferrer">{}</a>"#,
                        html_escape_attr(&url),
                        html_escape(&title)
                    )
                } else {
                    format!(r#"<span class="paper-title">{}</span>"#, html_escape(&title))
                };

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
                                html_escape(subject),
                                html_escape(predicate),
                                html_escape(object)
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
                                {}
                                <span class="info-tip">i
                                    <span class="tooltip-card">{}</span>
                                </span>
                                {}
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
                    title_html,
                    chunk_tooltip,
                    if duplicate_count > 1 {
                        format!(
                            r#"<span class="badge badge-outline">{} papers merged</span>"#,
                            duplicate_count
                        )
                    } else {
                        String::new()
                    },
                    rows.len(),
                    rows_html
                )
            })
            .collect()
    };

    let predicate_options_html = render_predicate_options_html(&pred_lc);
    let confidence_options_html = render_confidence_filter_options_html(&confidence_filter_lc);
    let view_2d_selected = if use_3d { "" } else { "selected" };
    let view_3d_selected = if use_3d { "selected" } else { "" };
    let lens_analysis_selected = if lens_mode == "analysis" {
        "selected"
    } else {
        ""
    };
    let lens_atlas_selected = if lens_mode == "atlas" { "selected" } else { "" };
    let lens_path_selected = if lens_mode == "path" { "selected" } else { "" };
    let preset_analytical_selected = if render_preset == "analytical" {
        "selected"
    } else {
        ""
    };
    let preset_dense_selected = if render_preset == "dense" {
        "selected"
    } else {
        ""
    };
    let hops_value = local_hops;
    let lens_badge = match lens_mode.as_str() {
        "atlas" => "Atlas",
        "path" => "Path",
        _ => "Analysis",
    };
    let preset_badge = if render_preset == "dense" {
        "Dense"
    } else {
        "Analytical"
    };
    let confidence_badge = match confidence_filter_lc.as_str() {
        "high" => "High",
        "medium" => "Medium",
        "low" => "Low",
        "provider" => "Provider",
        "extracted" => "Extracted",
        "generic" => "Generic",
        _ => "All",
    };
    let lens_status_html = if lens_status.trim().is_empty() {
        String::new()
    } else {
        format!(
            r#"<div style="padding:8px 12px; border-top:1px solid var(--border-color); color:var(--text-muted); font-size:0.88rem;">{}</div>"#,
            html_escape(&lens_status)
        )
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
            grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
            gap: 12px;
            align-items: end;
        }}

        .kg-controls-form {{
            display: flex;
            flex-direction: column;
            gap: 0.9rem;
            padding: 1rem;
        }}

        .kg-advanced {{
            border: 1px solid var(--border-glass);
            border-radius: 12px;
            overflow: hidden;
            background: rgba(14, 21, 33, 0.54);
        }}

        .kg-advanced summary {{
            cursor: pointer;
            list-style: none;
            padding: 0.7rem 0.9rem;
            font-weight: 600;
            font-family: 'Outfit', sans-serif;
            color: var(--text-main);
        }}

        .kg-advanced summary::-webkit-details-marker {{
            display: none;
        }}

        .kg-advanced[open] summary {{
            border-bottom: 1px solid var(--border-glass);
        }}

        .kg-advanced-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
            gap: 12px;
            padding: 0.8rem 0.9rem 0.95rem;
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

        .paper-title-link {{
            color: var(--text-main);
            font-weight: 600;
            font-size: 0.92rem;
            line-height: 1.25;
            max-width: 840px;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
            text-decoration: none;
            border-bottom: 1px dashed rgba(122, 193, 255, 0.35);
        }}

        .paper-title-link:hover {{
            color: var(--accent-blue);
            border-bottom-color: rgba(122, 193, 255, 0.75);
        }}

        @media (max-width: 1100px) {{
            .kg-filters {{
                grid-template-columns: 1fr;
            }}
            .kg-advanced-grid {{
                grid-template-columns: 1fr;
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

    <div class="card mb-4">
        <form class="kg-controls-form" method="GET" action="/kg">
            <div class="kg-filters">
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
                        {}
                    </select>
                </div>
                <div>
                    <label class="form-label">Confidence</label>
                    <select name="confidence_tier" class="form-control">
                        {}
                    </select>
                </div>
                <div>
                    <label class="form-label">Graph Mode</label>
                    <select name="view" class="form-control">
                        <option value="2d" {}>2D</option>
                        <option value="3d" {}>3D (Rust layout)</option>
                    </select>
                </div>
                <div>
                    <label class="form-label">Papers</label>
                    <input type="number" name="max_papers" class="form-control" min="10" max="200" value="{}">
                </div>
                <button type="submit" class="btn btn-primary">Apply</button>
            </div>
            <details class="kg-advanced">
                <summary>Advanced Graph Controls</summary>
                <div class="kg-advanced-grid">
                    <div>
                        <label class="form-label">Lens</label>
                        <select name="lens" class="form-control">
                            <option value="analysis" {}>Analysis</option>
                            <option value="atlas" {}>Atlas</option>
                            <option value="path" {}>Path</option>
                        </select>
                    </div>
                    <div>
                        <label class="form-label">Render Preset</label>
                        <select name="preset" class="form-control">
                            <option value="analytical" {}>Analytical</option>
                            <option value="dense" {}>Dense</option>
                        </select>
                    </div>
                    <div>
                        <label class="form-label">Path Source</label>
                        <input type="text" id="path-source-input" name="source" class="form-control" list="entity-options" autocomplete="off" placeholder="e.g. KRAS" value="{}">
                    </div>
                    <div>
                        <label class="form-label">Path Target</label>
                        <input type="text" id="path-target-input" name="target" class="form-control" list="entity-options" autocomplete="off" placeholder="e.g. Pancreatic Cancer" value="{}">
                    </div>
                    <div>
                        <label class="form-label">Local Hops</label>
                        <input type="number" name="hops" class="form-control" min="1" max="3" value="{}">
                    </div>
                </div>
            </details>
        </form>
    </div>

    <div class="card mb-4" style="padding:0; overflow:hidden;">
        <div class="card-header">
            <div>Entity Graph around <span class="text-gradient">{}</span></div>
            <div class="d-flex gap-2">
                <span class="badge badge-outline">{} papers in DB</span>
                <span class="badge badge-outline">{} facts scanned</span>
                <span class="badge badge-outline">{} papers matched</span>
                <span class="badge badge-outline text-muted">{} raw paper ids</span>
                <span class="badge badge-outline">lens: {}</span>
                <span class="badge badge-outline">preset: {}</span>
                <span class="badge badge-outline">confidence: {}</span>
                <span class="badge badge-outline">{} nodes</span>
                <span class="badge badge-outline">{} links</span>
            </div>
        </div>
        {}
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
    const graphViewMode = "{}";
    const graphRenderPreset = "{}";
    const focusInput = document.getElementById('entity-focus-input');
    const entityOptions = document.getElementById('entity-options');
    const suggestCache = new Map();
    let suggestTimer = null;
    let inFlight = null;

    function stableHash32(value) {{
        const s = String(value || '');
        let h = 2166136261 >>> 0;
        for (let i = 0; i < s.length; i++) {{
            h ^= s.charCodeAt(i);
            h = Math.imul(h, 16777619);
        }}
        return h >>> 0;
    }}

    function deterministicOffset(id, axis, span) {{
        const hash = stableHash32(`${{String(id || 'node')}}|${{axis}}`);
        const unit = (hash / 4294967295) - 0.5;
        return unit * span;
    }}

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
        if (graphViewMode === '3d') {{
            // Compatibility shim for UMD builds that expect Node-style globals.
            if (typeof window.global === 'undefined') window.global = window;
            if (typeof window.process === 'undefined') window.process = {{ env: {{}} }};
        }}
        const script = document.createElement('script');
        script.src = graphViewMode === '3d'
            ? 'https://unpkg.com/3d-force-graph'
            : 'https://unpkg.com/force-graph';
        script.onload = () => startGraph();
        document.body.appendChild(script);
    }}

    function startGraph() {{
        if (graphViewMode === '3d') {{
            startGraph3D();
            return;
        }}
        startGraph2D();
    }}

    function startGraph2D() {{
        if (!graphData.nodes || graphData.nodes.length === 0) {{
            elem.innerHTML = '<div style="display:flex; height:100%; align-items:center; justify-content:center; color: var(--text-muted);">No graph edges matched your filters.</div>';
            return;
        }}
        const densePreset = graphRenderPreset === 'dense';

        graphData.nodes.forEach((node) => {{
            const seedX = Number.isFinite(node.x) ? node.x : deterministicOffset(node.id, 'x2d', 220);
            const seedY = Number.isFinite(node.y) ? node.y : deterministicOffset(node.id, 'y2d', 220);
            node.x = seedX;
            node.y = seedY;
            if (node.group === 1) {{
                // Anchor the focus node so the simulation does not drift off-screen.
                node.x = 0;
                node.y = 0;
                node.fx = 0;
                node.fy = 0;
            }} else {{
                node.fx = undefined;
                node.fy = undefined;
            }}
        }});
        const seededNodes = graphData.nodes.map((n) => ({{
            id: n.id,
            x: Number.isFinite(n.x) ? n.x : deterministicOffset(n.id, 'xseed', 220),
            y: Number.isFinite(n.y) ? n.y : deterministicOffset(n.id, 'yseed', 220),
        }}));

        function softPositionBiasForce2D(seedNodes, strength) {{
            let activeNodes = [];
            const seeds = new Map((seedNodes || []).map((n) => [n.id, {{ x: n.x, y: n.y }}]));
            function force(alpha) {{
                const k = alpha * strength;
                for (const n of activeNodes) {{
                    const s = seeds.get(n.id);
                    if (!s || n.fx !== undefined || n.fy !== undefined) continue;
                    n.vx += (s.x - n.x) * k;
                    n.vy += (s.y - n.y) * k;
                }}
            }}
            force.initialize = (nodesForForce) => {{
                activeNodes = Array.isArray(nodesForForce) ? nodesForForce : [];
            }};
            return force;
        }}

        let recenterInterval = null;

        const Graph = ForceGraph()(elem)
            .graphData(graphData)
            .backgroundColor('transparent')
            .nodeLabel(node => `${{node.name}} (${{node.degree}} links)`)
            .nodeVal(node => densePreset ? Math.max(1.2, (node.size || 3) * 0.55) : (node.size || 3))
            .linkColor(link => {{
                const l = (link.label || '').toLowerCase();
                if (l.includes('inhibit') || l.includes('suppress')) return 'rgba(255,125,144,0.62)';
                if (l.includes('activate') || l.includes('promote') || l.includes('drive')) return 'rgba(74,220,169,0.58)';
                if (l.includes('resistance') || l.includes('prognostic')) return 'rgba(255,196,106,0.62)';
                if (l.includes('biomarker') || l.includes('mutation')) return 'rgba(153,168,255,0.58)';
                if (l.includes('synthetic_lethal') || l.includes('required_for_viability')) return 'rgba(108,219,255,0.62)';
                return 'rgba(109,168,255,0.42)';
            }})
            .linkWidth(link => {{
                const score = Math.max(0, Number(link.score) || 0);
                const base = densePreset ? 0.20 : 0.45;
                const scoreGain = densePreset ? 0.75 : 1.8;
                const scale = densePreset ? 0.52 : 0.78;
                return Math.min(densePreset ? 1.45 : 2.8, base + Math.log2((link.weight || 1) + 1) * scale + score * scoreGain);
            }})
            .cooldownTime(densePreset ? 1700 : 1300)
            .d3AlphaDecay(densePreset ? 0.14 : 0.12)
            .d3VelocityDecay(densePreset ? 0.54 : 0.5)
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

                const showLabel = !densePreset && globalScale > 2.4 && (node.group === 1 || node.degree >= 14);
                if (showLabel) {{
                    ctx.font = `${{Math.max(9, 11 / globalScale)}}px Inter`;
                    ctx.fillStyle = 'rgba(235, 242, 255, 0.72)';
                    ctx.textAlign = 'center';
                    ctx.fillText(node.short || node.name, node.x, node.y + r + 8);
                }}
            }})
            .linkCanvasObjectMode(() => 'after')
            .linkCanvasObject((link, ctx, globalScale) => {{
                if (globalScale < 3.1) return;
                const start = link.source;
                const end = link.target;
                if (!start || !end || typeof start !== 'object' || typeof end !== 'object') return;
                const label = String(link.label || '').replaceAll('_', ' ');
                if (!label) return;
                const text = label.length > 26 ? `${{label.slice(0, 25)}}...` : label;
                const x = (start.x + end.x) / 2;
                const y = (start.y + end.y) / 2;
                ctx.save();
                ctx.font = `${{Math.max(8, 10 / globalScale)}}px Inter`;
                ctx.fillStyle = 'rgba(235, 242, 255, 0.62)';
                ctx.textAlign = 'center';
                ctx.fillText(text, x, y);
                ctx.restore();
            }})
            .onEngineStop(() => {{
                if (recenterInterval) {{
                    clearInterval(recenterInterval);
                    recenterInterval = null;
                }}
                recenterGraph();
                Graph.pauseAnimation();
            }});
        Graph.d3Force(
            'position-bias',
            softPositionBiasForce2D(seededNodes, densePreset ? 0.012 : 0.018),
        );

        function recenterGraph() {{
            const nodes = ((Graph.graphData() && Graph.graphData().nodes) || [])
                .filter(n => Number.isFinite(n.x) && Number.isFinite(n.y));
            if (!nodes.length) return;

            const xs = nodes.map(n => n.x).sort((a, b) => a - b);
            const ys = nodes.map(n => n.y).sort((a, b) => a - b);
            const q = (arr, p) => {{
                if (!arr.length) return 0;
                const idx = Math.max(0, Math.min(arr.length - 1, Math.floor((arr.length - 1) * p)));
                return arr[idx];
            }};

            // Center on mass centroid to keep the dense cluster in view.
            const cx = nodes.reduce((acc, n) => acc + n.x, 0) / nodes.length;
            const cy = nodes.reduce((acc, n) => acc + n.y, 0) / nodes.length;

            // Use inter-quantile spread for zoom so sparse outliers do not over-zoom out.
            const minX = q(xs, 0.15);
            const maxX = q(xs, 0.85);
            const minY = q(ys, 0.15);
            const maxY = q(ys, 0.85);

            const spanX = Math.max(80, maxX - minX);
            const spanY = Math.max(80, maxY - minY);
            const span = Math.max(spanX, spanY);
            const viewport = Math.max(320, Math.min(elem.clientWidth || 320, elem.clientHeight || 320));
            const targetZoom = Math.max(0.78, Math.min(3.4, viewport / (span * 1.75)));
            const cameraX = cx + spanX * 0.3;
            const cameraY = cy + spanY * 0.34;

            Graph.centerAt(cameraX, cameraY, 220);
            Graph.zoom(targetZoom, 320);
        }}

        let recenterPasses = 0;
        recenterInterval = setInterval(() => {{
            recenterGraph();
            recenterPasses += 1;
            if (recenterPasses >= 12) {{
                clearInterval(recenterInterval);
                recenterInterval = null;
            }}
        }}, 360);

        setTimeout(recenterGraph, 220);

        window.addEventListener('resize', () => {{
            if (elem.clientWidth > 0 && elem.clientHeight > 0) {{
                Graph.width(elem.clientWidth).height(elem.clientHeight);
                setTimeout(recenterGraph, 120);
            }}
        }});
    }}

    function startGraph3D() {{
        if (!graphData.nodes || graphData.nodes.length === 0) {{
            elem.innerHTML = '<div style="display:flex; height:100%; align-items:center; justify-content:center; color: var(--text-muted);">No graph edges matched your filters.</div>';
            return;
        }}
        const densePreset = graphRenderPreset === 'dense';

        const nodes = (graphData.nodes || []).map((n) => {{
            const fallbackX = deterministicOffset(n.id, 'x3d', 120);
            const fallbackY = deterministicOffset(n.id, 'y3d', 120);
            const fallbackZ = deterministicOffset(n.id, 'z3d', 120);
            return {{
                ...n,
                x: Number.isFinite(n.x3d) ? n.x3d : fallbackX,
                y: Number.isFinite(n.y3d) ? n.y3d : fallbackY,
                z: Number.isFinite(n.z3d) ? n.z3d : fallbackZ,
                vx: 0,
                vy: 0,
                vz: 0,
            }};
        }});

        const links = (graphData.links || []).map((l) => ({{ ...l }}));
        const nodeCount = nodes.length || 1;
        const linkCount = links.length || 1;
        const density = linkCount / nodeCount;
        const enablePointers = !densePreset && nodeCount <= 1200 && linkCount <= 3800;
        const nodeResolution = densePreset ? (nodeCount > 850 ? 4 : 5) : (nodeCount > 850 ? 5 : 8);
        const linkResolution = densePreset ? (linkCount > 2200 ? 2 : 3) : (linkCount > 2200 ? 4 : 6);
        const warmupTicks = Math.min(
            densePreset ? 160 : 220,
            Math.max(24, Math.round((densePreset ? 26 : 44) + Math.sqrt(nodeCount) * (densePreset ? 5 : 8) + density * 6))
        );
        const cooldownTicks = Math.min(
            densePreset ? 220 : 280,
            Math.max(80, Math.round((densePreset ? 90 : 120) + Math.sqrt(nodeCount) * (densePreset ? 5 : 7)))
        );
        const cooldownTime = Math.min(
            densePreset ? 7200 : 9500,
            Math.max(2200, (densePreset ? 1700 : 2200) + nodeCount * (densePreset ? 6 : 8) + linkCount * (densePreset ? 2 : 3))
        );
        const focusNodeId = (nodes.find((n) => n.group === 1) || {{}}).id || null;

        function relColorHex(label) {{
            const l = String(label || '').toLowerCase();
            if (l.includes('inhibit') || l.includes('suppress')) return '#ff7d90';
            if (l.includes('activate') || l.includes('promote') || l.includes('drive')) return '#4adca9';
            if (l.includes('resistance') || l.includes('prognostic')) return '#ffc46a';
            if (l.includes('biomarker') || l.includes('mutation')) return '#99a8ff';
            if (l.includes('synthetic_lethal') || l.includes('required_for_viability')) return '#6cdbff';
            return '#6da8ff';
        }}
        function relOpacity(link) {{
            const l = String((link && link.label) || '').toLowerCase();
            const score = Math.max(0, Number(link && link.score) || 0);
            const scoreBoost = Math.min(0.20, score * 0.24);
            const denseScale = densePreset ? 0.55 : 1.0;
            if (l.includes('inhibit') || l.includes('suppress')) return (0.78 + scoreBoost) * denseScale;
            if (l.includes('activate') || l.includes('promote') || l.includes('drive')) return (0.72 + scoreBoost) * denseScale;
            if (l.includes('resistance') || l.includes('prognostic')) return (0.76 + scoreBoost) * denseScale;
            if (l.includes('biomarker') || l.includes('mutation')) return (0.70 + scoreBoost) * denseScale;
            if (l.includes('synthetic_lethal') || l.includes('required_for_viability')) return (0.78 + scoreBoost) * denseScale;
            return (0.52 + scoreBoost) * denseScale;
        }}
        function relDistance(link) {{
            const l = String(link.label || '').toLowerCase();
            const weight = Math.max(1, Number(link.weight) || 1);
            let base = 114;
            if (l.includes('synthetic_lethal') || l.includes('required_for_viability')) base = 78;
            else if (l.includes('inhibit') || l.includes('suppress')) base = 84;
            else if (l.includes('activate') || l.includes('promote') || l.includes('drive')) base = 92;
            else if (l.includes('biomarker') || l.includes('mutation')) base = 98;
            else if (l.includes('mentions') || l.includes('associated_with')) base = 122;
            return Math.max(42, Math.min(170, base - Math.log2(weight + 1) * 11));
        }}
        function relStrength(link) {{
            const l = String(link.label || '').toLowerCase();
            const weight = Math.max(1, Number(link.weight) || 1);
            let base = 0.08;
            if (l.includes('synthetic_lethal') || l.includes('required_for_viability')) base = 0.18;
            else if (l.includes('inhibit') || l.includes('activate') || l.includes('drive')) base = 0.13;
            else if (l.includes('mentions') || l.includes('associated_with')) base = 0.06;
            return Math.max(0.03, Math.min(0.34, base + Math.log2(weight + 1) * 0.028));
        }}
        function softPositionBiasForce(seedNodes, strength) {{
            let activeNodes = [];
            const seeds = new Map((seedNodes || []).map((n) => [n.id, {{ x: n.x, y: n.y, z: n.z }}]));
            function force(alpha) {{
                const k = alpha * strength;
                for (const n of activeNodes) {{
                    const s = seeds.get(n.id);
                    if (!s) continue;
                    n.vx += (s.x - n.x) * k;
                    n.vy += (s.y - n.y) * k;
                    n.vz += (s.z - n.z) * k;
                }}
            }}
            force.initialize = (nodesForForce) => {{
                activeNodes = Array.isArray(nodesForForce) ? nodesForForce : [];
            }};
            return force;
        }}
        function focusSoftAnchorForce(targetId, strength) {{
            let activeNodes = [];
            function force(alpha) {{
                const k = alpha * strength;
                for (const n of activeNodes) {{
                    if (n.id !== targetId) continue;
                    n.vx += (0 - n.x) * k;
                    n.vy += (0 - n.y) * k;
                    n.vz += (0 - n.z) * k;
                }}
            }}
            force.initialize = (nodesForForce) => {{
                activeNodes = Array.isArray(nodesForForce) ? nodesForForce : [];
            }};
            return force;
        }}

        const Graph3D = ForceGraph3D()(elem)
            .graphData({{ nodes, links }})
            .backgroundColor('#060c16')
            .showNavInfo(false)
            .enableNodeDrag(false)
            .enablePointerInteraction(enablePointers)
            .nodeResolution(nodeResolution)
            .linkResolution(linkResolution)
            .forceEngine('d3')
            .numDimensions(3)
            .nodeLabel(node => `${{node.name}} (${{node.degree}} links)`)
            .nodeColor(node => node.group === 1 ? '#c18eff' : (node.degree >= 8 ? '#6ecaff' : '#5f92eb'))
            .nodeVal(node => densePreset ? Math.max(1.0, (node.size || 3) * 0.52) : (node.size || 3))
            .linkColor(link => relColorHex(link.label))
            .linkOpacity(link => Math.min(0.92, relOpacity(link)))
            .linkWidth(link => {{
                const score = Math.max(0, Number(link.score) || 0);
                const base = densePreset ? 0.16 : 0.52;
                const scoreGain = densePreset ? 0.65 : 1.6;
                const scale = densePreset ? 0.45 : 0.72;
                return Math.min(densePreset ? 1.35 : 2.6, base + Math.log2((link.weight || 1) + 1) * scale + score * scoreGain);
            }})
            .d3AlphaDecay(nodeCount > 420 ? 0.10 : 0.078)
            .d3VelocityDecay(nodeCount > 420 ? 0.56 : 0.46)
            .warmupTicks(warmupTicks)
            .cooldownTicks(cooldownTicks)
            .cooldownTime(cooldownTime);
        window.__ferruGraph3D = Graph3D;

        const linkForce = Graph3D.d3Force('link');
        if (linkForce) {{
            linkForce
                .distance((l) => relDistance(l))
                .strength((l) => relStrength(l))
                .iterations(nodeCount > 300 ? 1 : 2);
        }}

        const chargeForce = Graph3D.d3Force('charge');
        if (chargeForce) {{
            chargeForce
                .strength((n) => {{
                    const deg = Math.max(1, Number(n.degree) || 1);
                    const charge = 26 + Math.log2(deg + 1) * 18;
                    return -Math.min(160, charge);
                }})
                .distanceMin(6)
                .distanceMax(nodeCount > 650 ? 600 : 460)
                .theta(0.9);
        }}

        Graph3D.d3Force(
            'position-bias',
            softPositionBiasForce(nodes, nodeCount > 600 ? 0.008 : 0.014),
        );
        if (focusNodeId) {{
            Graph3D.d3Force(
                'focus-anchor',
                focusSoftAnchorForce(focusNodeId, nodeCount > 600 ? 0.03 : 0.05),
            );
        }}

        function positionCamera(durationMs) {{
            const active = (Graph3D.graphData().nodes || []).filter((n) =>
                Number.isFinite(n.x) && Number.isFinite(n.y) && Number.isFinite(n.z)
            );
            if (!active.length) {{
                Graph3D.cameraPosition({{ x: 0, y: 0, z: 460 }}, {{ x: 0, y: 0, z: 0 }}, durationMs);
                return;
            }}
            const center = active.reduce(
                (acc, n) => {{
                    acc.x += n.x;
                    acc.y += n.y;
                    acc.z += n.z;
                    return acc;
                }},
                {{ x: 0, y: 0, z: 0 }},
            );
            center.x /= active.length;
            center.y /= active.length;
            center.z /= active.length;
            const radius = Math.max(
                110,
                ...active.map((n) => {{
                    const dx = n.x - center.x;
                    const dy = n.y - center.y;
                    const dz = n.z - center.z;
                    return Math.sqrt(dx * dx + dy * dy + dz * dz);
                }}),
            );
            const cam = {{
                x: center.x + radius * 0.48,
                y: center.y + radius * 0.34,
                z: center.z + Math.max(260, radius * 2.1),
            }};
            Graph3D.cameraPosition(cam, center, durationMs);
        }}

        positionCamera(900);
        Graph3D.onEngineStop(() => positionCamera(500));

        window.addEventListener('resize', () => {{
            if (elem.clientWidth > 0 && elem.clientHeight > 0) {{
                Graph3D.width(elem.clientWidth).height(elem.clientHeight);
                setTimeout(() => positionCamera(220), 80);
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
        predicate_options_html,
        confidence_options_html,
        view_2d_selected,
        view_3d_selected,
        max_papers,
        lens_analysis_selected,
        lens_atlas_selected,
        lens_path_selected,
        preset_analytical_selected,
        preset_dense_selected,
        html_escape(&path_source),
        html_escape(&path_target),
        hops_value,
        html_escape(&focus_label),
        total_papers,
        scanned_facts,
        matched_paper_count,
        raw_matched_paper_count,
        lens_badge,
        preset_badge,
        confidence_badge,
        final_node_degree.len(),
        graph_link_count,
        lens_status_html,
        max_papers,
        paper_html,
        graph_json,
        view_mode,
        render_preset,
    );
    kg_cache_put(cache_key, html.clone());
    Html(html)
}

fn env_usize(key: &str, default: usize, min: usize, max: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default)
        .clamp(min, max)
}

fn env_f64(key: &str, default: f64, min: f64, max: f64) -> f64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(default)
        .clamp(min, max)
}

fn kg_default_mode() -> String {
    if std::env::var("FERRUMYX_KG_DEFAULT_MODE")
        .ok()
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("3d"))
    {
        "3d".to_string()
    } else {
        "2d".to_string()
    }
}

fn kg_2d_max_nodes() -> usize {
    env_usize("FERRUMYX_KG_2D_MAX_NODES", 260, 80, 3000)
}

fn kg_2d_max_links() -> usize {
    env_usize("FERRUMYX_KG_2D_MAX_LINKS", 1000, 120, 12_000)
}

fn kg_3d_max_nodes() -> usize {
    env_usize("FERRUMYX_KG_3D_MAX_NODES", 200, 60, 2500)
}

fn kg_3d_max_links() -> usize {
    env_usize("FERRUMYX_KG_3D_MAX_LINKS", 650, 100, 10_000)
}

fn normalize_lens_mode(raw: &str) -> String {
    let lens = raw.trim().to_lowercase();
    match lens.as_str() {
        "atlas" => "atlas".to_string(),
        "path" => "path".to_string(),
        _ => "analysis".to_string(),
    }
}

fn kg_default_lens_mode() -> String {
    normalize_lens_mode(
        &std::env::var("FERRUMYX_KG_DEFAULT_LENS").unwrap_or_else(|_| "analysis".to_string()),
    )
}

fn kg_default_render_preset() -> String {
    if std::env::var("FERRUMYX_KG_RENDER_PRESET")
        .ok()
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("dense"))
    {
        "dense".to_string()
    } else {
        "analytical".to_string()
    }
}

fn normalize_render_preset(raw: &str) -> String {
    if raw.trim().eq_ignore_ascii_case("dense") {
        "dense".to_string()
    } else {
        "analytical".to_string()
    }
}

fn kg_atlas_max_clusters() -> usize {
    env_usize("FERRUMYX_KG_ATLAS_MAX_CLUSTERS", 40, 5, 300)
}

fn kg_per_node_edge_cap() -> usize {
    env_usize("FERRUMYX_KG_PER_NODE_EDGE_CAP", 12, 3, 200)
}

fn kg_hub_suppression_strength() -> f64 {
    env_f64("FERRUMYX_KG_HUB_SUPPRESSION", 0.62, 0.0, 3.0)
}

fn predicate_specificity(predicate: &str) -> f64 {
    let p = predicate.trim().to_lowercase();
    match p.as_str() {
        "synthetic_lethal_with" | "required_for_viability" => 1.35,
        "inhibits"
        | "targets"
        | "activates"
        | "promotes_proliferation"
        | "promotes_tumorigenesis"
        | "drives_metastasis"
        | "drives_invasion" => 1.20,
        "confers_resistance"
        | "sensitizes_to"
        | "biomarker_of"
        | "prognostic_for_poor_outcome"
        | "prognostic_for_better_outcome"
        | "mutated_in"
        | "has_mutation"
        | "upregulated_in"
        | "downregulated_in" => 1.10,
        "associated_with" => 0.62,
        "mentions" => 0.42,
        _ => 1.0,
    }
}

fn edge_quality_score(
    evidence_count: usize,
    avg_confidence: f64,
    specificity: f64,
    src_degree: usize,
    dst_degree: usize,
    hub_suppression_strength: f64,
) -> f64 {
    let evidence_term = ((evidence_count as f64).ln_1p() / 3.6).clamp(0.12, 1.0);
    let confidence_term = avg_confidence.clamp(0.08, 1.0);
    let base = (confidence_term * 0.58) + (evidence_term * 0.42);
    let hub_mass = (src_degree + dst_degree).max(1) as f64;
    let hub_penalty = 1.0 / (1.0 + hub_suppression_strength * hub_mass.ln_1p());
    (base * specificity * hub_penalty).clamp(0.0, 2.5)
}

fn compute_rust_3d_layout(
    node_pairs: &[(String, usize)],
    graph_links: &[serde_json::Value],
    focus_gene: &str,
) -> HashMap<String, (f64, f64, f64)> {
    let mut coords = HashMap::with_capacity(node_pairs.len());
    if node_pairs.is_empty() {
        return coords;
    }

    let focus_gene_lc = focus_gene.trim().to_lowercase();
    let degree_map: HashMap<String, usize> = node_pairs
        .iter()
        .map(|(name, deg)| (name.clone(), *deg))
        .collect();

    let mut adjacency: HashMap<String, Vec<String>> = node_pairs
        .iter()
        .map(|(name, _)| (name.clone(), Vec::new()))
        .collect();

    for link in graph_links {
        let source = link.get("source").and_then(|v| v.as_str());
        let target = link.get("target").and_then(|v| v.as_str());
        let (Some(src), Some(dst)) = (source, target) else {
            continue;
        };
        if src == dst {
            continue;
        }
        if adjacency.contains_key(src) && adjacency.contains_key(dst) {
            if let Some(neighbors) = adjacency.get_mut(src) {
                neighbors.push(dst.to_string());
            }
            if let Some(neighbors) = adjacency.get_mut(dst) {
                neighbors.push(src.to_string());
            }
        }
    }

    // Topology-aware component layout: separate disconnected clusters first,
    // then distribute nodes within each cluster by local degree.
    let mut visited: HashSet<String> = HashSet::new();
    let mut components: Vec<Vec<String>> = Vec::new();
    for (name, _) in node_pairs {
        if visited.contains(name) {
            continue;
        }
        let mut queue = VecDeque::new();
        let mut component = Vec::new();
        queue.push_back(name.clone());
        visited.insert(name.clone());
        while let Some(curr) = queue.pop_front() {
            component.push(curr.clone());
            if let Some(neighbors) = adjacency.get(&curr) {
                for next in neighbors {
                    if visited.insert(next.clone()) {
                        queue.push_back(next.clone());
                    }
                }
            }
        }
        components.push(component);
    }

    components.sort_by(|a, b| {
        b.len().cmp(&a.len()).then_with(|| {
            let a_deg = a
                .iter()
                .map(|n| degree_map.get(n).copied().unwrap_or(0))
                .max()
                .unwrap_or(0);
            let b_deg = b
                .iter()
                .map(|n| degree_map.get(n).copied().unwrap_or(0))
                .max()
                .unwrap_or(0);
            b_deg.cmp(&a_deg)
        })
    });

    let component_count = components.len().max(1);
    let golden_angle = std::f64::consts::PI * (3.0 - 5.0_f64.sqrt());

    for (component_idx, component) in components.iter().enumerate() {
        let i = component_idx as f64;
        let t = (i + 0.5) / (component_count as f64);
        let base_y = 1.0 - 2.0 * t;
        let base_r = (1.0 - base_y * base_y).max(0.0).sqrt();
        let theta = golden_angle * i;

        let is_primary_component = component_idx == 0;
        let component_radius = if is_primary_component {
            0.0
        } else {
            190.0 + (component_idx as f64).sqrt() * 52.0
        };
        let center_x = theta.cos() * base_r * component_radius;
        let center_y = base_y * component_radius * 0.92;
        let center_z = theta.sin() * base_r * component_radius;

        let mut ordered_nodes: Vec<(String, usize)> = component
            .iter()
            .map(|name| (name.clone(), degree_map.get(name).copied().unwrap_or(0)))
            .collect();
        ordered_nodes.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let local_total = ordered_nodes.len().max(1);
        for (local_idx, (name, deg)) in ordered_nodes.iter().enumerate() {
            if !focus_gene_lc.is_empty() && name.eq_ignore_ascii_case(&focus_gene_lc) {
                coords.insert(name.clone(), (0.0, 0.0, 0.0));
                continue;
            }

            let li = local_idx as f64;
            let lt = (li + 0.5) / (local_total as f64);
            let local_y = 1.0 - 2.0 * lt;
            let local_r = (1.0 - local_y * local_y).max(0.0).sqrt();
            let local_theta = golden_angle * li;

            let local_base = 34.0 + (local_total as f64).ln_1p() * 20.0;
            let spread = local_base + (*deg as f64).ln_1p() * 14.0;
            let x = center_x + local_theta.cos() * local_r * spread;
            let y = center_y + local_y * spread;
            let z = center_z + local_theta.sin() * local_r * spread;
            coords.insert(name.clone(), (x, y, z));
        }
    }

    coords
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

fn canonical_text_bucket(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_space = false;
    for ch in input.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            ' '
        };
        if mapped == ' ' {
            if prev_space {
                continue;
            }
            prev_space = true;
            out.push(' ');
        } else {
            prev_space = false;
            out.push(mapped);
        }
    }
    out.trim().to_string()
}

fn is_virtual_predicate_filter(filter: &str) -> bool {
    matches!(filter, "" | "all" | "specific" | "mechanistic" | "clinical")
}

fn predicate_matches_filter(predicate_lc: &str, filter_lc: &str) -> bool {
    match filter_lc {
        "" | "all" => true,
        "specific" => predicate_lc != "mentions" && predicate_lc != "associated_with",
        "mechanistic" => matches!(
            predicate_lc,
            "inhibits"
                | "targets"
                | "activates"
                | "promotes_proliferation"
                | "promotes_tumorigenesis"
                | "drives_metastasis"
                | "drives_invasion"
                | "synthetic_lethal_with"
                | "required_for_viability"
        ),
        "clinical" => matches!(
            predicate_lc,
            "confers_resistance"
                | "sensitizes_to"
                | "biomarker_of"
                | "prognostic_for_poor_outcome"
                | "prognostic_for_better_outcome"
                | "mutated_in"
                | "has_mutation"
                | "upregulated_in"
                | "downregulated_in"
        ),
        _ => predicate_lc == filter_lc,
    }
}

fn render_predicate_options_html(selected_lc: &str) -> String {
    let options = [
        ("specific", "Specific (no generic mentions)"),
        ("all", "All predicates"),
        ("mechanistic", "Mechanistic relations"),
        ("clinical", "Clinical/biomarker relations"),
        ("inhibits", "inhibits"),
        ("targets", "targets"),
        ("activates", "activates"),
        ("promotes_proliferation", "promotes_proliferation"),
        ("promotes_tumorigenesis", "promotes_tumorigenesis"),
        ("drives_metastasis", "drives_metastasis"),
        ("drives_invasion", "drives_invasion"),
        ("confers_resistance", "confers_resistance"),
        ("sensitizes_to", "sensitizes_to"),
        ("biomarker_of", "biomarker_of"),
        ("prognostic_for_poor_outcome", "prognostic_for_poor_outcome"),
        (
            "prognostic_for_better_outcome",
            "prognostic_for_better_outcome",
        ),
        ("synthetic_lethal_with", "synthetic_lethal_with"),
        ("required_for_viability", "required_for_viability"),
        ("mutated_in", "mutated_in"),
        ("has_mutation", "has_mutation"),
        ("upregulated_in", "upregulated_in"),
        ("downregulated_in", "downregulated_in"),
        ("associated_with", "associated_with"),
        ("mentions", "mentions"),
    ];
    options
        .iter()
        .map(|(value, label)| {
            format!(
                r#"<option value="{}" {}>{}</option>"#,
                value,
                if selected_lc == *value {
                    "selected"
                } else {
                    ""
                },
                label
            )
        })
        .collect()
}

fn normalize_confidence_filter(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "high" => "high".to_string(),
        "medium" => "medium".to_string(),
        "low" => "low".to_string(),
        "provider" => "provider".to_string(),
        "extracted" => "extracted".to_string(),
        "generic" => "generic".to_string(),
        _ => "all".to_string(),
    }
}

fn classify_fact_provenance(f: &ferrumyx_db::schema::KgFact) -> &'static str {
    let evidence_type = f.evidence_type.trim().to_ascii_lowercase();
    let predicate = f.predicate.trim().to_ascii_lowercase();
    let evidence = f.evidence.as_deref().unwrap_or("").to_ascii_lowercase();

    let provider_predicate = predicate.ends_with("_cbioportal")
        || predicate.ends_with("_cosmic")
        || predicate.ends_with("_tcga")
        || predicate.ends_with("_gtex")
        || predicate.ends_with("_chembl")
        || predicate.ends_with("_reactome");
    let provider_evidence = evidence.contains("provider=");
    if evidence_type.contains("provider") || provider_predicate || provider_evidence {
        "provider"
    } else if evidence_type.contains("mention")
        || evidence_type.contains("generic")
        || predicate == "mentions"
        || predicate == "associated_with"
    {
        "generic"
    } else {
        "extracted"
    }
}

fn classify_confidence_tier(f: &ferrumyx_db::schema::KgFact) -> &'static str {
    let provenance = classify_fact_provenance(f);
    let confidence = (f.confidence as f64).clamp(0.0, 1.0);
    match provenance {
        "provider" => {
            if confidence >= 0.62 {
                "high"
            } else {
                "medium"
            }
        }
        "generic" => {
            if confidence >= 0.80 && !f.predicate.eq_ignore_ascii_case("mentions") {
                "medium"
            } else {
                "low"
            }
        }
        _ => {
            if confidence >= 0.78 {
                "high"
            } else if confidence >= 0.52 {
                "medium"
            } else {
                "low"
            }
        }
    }
}

fn confidence_filter_matches(tier: &str, provenance: &str, filter: &str) -> bool {
    match filter {
        "" | "all" => true,
        "high" => tier == "high",
        "medium" => tier == "medium",
        "low" => tier == "low",
        "provider" => provenance == "provider",
        "extracted" => provenance == "extracted",
        "generic" => provenance == "generic",
        _ => true,
    }
}

fn render_confidence_filter_options_html(selected_lc: &str) -> String {
    let options = [
        ("all", "All confidence"),
        ("high", "High confidence"),
        ("medium", "Medium confidence"),
        ("low", "Low confidence"),
        ("provider", "Provider-backed"),
        ("extracted", "Extracted relations"),
        ("generic", "Generic mentions"),
    ];
    options
        .iter()
        .map(|(value, label)| {
            format!(
                r#"<option value="{}" {}>{}</option>"#,
                value,
                if selected_lc == *value {
                    "selected"
                } else {
                    ""
                },
                label
            )
        })
        .collect()
}

fn dominant_label<'a>(counts: &[(&'a str, usize)], fallback: &'a str) -> &'a str {
    counts
        .iter()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(b.0)))
        .and_then(|(label, count)| if *count > 0 { Some(*label) } else { None })
        .unwrap_or(fallback)
}

fn stable_unit_hash(seed: &str, salt: u64) -> f64 {
    let mut h = 1469598103934665603u64 ^ salt;
    for b in seed.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    (h as f64 / u64::MAX as f64) - 0.5
}

fn project_layout_to_2d(name: &str, x: f64, y: f64, z: f64, is_focus: bool) -> (f64, f64) {
    if is_focus {
        return (0.0, 0.0);
    }
    let depth = 1.0 / (1.0 + y.abs() * 0.0026);
    let mut px = (x + z * 0.34) * depth;
    let mut py = (y * 0.86 - z * 0.18) * depth;

    if !px.is_finite() || !py.is_finite() || (px.abs() < 0.001 && py.abs() < 0.001) {
        px = stable_unit_hash(name, 0x41_32_AA_10) * 220.0;
        py = stable_unit_hash(name, 0x7F_55_03_CD) * 220.0;
    }
    (px, py)
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn html_escape_attr(input: &str) -> String {
    html_escape(input)
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn derive_paper_title(paper_key: &str, paper_ref: Option<&PaperReference>) -> String {
    if let Some(reference) = paper_ref {
        if let Some(title) = normalize_paper_title(&reference.title) {
            return title;
        }
        if let Some(doi) = reference
            .doi
            .as_deref()
            .and_then(normalize_doi_for_link)
            .or_else(|| {
                reference
                    .published_version_doi
                    .as_deref()
                    .and_then(normalize_doi_for_link)
            })
        {
            return format!("DOI {}", doi);
        }
        if let Some(pmid) = reference.pmid.as_deref().and_then(normalize_id_label) {
            return format!("PMID {}", pmid);
        }
        if let Some(source_id) = reference.source_id.as_deref().and_then(normalize_id_label) {
            let source = reference
                .source
                .as_deref()
                .map(|v| v.trim().to_uppercase())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "PAPER".to_string());
            return format!("{} {}", source, source_id);
        }
    }
    if let Some(id) = paper_key.strip_prefix("paper-") {
        let short = id.chars().take(8).collect::<String>();
        if !short.is_empty() {
            return format!("Paper {}", short);
        }
    }
    normalize_paper_title(paper_key).unwrap_or_else(|| truncate(paper_key, 56))
}

fn normalize_paper_title(raw: &str) -> Option<String> {
    let title = raw.trim();
    if title.is_empty() {
        return None;
    }
    let lower = title.to_ascii_lowercase();
    if lower == "unknown-paper"
        || lower == "unknown"
        || lower == "untitled"
        || lower == "paper"
        || uuid::Uuid::parse_str(title).is_ok()
    {
        return None;
    }
    if let Some(rest) = lower.strip_prefix("paper-") {
        if uuid::Uuid::parse_str(rest).is_ok() {
            return None;
        }
    }
    Some(title.to_string())
}

fn normalize_id_label(raw: &str) -> Option<String> {
    let v = raw.trim();
    if v.is_empty() {
        None
    } else {
        Some(v.to_string())
    }
}

fn normalize_doi_for_link(raw: &str) -> Option<String> {
    let mut doi = raw.trim().to_string();
    if doi.is_empty() {
        return None;
    }
    doi = doi
        .trim_start_matches("https://doi.org/")
        .trim_start_matches("http://doi.org/")
        .trim_start_matches("doi.org/")
        .trim_start_matches("doi:")
        .trim()
        .to_string();
    if doi.is_empty() {
        None
    } else {
        Some(doi)
    }
}

fn build_paper_external_url(reference: &PaperReference) -> Option<String> {
    if let Some(doi) = reference
        .doi
        .as_deref()
        .and_then(normalize_doi_for_link)
        .or_else(|| {
            reference
                .published_version_doi
                .as_deref()
                .and_then(normalize_doi_for_link)
        })
    {
        return Some(format!("https://doi.org/{}", doi));
    }
    if let Some(pmid) = reference.pmid.as_deref().and_then(normalize_id_label) {
        return Some(format!("https://pubmed.ncbi.nlm.nih.gov/{}/", pmid));
    }

    let source = reference
        .source
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let source_id = match reference.source_id.as_deref().and_then(normalize_id_label) {
        Some(v) => v,
        None => return None,
    };

    match source.as_str() {
        "semanticscholar" | "semantic_scholar" => Some(format!(
            "https://www.semanticscholar.org/paper/{}",
            source_id
        )),
        "arxiv" => Some(format!("https://arxiv.org/abs/{}", source_id)),
        "europepmc" => {
            if source_id.to_ascii_uppercase().starts_with("PMC") {
                Some(format!(
                    "https://europepmc.org/articles/{}",
                    source_id.to_ascii_uppercase()
                ))
            } else {
                None
            }
        }
        "biorxiv" => Some(format!("https://doi.org/{}", source_id)),
        _ => None,
    }
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
