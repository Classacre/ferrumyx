//! Entity aggregation for knowledge graph construction.
//!
//! Aggregates extracted entities across papers to build:
//! - Entity statistics (mention counts, paper counts)
//! - Co-occurrence networks (gene-disease, drug-gene)
//! - Knowledge graph triples (gene)-[associated_with]-(disease)

use std::collections::HashMap;
use anyhow::Result;
use tracing::info;

#[cfg(feature = "db")]
use sqlx::{PgPool, Row};

#[cfg(feature = "db")]
use uuid::Uuid;

/// Entity aggregator for building knowledge graph from paper entities.
#[cfg(feature = "db")]
pub struct EntityAggregator {
    pool: PgPool,
}

/// Entity aggregator stub when db feature is not enabled.
#[cfg(not(feature = "db"))]
pub struct EntityAggregator;

/// Co-occurrence between two entities in a paper.
#[derive(Debug, Clone)]
pub struct EntityCooccurrence {
    #[cfg(feature = "db")]
    pub paper_id: Uuid,
    #[cfg(not(feature = "db"))]
    pub paper_id: String,
    #[cfg(feature = "db")]
    pub entity_a_id: Uuid,
    #[cfg(not(feature = "db"))]
    pub entity_a_id: String,
    #[cfg(feature = "db")]
    pub entity_b_id: Uuid,
    #[cfg(not(feature = "db"))]
    pub entity_b_id: String,
    pub entity_a_type: String,
    pub entity_b_type: String,
    pub sentence_ids: Vec<i32>,
    pub confidence: f32,
}

/// Knowledge graph triple (subject-predicate-object).
#[derive(Debug, Clone)]
pub struct KgTriple {
    pub subject_id: String,
    pub subject_type: String,
    pub predicate: String,
    pub object_id: String,
    pub object_type: String,
    pub evidence_count: i32,
    pub confidence: f32,
}

#[cfg(feature = "db")]
impl EntityAggregator {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Aggregate all entities for a paper, building co-occurrences.
    pub async fn aggregate_paper(&self, paper_id: Uuid) -> Result<AggregationResult> {
        info!("Aggregating entities for paper {}", paper_id);

        // Get all entities for this paper
        let entities = self.get_paper_entities(paper_id).await?;
        
        if entities.is_empty() {
            return Ok(AggregationResult::default());
        }

        // Build co-occurrences
        let cooccurrences = self.build_cooccurrences(paper_id, &entities).await?;
        
        // Update entity statistics
        self.update_entity_statistics(&entities).await?;
        
        // Store co-occurrences
        self.store_cooccurrences(&cooccurrences).await?;
        
        // Extract knowledge graph triples
        let triples = self.extract_triples(&cooccurrences).await?;

        info!(
            "Aggregated {} entities, {} co-occurrences, {} triples for paper {}",
            entities.len(), cooccurrences.len(), triples.len(), paper_id
        );

        Ok(AggregationResult {
            entity_count: entities.len(),
            cooccurrence_count: cooccurrences.len(),
            triples,
        })
    }

    /// Aggregate entities across all papers (batch operation).
    pub async fn aggregate_all(&self) -> Result<BatchAggregationResult> {
        info!("Starting batch entity aggregation");
        
        let paper_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM papers WHERE parse_status = 'parsed' ORDER BY ingested_at"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut total_entities = 0;
        let mut total_cooccurrences = 0;
        let mut all_triples: Vec<KgTriple> = Vec::new();

        for paper_id in paper_ids {
            match self.aggregate_paper(paper_id).await {
                Ok(result) => {
                    total_entities += result.entity_count;
                    total_cooccurrences += result.cooccurrence_count;
                    all_triples.extend(result.triples);
                }
                Err(e) => {
                    tracing::warn!("Failed to aggregate paper {}: {}", paper_id, e);
                }
            }
        }

        // Deduplicate and merge triples
        let merged_triples = self.merge_triples(all_triples).await?;

        info!(
            "Batch aggregation complete: {} entities, {} co-occurrences, {} unique triples",
            total_entities, total_cooccurrences, merged_triples.len()
        );

        Ok(BatchAggregationResult {
            papers_processed: paper_ids.len(),
            total_entities,
            total_cooccurrences,
            unique_triples: merged_triples.len(),
            triples: merged_triples,
        })
    }

