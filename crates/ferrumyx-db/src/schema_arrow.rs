//! Arrow schema and conversion utilities for LanceDB.
//!
//! This module provides the Arrow record batch conversion functions
//! needed to work with LanceDB's API.

use crate::error::{DbError, Result};
use crate::schema::*;
use arrow_array::{Array, FixedSizeListArray, Float32Array, Int64Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use std::sync::Arc;

/// Embedding dimension (BiomedBERT-base outputs 768-dim vectors)
pub const EMBEDDING_DIM: usize = 768;
/// Large Embedding dimension for high-precision mode
pub const EMBEDDING_LARGE_DIM: usize = 1024;

// =============================================================================
// Paper Arrow Conversion
// =============================================================================

pub fn paper_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("doi", DataType::Utf8, true),
        Field::new("pmid", DataType::Utf8, true),
        Field::new("title", DataType::Utf8, false),
        Field::new("abstract_text", DataType::Utf8, true),
        Field::new("full_text", DataType::Utf8, true),
        Field::new("raw_json", DataType::Utf8, true),
        Field::new("source", DataType::Utf8, false),
        Field::new("source_id", DataType::Utf8, true),
        Field::new("published_at", DataType::Utf8, true),
        Field::new("authors", DataType::Utf8, true),
        Field::new("journal", DataType::Utf8, true),
        Field::new("volume", DataType::Utf8, true),
        Field::new("issue", DataType::Utf8, true),
        Field::new("pages", DataType::Utf8, true),
        Field::new("parse_status", DataType::Utf8, false),
        Field::new("open_access", DataType::Boolean, false),
        Field::new("retrieval_tier", DataType::Int32, true),
        Field::new("ingested_at", DataType::Utf8, false),
        Field::new("abstract_simhash", DataType::Int64, true),
        Field::new("published_version_doi", DataType::Utf8, true),
    ]))
}

pub fn paper_to_record(paper: &Paper) -> Result<RecordBatch> {
    let schema = paper_schema();

    let id = StringArray::from(vec![paper.id.to_string()]);
    let doi = StringArray::from(vec![paper.doi.as_deref()]);
    let pmid = StringArray::from(vec![paper.pmid.as_deref()]);
    let title = StringArray::from(vec![paper.title.as_str()]);
    let abstract_text = StringArray::from(vec![paper.abstract_text.as_deref()]);
    let full_text = StringArray::from(vec![paper.full_text.as_deref()]);
    let raw_json = StringArray::from(vec![paper.raw_json.as_deref()]);
    let source = StringArray::from(vec![paper.source.as_str()]);
    let source_id = StringArray::from(vec![paper.source_id.as_deref()]);
    let published_at = StringArray::from(vec![paper.published_at.map(|dt| dt.to_rfc3339())]);
    let authors = StringArray::from(vec![paper.authors.as_deref()]);
    let journal = StringArray::from(vec![paper.journal.as_deref()]);
    let volume = StringArray::from(vec![paper.volume.as_deref()]);
    let issue = StringArray::from(vec![paper.issue.as_deref()]);
    let pages = StringArray::from(vec![paper.pages.as_deref()]);
    let parse_status = StringArray::from(vec![paper.parse_status.as_str()]);
    let open_access = arrow_array::BooleanArray::from(vec![Some(paper.open_access)]);
    let retrieval_tier = arrow_array::Int32Array::from(vec![paper.retrieval_tier]);
    let ingested_at = StringArray::from(vec![paper.ingested_at.to_rfc3339()]);
    let abstract_simhash = Int64Array::from(vec![paper.abstract_simhash]);
    let published_version_doi = StringArray::from(vec![paper.published_version_doi.as_deref()]);

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id) as Arc<dyn Array>,
            Arc::new(doi),
            Arc::new(pmid),
            Arc::new(title),
            Arc::new(abstract_text),
            Arc::new(full_text),
            Arc::new(raw_json),
            Arc::new(source),
            Arc::new(source_id),
            Arc::new(published_at),
            Arc::new(authors),
            Arc::new(journal),
            Arc::new(volume),
            Arc::new(issue),
            Arc::new(pages),
            Arc::new(parse_status),
            Arc::new(open_access),
            Arc::new(retrieval_tier),
            Arc::new(ingested_at),
            Arc::new(abstract_simhash),
            Arc::new(published_version_doi),
        ],
    )
    .map_err(|e| DbError::Arrow(e.to_string()))
}

pub fn record_to_paper(batch: &RecordBatch, row: usize) -> Result<Paper> {
    let get_string = |col: usize| -> String {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        arr.value(row).to_string()
    };

    let get_opt_string = |col: usize| -> Option<String> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row).to_string())
        }
    };

    let get_opt_i64 = |col: usize| -> Option<i64> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    let get_opt_i32 = |col: usize| -> Option<i32> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Int32Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };

    let get_bool = |col: usize| -> bool {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::BooleanArray>()
            .unwrap();
        if arr.is_null(row) {
            false
        } else {
            arr.value(row)
        }
    };

    Ok(Paper {
        id: uuid::Uuid::parse_str(&get_string(0))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        doi: get_opt_string(1),
        pmid: get_opt_string(2),
        title: get_string(3),
        abstract_text: get_opt_string(4),
        full_text: get_opt_string(5),
        raw_json: get_opt_string(6),
        source: get_string(7),
        source_id: get_opt_string(8),
        published_at: get_opt_string(9)
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc)),
        authors: get_opt_string(10),
        journal: get_opt_string(11),
        volume: get_opt_string(12),
        issue: get_opt_string(13),
        pages: get_opt_string(14),
        parse_status: get_string(15),
        open_access: get_bool(16),
        retrieval_tier: get_opt_i32(17),
        ingested_at: chrono::DateTime::parse_from_rfc3339(&get_string(18))
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
        abstract_simhash: get_opt_i64(19),
        published_version_doi: get_opt_string(20),
    })
}

// =============================================================================
// Chunk Arrow Conversion
// =============================================================================

pub fn chunk_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("paper_id", DataType::Utf8, false),
        Field::new("chunk_index", DataType::Int64, false),
        Field::new("token_count", DataType::Int32, false),
        Field::new("content", DataType::Utf8, false),
        Field::new("section", DataType::Utf8, true),
        Field::new("page", DataType::Int64, true),
        Field::new("created_at", DataType::Utf8, false),
        Field::new(
            "embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, false)),
                EMBEDDING_DIM as i32,
            ),
            true,
        ),
        Field::new(
            "embedding_large",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, false)),
                EMBEDDING_LARGE_DIM as i32,
            ),
            true,
        ),
    ]))
}

