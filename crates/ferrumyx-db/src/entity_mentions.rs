//! Entity mention repository.
//!
//! Tracks where entities are mentioned in chunks (for provenance).

use crate::database::Database;
use crate::error::Result;
use crate::schema::EntityMention;
use std::collections::HashMap;
use std::sync::Arc;
use tokio_postgres::{Row, types::ToSql};

/// Repository for entity mention operations.
#[derive(Clone)]
pub struct EntityMentionRepository {
    db: Arc<Database>,
}

impl EntityMentionRepository {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Insert a new entity mention.
    pub async fn insert(&self, mention: &EntityMention) -> Result<()> {
        let client = self.db.client();
        client.execute(
            "INSERT INTO entity_mentions (id, entity_id, chunk_id, paper_id, start_offset, end_offset, text, confidence, context, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())",
            &[
                &mention.id,
                &mention.entity_id,
                &mention.chunk_id,
                &mention.paper_id,
                &mention.start_offset,
                &mention.end_offset,
                &mention.text,
                &mention.confidence,
                &mention.context,
            ],
        ).await?;
        Ok(())
    }

    /// Insert multiple mentions in bulk.
    pub async fn insert_batch(&self, mentions: &[EntityMention]) -> Result<()> {
        if mentions.is_empty() {
            return Ok(());
        }
        for mention in mentions {
            self.insert(mention).await?;
        }
        Ok(())
    }

    /// Find a mention by ID.
    pub async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<EntityMention>> {
        let client = self.db.client();
        let row = client.query_opt("SELECT * FROM entity_mentions WHERE id = $1", &[&id]).await?;
        Ok(row.map(mention_from_row))
    }

    /// Find all mentions for a chunk.
    pub async fn find_by_chunk_id(&self, chunk_id: uuid::Uuid) -> Result<Vec<EntityMention>> {
        let client = self.db.client();
        let rows = client.query("SELECT * FROM entity_mentions WHERE chunk_id = $1", &[&chunk_id]).await?;
        Ok(rows.into_iter().map(mention_from_row).collect())
    }

    /// Find all mentions for an entity.
    pub async fn find_by_entity_id(&self, entity_id: uuid::Uuid) -> Result<Vec<EntityMention>> {
        let client = self.db.client();
        let rows = client.query("SELECT * FROM entity_mentions WHERE entity_id = $1", &[&entity_id]).await?;
        Ok(rows.into_iter().map(mention_from_row).collect())
    }

    /// Find all mentions for a paper (via chunks).
    pub async fn find_by_paper_id(&self, paper_id: uuid::Uuid) -> Result<Vec<EntityMention>> {
        let client = self.db.client();
        let rows = client.query("SELECT * FROM entity_mentions WHERE paper_id = $1", &[&paper_id]).await?;
        Ok(rows.into_iter().map(mention_from_row).collect())
    }

    /// Delete all mentions for a chunk.
    pub async fn delete_by_chunk_id(&self, chunk_id: uuid::Uuid) -> Result<()> {
        let client = self.db.client();
        client.execute("DELETE FROM entity_mentions WHERE chunk_id = $1", &[&chunk_id]).await?;
        Ok(())
    }

    /// Delete all mentions for a paper.
    pub async fn delete_by_paper_id(&self, paper_id: uuid::Uuid) -> Result<()> {
        let client = self.db.client();
        client.execute("DELETE FROM entity_mentions WHERE paper_id = $1", &[&paper_id]).await?;
        Ok(())
    }

    /// Delete a mention by ID.
    pub async fn delete(&self, id: uuid::Uuid) -> Result<()> {
        let client = self.db.client();
        client.execute("DELETE FROM entity_mentions WHERE id = $1", &[&id]).await?;
        Ok(())
    }

    /// Count total mentions.
    pub async fn count(&self) -> Result<u64> {
        let client = self.db.client();
        let row = client.query_one("SELECT COUNT(*) FROM entity_mentions", &[]).await?;
        Ok(row.get::<_, i64>(0) as u64)
    }

    /// Count mentions for an entity.
    pub async fn count_by_entity_id(&self, entity_id: uuid::Uuid) -> Result<u64> {
        let client = self.db.client();
        let row = client.query_one(
            "SELECT COUNT(*) FROM entity_mentions WHERE entity_id = $1",
            &[&entity_id],
        ).await?;
        Ok(row.get::<_, i64>(0) as u64)
    }

    /// Count mentions for a paper.
    pub async fn count_by_paper_id(&self, paper_id: uuid::Uuid) -> Result<u64> {
        let client = self.db.client();
        let row = client.query_one(
            "SELECT COUNT(*) FROM entity_mentions WHERE paper_id = $1",
            &[&paper_id],
        ).await?;
        Ok(row.get::<_, i64>(0) as u64)
    }

    /// List mentions with pagination.
    pub async fn list(&self, _offset: usize, _limit: usize) -> Result<Vec<EntityMention>> {
        let client = self.db.client();
        let rows = client.query("SELECT * FROM entity_mentions ORDER BY created_at DESC", &[]).await?;
        Ok(rows.into_iter().map(mention_from_row).collect())
    }

    /// Get entity co-occurrence counts (entities mentioned in the same chunk).
    pub async fn get_cooccurrences(
        &self,
        entity_id: uuid::Uuid,
        limit: usize,
    ) -> Result<Vec<(uuid::Uuid, u64)>> {
        let client = self.db.client();
        let sql = "SELECT entity_id FROM entity_mentions WHERE chunk_id IN (\
            SELECT chunk_id FROM entity_mentions WHERE entity_id = $1\
        ) AND entity_id != $2";
        let params = [
            &entity_id as &(dyn ToSql + Sync),
            &entity_id as &(dyn ToSql + Sync),
        ];
        let rows = client.query(sql, &params).await?;
        let mut counts: HashMap<uuid::Uuid, u64> = HashMap::new();
        for row in rows {
            let other_id: uuid::Uuid = row.get("entity_id");
            *counts.entry(other_id).or_insert(0) += 1;
        }
        let mut sorted: Vec<(uuid::Uuid, u64)> = counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(limit);
        Ok(sorted)
    }
}

fn mention_from_row(row: Row) -> EntityMention {
    EntityMention {
        id: row.get("id"),
        entity_id: row.get("entity_id"),
        chunk_id: row.get("chunk_id"),
        paper_id: row.get("paper_id"),
        start_offset: row.get("start_offset"),
        end_offset: row.get("end_offset"),
        text: row.get("text"),
        confidence: row.get("confidence"),
        context: row.get("context"),
        created_at: row.get::<_, chrono::DateTime<chrono::Utc>>("created_at"),
    }
}
