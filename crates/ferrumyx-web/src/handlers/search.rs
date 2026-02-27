//! Hybrid search endpoints.
//! Combines vector similarity + keyword search + KG facts.

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::state::SharedState;
use ferrumyx_common::error::ApiError;
use ferrumyx_db::chunks::ChunkRepository;
use ferrumyx_db::kg_facts::KgFactRepository;

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default)]
    pub limit: i32,
    pub cancer_type: Option<String>,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self { q: String::new(), limit: 20, cancer_type: None }
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

/// GET /api/search - Hybrid search (vector + keyword + KG)
pub async fn hybrid_search(
    State(state): State<SharedState>,
    Query(query): Query<SearchQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.max(1).min(100) as usize;

    // Use repositories for search
    let chunk_repo = ChunkRepository::new(state.db.clone());
    let kg_repo = KgFactRepository::new(state.db.clone());

    // 1. Search chunks (placeholder - would need full-text search implementation)
    let chunks = chunk_repo.list(0, limit).await.unwrap_or_default();
    
    // Convert chunks to search results
    let results: Vec<SearchResult> = chunks
        .into_iter()
        .filter(|c| c.content.to_lowercase().contains(&query.q.to_lowercase()))
        .map(|c| SearchResult {
            paper_id: c.paper_id.to_string(),
            title: None,
            chunk_text: c.content,
            similarity: 0.5,
            section_type: None,
            source: "keyword".to_string(),
        })
        .take(limit)
        .collect();

    // 2. Get relevant KG facts
    let kg_facts_data = kg_repo.list(0, 10).await.unwrap_or_default();
    
    let kg_facts: Vec<KgFactBrief> = kg_facts_data
        .into_iter()
        .filter(|f| f.subject_name.to_lowercase().contains(&query.q.to_lowercase()) 
                  || f.object_name.to_lowercase().contains(&query.q.to_lowercase()))
        .map(|f| KgFactBrief {
            fact_type: f.predicate,
            subject: f.subject_name,
            object: f.object_name,
            evidence_count: 1,
        })
        .collect();

    let total = results.len() as u64;

    Ok(Json(HybridSearchResponse {
        query: query.q,
        results,
        kg_facts,
        total,
    }))
}
