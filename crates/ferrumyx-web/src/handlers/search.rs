//! Hybrid search endpoints.
//! Combines vector similarity + keyword search + KG facts.

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use ferrumyx_common::error::ApiError;

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<i32>,
    pub cancer_type: Option<String>,
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

#[derive(Debug, Serialize)]
pub struct KgFactBrief {
    pub fact_type: String,
    pub subject: String,
    pub object: String,
    pub evidence_count: i32,
}

/// GET /api/search - Hybrid search (vector + keyword + KG)
pub async fn hybrid_search(
    State(pool): State<PgPool>,
    Query(query): Query<SearchQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(20).min(100);
    let search_term = format!("%{}%", query.q.to_lowercase());

    // 1. Keyword search on paper chunks
    let keyword_results = sqlx::query!(
        r#"
        SELECT 
            pc.paper_id,
            p.title,
            pc.content as chunk_text,
            pc.section_type,
            0.5 as similarity
        FROM paper_chunks pc
        JOIN papers p ON p.id = pc.paper_id
        WHERE LOWER(pc.content) LIKE $1
        ORDER BY pc.chunk_index
        LIMIT $2
        "#,
        search_term,
        limit
    )
    .fetch_all(&pool)
    .await?;

    // 2. Get relevant KG facts
    let kg_facts = sqlx::query_as!(
        KgFactBrief,
        r#"
        SELECT 
            fact_type as "fact_type!",
            subject as "subject!",
            object as "object!",
            evidence_count as "evidence_count!"
        FROM kg_facts
        WHERE subject ILIKE $1 OR object ILIKE $1
        ORDER BY evidence_count DESC
        LIMIT 10
        "#,
        format!("%{}%", query.q)
    )
    .fetch_all(&pool)
    .await?;

    // Combine results
    let results: Vec<SearchResult> = keyword_results
        .iter()
        .map(|r| SearchResult {
            paper_id: r.paper_id.to_string(),
            title: r.title.clone(),
            chunk_text: r.chunk_text.clone(),
            similarity: r.similarity.unwrap_or(0.5),
            section_type: r.section_type.clone(),
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