    /// Get all entities for a paper.
    async fn get_paper_entities(&self, paper_id: Uuid) -> Result<Vec<PaperEntity>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                id, entity_type, entity_text, normalized_id, 
                normalized_source, confidence, chunk_id
            FROM entities
            WHERE paper_id = $1
            ORDER BY first_mention_offset
            "#
        )
        .bind(paper_id)
        .fetch_all(&self.pool)
        .await?;

        let mut entities = Vec::new();
        for row in rows {
            entities.push(PaperEntity {
                id: row.try_get("id")?,
                entity_type: row.try_get("entity_type")?,
                entity_text: row.try_get("entity_text")?,
                normalized_id: row.try_get("normalized_id")?,
                normalized_source: row.try_get("normalized_source")?,
                confidence: row.try_get("confidence")?,
                chunk_id: row.try_get("chunk_id")?,
            });
        }

        Ok(entities)
    }

    /// Build co-occurrences between entities in the same paper.
    async fn build_cooccurrences(
        &self,
        paper_id: Uuid,
        entities: &[PaperEntity],
    ) -> Result<Vec<EntityCooccurrence>> {
        let mut cooccurrences = Vec::new();
        
        // Group entities by chunk (co-occur if in same chunk)
        let mut chunk_entities: HashMap<Uuid, Vec<&PaperEntity>> = HashMap::new();
        for entity in entities {
            if let Some(chunk_id) = entity.chunk_id {
                chunk_entities.entry(chunk_id).or_default().push(entity);
            }
        }

        // Build co-occurrences within each chunk
        for (_chunk_id, chunk_ents) in chunk_entities {
            for (i, entity_a) in chunk_ents.iter().enumerate() {
                for entity_b in chunk_ents.iter().skip(i + 1) {
                    // Skip if same entity
                    if entity_a.id == entity_b.id {
                        continue;
                    }

                    // Calculate combined confidence
                    let confidence = (entity_a.confidence.unwrap_or(0.5) + 
                                    entity_b.confidence.unwrap_or(0.5)) / 2.0;

                    cooccurrences.push(EntityCooccurrence {
                        paper_id,
                        entity_a_id: entity_a.id,
                        entity_b_id: entity_b.id,
                        entity_a_type: entity_a.entity_type.clone(),
                        entity_b_type: entity_b.entity_type.clone(),
                        sentence_ids: vec![], // Would need sentence-level parsing
                        confidence,
                    });
                }
            }
        }

        Ok(cooccurrences)
    }

    /// Update entity statistics table.
    async fn update_entity_statistics(&self, entities: &[PaperEntity]) -> Result<()> {
        for entity in entities {
            if let Some(ref norm_id) = entity.normalized_id {
                sqlx::query(
                    r#"
                    INSERT INTO entity_statistics 
                        (normalized_id, entity_type, entity_name, mention_count_total, paper_count)
                    VALUES ($1, $2, $3, 1, 1)
                    ON CONFLICT (normalized_id) DO UPDATE SET
                        mention_count_total = entity_statistics.mention_count_total + 1,
                        paper_count = (
                            SELECT COUNT(DISTINCT paper_id) 
                            FROM entities 
                            WHERE normalized_id = $1
                        ),
                        last_seen = NOW(),
                        updated_at = NOW()
                    "#
                )
                .bind(norm_id)
                .bind(&entity.entity_type)
                .bind(&entity.entity_text)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    /// Store co-occurrences in database.
    async fn store_cooccurrences(&self, cooccurrences: &[EntityCooccurrence]) -> Result<()> {
        for cooc in cooccurrences {
            sqlx::query(
                r#"
                INSERT INTO entity_cooccurrences 
                    (paper_id, entity_a_id, entity_b_id, entity_a_type, entity_b_type,
                     cooccurrence_count, confidence)
                VALUES ($1, $2, $3, $4, $5, 1, $6)
                ON CONFLICT (paper_id, entity_a_id, entity_b_id) DO UPDATE SET
                    cooccurrence_count = entity_cooccurrences.cooccurrence_count + 1,
                    confidence = GREATEST(entity_cooccurrences.confidence, EXCLUDED.confidence)
                "#
            )
            .bind(cooc.paper_id)
            .bind(cooc.entity_a_id)
            .bind(cooc.entity_b_id)
            .bind(&cooc.entity_a_type)
            .bind(&cooc.entity_b_type)
            .bind(cooc.confidence)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Extract knowledge graph triples from co-occurrences.
    async fn extract_triples(&self, cooccurrences: &[EntityCooccurrence]) -> Result<Vec<KgTriple>> {
        let mut triples = Vec::new();

        for cooc in cooccurrences {
            let predicate = Self::infer_predicate(&cooc.entity_a_type, &cooc.entity_b_type);
            
            // Get normalized IDs for subject and object
            let (subject_id, subject_type) = self.get_entity_info(cooc.entity_a_id).await?;
            let (object_id, object_type) = self.get_entity_info(cooc.entity_b_id).await?;

            triples.push(KgTriple {
                subject_id,
                subject_type,
                predicate,
                object_id,
                object_type,
                evidence_count: 1,
                confidence: cooc.confidence,
            });
        }

        Ok(triples)
    }

    /// Get entity info (normalized_id, type) from database.
    async fn get_entity_info(&self, entity_id: Uuid) -> Result<(String, String)> {
        let row = sqlx::query(
            "SELECT COALESCE(normalized_id, entity_text) as id, entity_type FROM entities WHERE id = $1"
        )
        .bind(entity_id)
        .fetch_one(&self.pool)
        .await?;

        let id: String = row.try_get("id")?;
        let entity_type: String = row.try_get("entity_type")?;

        Ok((id, entity_type))
    }

    /// Infer predicate from entity types.
    fn infer_predicate(type_a: &str, type_b: &str) -> String {
        match (type_a, type_b) {
            ("GENE", "DISEASE") | ("DISEASE", "GENE") => "associated_with",
            ("GENE", "CHEMICAL") | ("CHEMICAL", "GENE") => "interacts_with",
            ("CHEMICAL", "DISEASE") | ("DISEASE", "CHEMICAL") => "treats",
            ("MUTATION", "DISEASE") | ("DISEASE", "MUTATION") => "causes",
            ("GENE", "MUTATION") | ("MUTATION", "GENE") => "has_variant",
            _ => "related_to",
        }.to_string()
    }

    /// Merge and deduplicate triples across papers.
    async fn merge_triples(&self, triples: Vec<KgTriple>) -> Result<Vec<KgTriple>> {
        let mut merged: HashMap<String, KgTriple> = HashMap::new();

        for triple in triples {
            let key = format!("{}|{}|{}", 
                triple.subject_id, 
                triple.predicate, 
                triple.object_id
            );

            if let Some(existing) = merged.get_mut(&key) {
                // Merge evidence
                existing.evidence_count += 1;
                existing.confidence = (existing.confidence + triple.confidence) / 2.0;
            } else {
                merged.insert(key, triple);
            }
        }

        Ok(merged.into_values().collect())
    }

    /// Get top co-occurring entity pairs (for knowledge graph construction).
    pub async fn get_top_cooccurrences(
        &self,
        entity_type_a: &str,
        entity_type_b: &str,
        limit: i64,
    ) -> Result<Vec<TopCooccurrence>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                e1.normalized_id as entity_a_id,
                e1.entity_text as entity_a_name,
                e2.normalized_id as entity_b_id,
                e2.entity_text as entity_b_name,
                SUM(c.cooccurrence_count) as total_cooccurrences,
                AVG(c.confidence) as avg_confidence,
                COUNT(DISTINCT c.paper_id) as paper_count
            FROM entity_cooccurrences c
            JOIN entities e1 ON c.entity_a_id = e1.id
            JOIN entities e2 ON c.entity_b_id = e2.id
            WHERE c.entity_a_type = $1 AND c.entity_b_type = $2
              AND e1.normalized_id IS NOT NULL 
              AND e2.normalized_id IS NOT NULL
            GROUP BY e1.normalized_id, e1.entity_text, e2.normalized_id, e2.entity_text
            ORDER BY total_cooccurrences DESC
            LIMIT $3
            "#
        )
        .bind(entity_type_a)
        .bind(entity_type_b)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::new();
        for row in rows {
            results.push(TopCooccurrence {
                entity_a_id: row.try_get("entity_a_id")?,
                entity_a_name: row.try_get("entity_a_name")?,
                entity_b_id: row.try_get("entity_b_id")?,
                entity_b_name: row.try_get("entity_b_name")?,
                total_cooccurrences: row.try_get::<i64, _>("total_cooccurrences")? as i32,
                avg_confidence: row.try_get("avg_confidence")?,
                paper_count: row.try_get::<i64, _>("paper_count")? as i32,
            });
        }

        Ok(results)
    }

    /// Export knowledge graph as RDF triples.
    pub async fn export_kg_rdf(&self) -> Result<String> {
        let mut rdf = String::new();
        rdf.push_str("@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n");
        rdf.push_str("@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n");
        rdf.push_str("@prefix kg: <http://ferrumyx.org/kg/> .\n\n");

        // Get all co-occurrences as triples
        let rows = sqlx::query(
            r#"
            SELECT 
                e1.normalized_id as subject,
                e1.entity_type as subject_type,
                e2.normalized_id as object,
                e2.entity_type as object_type,
                c.entity_a_type,
                c.entity_b_type,
                SUM(c.cooccurrence_count) as evidence_count,
                AVG(c.confidence) as confidence
            FROM entity_cooccurrences c
            JOIN entities e1 ON c.entity_a_id = e1.id
            JOIN entities e2 ON c.entity_b_id = e2.id
            WHERE e1.normalized_id IS NOT NULL AND e2.normalized_id IS NOT NULL
            GROUP BY e1.normalized_id, e1.entity_type, e2.normalized_id, e2.entity_type,
                     c.entity_a_type, c.entity_b_type
            ORDER BY evidence_count DESC
            LIMIT 10000
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        for row in rows {
            let subject: String = row.try_get("subject")?;
            let subject_type: String = row.try_get("subject_type")?;
            let object: String = row.try_get("object")?;
            let object_type: String = row.try_get("object_type")?;
            let type_a: String = row.try_get("entity_a_type")?;
            let type_b: String = row.try_get("entity_b_type")?;
            let evidence: i64 = row.try_get("evidence_count")?;
            let confidence: f32 = row.try_get("confidence")?;

            let predicate = Self::infer_predicate(&type_a, &type_b);

            rdf.push_str(&format!(
                "kg:{} kg:{} kg:{} . # evidence={}, conf={:.2}\n",
                subject.replace(":", "_"),
                predicate,
                object.replace(":", "_"),
                evidence,
                confidence
            ));
        }

        Ok(rdf)
    }
}

/// Entity from a paper.
#[cfg(feature = "db")]
#[derive(Debug, Clone)]
struct PaperEntity {
    id: Uuid,
    entity_type: String,
    entity_text: String,
    normalized_id: Option<String>,
    normalized_source: Option<String>,
    confidence: Option<f32>,
    chunk_id: Option<Uuid>,
}

/// Result of aggregating a single paper.
#[derive(Debug, Default)]
pub struct AggregationResult {
    pub entity_count: usize,
    pub cooccurrence_count: usize,
    pub triples: Vec<KgTriple>,
}

/// Result of batch aggregation.
#[derive(Debug)]
pub struct BatchAggregationResult {
    pub papers_processed: usize,
    pub total_entities: usize,
    pub total_cooccurrences: usize,
    pub unique_triples: usize,
    pub triples: Vec<KgTriple>,
}

/// Top co-occurring entity pair.
#[derive(Debug)]
pub struct TopCooccurrence {
    pub entity_a_id: String,
    pub entity_a_name: String,
    pub entity_b_id: String,
    pub entity_b_name: String,
    pub total_cooccurrences: i32,
    pub avg_confidence: f32,
    pub paper_count: i32,
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_infer_predicate() {
        use super::EntityAggregator;
        
        assert_eq!(
            EntityAggregator::infer_predicate("GENE", "DISEASE"),
            "associated_with"
        );
        assert_eq!(
            EntityAggregator::infer_predicate("CHEMICAL", "GENE"),
            "interacts_with"
        );
        assert_eq!(
            EntityAggregator::infer_predicate("CHEMICAL", "DISEASE"),
            "treats"
        );
    }
}
