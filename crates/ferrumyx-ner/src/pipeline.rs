//! Multi-model biomedical NER pipeline.
//!
//! Combines multiple specialized models for comprehensive entity extraction.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::{Result, NerModel, NerConfig, NerEntity};

/// A multi-model NER pipeline that combines results from multiple specialized models.
pub struct NerPipeline {
    models: Arc<RwLock<HashMap<String, NerModel>>>,
    configs: Vec<(String, NerConfig)>,
}

impl NerPipeline {
    /// Create a new pipeline with the given model configurations.
    pub async fn new(configs: Vec<(String, NerConfig)>) -> Result<Self> {
        let mut models = HashMap::new();
        
        for (name, config) in &configs {
            info!("Loading {} model: {}...", name, config.model_id);
            let model = NerModel::new(config.clone()).await?;
            models.insert(name.clone(), model);
        }
        
        Ok(Self {
            models: Arc::new(RwLock::new(models)),
            configs,
        })
    }
    
    /// Create a biomedical pipeline with disease and chemical detection.
    pub async fn biomedical() -> Result<Self> {
        Self::new(vec![
            ("disease".to_string(), NerConfig::diseases()),
            ("pharmaceutical".to_string(), NerConfig::pharmaceuticals()),
        ]).await
    }
    
    /// Extract entities using all loaded models.
    pub async fn extract(&self, text: &str) -> Result<Vec<PipelineEntity>> {
        let models = self.models.read().await;
        let mut all_entities = Vec::new();
        
        for (model_name, model) in models.iter() {
            let entities = model.extract(text)?;
            for entity in entities {
                all_entities.push(PipelineEntity {
                    entity,
                    model_name: model_name.clone(),
                });
            }
        }
        
        // Deduplicate overlapping entities, keeping highest scoring
        all_entities = self.deduplicate_entities(all_entities);
        
        Ok(all_entities)
    }
    
    /// Extract entities from multiple texts in batch.
    pub async fn extract_batch(&self, texts: &[String]) -> Result<Vec<Vec<PipelineEntity>>> {
        let mut results = Vec::new();
        for text in texts {
            let entities = self.extract(text).await?;
            results.push(entities);
        }
        Ok(results)
    }
    
    fn deduplicate_entities(&self, entities: Vec<PipelineEntity>) -> Vec<PipelineEntity> {
        if entities.is_empty() {
            return entities;
        }
        
        // Sort by start position, then by score (descending)
        let mut entities = entities;
        entities.sort_by(|a, b| {
            a.entity.start
                .cmp(&b.entity.start)
                .then_with(|| {
                    // Handle NaN scores gracefully - treat NaN as lowest priority
                    match (a.entity.score.is_nan(), b.entity.score.is_nan()) {
                        (true, true) => std::cmp::Ordering::Equal,
                        (true, false) => std::cmp::Ordering::Greater,
                        (false, true) => std::cmp::Ordering::Less,
                        (false, false) => b.entity.score.partial_cmp(&a.entity.score).unwrap_or(std::cmp::Ordering::Equal),
                    }
                })
        });
        
        let mut result = Vec::new();
        let mut current: Option<PipelineEntity> = None;
        
        for entity in entities {
            match &current {
                None => current = Some(entity),
                Some(curr) => {
                    // Check for overlap
                    if entity.entity.start < curr.entity.end {
                        // Overlapping - keep higher score
                        if entity.entity.score > curr.entity.score {
                            current = Some(entity);
                        }
                    } else {
                        // No overlap - save current and start new
                        if let Some(c) = current.take() {
                            result.push(c);
                        }
                        current = Some(entity);
                    }
                }
            }
        }
        
        if let Some(entity) = current {
            result.push(entity);
        }
        
        result
    }
    
    /// Get list of loaded models.
    pub fn model_names(&self) -> Vec<&String> {
        self.configs.iter().map(|(name, _)| name).collect()
    }
    
    /// Reload a specific model.
    pub async fn reload_model(&self, name: &str, config: NerConfig) -> Result<()> {
        let model = NerModel::new(config).await?;
        let mut models = self.models.write().await;
        models.insert(name.to_string(), model);
        Ok(())
    }
}

/// An entity extracted by the pipeline, with source model information.
#[derive(Debug, Clone)]
pub struct PipelineEntity {
    pub entity: NerEntity,
    pub model_name: String,
}

impl PipelineEntity {
    /// Get the entity text.
    pub fn text(&self) -> &str {
        &self.entity.text
    }
    
    /// Get the entity label.
    pub fn label(&self) -> &str {
        &self.entity.label
    }
    
    /// Get the confidence score.
    pub fn score(&self) -> f32 {
        self.entity.score
    }
    
    /// Get the character span.
    pub fn span(&self) -> (usize, usize) {
        (self.entity.start, self.entity.end)
    }
}
