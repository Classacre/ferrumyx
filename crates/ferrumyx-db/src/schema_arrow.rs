//! Arrow schema and conversion utilities for LanceDB.
//!
//! This module provides the Arrow record batch conversion functions
//! needed to work with LanceDB's API.

use crate::error::{DbError, Result};
use crate::schema::*;
use arrow_array::{Array, RecordBatch, StringArray, Int64Array, Float32Array, FixedSizeListArray};
use arrow_schema::{Field, Schema, DataType};
use std::sync::Arc;

/// Embedding dimension (BiomedBERT-base outputs 768-dim vectors)
pub const EMBEDDING_DIM: usize = 768;

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
        Field::new("source", DataType::Utf8, false),
        Field::new("source_id", DataType::Utf8, true),
        Field::new("published_at", DataType::Utf8, true),
        Field::new("authors", DataType::Utf8, true),
        Field::new("journal", DataType::Utf8, true),
        Field::new("volume", DataType::Utf8, true),
        Field::new("issue", DataType::Utf8, true),
        Field::new("pages", DataType::Utf8, true),
        Field::new("parse_status", DataType::Utf8, false),
        Field::new("ingested_at", DataType::Utf8, false),
        Field::new("abstract_simhash", DataType::Int64, true),
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
    let source = StringArray::from(vec![paper.source.as_str()]);
    let source_id = StringArray::from(vec![paper.source_id.as_deref()]);
    let published_at = StringArray::from(vec![
        paper.published_at.map(|dt| dt.to_rfc3339())
    ]);
    let authors = StringArray::from(vec![paper.authors.as_deref()]);
    let journal = StringArray::from(vec![paper.journal.as_deref()]);
    let volume = StringArray::from(vec![paper.volume.as_deref()]);
    let issue = StringArray::from(vec![paper.issue.as_deref()]);
    let pages = StringArray::from(vec![paper.pages.as_deref()]);
    let parse_status = StringArray::from(vec![paper.parse_status.as_str()]);
    let ingested_at = StringArray::from(vec![paper.ingested_at.to_rfc3339()]);
    let abstract_simhash = Int64Array::from(vec![paper.abstract_simhash]);
    
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id) as Arc<dyn Array>,
            Arc::new(doi),
            Arc::new(pmid),
            Arc::new(title),
            Arc::new(abstract_text),
            Arc::new(full_text),
            Arc::new(source),
            Arc::new(source_id),
            Arc::new(published_at),
            Arc::new(authors),
            Arc::new(journal),
            Arc::new(volume),
            Arc::new(issue),
            Arc::new(pages),
            Arc::new(parse_status),
            Arc::new(ingested_at),
            Arc::new(abstract_simhash),
        ],
    ).map_err(|e| DbError::Arrow(e.to_string()))
}

pub fn record_to_paper(batch: &RecordBatch, row: usize) -> Result<Paper> {
    let get_string = |col: usize| -> String {
        let arr = batch.column(col).as_any().downcast_ref::<StringArray>().unwrap();
        arr.value(row).to_string()
    };
    
    let get_opt_string = |col: usize| -> Option<String> {
        let arr = batch.column(col).as_any().downcast_ref::<StringArray>().unwrap();
        if arr.is_null(row) { None } else { Some(arr.value(row).to_string()) }
    };
    
    let get_opt_i64 = |col: usize| -> Option<i64> {
        let arr = batch.column(col).as_any().downcast_ref::<Int64Array>().unwrap();
        if arr.is_null(row) { None } else { Some(arr.value(row)) }
    };
    
    Ok(Paper {
        id: uuid::Uuid::parse_str(&get_string(0)).map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        doi: get_opt_string(1),
        pmid: get_opt_string(2),
        title: get_string(3),
        abstract_text: get_opt_string(4),
        full_text: get_opt_string(5),
        source: get_string(6),
        source_id: get_opt_string(7),
        published_at: get_opt_string(8).and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc)),
        authors: get_opt_string(9),
        journal: get_opt_string(10),
        volume: get_opt_string(11),
        issue: get_opt_string(12),
        pages: get_opt_string(13),
        parse_status: get_string(14),
        ingested_at: chrono::DateTime::parse_from_rfc3339(&get_string(15))
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
        abstract_simhash: get_opt_i64(16),
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
        Field::new("content", DataType::Utf8, false),
        Field::new("section", DataType::Utf8, true),
        Field::new("page", DataType::Int64, true),
        Field::new("created_at", DataType::Utf8, false),
        Field::new("embedding", DataType::FixedSizeList(
            Arc::new(Field::new("item", DataType::Float32, false)),
            EMBEDDING_DIM as i32
        ), true),
    ]))
}

