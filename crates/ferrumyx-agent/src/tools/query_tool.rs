use async_trait::async_trait;
use ferrumyx_common::query::{QueryRequest, QueryResult};
use ferrumyx_db::chunks::ChunkRepository;
use ferrumyx_db::Database;
use ferrumyx_db::papers::PaperRepository;
use ferrumyx_ingestion::embedding::{
    hybrid_search as ingestion_hybrid_search, EmbeddingClient, EmbeddingConfig, HybridSearchConfig,
    SearchResult,
};
use ferrumyx_ingestion::repository::IngestionRepository;
use ferrumyx_ranker::TargetQueryEngine;
use ferrumyx_runtime::context::JobContext;
use ferrumyx_runtime::tools::{Tool, ToolError, ToolOutput};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

/// Tool to run target prioritization queries from REPL/Gateway.
pub struct TargetQueryTool {
    db: Arc<Database>,
}

impl TargetQueryTool {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct SemanticRerankSummary {
    enabled: bool,
    applied: bool,
    chunks_considered: usize,
    genes_with_evidence: usize,
    rerank_weight: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
struct RagSnippet {
    rank: usize,
    chunk_id: String,
    paper_id: String,
    score: f32,
    section: Option<String>,
    page: Option<i64>,
    date: Option<String>,
    topic_id: Option<usize>,
    dup_group: Option<usize>,
    genes: Vec<String>,
    text: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct RagContextSelection {
    selected_count: usize,
    snippets: Vec<RagSnippet>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct GeneLinkSummary {
    a: String,
    b: String,
    score: f64,
    shared_chunks: usize,
    shared_topics: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
struct NoveltySignalSummary {
    score: f64,
    evidence_chunks: usize,
    unique_chunks: usize,
    shared_chunks: usize,
    topic_count: usize,
    recent_ratio: f64,
    dup_ratio: f64,
    class: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct DedupGroupSummary {
    id: usize,
    representative_chunk_id: String,
    size: usize,
    max_similarity: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
struct DedupSummary {
    mode: String,
    pair_count: usize,
    near_dup_pairs: usize,
    near_dup_ratio: f64,
    max_similarity: f64,
    groups: Vec<DedupGroupSummary>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct TopicClusterSummary {
    id: usize,
    label: String,
    size: usize,
    score: f64,
    terms: Vec<String>,
    genes: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct TopicSummary {
    cluster_count: usize,
    clusters: Vec<TopicClusterSummary>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct DriftSummary {
    recent_cutoff_days: i64,
    recent_count: usize,
    older_count: usize,
    recent_ratio: f64,
    median_age_days: Option<f64>,
    oldest_age_days: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct GeneFeatureBlock {
    fields: Vec<String>,
    values: BTreeMap<String, Vec<f64>>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct DownstreamEmbeddingSummary {
    available: bool,
    mode: String,
    rag_context: RagContextSelection,
    gene_links: Vec<GeneLinkSummary>,
    novelty_signals: BTreeMap<String, NoveltySignalSummary>,
    dedup: DedupSummary,
    topics: TopicSummary,
    drift: DriftSummary,
    gene_features: GeneFeatureBlock,
}

#[derive(Debug, Clone)]
struct DownstreamContext {
    chunk_id: uuid::Uuid,
    paper_id: uuid::Uuid,
    score: f32,
    section: Option<String>,
    page: Option<i64>,
    evidence_date: Option<chrono::DateTime<chrono::Utc>>,
    text: String,
    genes: Vec<String>,
    tokens: HashSet<String>,
    embedding: Option<Vec<f32>>,
    embedding_dim: Option<usize>,
    dup_group: Option<usize>,
    topic_id: Option<usize>,
}

#[derive(Debug, Clone, Default)]
struct GeneContextProfile {
    contexts: Vec<usize>,
    shared_chunks: usize,
    topic_ids: BTreeSet<usize>,
    recent_chunks: usize,
    dup_chunks: usize,
}

#[derive(Debug, Clone)]
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl UnionFind {
    fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
            rank: vec![0; size],
        }
    }

    fn find(&mut self, mut idx: usize) -> usize {
        let mut root = idx;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        while self.parent[idx] != idx {
            let next = self.parent[idx];
            self.parent[idx] = root;
            idx = next;
        }
        root
    }

    fn union(&mut self, a: usize, b: usize) {
        let mut a_root = self.find(a);
        let mut b_root = self.find(b);
        if a_root == b_root {
            return;
        }
        if self.rank[a_root] < self.rank[b_root] {
            std::mem::swap(&mut a_root, &mut b_root);
        }
        self.parent[b_root] = a_root;
        if self.rank[a_root] == self.rank[b_root] {
            self.rank[a_root] = self.rank[a_root].saturating_add(1);
        }
    }
}

#[async_trait]
impl Tool for TargetQueryTool {
    fn name(&self) -> &str {
        "query_targets"
    }

    fn description(&self) -> &str {
        "Executes a Ferrumyx target query and returns ranked targets with score/tier evidence."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query_text": {
                    "type": "string",
                    "description": "Natural-language research question"
                },
                "cancer_code": {
                    "type": "string",
                    "description": "Cancer code (e.g. PAAD, LUAD)"
                },
                "gene_symbol": {
                    "type": "string",
                    "description": "Optional gene filter (e.g. KRAS)"
                },
                "mutation": {
                    "type": "string",
                    "description": "Optional mutation filter (e.g. G12D)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Max ranked rows (default: 20)"
                }
            },
            "required": ["query_text"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let req = QueryRequest {
            query_text: require_str(&params, "query_text")?.to_string(),
            cancer_code: params
                .get("cancer_code")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            gene_symbol: params
                .get("gene_symbol")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            mutation: params
                .get("mutation")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            max_results: params
                .get("max_results")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize)
                .unwrap_or(20),
        };

        let started = std::time::Instant::now();
        let engine = TargetQueryEngine::new(self.db.clone());
        let mut results = engine
            .execute_query(req.clone())
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("query execution failed: {e}")))?;
        let downstream_enabled = downstream_embedding_enabled();
        let collect_semantic_rows = semantic_rerank_enabled() || downstream_enabled;
        let semantic_chunks = if collect_semantic_rows {
            collect_semantic_chunk_rows(self.db.clone(), &req, results.len()).await
        } else {
            Vec::new()
        };
        let semantic_rerank = apply_semantic_rerank(&req, &mut results, &semantic_chunks);
        let downstream_embedding = if downstream_enabled {
            build_downstream_embedding_summary(self.db.clone(), &req, &results, &semantic_chunks)
                .await
        } else {
            downstream_embedding_disabled_summary()
        };

        let payload = json!({
            "query_text": req.query_text,
            "result_count": results.len(),
            "results": results,
            "semantic_rerank": semantic_rerank,
            "downstream_embedding": downstream_embedding,
        });

        Ok(ToolOutput::success(payload, started.elapsed()))
    }
}

fn require_str<'a>(params: &'a serde_json::Value, name: &str) -> Result<&'a str, ToolError> {
    params
        .get(name)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            ToolError::InvalidParameters(format!("missing required string parameter: {name}"))
        })
}

fn semantic_rerank_enabled() -> bool {
    std::env::var("FERRUMYX_QUERY_SEMANTIC_RERANK")
        .ok()
        .is_none_or(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

fn downstream_embedding_enabled() -> bool {
    std::env::var("FERRUMYX_QUERY_DOWNSTREAM_EMBEDDING")
        .ok()
        .is_none_or(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

fn semantic_rerank_limit(max_results: usize) -> usize {
    std::env::var("FERRUMYX_QUERY_SEMANTIC_TOPK")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or_else(|| max_results.saturating_mul(12).clamp(24, 180))
        .clamp(16, 320)
}

fn semantic_rerank_weight() -> f64 {
    std::env::var("FERRUMYX_QUERY_SEMANTIC_WEIGHT")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.045)
        .clamp(0.0, 0.20)
}

fn push_unique_flag(flags: &mut Vec<String>, flag: String) {
    if !flags.iter().any(|f| f == &flag) {
        flags.push(flag);
    }
}

fn contains_gene_symbol(text: &str, symbol_upper: &str) -> bool {
    let text_upper = text.to_ascii_uppercase();
    let bytes = text_upper.as_bytes();
    let needle = symbol_upper.as_bytes();
    if needle.is_empty() || bytes.len() < needle.len() {
        return false;
    }
    let mut i = 0usize;
    while i + needle.len() <= bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            let left_ok = if i == 0 {
                true
            } else {
                !bytes[i - 1].is_ascii_alphanumeric()
            };
            let right_idx = i + needle.len();
            let right_ok = if right_idx >= bytes.len() {
                true
            } else {
                !bytes[right_idx].is_ascii_alphanumeric()
            };
            if left_ok && right_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

#[derive(Debug, Clone, Default)]
struct SemanticEvidence {
    hits: usize,
    best_rank: usize,
    best_score: f32,
}

fn apply_semantic_rerank(
    _req: &QueryRequest,
    results: &mut Vec<QueryResult>,
    chunk_rows: &[SearchResult],
) -> SemanticRerankSummary {
    if results.is_empty() || !semantic_rerank_enabled() {
        return SemanticRerankSummary {
            enabled: semantic_rerank_enabled(),
            applied: false,
            chunks_considered: 0,
            genes_with_evidence: 0,
            rerank_weight: semantic_rerank_weight(),
        };
    }

    let rerank_weight = semantic_rerank_weight();

    if chunk_rows.is_empty() {
        return SemanticRerankSummary {
            enabled: true,
            applied: false,
            chunks_considered: 0,
            genes_with_evidence: 0,
            rerank_weight,
        };
    }

    let gene_symbols: Vec<String> = results
        .iter()
        .map(|r| r.gene_symbol.trim().to_ascii_uppercase())
        .filter(|s| !s.is_empty())
        .collect();

    let mut evidence_by_gene: HashMap<String, SemanticEvidence> = HashMap::new();
    for (idx, row) in chunk_rows.iter().enumerate() {
        let rank = idx + 1;
        for symbol in &gene_symbols {
            if contains_gene_symbol(&row.content, symbol) {
                let entry = evidence_by_gene
                    .entry(symbol.clone())
                    .or_insert_with(|| SemanticEvidence {
                        hits: 0,
                        best_rank: rank,
                        best_score: row.score,
                    });
                entry.hits += 1;
                if rank < entry.best_rank {
                    entry.best_rank = rank;
                }
                if row.score > entry.best_score {
                    entry.best_score = row.score;
                }
            }
        }
    }

    if evidence_by_gene.is_empty() {
        return SemanticRerankSummary {
            enabled: true,
            applied: false,
            chunks_considered: chunk_rows.len(),
            genes_with_evidence: 0,
            rerank_weight,
        };
    }

    let total_chunks = chunk_rows.len().max(1) as f64;
    let mut genes_with_evidence = 0usize;
    for row in results.iter_mut() {
        let symbol = row.gene_symbol.trim().to_ascii_uppercase();
        let Some(evidence) = evidence_by_gene.get(&symbol) else {
            continue;
        };
        genes_with_evidence += 1;

        let rank_strength = (1.0 - ((evidence.best_rank.saturating_sub(1) as f64) / total_chunks))
            .clamp(0.0, 1.0);
        let hit_strength = (evidence.hits.min(5) as f64 / 5.0).clamp(0.0, 1.0);
        let score_strength = (evidence.best_score as f64).clamp(0.0, 1.0);
        let semantic_strength =
            (0.60 * rank_strength + 0.25 * hit_strength + 0.15 * score_strength).clamp(0.0, 1.0);
        let boost = (rerank_weight * semantic_strength).clamp(0.0, 0.20);

        row.composite_score = (row.composite_score + boost).clamp(0.0, 0.98);
        row.confidence_adj = (row.confidence_adj + (boost * 0.85)).clamp(0.0, 0.95);
        push_unique_flag(
            &mut row.flags,
            format!(
                "SEMANTIC_EVIDENCE(rank={},hits={})",
                evidence.best_rank, evidence.hits
            ),
        );

        if row.component_sources.is_none() {
            row.component_sources = Some(BTreeMap::new());
        }
        if let Some(ref mut sources) = row.component_sources {
            sources.insert(
                "semantic_retrieval".to_string(),
                "hybrid_search_rrf".to_string(),
            );
        }
    }

    results.sort_by(|a, b| {
        b.composite_score
            .partial_cmp(&a.composite_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                b.confidence_adj
                    .partial_cmp(&a.confidence_adj)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let total = results.len().max(1) as f64;
    for (idx, row) in results.iter_mut().enumerate() {
        row.rank = idx + 1;
        row.percentile = Some((100.0 * (1.0 - (idx as f64 / total))).clamp(0.0, 100.0));
    }

    SemanticRerankSummary {
        enabled: true,
        applied: true,
        chunks_considered: chunk_rows.len(),
        genes_with_evidence,
        rerank_weight,
    }
}

const DOWNSTREAM_CONTEXT_LIMIT: usize = 32;
const RAG_SNIPPET_LIMIT: usize = 8;
const DEDUP_GROUP_LIMIT: usize = 5;
const TOPIC_CLUSTER_LIMIT: usize = 4;
const DRIFT_RECENT_CUTOFF_DAYS: i64 = 365;

async fn collect_semantic_chunk_rows(
    db: Arc<Database>,
    req: &QueryRequest,
    max_results: usize,
) -> Vec<SearchResult> {
    let topk = semantic_rerank_limit(max_results.max(1));
    let mut search_cfg = HybridSearchConfig {
        limit: topk,
        pre_fusion_limit: (topk.saturating_mul(4)).clamp(80, 1200),
        ..HybridSearchConfig::default()
    };

    let ingestion_repo = IngestionRepository::new(db);
    let embed_client = EmbeddingClient::new(EmbeddingConfig::default());
    let query_vec = embed_client
        .embed_batch(&[req.query_text.clone()])
        .await
        .ok()
        .and_then(|mut rows| rows.pop());

    if let Some(vec) = query_vec {
        match ingestion_hybrid_search(&ingestion_repo, &req.query_text, Some(vec), &search_cfg).await
        {
            Ok(rows) => rows,
            Err(_) => {
                search_cfg.use_vector = false;
                ingestion_hybrid_search(&ingestion_repo, &req.query_text, None, &search_cfg)
                    .await
                    .unwrap_or_default()
            }
        }
    } else {
        search_cfg.use_vector = false;
        ingestion_hybrid_search(&ingestion_repo, &req.query_text, None, &search_cfg)
            .await
            .unwrap_or_default()
    }
}

async fn build_downstream_embedding_summary(
    db: Arc<Database>,
    _req: &QueryRequest,
    results: &[QueryResult],
    chunk_rows: &[SearchResult],
) -> DownstreamEmbeddingSummary {
    let gene_symbols: Vec<String> = results
        .iter()
        .map(|r| r.gene_symbol.trim().to_ascii_uppercase())
        .filter(|s| !s.is_empty())
        .collect();
    let gene_set: HashSet<String> = gene_symbols.iter().cloned().collect();

    let mut contexts = load_downstream_contexts(db, chunk_rows, &gene_set, &gene_symbols).await;
    let dedup = assign_dedup_groups(&mut contexts);
    let topics = assign_topic_groups_simple(&mut contexts, &gene_set);
    let drift = build_drift_summary(&contexts);
    let gene_profiles = build_gene_profiles(&contexts);
    let rag_context = build_rag_snippets(&contexts);
    let novelty_signals = build_novelty_signals(results, &gene_profiles);
    let gene_features = build_gene_feature_block(results, &gene_profiles);
    let gene_links = build_gene_links(results, &gene_profiles);

    let mode = if contexts.is_empty() {
        "none"
    } else if contexts.iter().all(|ctx| ctx.embedding.is_some()) {
        "embedding"
    } else if contexts.iter().any(|ctx| ctx.embedding.is_some()) {
        "mixed"
    } else {
        "lexical"
    }
    .to_string();

    DownstreamEmbeddingSummary {
        available: !contexts.is_empty(),
        mode,
        rag_context,
        gene_links,
        novelty_signals,
        dedup,
        topics,
        drift,
        gene_features,
    }
}

fn downstream_embedding_disabled_summary() -> DownstreamEmbeddingSummary {
    DownstreamEmbeddingSummary {
        available: false,
        mode: "disabled".to_string(),
        rag_context: RagContextSelection {
            selected_count: 0,
            snippets: Vec::new(),
        },
        gene_links: Vec::new(),
        novelty_signals: BTreeMap::new(),
        dedup: DedupSummary {
            mode: "disabled".to_string(),
            pair_count: 0,
            near_dup_pairs: 0,
            near_dup_ratio: 0.0,
            max_similarity: 0.0,
            groups: Vec::new(),
        },
        topics: TopicSummary {
            cluster_count: 0,
            clusters: Vec::new(),
        },
        drift: DriftSummary {
            recent_cutoff_days: DRIFT_RECENT_CUTOFF_DAYS,
            recent_count: 0,
            older_count: 0,
            recent_ratio: 0.0,
            median_age_days: None,
            oldest_age_days: None,
        },
        gene_features: GeneFeatureBlock {
            fields: Vec::new(),
            values: BTreeMap::new(),
        },
    }
}

async fn load_downstream_contexts(
    db: Arc<Database>,
    chunk_rows: &[SearchResult],
    gene_set: &HashSet<String>,
    gene_symbols: &[String],
) -> Vec<DownstreamContext> {
    if chunk_rows.is_empty() {
        return Vec::new();
    }

    let chunk_repo = ChunkRepository::new(db.clone());
    let paper_repo = PaperRepository::new(db);
    let analysis_rows: Vec<&SearchResult> = chunk_rows.iter().take(DOWNSTREAM_CONTEXT_LIMIT).collect();
    let chunk_ids: Vec<uuid::Uuid> = analysis_rows.iter().map(|row| row.chunk_id).collect();
    let chunk_map = chunk_repo.find_by_ids(&chunk_ids).await.unwrap_or_default();

    let mut paper_ids = Vec::new();
    let mut seen_papers = HashSet::new();
    for chunk in chunk_map.values() {
        if seen_papers.insert(chunk.paper_id) {
            paper_ids.push(chunk.paper_id);
        }
    }

    let paper_map: HashMap<uuid::Uuid, chrono::DateTime<chrono::Utc>> = paper_repo
        .find_published_at_by_ids(&paper_ids)
        .await
        .unwrap_or_default();

    let mut contexts = Vec::with_capacity(analysis_rows.len());
    for row in analysis_rows {
        let chunk = chunk_map.get(&row.chunk_id);
        let paper_id = chunk.map(|c| c.paper_id).unwrap_or(row.paper_id);
        let section = chunk.and_then(|c| c.section.clone());
        let page = chunk.and_then(|c| c.page);
        let evidence_date = chunk
            .and_then(|c| paper_map.get(&c.paper_id).copied())
            .or_else(|| chunk.map(|c| c.created_at));
        let embedding = chunk.and_then(preferred_context_embedding);
        let embedding_dim = embedding.as_ref().map(|vec| vec.len());
        let genes = collect_gene_hits(&row.content, gene_symbols);
        let tokens = tokenize_context_terms(&row.content, gene_set);

        contexts.push(DownstreamContext {
            chunk_id: row.chunk_id,
            paper_id,
            score: row.score,
            section,
            page,
            evidence_date,
            text: row.content.clone(),
            genes,
            tokens,
            embedding,
            embedding_dim,
            dup_group: None,
            topic_id: None,
        });
    }

    contexts.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.chunk_id.cmp(&b.chunk_id))
    });

    contexts
}

fn preferred_context_embedding(chunk: &ferrumyx_db::schema::Chunk) -> Option<Vec<f32>> {
    if let Some(ref emb) = chunk.embedding_large {
        if !emb.is_empty() {
            return Some(emb.clone());
        }
    }
    if let Some(ref emb) = chunk.embedding {
        if !emb.is_empty() {
            return Some(emb.clone());
        }
    }
    None
}

fn collect_gene_hits(content: &str, gene_symbols: &[String]) -> Vec<String> {
    let mut hits = Vec::new();
    for symbol in gene_symbols {
        if contains_gene_symbol(content, symbol) && !hits.iter().any(|existing| existing == symbol) {
            hits.push(symbol.clone());
        }
    }
    hits
}

fn tokenize_context_terms(content: &str, gene_set: &HashSet<String>) -> HashSet<String> {
    let stopwords = topic_stopwords();
    let mut tokens = HashSet::new();
    for raw in content.split(|c: char| !c.is_ascii_alphanumeric()) {
        let token = raw.trim().to_ascii_lowercase();
        if token.len() < 3 || stopwords.contains(token.as_str()) {
            continue;
        }
        if gene_set.contains(&token.to_ascii_uppercase()) {
            continue;
        }
        tokens.insert(token);
    }
    tokens
}

fn topic_stopwords() -> &'static HashSet<&'static str> {
    static STOPWORDS: std::sync::OnceLock<HashSet<&'static str>> = std::sync::OnceLock::new();
    STOPWORDS.get_or_init(|| {
        [
            "and", "the", "for", "with", "from", "that", "this", "these", "those", "into",
            "between", "among", "within", "without", "using", "used", "use", "been", "were",
            "was", "are", "via", "after", "before", "over", "under", "more", "most", "less",
            "than", "then", "also", "such", "show", "shows", "showed", "study", "studies",
            "result", "results", "data", "figure", "fig", "analysis", "patient", "patients",
            "cell", "cells", "cancer", "tumor", "tumour", "gene", "genes",
        ]
        .into_iter()
        .collect()
    })
}

fn build_drift_summary(contexts: &[DownstreamContext]) -> DriftSummary {
    let cutoff_days = DRIFT_RECENT_CUTOFF_DAYS;
    if contexts.is_empty() {
        return DriftSummary {
            recent_cutoff_days: cutoff_days,
            recent_count: 0,
            older_count: 0,
            recent_ratio: 0.0,
            median_age_days: None,
            oldest_age_days: None,
        };
    }

    let now = chrono::Utc::now();
    let mut ages = Vec::new();
    let mut recent_count = 0usize;
    let mut older_count = 0usize;
    for ctx in contexts {
        if let Some(evidence_date) = ctx.evidence_date {
            let age_days = now
                .signed_duration_since(evidence_date)
                .num_seconds()
                .max(0) as f64
                / 86_400.0;
            ages.push(age_days);
            if age_days <= cutoff_days as f64 {
                recent_count += 1;
            } else {
                older_count += 1;
            }
        }
    }

    ages.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_age_days = if ages.is_empty() {
        None
    } else if ages.len() % 2 == 1 {
        Some(ages[ages.len() / 2])
    } else {
        Some((ages[ages.len() / 2 - 1] + ages[ages.len() / 2]) / 2.0)
    };

    let total = recent_count + older_count;
    DriftSummary {
        recent_cutoff_days: cutoff_days,
        recent_count,
        older_count,
        recent_ratio: if total == 0 {
            0.0
        } else {
            recent_count as f64 / total as f64
        },
        median_age_days,
        oldest_age_days: ages.last().copied(),
    }
}

fn assign_dedup_groups(contexts: &mut [DownstreamContext]) -> DedupSummary {
    if contexts.is_empty() {
        return DedupSummary {
            mode: "none".to_string(),
            pair_count: 0,
            near_dup_pairs: 0,
            near_dup_ratio: 0.0,
            max_similarity: 0.0,
            groups: Vec::new(),
        };
    }

    if contexts.len() == 1 {
        return DedupSummary {
            mode: if contexts[0].embedding.is_some() {
                "embedding".to_string()
            } else {
                "lexical".to_string()
            },
            pair_count: 0,
            near_dup_pairs: 0,
            near_dup_ratio: 0.0,
            max_similarity: 0.0,
            groups: Vec::new(),
        };
    }

    let mut uf = UnionFind::new(contexts.len());
    let mut pair_count = 0usize;
    let mut near_dup_pairs = 0usize;
    let mut max_similarity = 0.0f64;
    let mut used_embedding = false;
    let mut used_lexical = false;

    for i in 0..contexts.len() {
        for j in (i + 1)..contexts.len() {
            pair_count += 1;
            let (similarity, via_embedding) = context_similarity(&contexts[i], &contexts[j]);
            if via_embedding {
                used_embedding = true;
            } else {
                used_lexical = true;
            }
            if similarity > max_similarity {
                max_similarity = similarity;
            }
            let threshold = if via_embedding { 0.86 } else { 0.38 };
            if similarity >= threshold {
                near_dup_pairs += 1;
                uf.union(i, j);
            }
        }
    }

    let mut groups_by_root: HashMap<usize, Vec<usize>> = HashMap::new();
    for idx in 0..contexts.len() {
        let root = uf.find(idx);
        groups_by_root.entry(root).or_default().push(idx);
    }

    let mut groups: Vec<(Vec<usize>, f64)> = Vec::new();
    for members in groups_by_root.into_values() {
        if members.len() < 2 {
            continue;
        }
        let mut group_max = 0.0f64;
        for i in 0..members.len() {
            for j in (i + 1)..members.len() {
                let (similarity, _) = context_similarity(&contexts[members[i]], &contexts[members[j]]);
                if similarity > group_max {
                    group_max = similarity;
                }
            }
        }
        groups.push((members, group_max));
    }

    groups.sort_by(|a, b| {
        b.0.len()
            .cmp(&a.0.len())
            .then_with(|| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| {
                let a_rep = a.0.iter().map(|idx| contexts[*idx].chunk_id).min();
                let b_rep = b.0.iter().map(|idx| contexts[*idx].chunk_id).min();
                a_rep.cmp(&b_rep)
            })
    });

    let mut summaries = Vec::new();
    for (group_id, (members, group_max)) in groups.into_iter().take(DEDUP_GROUP_LIMIT).enumerate() {
        let representative_idx = members
            .iter()
            .copied()
            .max_by(|a, b| {
                contexts[*a]
                    .score
                    .partial_cmp(&contexts[*b].score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| contexts[*a].chunk_id.cmp(&contexts[*b].chunk_id))
            })
            .unwrap_or(members[0]);
        for idx in &members {
            contexts[*idx].dup_group = Some(group_id);
        }
        summaries.push(DedupGroupSummary {
            id: group_id,
            representative_chunk_id: contexts[representative_idx].chunk_id.to_string(),
            size: members.len(),
            max_similarity: group_max,
        });
    }

    let mode = if used_embedding && used_lexical {
        "mixed"
    } else if used_embedding {
        "embedding"
    } else {
        "lexical"
    }
    .to_string();

    DedupSummary {
        mode,
        pair_count,
        near_dup_pairs,
        near_dup_ratio: if pair_count == 0 {
            0.0
        } else {
            near_dup_pairs as f64 / pair_count as f64
        },
        max_similarity,
        groups: summaries,
    }
}

fn assign_topic_groups_simple(
    contexts: &mut [DownstreamContext],
    gene_set: &HashSet<String>,
) -> TopicSummary {
    if contexts.is_empty() {
        return TopicSummary {
            cluster_count: 0,
            clusters: Vec::new(),
        };
    }

    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (idx, ctx) in contexts.iter().enumerate() {
        let label = topic_signature(ctx, gene_set);
        groups.entry(label).or_default().push(idx);
    }

    let mut ranked: Vec<(String, Vec<usize>, f64)> = groups
        .into_iter()
        .map(|(label, members)| {
            let avg = if members.is_empty() {
                0.0
            } else {
                members.iter().map(|idx| contexts[*idx].score as f64).sum::<f64>()
                    / members.len() as f64
            };
            (label, members, avg)
        })
        .collect();
    ranked.sort_by(|a, b| {
        b.1.len()
            .cmp(&a.1.len())
            .then_with(|| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| a.0.cmp(&b.0))
    });

    let mut summaries = Vec::new();
    for (cluster_id, (label, members, avg_score)) in ranked.into_iter().take(TOPIC_CLUSTER_LIMIT).enumerate() {
        for idx in &members {
            contexts[*idx].topic_id = Some(cluster_id);
        }
        let terms = top_terms_for_group(&members, contexts, gene_set, 3);
        let mut genes = BTreeSet::new();
        for idx in &members {
            for gene in &contexts[*idx].genes {
                genes.insert(gene.clone());
            }
        }
        summaries.push(TopicClusterSummary {
            id: cluster_id,
            label,
            size: members.len(),
            score: avg_score,
            terms,
            genes: genes.into_iter().collect(),
        });
    }

    TopicSummary {
        cluster_count: summaries.len(),
        clusters: summaries,
    }
}

fn topic_signature(ctx: &DownstreamContext, gene_set: &HashSet<String>) -> String {
    let mut terms = top_terms_from_tokens(&ctx.tokens, gene_set, 2);
    if let Some(section) = &ctx.section {
        let section = section.trim().to_ascii_lowercase();
        if !section.is_empty() {
            terms.insert(0, section);
        }
    }
    if terms.is_empty() {
        "mixed".to_string()
    } else {
        terms.join("/")
    }
}

fn top_terms_for_group(
    members: &[usize],
    contexts: &[DownstreamContext],
    gene_set: &HashSet<String>,
    limit: usize,
) -> Vec<String> {
    let mut freq: BTreeMap<String, usize> = BTreeMap::new();
    for idx in members {
        for token in &contexts[*idx].tokens {
            if gene_set.contains(&token.to_ascii_uppercase()) {
                continue;
            }
            *freq.entry(token.clone()).or_insert(0) += 1;
        }
    }
    let mut terms: Vec<(String, usize)> = freq.into_iter().collect();
    terms.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    terms.into_iter().take(limit).map(|(term, _)| term).collect()
}

fn top_terms_from_tokens(
    tokens: &HashSet<String>,
    gene_set: &HashSet<String>,
    limit: usize,
) -> Vec<String> {
    let mut freq: BTreeMap<String, usize> = BTreeMap::new();
    for token in tokens {
        if gene_set.contains(&token.to_ascii_uppercase()) {
            continue;
        }
        *freq.entry(token.clone()).or_insert(0) += 1;
    }
    let mut terms: Vec<(String, usize)> = freq.into_iter().collect();
    terms.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    terms.into_iter().take(limit).map(|(term, _)| term).collect()
}

fn build_gene_profiles(contexts: &[DownstreamContext]) -> BTreeMap<String, GeneContextProfile> {
    let mut profiles: BTreeMap<String, GeneContextProfile> = BTreeMap::new();
    let now = chrono::Utc::now();
    for (idx, ctx) in contexts.iter().enumerate() {
        let is_recent = ctx
            .evidence_date
            .map(|dt| now.signed_duration_since(dt).num_days() <= DRIFT_RECENT_CUTOFF_DAYS)
            .unwrap_or(false);
        let is_shared = ctx.genes.len() > 1;
        for gene in &ctx.genes {
            let entry = profiles.entry(gene.clone()).or_default();
            entry.contexts.push(idx);
            if is_recent {
                entry.recent_chunks += 1;
            }
            if is_shared {
                entry.shared_chunks += 1;
            }
            if ctx.dup_group.is_some() {
                entry.dup_chunks += 1;
            }
            if let Some(topic_id) = ctx.topic_id {
                entry.topic_ids.insert(topic_id);
            }
        }
    }
    profiles
}

fn build_rag_snippets(contexts: &[DownstreamContext]) -> RagContextSelection {
    if contexts.is_empty() {
        return RagContextSelection {
            selected_count: 0,
            snippets: Vec::new(),
        };
    }

    let mut ordered: Vec<usize> = (0..contexts.len()).collect();
    ordered.sort_by(|a, b| {
        contexts[*b]
            .score
            .partial_cmp(&contexts[*a].score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| contexts[*a].chunk_id.cmp(&contexts[*b].chunk_id))
    });

    let mut selected = Vec::new();
    let mut seen_groups = HashSet::new();
    for idx in &ordered {
        if let Some(group_id) = contexts[*idx].dup_group {
            if seen_groups.contains(&group_id) {
                continue;
            }
            seen_groups.insert(group_id);
        }
        selected.push(*idx);
        if selected.len() >= RAG_SNIPPET_LIMIT {
            break;
        }
    }
    if selected.len() < RAG_SNIPPET_LIMIT {
        for idx in ordered {
            if selected.contains(&idx) {
                continue;
            }
            selected.push(idx);
            if selected.len() >= RAG_SNIPPET_LIMIT {
                break;
            }
        }
    }

    let snippets: Vec<RagSnippet> = selected
        .into_iter()
        .enumerate()
        .map(|(rank, idx)| {
            let ctx = &contexts[idx];
            RagSnippet {
                rank: rank + 1,
                chunk_id: ctx.chunk_id.to_string(),
                paper_id: ctx.paper_id.to_string(),
                score: ctx.score,
                section: ctx.section.clone(),
                page: ctx.page,
                date: ctx
                    .evidence_date
                    .map(|dt| dt.format("%Y-%m-%d").to_string()),
                topic_id: ctx.topic_id,
                dup_group: ctx.dup_group,
                genes: ctx.genes.clone(),
                text: truncate_text(&ctx.text, 240),
            }
        })
        .collect();

    RagContextSelection {
        selected_count: snippets.len(),
        snippets,
    }
}

fn build_novelty_signals(
    results: &[QueryResult],
    profiles: &BTreeMap<String, GeneContextProfile>,
) -> BTreeMap<String, NoveltySignalSummary> {
    let mut out = BTreeMap::new();
    for result in results {
        let gene = result.gene_symbol.trim().to_ascii_uppercase();
        let profile = profiles.get(&gene).cloned().unwrap_or_default();
        let evidence_chunks = profile.contexts.len();
        let unique_chunks = evidence_chunks.saturating_sub(profile.shared_chunks);
        let recent_ratio = if evidence_chunks == 0 {
            0.0
        } else {
            profile.recent_chunks as f64 / evidence_chunks as f64
        };
        let dup_ratio = if evidence_chunks == 0 {
            0.0
        } else {
            profile.dup_chunks as f64 / evidence_chunks as f64
        };
        let lit_novelty = result
            .metrics
            .as_ref()
            .map(|m| m.literature_novelty_velocity.clamp(0.0, 1.0))
            .unwrap_or(0.0);
        let sparse_bonus = 1.0 - (evidence_chunks.min(6) as f64 / 6.0);
        let score = (0.45 * lit_novelty
            + 0.25 * sparse_bonus
            + 0.15 * recent_ratio
            + 0.15 * (1.0 - dup_ratio))
            .clamp(0.0, 1.0);
        let class = if score >= 0.70 {
            "novel"
        } else if score >= 0.45 {
            "mixed"
        } else {
            "established"
        }
        .to_string();
        out.insert(
            gene,
            NoveltySignalSummary {
                score,
                evidence_chunks,
                unique_chunks,
                shared_chunks: profile.shared_chunks,
                topic_count: profile.topic_ids.len(),
                recent_ratio,
                dup_ratio,
                class,
            },
        );
    }
    out
}

fn build_gene_feature_block(
    results: &[QueryResult],
    profiles: &BTreeMap<String, GeneContextProfile>,
) -> GeneFeatureBlock {
    let fields = vec![
        "score".to_string(),
        "confidence".to_string(),
        "mutation_freq".to_string(),
        "crispr_dependency".to_string(),
        "survival_correlation".to_string(),
        "expression_specificity".to_string(),
        "pdb_structure_count".to_string(),
        "af_plddt_mean".to_string(),
        "fpocket_best_score".to_string(),
        "chembl_inhibitor_count".to_string(),
        "reactome_escape_pathway_count".to_string(),
        "literature_novelty_velocity".to_string(),
        "evidence_chunks".to_string(),
        "unique_chunks".to_string(),
        "shared_chunks".to_string(),
        "recent_ratio".to_string(),
        "dup_ratio".to_string(),
        "topic_count".to_string(),
    ];

    let mut values = BTreeMap::new();
    for result in results {
        let gene = result.gene_symbol.trim().to_ascii_uppercase();
        let profile = profiles.get(&gene).cloned().unwrap_or_default();
        let metrics = result.metrics.clone().unwrap_or_default();
        let evidence_chunks = profile.contexts.len() as f64;
        let unique_chunks = evidence_chunks - profile.shared_chunks as f64;
        let recent_ratio = if evidence_chunks == 0.0 {
            0.0
        } else {
            profile.recent_chunks as f64 / evidence_chunks
        };
        let dup_ratio = if evidence_chunks == 0.0 {
            0.0
        } else {
            profile.dup_chunks as f64 / evidence_chunks
        };
        values.insert(
            gene,
            vec![
                result.composite_score,
                result.confidence_adj,
                metrics.mutation_freq,
                metrics.crispr_dependency,
                metrics.survival_correlation,
                metrics.expression_specificity,
                metrics.pdb_structure_count as f64,
                metrics.af_plddt_mean,
                metrics.fpocket_best_score,
                metrics.chembl_inhibitor_count as f64,
                metrics.reactome_escape_pathway_count as f64,
                metrics.literature_novelty_velocity,
                evidence_chunks,
                unique_chunks,
                profile.shared_chunks as f64,
                recent_ratio,
                dup_ratio,
                profile.topic_ids.len() as f64,
            ],
        );
    }

    GeneFeatureBlock { fields, values }
}

fn build_gene_links(
    results: &[QueryResult],
    profiles: &BTreeMap<String, GeneContextProfile>,
) -> Vec<GeneLinkSummary> {
    let mut links = Vec::new();
    for i in 0..results.len() {
        for j in (i + 1)..results.len() {
            let a = results[i].gene_symbol.trim().to_ascii_uppercase();
            let b = results[j].gene_symbol.trim().to_ascii_uppercase();
            let profile_a = profiles.get(&a).cloned().unwrap_or_default();
            let profile_b = profiles.get(&b).cloned().unwrap_or_default();
            let feature_a = gene_similarity_vector(&results[i], &profile_a);
            let feature_b = gene_similarity_vector(&results[j], &profile_b);
            let vector_score = cosine_similarity_f64(&feature_a, &feature_b);

            let set_a: HashSet<usize> = profile_a.contexts.iter().copied().collect();
            let set_b: HashSet<usize> = profile_b.contexts.iter().copied().collect();
            let shared_chunks = set_a.intersection(&set_b).count();
            let union_chunks = set_a.union(&set_b).count().max(1);
            let shared_topics = profile_a.topic_ids.intersection(&profile_b.topic_ids).count();
            let union_topics = profile_a.topic_ids.union(&profile_b.topic_ids).count().max(1);
            let score = (0.70 * vector_score
                + 0.20 * (shared_chunks as f64 / union_chunks as f64)
                + 0.10 * (shared_topics as f64 / union_topics as f64))
                .clamp(0.0, 1.0);

            if score >= 0.55 || shared_chunks > 0 || shared_topics > 0 {
                links.push(GeneLinkSummary {
                    a,
                    b,
                    score,
                    shared_chunks,
                    shared_topics,
                });
            }
        }
    }

    links.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.a.cmp(&b.a))
            .then_with(|| a.b.cmp(&b.b))
    });
    links.truncate(12);
    links
}

fn context_similarity(a: &DownstreamContext, b: &DownstreamContext) -> (f64, bool) {
    let lexical = lexical_similarity(&a.tokens, &b.tokens);
    if let (Some(a_vec), Some(b_vec)) = (a.embedding.as_ref(), b.embedding.as_ref()) {
        if a.embedding_dim == b.embedding_dim && a.embedding_dim == Some(a_vec.len()) {
            let a_vec_f64: Vec<f64> = a_vec.iter().map(|v| *v as f64).collect();
            let b_vec_f64: Vec<f64> = b_vec.iter().map(|v| *v as f64).collect();
            let embed = cosine_similarity_f64(&a_vec_f64, &b_vec_f64);
            return ((0.80 * embed + 0.20 * lexical).clamp(0.0, 1.0), true);
        }
    }
    (lexical, false)
}

fn lexical_similarity(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let shared = a.intersection(b).count() as f64;
    shared / a.union(b).count().max(1) as f64
}

fn cosine_similarity_f64(a: &[f64], b: &[f64]) -> f64 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        ((dot / (norm_a.sqrt() * norm_b.sqrt())) + 1.0) / 2.0
    }
}

fn gene_similarity_vector(result: &QueryResult, profile: &GeneContextProfile) -> Vec<f64> {
    let metrics = result.metrics.clone().unwrap_or_default();
    let evidence_chunks = profile.contexts.len() as f64;
    let unique_chunks = evidence_chunks - profile.shared_chunks as f64;
    let recent_ratio = if evidence_chunks == 0.0 {
        0.0
    } else {
        profile.recent_chunks as f64 / evidence_chunks
    };
    let dup_ratio = if evidence_chunks == 0.0 {
        0.0
    } else {
        profile.dup_chunks as f64 / evidence_chunks
    };
    vec![
        result.composite_score.clamp(0.0, 1.0),
        result.confidence_adj.clamp(0.0, 1.0),
        metrics.mutation_freq.clamp(0.0, 1.0),
        normalise_dependency(metrics.crispr_dependency),
        normalise_bounded(metrics.survival_correlation, -1.0, 1.0),
        normalise_positive(metrics.expression_specificity, 5.0),
        normalise_positive(metrics.pdb_structure_count as f64, 10.0),
        normalise_positive(metrics.af_plddt_mean, 100.0),
        normalise_positive(metrics.fpocket_best_score, 3.0),
        normalise_positive(metrics.chembl_inhibitor_count as f64, 100.0),
        normalise_positive(metrics.reactome_escape_pathway_count as f64, 25.0),
        metrics.literature_novelty_velocity.clamp(0.0, 1.0),
        normalise_positive(evidence_chunks, 8.0),
        normalise_positive(unique_chunks.max(0.0), 8.0),
        normalise_positive(profile.shared_chunks as f64, 8.0),
        recent_ratio,
        (1.0 - dup_ratio).clamp(0.0, 1.0),
        normalise_positive(profile.topic_ids.len() as f64, 6.0),
    ]
}

fn normalise_positive(value: f64, scale: f64) -> f64 {
    if scale <= 0.0 {
        0.0
    } else {
        (value / scale).clamp(0.0, 1.0)
    }
}

fn normalise_bounded(value: f64, min: f64, max: f64) -> f64 {
    if max <= min {
        return 0.0;
    }
    ((value - min) / (max - min)).clamp(0.0, 1.0)
}

fn normalise_dependency(value: f64) -> f64 {
    ((-value).max(0.0) / 2.0).clamp(0.0, 1.0)
}

fn truncate_text(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    text.chars().take(limit).collect::<String>() + "..."
}