pub fn chunk_to_record(chunk: &Chunk) -> Result<RecordBatch> {
    let schema = chunk_schema();

    let id = StringArray::from(vec![chunk.id.to_string()]);
    let paper_id = StringArray::from(vec![chunk.paper_id.to_string()]);
    let chunk_index = Int64Array::from(vec![chunk.chunk_index]);
    let token_count = arrow_array::Int32Array::from(vec![chunk.token_count]);
    let content = StringArray::from(vec![chunk.content.as_str()]);
    let section = StringArray::from(vec![chunk.section.as_deref()]);
    let page = Int64Array::from(vec![chunk.page]);
    let created_at = StringArray::from(vec![chunk.created_at.to_rfc3339()]);

    // Handle embedding
    let embedding: Arc<dyn Array> = if let Some(ref emb) = chunk.embedding {
        let values = Float32Array::from(emb.clone());
        let field = Arc::new(Field::new("item", DataType::Float32, false));
        Arc::new(
            FixedSizeListArray::try_new(field, EMBEDDING_DIM as i32, Arc::new(values), None)
                .map_err(|e| DbError::Arrow(e.to_string()))?,
        )
    } else {
        Arc::new(FixedSizeListArray::new_null(
            Arc::new(Field::new("item", DataType::Float32, false)),
            EMBEDDING_DIM as i32,
            1,
        ))
    };

    let embedding_large: Arc<dyn Array> = if let Some(ref emb) = chunk.embedding_large {
        let values = Float32Array::from(emb.clone());
        let field = Arc::new(Field::new("item", DataType::Float32, false));
        Arc::new(
            FixedSizeListArray::try_new(field, EMBEDDING_LARGE_DIM as i32, Arc::new(values), None)
                .map_err(|e| DbError::Arrow(e.to_string()))?,
        )
    } else {
        Arc::new(FixedSizeListArray::new_null(
            Arc::new(Field::new("item", DataType::Float32, false)),
            EMBEDDING_LARGE_DIM as i32,
            1,
        ))
    };

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id) as Arc<dyn Array>,
            Arc::new(paper_id),
            Arc::new(chunk_index),
            Arc::new(token_count),
            Arc::new(content),
            Arc::new(section),
            Arc::new(page),
            Arc::new(created_at),
            embedding,
            embedding_large,
        ],
    )
    .map_err(|e| DbError::Arrow(e.to_string()))
}

pub fn record_to_chunk(batch: &RecordBatch, row: usize) -> Result<Chunk> {
    let get_string = |col: usize| -> String {
        batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .value(row)
            .to_string()
    };

    let get_opt_string = |col: usize| -> Option<String> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row).to_string())
        }
    };

    let get_i64 = |col: usize| -> i64 {
        batch
            .column(col)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .value(row)
    };

    let get_opt_i64 = |col: usize| -> Option<i64> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };

    let get_embedding = |col: usize| -> Option<Vec<f32>> {
        let arr = batch.column(col);
        if arr.is_null(row) {
            return None;
        }
        let list_arr = arr.as_any().downcast_ref::<FixedSizeListArray>().unwrap();
        if list_arr.is_null(row) {
            return None;
        }
        let values = list_arr.value(row);
        let float_arr = values.as_any().downcast_ref::<Float32Array>().unwrap();
        Some(float_arr.values().to_vec())
    };
    let get_i32 = |col: usize| -> i32 {
        batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Int32Array>()
            .unwrap()
            .value(row)
    };

    Ok(Chunk {
        id: uuid::Uuid::parse_str(&get_string(0))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        paper_id: uuid::Uuid::parse_str(&get_string(1))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        chunk_index: get_i64(2),
        token_count: get_i32(3),
        content: get_string(4),
        section: get_opt_string(5),
        page: get_opt_i64(6),
        created_at: chrono::DateTime::parse_from_rfc3339(&get_string(7))
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
        embedding: get_embedding(8),
        embedding_large: get_embedding(9),
    })
}

// =============================================================================
// Entity Arrow Conversion
// =============================================================================

pub fn entity_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("external_id", DataType::Utf8, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("canonical_name", DataType::Utf8, true),
        Field::new("entity_type", DataType::Utf8, false),
        Field::new("synonyms", DataType::Utf8, true),
        Field::new("description", DataType::Utf8, true),
        Field::new("source_db", DataType::Utf8, false),
        Field::new("metadata", DataType::Utf8, true),
        Field::new("created_at", DataType::Utf8, false),
        Field::new("updated_at", DataType::Utf8, false),
    ]))
}

pub fn entity_to_record(entity: &Entity) -> Result<RecordBatch> {
    let schema = entity_schema();

    let id = StringArray::from(vec![entity.id.to_string()]);
    let external_id = StringArray::from(vec![entity.external_id.as_str()]);
    let name = StringArray::from(vec![entity.name.as_str()]);
    let canonical_name = StringArray::from(vec![entity.canonical_name.as_deref()]);
    let entity_type = StringArray::from(vec![entity.entity_type.as_str()]);
    let synonyms = StringArray::from(vec![entity.synonyms.as_deref()]);
    let description = StringArray::from(vec![entity.description.as_deref()]);
    let source_db = StringArray::from(vec![entity.source_db.as_str()]);
    let metadata = StringArray::from(vec![entity.metadata.as_deref()]);
    let created_at = StringArray::from(vec![entity.created_at.to_rfc3339()]);
    let updated_at = StringArray::from(vec![entity.updated_at.to_rfc3339()]);

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id) as Arc<dyn Array>,
            Arc::new(external_id),
            Arc::new(name),
            Arc::new(canonical_name),
            Arc::new(entity_type),
            Arc::new(synonyms),
            Arc::new(description),
            Arc::new(source_db),
            Arc::new(metadata),
            Arc::new(created_at),
            Arc::new(updated_at),
        ],
    )
    .map_err(|e| DbError::Arrow(e.to_string()))
}

pub fn record_to_entity(batch: &RecordBatch, row: usize) -> Result<Entity> {
    let get_string = |col: usize| -> String {
        batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .value(row)
            .to_string()
    };

    let get_opt_string = |col: usize| -> Option<String> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row).to_string())
        }
    };

    Ok(Entity {
        id: uuid::Uuid::parse_str(&get_string(0))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        external_id: get_string(1),
        name: get_string(2),
        canonical_name: get_opt_string(3),
        entity_type: get_string(4),
        synonyms: get_opt_string(5),
        description: get_opt_string(6),
        source_db: get_string(7),
        metadata: get_opt_string(8),
        created_at: chrono::DateTime::parse_from_rfc3339(&get_string(9))
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
        updated_at: chrono::DateTime::parse_from_rfc3339(&get_string(10))
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
    })
}

