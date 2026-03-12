//! Hybrid search endpoints.
//! Combines LanceDB FTS + vector retrieval with RRF fusion + KG evidence aggregation.

use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::state::SharedState;
use ferrumyx_common::error::ApiError;
use ferrumyx_db::{kg_facts::KgFactRepository, papers::PaperRepository};
use ferrumyx_ingestion::embedding::{
    hybrid_search as ingestion_hybrid_search, EmbeddingClient, EmbeddingConfig, HybridSearchConfig,
};
use ferrumyx_ingestion::repository::IngestionRepository;

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default)]
    pub limit: i32,
    pub cancer_type: Option<String>,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            q: String::new(),
            limit: 20,
            cancer_type: None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub paper_id: String,
    pub title: Option<String>,
    pub chunk_text: String,
    pub similarity: f64,
    pub section_type: Option<String>,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct HybridSearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub kg_facts: Vec<KgFactBrief>,
    pub total: u64,
}

#[derive(Debug, Serialize)]
pub struct KgFactBrief {
    pub fact_type: String,
    pub subject: String,
    pub object: String,
    pub evidence_count: i32,
}

/// GET /api/search - Hybrid search (LanceDB FTS + vector + KG)
pub async fn hybrid_search(
    State(state): State<SharedState>,
    Query(query): Query<SearchQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let q = query.q.trim();
    if q.is_empty() {
        return Ok(Json(HybridSearchResponse {
            query: String::new(),
            results: Vec::new(),
            kg_facts: Vec::new(),
            total: 0,
        }));
    }

    let limit = query.limit.max(1).min(100) as usize;
    let scan_limit = (limit * 20).clamp(100, 3000);
    let ingestion_repo = IngestionRepository::new(state.db.clone());
    let kg_repo = KgFactRepository::new(state.db.clone());
    let paper_repo = PaperRepository::new(state.db.clone());

    let q_lower = q.to_ascii_lowercase();
    let cancer_filter = query
        .cancer_type
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_ascii_lowercase());

    let embed_client = EmbeddingClient::new(EmbeddingConfig::default());
    let query_vec = embed_client
        .embed_batch(&[q.to_string()])
        .await
        .ok()
        .and_then(|mut v| v.pop());

    let mut cfg = HybridSearchConfig {
        limit,
        pre_fusion_limit: scan_limit,
        ..HybridSearchConfig::default()
    };

    let mut hybrid_rows = if let Some(v) = query_vec {
        match ingestion_hybrid_search(&ingestion_repo, q, Some(v), &cfg).await {
            Ok(rows) => rows,
            Err(_) => {
                cfg.use_vector = false;
                ingestion_hybrid_search(&ingestion_repo, q, None, &cfg)
                    .await
                    .unwrap_or_default()
            }
        }
    } else {
        cfg.use_vector = false;
        ingestion_hybrid_search(&ingestion_repo, q, None, &cfg)
            .await
            .unwrap_or_default()
    };

    if let Some(cancer) = &cancer_filter {
        hybrid_rows.retain(|r| r.content.to_ascii_lowercase().contains(cancer));
    }

    let paper_ids: Vec<uuid::Uuid> = hybrid_rows.iter().map(|r| r.paper_id).collect();
    let titles_by_id = paper_repo
        .find_titles_by_ids(&paper_ids)
        .await
        .unwrap_or_default();

    let results: Vec<SearchResult> = hybrid_rows
        .into_iter()
        .map(|r| {
            let source = if r.is_hybrid() {
                "hybrid-rrf".to_string()
            } else if r.vector_rank.is_some() {
                "vector".to_string()
            } else {
                "fts".to_string()
            };
            SearchResult {
                paper_id: r.paper_id.to_string(),
                title: titles_by_id.get(&r.paper_id).cloned(),
                chunk_text: r.content,
                similarity: r.score as f64,
                section_type: None,
                source,
            }
        })
        .collect();

    let mut fact_counts: HashMap<(String, String, String), i32> = HashMap::new();
    for fact in kg_repo
        .list_filtered(None, Some(&q_lower), None, (limit * 15).clamp(50, 1200))
        .await
        .unwrap_or_default()
    {
        let key = (fact.predicate, fact.subject_name, fact.object_name);
        *fact_counts.entry(key).or_insert(0) += 1;
    }

    let mut kg_facts: Vec<KgFactBrief> = fact_counts
        .into_iter()
        .map(
            |((fact_type, subject, object), evidence_count)| KgFactBrief {
                fact_type,
                subject,
                object,
                evidence_count,
            },
        )
        .collect();

    kg_facts.sort_by(|a, b| b.evidence_count.cmp(&a.evidence_count));
    kg_facts.truncate(limit);

    Ok(Json(HybridSearchResponse {
        query: q.to_string(),
        total: results.len() as u64,
        results,
        kg_facts,
    }))
}
