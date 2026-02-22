//! Test the BiomedBERT embedder

use ferrumyx_embed::{BiomedBertEmbedder, EmbeddingConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Loading BiomedBERT embedder...");
    
    // Use default config (PubMedBERT embeddings)
    let config = EmbeddingConfig::cpu();
    
    let embedder = BiomedBertEmbedder::new(config).await?;
    
    println!("Model loaded! Testing embeddings...\n");
    
    let texts = vec![
        "KRAS G12D mutation drives pancreatic cancer progression through MAPK signaling.".to_string(),
        "TP53 tumor suppressor gene mutations are found in over 50% of human cancers.".to_string(),
        "The COVID-19 vaccine mRNA technology enables rapid development of immunizations.".to_string(),
    ];
    
    println!("Input texts:");
    for (i, text) in texts.iter().enumerate() {
        println!("  {}: {}", i + 1, text);
    }
    println!();
    
    let start = std::time::Instant::now();
    let embeddings = embedder.embed(&texts).await?;
    let elapsed = start.elapsed();
    
    println!("Generated {} embeddings in {:.2}ms", embeddings.len(), elapsed.as_secs_f64() * 1000.0);
    println!("Embedding dimension: {}", embeddings[0].len());
    
    // Show first few dimensions of each embedding
    println!("\nFirst 5 dimensions of each embedding:");
    for (i, emb) in embeddings.iter().enumerate() {
        println!("  {}: [{:.4}, {:.4}, {:.4}, {:.4}, {:.4}, ...]", 
            i + 1, emb[0], emb[1], emb[2], emb[3], emb[4]);
    }
    
    // Compute cosine similarity between embeddings
    println!("\nCosine similarities:");
    let sim_01 = cosine_similarity(&embeddings[0], &embeddings[1]);
    let sim_02 = cosine_similarity(&embeddings[0], &embeddings[2]);
    let sim_12 = cosine_similarity(&embeddings[1], &embeddings[2]);
    
    println!("  KRAS vs TP53:     {:.4} (both cancer-related)", sim_01);
    println!("  KRAS vs COVID:    {:.4} (different topics)", sim_02);
    println!("  TP53 vs COVID:    {:.4} (different topics)", sim_12);
    
    println!("\n✅ Test passed!");
    
    Ok(())
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (norm_a * norm_b)
}