// =============================================================================
// KgFact Arrow Conversion
// =============================================================================

pub fn kg_fact_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("paper_id", DataType::Utf8, false),
        Field::new("subject_id", DataType::Utf8, false),
        Field::new("subject_name", DataType::Utf8, false),
        Field::new("predicate", DataType::Utf8, false),
        Field::new("object_id", DataType::Utf8, false),
        Field::new("object_name", DataType::Utf8, false),
        Field::new("confidence", DataType::Float32, false),
        Field::new("evidence", DataType::Utf8, true),
        Field::new("evidence_type", DataType::Utf8, false),
        Field::new("study_type", DataType::Utf8, true),
        Field::new("sample_size", DataType::Int32, true),
        Field::new("valid_from", DataType::Utf8, false),
        Field::new("valid_until", DataType::Utf8, true),
        Field::new("created_at", DataType::Utf8, false),
    ]))
}

pub fn kg_fact_to_record(fact: &KgFact) -> Result<RecordBatch> {
    let schema = kg_fact_schema();

    let id = StringArray::from(vec![fact.id.to_string()]);
    let paper_id = StringArray::from(vec![fact.paper_id.to_string()]);
    let subject_id = StringArray::from(vec![fact.subject_id.to_string()]);
    let subject_name = StringArray::from(vec![fact.subject_name.as_str()]);
    let predicate = StringArray::from(vec![fact.predicate.as_str()]);
    let object_id = StringArray::from(vec![fact.object_id.to_string()]);
    let object_name = StringArray::from(vec![fact.object_name.as_str()]);
    let confidence = Float32Array::from(vec![fact.confidence]);
    let evidence = StringArray::from(vec![fact.evidence.as_deref()]);
    let evidence_type = StringArray::from(vec![fact.evidence_type.as_str()]);
    let study_type = StringArray::from(vec![fact.study_type.as_deref()]);
    let sample_size = arrow_array::Int32Array::from(vec![fact.sample_size]);
    let valid_from = StringArray::from(vec![fact.valid_from.to_rfc3339()]);
    let valid_until = StringArray::from(vec![fact.valid_until.map(|dt| dt.to_rfc3339())]);
    let created_at = StringArray::from(vec![fact.created_at.to_rfc3339()]);

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id) as Arc<dyn Array>,
            Arc::new(paper_id),
            Arc::new(subject_id),
            Arc::new(subject_name),
            Arc::new(predicate),
            Arc::new(object_id),
            Arc::new(object_name),
            Arc::new(confidence),
            Arc::new(evidence),
            Arc::new(evidence_type),
            Arc::new(study_type),
            Arc::new(sample_size),
            Arc::new(valid_from),
            Arc::new(valid_until),
            Arc::new(created_at),
        ],
    )
    .map_err(|e| DbError::Arrow(e.to_string()))
}

pub fn record_to_kg_fact(batch: &RecordBatch, row: usize) -> Result<KgFact> {
    let get_string = |col: usize| -> String {
        batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .value(row)
            .to_string()
    };

    let get_opt_string = |col: usize| -> Option<String> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row).to_string())
        }
    };

    let get_f32 = |col: usize| -> f32 {
        batch
            .column(col)
            .as_any()
            .downcast_ref::<Float32Array>()
            .unwrap()
            .value(row)
    };

    let get_opt_i32 = |col: usize| -> Option<i32> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Int32Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };

    Ok(KgFact {
        id: uuid::Uuid::parse_str(&get_string(0))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        paper_id: uuid::Uuid::parse_str(&get_string(1))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        subject_id: uuid::Uuid::parse_str(&get_string(2))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        subject_name: get_string(3),
        predicate: get_string(4),
        object_id: uuid::Uuid::parse_str(&get_string(5))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        object_name: get_string(6),
        confidence: get_f32(7),
        evidence: get_opt_string(8),
        evidence_type: get_string(9),
        study_type: get_opt_string(10),
        sample_size: get_opt_i32(11),
        valid_from: chrono::DateTime::parse_from_rfc3339(&get_string(12))
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
        valid_until: get_opt_string(13)
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc)),
        created_at: chrono::DateTime::parse_from_rfc3339(&get_string(14))
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
    })
}

// =============================================================================
// EntityMention Arrow Conversion
// =============================================================================

pub fn entity_mention_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("entity_id", DataType::Utf8, false),
        Field::new("chunk_id", DataType::Utf8, false),
        Field::new("paper_id", DataType::Utf8, false),
        Field::new("start_offset", DataType::Int64, false),
        Field::new("end_offset", DataType::Int64, false),
        Field::new("text", DataType::Utf8, false),
        Field::new("confidence", DataType::Float32, true),
        Field::new("context", DataType::Utf8, true),
        Field::new("created_at", DataType::Utf8, false),
    ]))
}

pub fn entity_mention_to_record(mention: &EntityMention) -> Result<RecordBatch> {
    let schema = entity_mention_schema();

    let id = StringArray::from(vec![mention.id.to_string()]);
    let entity_id = StringArray::from(vec![mention.entity_id.to_string()]);
    let chunk_id = StringArray::from(vec![mention.chunk_id.to_string()]);
    let paper_id = StringArray::from(vec![mention.paper_id.to_string()]);
    let start_offset = Int64Array::from(vec![mention.start_offset]);
    let end_offset = Int64Array::from(vec![mention.end_offset]);
    let text = StringArray::from(vec![mention.text.as_str()]);
    let confidence = Float32Array::from(vec![mention.confidence]);
    let context = StringArray::from(vec![mention.context.as_deref()]);
    let created_at = StringArray::from(vec![mention.created_at.to_rfc3339()]);

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id) as Arc<dyn Array>,
            Arc::new(entity_id),
            Arc::new(chunk_id),
            Arc::new(paper_id),
            Arc::new(start_offset),
            Arc::new(end_offset),
            Arc::new(text),
            Arc::new(confidence),
            Arc::new(context),
            Arc::new(created_at),
        ],
    )
    .map_err(|e| DbError::Arrow(e.to_string()))
}

pub fn record_to_entity_mention(batch: &RecordBatch, row: usize) -> Result<EntityMention> {
    let get_string = |col: usize| -> String {
        batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .value(row)
            .to_string()
    };

    let get_opt_string = |col: usize| -> Option<String> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row).to_string())
        }
    };

    let get_i64 = |col: usize| -> i64 {
        batch
            .column(col)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap()
            .value(row)
    };

    let get_opt_f32 = |col: usize| -> Option<f32> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<Float32Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };

    Ok(EntityMention {
        id: uuid::Uuid::parse_str(&get_string(0))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        entity_id: uuid::Uuid::parse_str(&get_string(1))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        chunk_id: uuid::Uuid::parse_str(&get_string(2))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        paper_id: uuid::Uuid::parse_str(&get_string(3))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        start_offset: get_i64(4),
        end_offset: get_i64(5),
        text: get_string(6),
        confidence: get_opt_f32(7),
        context: get_opt_string(8),
        created_at: chrono::DateTime::parse_from_rfc3339(&get_string(9))
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
    })
}