pub fn chunk_to_record(chunk: &Chunk) -> Result<RecordBatch> {
    let schema = chunk_schema();
    
    let id = StringArray::from(vec![chunk.id.to_string()]);
    let paper_id = StringArray::from(vec![chunk.paper_id.to_string()]);
    let chunk_index = Int64Array::from(vec![chunk.chunk_index]);
    let content = StringArray::from(vec![chunk.content.as_str()]);
    let section = StringArray::from(vec![chunk.section.as_deref()]);
    let page = Int64Array::from(vec![chunk.page]);
    let created_at = StringArray::from(vec![chunk.created_at.to_rfc3339()]);
    
    // Handle embedding
    let embedding: Arc<dyn Array> = if let Some(ref emb) = chunk.embedding {
        let values = Float32Array::from(emb.clone());
        let field = Arc::new(Field::new("item", DataType::Float32, false));
        Arc::new(FixedSizeListArray::try_new(field, EMBEDDING_DIM as i32, Arc::new(values), None)
            .map_err(|e| DbError::Arrow(e.to_string()))?)
    } else {
        Arc::new(FixedSizeListArray::new_null(
            Arc::new(Field::new("item", DataType::Float32, false)),
            EMBEDDING_DIM as i32,
            1,
        ))
    };
    
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id) as Arc<dyn Array>,
            Arc::new(paper_id),
            Arc::new(chunk_index),
            Arc::new(content),
            Arc::new(section),
            Arc::new(page),
            Arc::new(created_at),
            embedding,
        ],
    ).map_err(|e| DbError::Arrow(e.to_string()))
}

