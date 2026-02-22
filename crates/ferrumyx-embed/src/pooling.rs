//! Pooling strategies for embedding extraction.

use candle_core::Tensor;
use serde::{Deserialize, Serialize};

/// Pooling strategy for converting token embeddings to sentence embeddings.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum PoolingStrategy {
    /// Mean pooling over all tokens (excluding padding)
    #[default]
    Mean,
    
    /// Use [CLS] token embedding
    Cls,
    
    /// Max pooling over all tokens
    Max,
}

impl PoolingStrategy {
    /// Apply pooling to token embeddings.
    ///
    /// # Arguments
    /// * `embeddings` - Tensor of shape (batch_size, seq_len, hidden_dim)
    /// * `attention_mask` - Tensor of shape (batch_size, seq_len)
    ///
    /// # Returns
    /// Tensor of shape (batch_size, hidden_dim)
    pub fn apply(&self, embeddings: &Tensor, attention_mask: &Tensor) -> candle_core::Result<Tensor> {
        match self {
            PoolingStrategy::Mean => mean_pool(embeddings, attention_mask),
            PoolingStrategy::Cls => cls_pool(embeddings),
            PoolingStrategy::Max => max_pool(embeddings, attention_mask),
        }
    }
}

/// Mean pooling over non-padding tokens.
///
/// This is the most common approach for sentence embeddings.
/// It averages the token embeddings, weighted by the attention mask.
fn mean_pool(embeddings: &Tensor, attention_mask: &Tensor) -> candle_core::Result<Tensor> {
    // embeddings: (batch, seq_len, hidden_dim)
    // attention_mask: (batch, seq_len)
    
    // Expand mask to match embedding dimensions: (batch, seq_len, 1)
    let mask_expanded = attention_mask
        .unsqueeze(2)?
        .expand(embeddings.shape())?;
    
    // Sum embeddings weighted by mask
    let sum_embeddings = (embeddings * &mask_expanded)?.sum(1)?;
    
    // Sum mask for normalization (clamp to avoid division by zero)
    let sum_mask = attention_mask
        .unsqueeze(2)?
        .sum(1)?
        .clamp(1e-9f32, f32::MAX)?;
    
    // Divide to get mean
    sum_embeddings.broadcast_div(&sum_mask)
}

/// Extract [CLS] token embedding (first token).
fn cls_pool(embeddings: &Tensor) -> candle_core::Result<Tensor> {
    // embeddings: (batch, seq_len, hidden_dim)
    // Select first token: (batch, hidden_dim)
    embeddings.narrow(1, 0, 1)?.squeeze(1)
}

/// Max pooling over non-padding tokens.
fn max_pool(embeddings: &Tensor, attention_mask: &Tensor) -> candle_core::Result<Tensor> {
    // embeddings: (batch, seq_len, hidden_dim)
    // attention_mask: (batch, seq_len)
    
    // Expand mask and set padding tokens to very negative value
    let mask_expanded = attention_mask
        .unsqueeze(2)?
        .expand(embeddings.shape())?;
    
    // Where mask is 0, set to large negative value (for max pooling)
    // mask_expanded - 1.0 gives 0 for real tokens, -1 for padding
    let mask_offset = (&mask_expanded - 1.0)?;
    let large_neg = Tensor::new(-1e9f32, embeddings.device())?;
    let mask_values = mask_offset.broadcast_mul(&large_neg)?;
    let masked_embeddings = embeddings.broadcast_add(&mask_values)?;
    
    // Max over sequence dimension
    masked_embeddings.max(1)
}

/// L2 normalize embeddings.
pub fn l2_normalize(embeddings: &Tensor) -> candle_core::Result<Tensor> {
    // embeddings: (batch, hidden_dim)
    let norms = embeddings.sqr()?.sum_keepdim(1)?.sqrt()?;
    let norms_clamped = norms.clamp(1e-9f32, f32::MAX)?;
    embeddings.broadcast_div(&norms_clamped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::Device;

    #[test]
    fn test_mean_pool() {
        let device = Device::Cpu;
        
        // Simple test: 2 sequences, 3 tokens each, 4-dim embeddings
        let embeddings = Tensor::from_vec(
            vec![
                // Seq 1
                1.0f32, 2.0, 3.0, 4.0,  // token 0
                2.0, 3.0, 4.0, 5.0,  // token 1
                3.0, 4.0, 5.0, 6.0,  // token 2
                // Seq 2
                1.0, 1.0, 1.0, 1.0,
                2.0, 2.0, 2.0, 2.0,
                0.0, 0.0, 0.0, 0.0,  // padding
            ],
            (2, 3, 4),
            &device,
        ).unwrap();
        
        let attention_mask = Tensor::from_vec(
            vec![1.0f32, 1.0, 1.0,  // Seq 1: all real
                 1.0, 1.0, 0.0], // Seq 2: last token padding
            (2, 3),
            &device,
        ).unwrap();
        
        let pooled = mean_pool(&embeddings, &attention_mask).unwrap();
        let result = pooled.to_vec2::<f32>().unwrap();
        
        // Seq 1: mean of [1,2,3,4], [2,3,4,5], [3,4,5,6] = [2,3,4,5]
        assert!((result[0][0] - 2.0).abs() < 1e-5);
        assert!((result[0][1] - 3.0).abs() < 1e-5);
        
        // Seq 2: mean of [1,1,1,1], [2,2,2,2] = [1.5, 1.5, 1.5, 1.5]
        assert!((result[1][0] - 1.5).abs() < 1e-5);
    }

    #[test]
    fn test_l2_normalize() {
        let device = Device::Cpu;
        
        let embeddings = Tensor::from_vec(
            vec![3.0f32, 4.0, 0.0, 0.0,  // norm = 5
                 1.0, 1.0, 1.0, 1.0], // norm = 2
            (2, 4),
            &device,
        ).unwrap();
        
        let normalized = l2_normalize(&embeddings).unwrap();
        let result = normalized.to_vec2::<f32>().unwrap();
        
        // First: [3/5, 4/5, 0, 0]
        assert!((result[0][0] - 0.6).abs() < 1e-5);
        assert!((result[0][1] - 0.8).abs() < 1e-5);
        
        // Check norms are 1
        for row in result {
            let norm: f32 = row.iter().map(|x| x * x).sum();
            assert!((norm - 1.0).abs() < 1e-5);
        }
    }
}