// =============================================================================
// KgConflict Arrow Conversion
// =============================================================================

pub fn kg_conflict_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("fact_a_id", DataType::Utf8, false),
        Field::new("fact_b_id", DataType::Utf8, false),
        Field::new("conflict_type", DataType::Utf8, false),
        Field::new("net_confidence", DataType::Float32, false),
        Field::new("resolution", DataType::Utf8, false),
        Field::new("detected_at", DataType::Utf8, false),
    ]))
}

pub fn kg_conflict_to_record(conflict: &KgConflict) -> Result<RecordBatch> {
    let schema = kg_conflict_schema();

    let id = StringArray::from(vec![conflict.id.to_string()]);
    let fact_a_id = StringArray::from(vec![conflict.fact_a_id.to_string()]);
    let fact_b_id = StringArray::from(vec![conflict.fact_b_id.to_string()]);
    let conflict_type = StringArray::from(vec![conflict.conflict_type.as_str()]);
    let net_confidence = Float32Array::from(vec![conflict.net_confidence]);
    let resolution = StringArray::from(vec![conflict.resolution.as_str()]);
    let detected_at = StringArray::from(vec![conflict.detected_at.to_rfc3339()]);

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id) as Arc<dyn Array>,
            Arc::new(fact_a_id),
            Arc::new(fact_b_id),
            Arc::new(conflict_type),
            Arc::new(net_confidence),
            Arc::new(resolution),
            Arc::new(detected_at),
        ],
    )
    .map_err(|e| DbError::Arrow(e.to_string()))
}

pub fn record_to_kg_conflict(batch: &RecordBatch, row: usize) -> Result<KgConflict> {
    let get_string = |col: usize| -> String {
        batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .value(row)
            .to_string()
    };

    let get_f32 = |col: usize| -> f32 {
        batch
            .column(col)
            .as_any()
            .downcast_ref::<Float32Array>()
            .unwrap()
            .value(row)
    };

    Ok(KgConflict {
        id: uuid::Uuid::parse_str(&get_string(0))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        fact_a_id: uuid::Uuid::parse_str(&get_string(1))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        fact_b_id: uuid::Uuid::parse_str(&get_string(2))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        conflict_type: get_string(3),
        net_confidence: get_f32(4),
        resolution: get_string(5),
        detected_at: chrono::DateTime::parse_from_rfc3339(&get_string(6))
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
    })
}
// =============================================================================
// Specific Entity Type Conversions (Phase 3)
// =============================================================================

pub fn ent_gene_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("hgnc_id", DataType::Utf8, true),
        Field::new("symbol", DataType::Utf8, false),
        Field::new("name", DataType::Utf8, true),
        Field::new("uniprot_id", DataType::Utf8, true),
        Field::new("ensembl_id", DataType::Utf8, true),
        Field::new("entrez_id", DataType::Utf8, true),
        Field::new("gene_biotype", DataType::Utf8, true),
        Field::new("chromosome", DataType::Utf8, true),
        Field::new("strand", DataType::Int16, true),
        Field::new("aliases", DataType::Utf8, true),
        Field::new("oncogene_flag", DataType::Boolean, false),
        Field::new("tsg_flag", DataType::Boolean, false),
        Field::new("created_at", DataType::Utf8, false),
    ]))
}

pub fn ent_gene_to_record(item: &EntGene) -> Result<RecordBatch> {
    let schema = ent_gene_schema();
    let id = StringArray::from(vec![item.id.to_string()]);
    let hgnc_id = StringArray::from(vec![item.hgnc_id.as_deref()]);
    let symbol = StringArray::from(vec![item.symbol.as_str()]);
    let name = StringArray::from(vec![item.name.as_deref()]);
    let uniprot_id = StringArray::from(vec![item.uniprot_id.as_deref()]);
    let ensembl_id = StringArray::from(vec![item.ensembl_id.as_deref()]);
    let entrez_id = StringArray::from(vec![item.entrez_id.as_deref()]);
    let gene_biotype = StringArray::from(vec![item.gene_biotype.as_deref()]);
    let chromosome = StringArray::from(vec![item.chromosome.as_deref()]);
    let strand = arrow_array::Int16Array::from(vec![item.strand]);
    let aliases = StringArray::from(vec![item
        .aliases
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_default())]);
    let oncogene_flag = arrow_array::BooleanArray::from(vec![Some(item.oncogene_flag)]);
    let tsg_flag = arrow_array::BooleanArray::from(vec![Some(item.tsg_flag)]);
    let created_at = StringArray::from(vec![item.created_at.to_rfc3339()]);
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id) as Arc<dyn Array>,
            Arc::new(hgnc_id),
            Arc::new(symbol),
            Arc::new(name),
            Arc::new(uniprot_id),
            Arc::new(ensembl_id),
            Arc::new(entrez_id),
            Arc::new(gene_biotype),
            Arc::new(chromosome),
            Arc::new(strand),
            Arc::new(aliases),
            Arc::new(oncogene_flag),
            Arc::new(tsg_flag),
            Arc::new(created_at),
        ],
    )
    .map_err(|e| DbError::Arrow(e.to_string()))
}

pub fn record_to_ent_gene(batch: &RecordBatch, row: usize) -> Result<EntGene> {
    let get_string = |col: usize| -> String {
        batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .value(row)
            .to_string()
    };
    let get_opt_string = |col: usize| -> Option<String> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row).to_string())
        }
    };
    let get_bool = |col: usize| -> bool {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::BooleanArray>()
            .unwrap();
        if arr.is_null(row) {
            false
        } else {
            arr.value(row)
        }
    };
    let get_opt_i16 = |col: usize| -> Option<i16> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Int16Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    let get_opt_i32 = |col: usize| -> Option<i32> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Int32Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    let get_opt_f32 = |col: usize| -> Option<f32> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Float32Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    Ok(EntGene {
        id: uuid::Uuid::parse_str(&get_string(0))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        hgnc_id: get_opt_string(1),
        symbol: get_string(2),
        name: get_opt_string(3),
        uniprot_id: get_opt_string(4),
        ensembl_id: get_opt_string(5),
        entrez_id: get_opt_string(6),
        gene_biotype: get_opt_string(7),
        chromosome: get_opt_string(8),
        strand: get_opt_i16(9),
        aliases: get_opt_string(10).and_then(|s| serde_json::from_str(&s).ok()),
        oncogene_flag: get_bool(11),
        tsg_flag: get_bool(12),
        created_at: chrono::DateTime::parse_from_rfc3339(&get_string(13))
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
    })
}

