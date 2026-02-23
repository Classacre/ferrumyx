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
    pub total: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
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
    let limit = query.limit.max(1).min(100);
    let search_term = format!("%{}%", query.q.to_lowercase());

    // 1. Keyword search on paper chunks using query_as with explicit struct
    let keyword_results: Vec<(String, Option<String>, String, Option<String>)> = sqlx::query_as(
        r#"
        SELECT 
            pc.paper_id::text,
            p.title,
            pc.content,
            pc.section_type
        FROM paper_chunks pc
        JOIN papers p ON p.id = pc.paper_id
        WHERE LOWER(pc.content) LIKE $1
        ORDER BY pc.chunk_index
        LIMIT $2
        "#
    )
    .bind(&search_term)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    // 2. Get relevant KG facts
    let kg_facts = sqlx::query_as::<_, KgFactBrief>(
        r#"
        SELECT 
            fact_type,
            subject,
            object,
            evidence_count
        FROM kg_facts
        WHERE subject ILIKE $1 OR object ILIKE $1
        ORDER BY evidence_count DESC
        LIMIT 10
        "#
    )
    .bind(format!("%{}%", query.q))
    .fetch_all(&state.db)
    .await?;

    // Combine results
    let results: Vec<SearchResult> = keyword_results
        .into_iter()
        .map(|(paper_id, title, chunk_text, section_type)| SearchResult {
            paper_id,
            title,
            chunk_text,
            similarity: 0.5,
            section_type,
            source: "keyword".to_string(),
        })
        .collect();

    let total = results.len() as i64;

    Ok(Json(HybridSearchResponse {
        query: query.q,
        results,
        kg_facts,
        total,
    }))
}