pub fn record_to_chunk(batch: &RecordBatch, row: usize) -> Result<Chunk> {
    let get_string = |col: usize| -> String {
        batch.column(col).as_any().downcast_ref::<StringArray>().unwrap().value(row).to_string()
    };
    
    let get_opt_string = |col: usize| -> Option<String> {
        let arr = batch.column(col).as_any().downcast_ref::<StringArray>().unwrap();
        if arr.is_null(row) { None } else { Some(arr.value(row).to_string()) }
    };
    
    let get_i64 = |col: usize| -> i64 {
        batch.column(col).as_any().downcast_ref::<Int64Array>().unwrap().value(row)
    };
    
    let get_opt_i64 = |col: usize| -> Option<i64> {
        let arr = batch.column(col).as_any().downcast_ref::<Int64Array>().unwrap();
        if arr.is_null(row) { None } else { Some(arr.value(row)) }
    };
    
    let get_embedding = |col: usize| -> Option<Vec<f32>> {
        let arr = batch.column(col);
        if arr.is_null(row) { return None; }
        let list_arr = arr.as_any().downcast_ref::<FixedSizeListArray>().unwrap();
        if list_arr.is_null(row) { return None; }
        let values = list_arr.value(row);
        let float_arr = values.as_any().downcast_ref::<Float32Array>().unwrap();
        Some(float_arr.values().to_vec())
    };
    
    Ok(Chunk {
        id: uuid::Uuid::parse_str(&get_string(0)).map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        paper_id: uuid::Uuid::parse_str(&get_string(1)).map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        chunk_index: get_i64(2),
        content: get_string(3),
        section: get_opt_string(4),
        page: get_opt_i64(5),
        created_at: chrono::DateTime::parse_from_rfc3339(&get_string(6))
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
        embedding: get_embedding(7),
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
    ).map_err(|e| DbError::Arrow(e.to_string()))
}

pub fn record_to_entity(batch: &RecordBatch, row: usize) -> Result<Entity> {
    let get_string = |col: usize| -> String {
        batch.column(col).as_any().downcast_ref::<StringArray>().unwrap().value(row).to_string()
    };
    
    let get_opt_string = |col: usize| -> Option<String> {
        let arr = batch.column(col).as_any().downcast_ref::<StringArray>().unwrap();
        if arr.is_null(row) { None } else { Some(arr.value(row).to_string()) }
    };
    
    Ok(Entity {
        id: uuid::Uuid::parse_str(&get_string(0)).map_err(|e| DbError::InvalidQuery(e.to_string()))?,
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
        Field::new("confidence", DataType::Float32, true),
        Field::new("evidence", DataType::Utf8, true),
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
            Arc::new(created_at),
        ],
    ).map_err(|e| DbError::Arrow(e.to_string()))
}

pub fn record_to_kg_fact(batch: &RecordBatch, row: usize) -> Result<KgFact> {
    let get_string = |col: usize| -> String {
        batch.column(col).as_any().downcast_ref::<StringArray>().unwrap().value(row).to_string()
    };
    
    let get_opt_string = |col: usize| -> Option<String> {
        let arr = batch.column(col).as_any().downcast_ref::<StringArray>().unwrap();
        if arr.is_null(row) { None } else { Some(arr.value(row).to_string()) }
    };
    
    let get_opt_f32 = |col: usize| -> Option<f32> {
        let arr = batch.column(col).as_any().downcast_ref::<Float32Array>().unwrap();
        if arr.is_null(row) { None } else { Some(arr.value(row)) }
    };
    
    Ok(KgFact {
        id: uuid::Uuid::parse_str(&get_string(0)).map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        paper_id: uuid::Uuid::parse_str(&get_string(1)).map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        subject_id: uuid::Uuid::parse_str(&get_string(2)).map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        subject_name: get_string(3),
        predicate: get_string(4),
        object_id: uuid::Uuid::parse_str(&get_string(5)).map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        object_name: get_string(6),
        confidence: get_opt_f32(7),
        evidence: get_opt_string(8),
        created_at: chrono::DateTime::parse_from_rfc3339(&get_string(9))
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
    ).map_err(|e| DbError::Arrow(e.to_string()))
}

pub fn record_to_entity_mention(batch: &RecordBatch, row: usize) -> Result<EntityMention> {
    let get_string = |col: usize| -> String {
        batch.column(col).as_any().downcast_ref::<StringArray>().unwrap().value(row).to_string()
    };
    
    let get_opt_string = |col: usize| -> Option<String> {
        let arr = batch.column(col).as_any().downcast_ref::<StringArray>().unwrap();
        if arr.is_null(row) { None } else { Some(arr.value(row).to_string()) }
    };
    
    let get_i64 = |col: usize| -> i64 {
        batch.column(col).as_any().downcast_ref::<Int64Array>().unwrap().value(row)
    };
    
    let get_opt_f32 = |col: usize| -> Option<f32> {
        let arr = batch.column(col).as_any().downcast_ref::<Float32Array>().unwrap();
        if arr.is_null(row) { None } else { Some(arr.value(row)) }
    };
    
    Ok(EntityMention {
        id: uuid::Uuid::parse_str(&get_string(0)).map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        entity_id: uuid::Uuid::parse_str(&get_string(1)).map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        chunk_id: uuid::Uuid::parse_str(&get_string(2)).map_err(|e| DbError::InvalidQuery(e.to_string()))?,
        paper_id: uuid::Uuid::parse_str(&get_string(3)).map_err(|e| DbError::InvalidQuery(e.to_string()))?,
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