pub fn ent_mutation_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("gene_id", DataType::Utf8, false),
        Field::new("hgvs_p", DataType::Utf8, true),
        Field::new("hgvs_c", DataType::Utf8, true),
        Field::new("rs_id", DataType::Utf8, true),
        Field::new("aa_ref", DataType::Utf8, true),
        Field::new("aa_alt", DataType::Utf8, true),
        Field::new("aa_position", DataType::Int32, true),
        Field::new("oncogenicity", DataType::Utf8, true),
        Field::new("hotspot_flag", DataType::Boolean, false),
        Field::new("vaf_context", DataType::Utf8, true),
        Field::new("created_at", DataType::Utf8, false),
    ]))
}

pub fn ent_mutation_to_record(item: &EntMutation) -> Result<RecordBatch> {
    let schema = ent_mutation_schema();
    let id = StringArray::from(vec![item.id.to_string()]);
    let gene_id = StringArray::from(vec![item.gene_id.to_string()]);
    let hgvs_p = StringArray::from(vec![item.hgvs_p.as_deref()]);
    let hgvs_c = StringArray::from(vec![item.hgvs_c.as_deref()]);
    let rs_id = StringArray::from(vec![item.rs_id.as_deref()]);
    let aa_ref = StringArray::from(vec![item.aa_ref.as_deref()]);
    let aa_alt = StringArray::from(vec![item.aa_alt.as_deref()]);
    let aa_position = arrow_array::Int32Array::from(vec![item.aa_position]);
    let oncogenicity = StringArray::from(vec![item.oncogenicity.as_deref()]);
    let hotspot_flag = arrow_array::BooleanArray::from(vec![Some(item.hotspot_flag)]);
    let vaf_context = StringArray::from(vec![item.vaf_context.as_deref()]);
    let created_at = StringArray::from(vec![item.created_at.to_rfc3339()]);
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id) as Arc<dyn Array>,
            Arc::new(gene_id),
            Arc::new(hgvs_p),
            Arc::new(hgvs_c),
            Arc::new(rs_id),
            Arc::new(aa_ref),
            Arc::new(aa_alt),
            Arc::new(aa_position),
            Arc::new(oncogenicity),
            Arc::new(hotspot_flag),
            Arc::new(vaf_context),
            Arc::new(created_at),
        ],
    )
    .map_err(|e| DbError::Arrow(e.to_string()))
}

pub fn record_to_ent_mutation(batch: &RecordBatch, row: usize) -> Result<EntMutation> {
    let get_string = |col: usize| -> String {
        batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .value(row)
            .to_string()
    };
    let get_opt_string = |col: usize| -> Option<String> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row).to_string())
        }
    };
    let get_bool = |col: usize| -> bool {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::BooleanArray>()
            .unwrap();
        if arr.is_null(row) {
            false
        } else {
            arr.value(row)
        }
    };
    let get_opt_i16 = |col: usize| -> Option<i16> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Int16Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    let get_opt_i32 = |col: usize| -> Option<i32> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Int32Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    let get_opt_f32 = |col: usize| -> Option<f32> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Float32Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    Ok(EntMutation {
        id: uuid::Uuid::parse_str(&get_string(0))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        gene_id: uuid::Uuid::parse_str(&get_string(1))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        hgvs_p: get_opt_string(2),
        hgvs_c: get_opt_string(3),
        rs_id: get_opt_string(4),
        aa_ref: get_opt_string(5),
        aa_alt: get_opt_string(6),
        aa_position: get_opt_i32(7),
        oncogenicity: get_opt_string(8),
        hotspot_flag: get_bool(9),
        vaf_context: get_opt_string(10),
        created_at: chrono::DateTime::parse_from_rfc3339(&get_string(11))
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
    })
}

pub fn ent_cancer_type_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("oncotree_code", DataType::Utf8, true),
        Field::new("oncotree_name", DataType::Utf8, true),
        Field::new("icd_o3_code", DataType::Utf8, true),
        Field::new("tissue", DataType::Utf8, true),
        Field::new("parent_code", DataType::Utf8, true),
        Field::new("level", DataType::Int32, true),
        Field::new("created_at", DataType::Utf8, false),
    ]))
}

pub fn ent_cancer_type_to_record(item: &EntCancerType) -> Result<RecordBatch> {
    let schema = ent_cancer_type_schema();
    let id = StringArray::from(vec![item.id.to_string()]);
    let oncotree_code = StringArray::from(vec![item.oncotree_code.as_deref()]);
    let oncotree_name = StringArray::from(vec![item.oncotree_name.as_deref()]);
    let icd_o3_code = StringArray::from(vec![item.icd_o3_code.as_deref()]);
    let tissue = StringArray::from(vec![item.tissue.as_deref()]);
    let parent_code = StringArray::from(vec![item.parent_code.as_deref()]);
    let level = arrow_array::Int32Array::from(vec![item.level]);
    let created_at = StringArray::from(vec![item.created_at.to_rfc3339()]);
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id) as Arc<dyn Array>,
            Arc::new(oncotree_code),
            Arc::new(oncotree_name),
            Arc::new(icd_o3_code),
            Arc::new(tissue),
            Arc::new(parent_code),
            Arc::new(level),
            Arc::new(created_at),
        ],
    )
    .map_err(|e| DbError::Arrow(e.to_string()))
}

pub fn record_to_ent_cancer_type(batch: &RecordBatch, row: usize) -> Result<EntCancerType> {
    let get_string = |col: usize| -> String {
        batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .value(row)
            .to_string()
    };
    let get_opt_string = |col: usize| -> Option<String> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row).to_string())
        }
    };
    let get_bool = |col: usize| -> bool {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::BooleanArray>()
            .unwrap();
        if arr.is_null(row) {
            false
        } else {
            arr.value(row)
        }
    };
    let get_opt_i16 = |col: usize| -> Option<i16> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Int16Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    let get_opt_i32 = |col: usize| -> Option<i32> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Int32Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    let get_opt_f32 = |col: usize| -> Option<f32> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Float32Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    Ok(EntCancerType {
        id: uuid::Uuid::parse_str(&get_string(0))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        oncotree_code: get_opt_string(1),
        oncotree_name: get_opt_string(2),
        icd_o3_code: get_opt_string(3),
        tissue: get_opt_string(4),
        parent_code: get_opt_string(5),
        level: get_opt_i32(6),
        created_at: chrono::DateTime::parse_from_rfc3339(&get_string(7))
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
    })
}

