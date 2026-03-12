//! Pooling strategies for embedding extraction.

use candle_core::Tensor;
use serde::{Deserialize, Serialize};

/// Pooling strategy for converting token embeddings to sentence embeddings.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum PoolingStrategy {
    #[default]
    Mean,
    Cls,
    Max,
}

impl PoolingStrategy {
    pub fn apply(
        &self,
        embeddings: &Tensor,
        attention_mask: &Tensor,
    ) -> candle_core::Result<Tensor> {
        match self {
            PoolingStrategy::Mean => mean_pool(embeddings, attention_mask),
            PoolingStrategy::Cls => cls_pool(embeddings),
            PoolingStrategy::Max => max_pool(embeddings, attention_mask),
        }
    }
}

fn mean_pool(embeddings: &Tensor, attention_mask: &Tensor) -> candle_core::Result<Tensor> {
    let mask_expanded = attention_mask.unsqueeze(2)?.expand(embeddings.shape())?;
    let sum_embeddings = (embeddings * &mask_expanded)?.sum(1)?;
    let sum_mask = attention_mask
        .unsqueeze(2)?
        .sum(1)?
        .clamp(1e-9f32, f32::MAX)?;
    sum_embeddings.broadcast_div(&sum_mask)
}

fn cls_pool(embeddings: &Tensor) -> candle_core::Result<Tensor> {
    embeddings.narrow(1, 0, 1)?.squeeze(1)
}

fn max_pool(embeddings: &Tensor, attention_mask: &Tensor) -> candle_core::Result<Tensor> {
    let mask_expanded = attention_mask.unsqueeze(2)?.expand(embeddings.shape())?;
    let mask_offset = (&mask_expanded - 1.0)?;
    let large_neg = Tensor::new(-1e9f32, embeddings.device())?;
    let mask_values = mask_offset.broadcast_mul(&large_neg)?;
    let masked_embeddings = embeddings.broadcast_add(&mask_values)?;
    masked_embeddings.max(1)
}

pub fn l2_normalize(embeddings: &Tensor) -> candle_core::Result<Tensor> {
    let norms = embeddings.sqr()?.sum_keepdim(1)?.sqrt()?;
    let norms_clamped = norms.clamp(1e-9f32, f32::MAX)?;
    embeddings.broadcast_div(&norms_clamped)
}