pub fn ent_pathway_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("kegg_id", DataType::Utf8, true),
        Field::new("reactome_id", DataType::Utf8, true),
        Field::new("go_term", DataType::Utf8, true),
        Field::new("name", DataType::Utf8, false),
        Field::new("gene_members", DataType::Utf8, true),
        Field::new("source", DataType::Utf8, true),
        Field::new("created_at", DataType::Utf8, false),
    ]))
}

pub fn ent_pathway_to_record(item: &EntPathway) -> Result<RecordBatch> {
    let schema = ent_pathway_schema();
    let id = StringArray::from(vec![item.id.to_string()]);
    let kegg_id = StringArray::from(vec![item.kegg_id.as_deref()]);
    let reactome_id = StringArray::from(vec![item.reactome_id.as_deref()]);
    let go_term = StringArray::from(vec![item.go_term.as_deref()]);
    let name = StringArray::from(vec![item.name.as_str()]);
    let gene_members = StringArray::from(vec![item
        .gene_members
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_default())]);
    let source = StringArray::from(vec![item.source.as_deref()]);
    let created_at = StringArray::from(vec![item.created_at.to_rfc3339()]);
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id) as Arc<dyn Array>,
            Arc::new(kegg_id),
            Arc::new(reactome_id),
            Arc::new(go_term),
            Arc::new(name),
            Arc::new(gene_members),
            Arc::new(source),
            Arc::new(created_at),
        ],
    )
    .map_err(|e| DbError::Arrow(e.to_string()))
}

pub fn record_to_ent_pathway(batch: &RecordBatch, row: usize) -> Result<EntPathway> {
    let get_string = |col: usize| -> String {
        batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .value(row)
            .to_string()
    };
    let get_opt_string = |col: usize| -> Option<String> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row).to_string())
        }
    };
    let get_bool = |col: usize| -> bool {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::BooleanArray>()
            .unwrap();
        if arr.is_null(row) {
            false
        } else {
            arr.value(row)
        }
    };
    let get_opt_i16 = |col: usize| -> Option<i16> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Int16Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    let get_opt_i32 = |col: usize| -> Option<i32> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Int32Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    let get_opt_f32 = |col: usize| -> Option<f32> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Float32Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    Ok(EntPathway {
        id: uuid::Uuid::parse_str(&get_string(0))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        kegg_id: get_opt_string(1),
        reactome_id: get_opt_string(2),
        go_term: get_opt_string(3),
        name: get_string(4),
        gene_members: get_opt_string(5).and_then(|s| serde_json::from_str(&s).ok()),
        source: get_opt_string(6),
        created_at: chrono::DateTime::parse_from_rfc3339(&get_string(7))
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
    })
}

pub fn ent_clinical_evidence_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("nct_id", DataType::Utf8, true),
        Field::new("pmid", DataType::Utf8, true),
        Field::new("doi", DataType::Utf8, true),
        Field::new("phase", DataType::Utf8, true),
        Field::new("intervention", DataType::Utf8, true),
        Field::new("target_gene_id", DataType::Utf8, false),
        Field::new("cancer_id", DataType::Utf8, false),
        Field::new("primary_endpoint", DataType::Utf8, true),
        Field::new("outcome", DataType::Utf8, true),
        Field::new("evidence_grade", DataType::Utf8, true),
        Field::new("created_at", DataType::Utf8, false),
    ]))
}

pub fn ent_clinical_evidence_to_record(item: &EntClinicalEvidence) -> Result<RecordBatch> {
    let schema = ent_clinical_evidence_schema();
    let id = StringArray::from(vec![item.id.to_string()]);
    let nct_id = StringArray::from(vec![item.nct_id.as_deref()]);
    let pmid = StringArray::from(vec![item.pmid.as_deref()]);
    let doi = StringArray::from(vec![item.doi.as_deref()]);
    let phase = StringArray::from(vec![item.phase.as_deref()]);
    let intervention = StringArray::from(vec![item.intervention.as_deref()]);
    let target_gene_id = StringArray::from(vec![item.target_gene_id.to_string()]);
    let cancer_id = StringArray::from(vec![item.cancer_id.to_string()]);
    let primary_endpoint = StringArray::from(vec![item.primary_endpoint.as_deref()]);
    let outcome = StringArray::from(vec![item.outcome.as_deref()]);
    let evidence_grade = StringArray::from(vec![item.evidence_grade.as_deref()]);
    let created_at = StringArray::from(vec![item.created_at.to_rfc3339()]);
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id) as Arc<dyn Array>,
            Arc::new(nct_id),
            Arc::new(pmid),
            Arc::new(doi),
            Arc::new(phase),
            Arc::new(intervention),
            Arc::new(target_gene_id),
            Arc::new(cancer_id),
            Arc::new(primary_endpoint),
            Arc::new(outcome),
            Arc::new(evidence_grade),
            Arc::new(created_at),
        ],
    )
    .map_err(|e| DbError::Arrow(e.to_string()))
}

pub fn record_to_ent_clinical_evidence(
    batch: &RecordBatch,
    row: usize,
) -> Result<EntClinicalEvidence> {
    let get_string = |col: usize| -> String {
        batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .value(row)
            .to_string()
    };
    let get_opt_string = |col: usize| -> Option<String> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row).to_string())
        }
    };
    let get_bool = |col: usize| -> bool {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::BooleanArray>()
            .unwrap();
        if arr.is_null(row) {
            false
        } else {
            arr.value(row)
        }
    };
    let get_opt_i16 = |col: usize| -> Option<i16> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Int16Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    let get_opt_i32 = |col: usize| -> Option<i32> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Int32Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    let get_opt_f32 = |col: usize| -> Option<f32> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Float32Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    Ok(EntClinicalEvidence {
        id: uuid::Uuid::parse_str(&get_string(0))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        nct_id: get_opt_string(1),
        pmid: get_opt_string(2),
        doi: get_opt_string(3),
        phase: get_opt_string(4),
        intervention: get_opt_string(5),
        target_gene_id: uuid::Uuid::parse_str(&get_string(6))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        cancer_id: uuid::Uuid::parse_str(&get_string(7))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        primary_endpoint: get_opt_string(8),
        outcome: get_opt_string(9),
        evidence_grade: get_opt_string(10),
        created_at: chrono::DateTime::parse_from_rfc3339(&get_string(11))
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
    })
}

pub fn ent_compound_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("chembl_id", DataType::Utf8, true),
        Field::new("name", DataType::Utf8, true),
        Field::new("smiles", DataType::Utf8, true),
        Field::new("inchi_key", DataType::Utf8, true),
        Field::new("moa", DataType::Utf8, true),
        Field::new("patent_status", DataType::Utf8, true),
        Field::new("max_phase", DataType::Int32, true),
        Field::new("target_gene_ids", DataType::Utf8, true),
        Field::new("created_at", DataType::Utf8, false),
    ]))
}

pub fn ent_compound_to_record(item: &EntCompound) -> Result<RecordBatch> {
    let schema = ent_compound_schema();
    let id = StringArray::from(vec![item.id.to_string()]);
    let chembl_id = StringArray::from(vec![item.chembl_id.as_deref()]);
    let name = StringArray::from(vec![item.name.as_deref()]);
    let smiles = StringArray::from(vec![item.smiles.as_deref()]);
    let inchi_key = StringArray::from(vec![item.inchi_key.as_deref()]);
    let moa = StringArray::from(vec![item.moa.as_deref()]);
    let patent_status = StringArray::from(vec![item.patent_status.as_deref()]);
    let max_phase = arrow_array::Int32Array::from(vec![item.max_phase]);
    let target_gene_ids = StringArray::from(vec![item
        .target_gene_ids
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_default())]);
    let created_at = StringArray::from(vec![item.created_at.to_rfc3339()]);
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id) as Arc<dyn Array>,
            Arc::new(chembl_id),
            Arc::new(name),
            Arc::new(smiles),
            Arc::new(inchi_key),
            Arc::new(moa),
            Arc::new(patent_status),
            Arc::new(max_phase),
            Arc::new(target_gene_ids),
            Arc::new(created_at),
        ],
    )
    .map_err(|e| DbError::Arrow(e.to_string()))
}

pub fn record_to_ent_compound(batch: &RecordBatch, row: usize) -> Result<EntCompound> {
    let get_string = |col: usize| -> String {
        batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .value(row)
            .to_string()
    };
    let get_opt_string = |col: usize| -> Option<String> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row).to_string())
        }
    };
    let get_bool = |col: usize| -> bool {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::BooleanArray>()
            .unwrap();
        if arr.is_null(row) {
            false
        } else {
            arr.value(row)
        }
    };
    let get_opt_i16 = |col: usize| -> Option<i16> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Int16Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    let get_opt_i32 = |col: usize| -> Option<i32> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Int32Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    let get_opt_f32 = |col: usize| -> Option<f32> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Float32Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    Ok(EntCompound {
        id: uuid::Uuid::parse_str(&get_string(0))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        chembl_id: get_opt_string(1),
        name: get_opt_string(2),
        smiles: get_opt_string(3),
        inchi_key: get_opt_string(4),
        moa: get_opt_string(5),
        patent_status: get_opt_string(6),
        max_phase: get_opt_i32(7),
        target_gene_ids: get_opt_string(8).and_then(|s| serde_json::from_str(&s).ok()),
        created_at: chrono::DateTime::parse_from_rfc3339(&get_string(9))
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
    })
}

pub fn ent_structure_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("gene_id", DataType::Utf8, false),
        Field::new("pdb_ids", DataType::Utf8, true),
        Field::new("best_resolution", DataType::Float32, true),
        Field::new("exp_method", DataType::Utf8, true),
        Field::new("af_accession", DataType::Utf8, true),
        Field::new("af_plddt_mean", DataType::Float32, true),
        Field::new("af_plddt_active", DataType::Float32, true),
        Field::new("has_pdb", DataType::Boolean, false),
        Field::new("has_alphafold", DataType::Boolean, false),
        Field::new("updated_at", DataType::Utf8, false),
    ]))
}

pub fn ent_structure_to_record(item: &EntStructure) -> Result<RecordBatch> {
    let schema = ent_structure_schema();
    let id = StringArray::from(vec![item.id.to_string()]);
    let gene_id = StringArray::from(vec![item.gene_id.to_string()]);
    let pdb_ids = StringArray::from(vec![item
        .pdb_ids
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_default())]);
    let best_resolution = arrow_array::Float32Array::from(vec![item.best_resolution]);
    let exp_method = StringArray::from(vec![item.exp_method.as_deref()]);
    let af_accession = StringArray::from(vec![item.af_accession.as_deref()]);
    let af_plddt_mean = arrow_array::Float32Array::from(vec![item.af_plddt_mean]);
    let af_plddt_active = arrow_array::Float32Array::from(vec![item.af_plddt_active]);
    let has_pdb = arrow_array::BooleanArray::from(vec![Some(item.has_pdb)]);
    let has_alphafold = arrow_array::BooleanArray::from(vec![Some(item.has_alphafold)]);
    let updated_at = StringArray::from(vec![item.updated_at.to_rfc3339()]);
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id) as Arc<dyn Array>,
            Arc::new(gene_id),
            Arc::new(pdb_ids),
            Arc::new(best_resolution),
            Arc::new(exp_method),
            Arc::new(af_accession),
            Arc::new(af_plddt_mean),
            Arc::new(af_plddt_active),
            Arc::new(has_pdb),
            Arc::new(has_alphafold),
            Arc::new(updated_at),
        ],
    )
    .map_err(|e| DbError::Arrow(e.to_string()))
}

pub fn record_to_ent_structure(batch: &RecordBatch, row: usize) -> Result<EntStructure> {
    let get_string = |col: usize| -> String {
        batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .value(row)
            .to_string()
    };
    let get_opt_string = |col: usize| -> Option<String> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row).to_string())
        }
    };
    let get_bool = |col: usize| -> bool {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::BooleanArray>()
            .unwrap();
        if arr.is_null(row) {
            false
        } else {
            arr.value(row)
        }
    };
    let get_opt_i16 = |col: usize| -> Option<i16> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Int16Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    let get_opt_i32 = |col: usize| -> Option<i32> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Int32Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    let get_opt_f32 = |col: usize| -> Option<f32> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Float32Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    Ok(EntStructure {
        id: uuid::Uuid::parse_str(&get_string(0))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        gene_id: uuid::Uuid::parse_str(&get_string(1))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        pdb_ids: get_opt_string(2).and_then(|s| serde_json::from_str(&s).ok()),
        best_resolution: get_opt_f32(3),
        exp_method: get_opt_string(4),
        af_accession: get_opt_string(5),
        af_plddt_mean: get_opt_f32(6),
        af_plddt_active: get_opt_f32(7),
        has_pdb: get_bool(8),
        has_alphafold: get_bool(9),
        updated_at: chrono::DateTime::parse_from_rfc3339(&get_string(10))
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
    })
}

pub fn ent_druggability_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("structure_id", DataType::Utf8, false),
        Field::new("fpocket_score", DataType::Float32, true),
        Field::new("fpocket_volume", DataType::Float32, true),
        Field::new("fpocket_pocket_count", DataType::Int32, true),
        Field::new("dogsitescorer", DataType::Float32, true),
        Field::new("overall_assessment", DataType::Utf8, true),
        Field::new("assessed_at", DataType::Utf8, false),
    ]))
}

pub fn ent_druggability_to_record(item: &EntDruggability) -> Result<RecordBatch> {
    let schema = ent_druggability_schema();
    let id = StringArray::from(vec![item.id.to_string()]);
    let structure_id = StringArray::from(vec![item.structure_id.to_string()]);
    let fpocket_score = arrow_array::Float32Array::from(vec![item.fpocket_score]);
    let fpocket_volume = arrow_array::Float32Array::from(vec![item.fpocket_volume]);
    let fpocket_pocket_count = arrow_array::Int32Array::from(vec![item.fpocket_pocket_count]);
    let dogsitescorer = arrow_array::Float32Array::from(vec![item.dogsitescorer]);
    let overall_assessment = StringArray::from(vec![item.overall_assessment.as_deref()]);
    let assessed_at = StringArray::from(vec![item.assessed_at.to_rfc3339()]);
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id) as Arc<dyn Array>,
            Arc::new(structure_id),
            Arc::new(fpocket_score),
            Arc::new(fpocket_volume),
            Arc::new(fpocket_pocket_count),
            Arc::new(dogsitescorer),
            Arc::new(overall_assessment),
            Arc::new(assessed_at),
        ],
    )
    .map_err(|e| DbError::Arrow(e.to_string()))
}

pub fn record_to_ent_druggability(batch: &RecordBatch, row: usize) -> Result<EntDruggability> {
    let get_string = |col: usize| -> String {
        batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .value(row)
            .to_string()
    };
    let get_opt_string = |col: usize| -> Option<String> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row).to_string())
        }
    };
    let get_bool = |col: usize| -> bool {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::BooleanArray>()
            .unwrap();
        if arr.is_null(row) {
            false
        } else {
            arr.value(row)
        }
    };
    let get_opt_i16 = |col: usize| -> Option<i16> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Int16Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    let get_opt_i32 = |col: usize| -> Option<i32> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Int32Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    let get_opt_f32 = |col: usize| -> Option<f32> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Float32Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    Ok(EntDruggability {
        id: uuid::Uuid::parse_str(&get_string(0))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        structure_id: uuid::Uuid::parse_str(&get_string(1))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        fpocket_score: get_opt_f32(2),
        fpocket_volume: get_opt_f32(3),
        fpocket_pocket_count: get_opt_i32(4),
        dogsitescorer: get_opt_f32(5),
        overall_assessment: get_opt_string(6),
        assessed_at: chrono::DateTime::parse_from_rfc3339(&get_string(7))
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
    })
}

pub fn ent_synthetic_lethality_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("gene1_id", DataType::Utf8, false),
        Field::new("gene2_id", DataType::Utf8, false),
        Field::new("cancer_id", DataType::Utf8, false),
        Field::new("evidence_type", DataType::Utf8, true),
        Field::new("source_db", DataType::Utf8, true),
        Field::new("screen_id", DataType::Utf8, true),
        Field::new("effect_size", DataType::Float32, true),
        Field::new("confidence", DataType::Float32, true),
        Field::new("pmid", DataType::Utf8, true),
        Field::new("created_at", DataType::Utf8, false),
    ]))
}

pub fn ent_synthetic_lethality_to_record(item: &EntSyntheticLethality) -> Result<RecordBatch> {
    let schema = ent_synthetic_lethality_schema();
    let id = StringArray::from(vec![item.id.to_string()]);
    let gene1_id = StringArray::from(vec![item.gene1_id.to_string()]);
    let gene2_id = StringArray::from(vec![item.gene2_id.to_string()]);
    let cancer_id = StringArray::from(vec![item.cancer_id.to_string()]);
    let evidence_type = StringArray::from(vec![item.evidence_type.as_deref()]);
    let source_db = StringArray::from(vec![item.source_db.as_deref()]);
    let screen_id = StringArray::from(vec![item.screen_id.as_deref()]);
    let effect_size = arrow_array::Float32Array::from(vec![item.effect_size]);
    let confidence = arrow_array::Float32Array::from(vec![item.confidence]);
    let pmid = StringArray::from(vec![item.pmid.as_deref()]);
    let created_at = StringArray::from(vec![item.created_at.to_rfc3339()]);
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id) as Arc<dyn Array>,
            Arc::new(gene1_id),
            Arc::new(gene2_id),
            Arc::new(cancer_id),
            Arc::new(evidence_type),
            Arc::new(source_db),
            Arc::new(screen_id),
            Arc::new(effect_size),
            Arc::new(confidence),
            Arc::new(pmid),
            Arc::new(created_at),
        ],
    )
    .map_err(|e| DbError::Arrow(e.to_string()))
}

pub fn record_to_ent_synthetic_lethality(
    batch: &RecordBatch,
    row: usize,
) -> Result<EntSyntheticLethality> {
    let get_string = |col: usize| -> String {
        batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .value(row)
            .to_string()
    };
    let get_opt_string = |col: usize| -> Option<String> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row).to_string())
        }
    };
    let get_bool = |col: usize| -> bool {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::BooleanArray>()
            .unwrap();
        if arr.is_null(row) {
            false
        } else {
            arr.value(row)
        }
    };
    let get_opt_i16 = |col: usize| -> Option<i16> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Int16Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    let get_opt_i32 = |col: usize| -> Option<i32> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Int32Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    let get_opt_f32 = |col: usize| -> Option<f32> {
        let arr = batch
            .column(col)
            .as_any()
            .downcast_ref::<arrow_array::Float32Array>()
            .unwrap();
        if arr.is_null(row) {
            None
        } else {
            Some(arr.value(row))
        }
    };
    Ok(EntSyntheticLethality {
        id: uuid::Uuid::parse_str(&get_string(0))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        gene1_id: uuid::Uuid::parse_str(&get_string(1))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        gene2_id: uuid::Uuid::parse_str(&get_string(2))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        cancer_id: uuid::Uuid::parse_str(&get_string(3))
            .map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        evidence_type: get_opt_string(4),
        source_db: get_opt_string(5),
        screen_id: get_opt_string(6),
        effect_size: get_opt_f32(7),
        confidence: get_opt_f32(8),
        pmid: get_opt_string(9),
        created_at: chrono::DateTime::parse_from_rfc3339(&get_string(10))
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
    })
}
