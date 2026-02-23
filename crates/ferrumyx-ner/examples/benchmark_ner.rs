//! NER Performance Benchmark
//! 
//! Run with: cargo run --example benchmark_ner --release --features cuda

use ferrumyx_ner::{NerModel, NerConfig, ModelPool};
use std::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    println!("=== NER Performance Benchmark ===\n");
    
    // Test texts of varying lengths
    let short_text = "The patient has diabetes.";
    let medium_text = "The patient was diagnosed with non-small cell lung carcinoma and started on chemotherapy.";
    let long_text = "The 58-year-old male patient presented with persistent cough, chest pain, and unexplained weight loss. \
        CT imaging revealed a mass in the right upper lobe. Biopsy confirmed stage IIIA non-small cell lung carcinoma. \
        Genetic testing identified EGFR exon 19 deletion. The patient was started on osimertinib therapy. \
        Follow-up imaging after 3 months showed partial response with 40% reduction in tumor size.";
    
    let batch_texts: Vec<&str> = (0..100).map(|i| match i % 3 {
        0 => short_text,
        1 => medium_text,
        _ => long_text,
    }).collect();
    
    // Benchmark 1: Single model, sequential processing
    println!("1. Sequential Processing (no pooling)");
    println!("   Loading model...");
    let start = Instant::now();
    let model = NerModel::new(NerConfig::diseases()).await?;
    println!("   Model loaded in {:.2}s", start.elapsed().as_secs_f64());
    
    let start = Instant::now();
    for (i, text) in batch_texts.iter().enumerate() {
        let _ = model.extract(text)?;
        if (i + 1) % 10 == 0 {
            print!("\r   Processed {}/{} texts", i + 1, batch_texts.len());
        }
    }
    let seq_time = start.elapsed();
    println!("\n   Total: {:.2}s, Avg: {:.2}ms/text", 
        seq_time.as_secs_f64(),
        seq_time.as_millis() as f64 / batch_texts.len() as f64
    );
    
    // Benchmark 2: With model pooling
    println!("\n2. With Model Pooling");
    let pool = ModelPool::new(4); // Max 4 concurrent models
    
    let start = Instant::now();
    for (i, text) in batch_texts.iter().enumerate() {
        let model = pool.get_or_load(NerConfig::diseases()).await?;
        let _ = model.extract(text)?;
        if (i + 1) % 10 == 0 {
            print!("\r   Processed {}/{} texts", i + 1, batch_texts.len());
        }
    }
    let pool_time = start.elapsed();
    println!("\n   Total: {:.2}s, Avg: {:.2}ms/text", 
        pool_time.as_secs_f64(),
        pool_time.as_millis() as f64 / batch_texts.len() as f64
    );
    
    // Benchmark 3: Batch processing
    println!("\n3. Batch Processing");
    let model = pool.get_or_load(NerConfig::diseases()).await?;
    
    let start = Instant::now();
    let results = model.extract_batch(&batch_texts)?;
    let batch_time = start.elapsed();
    println!("   Processed {} texts in {:.2}s", results.len(), batch_time.as_secs_f64());
    println!("   Avg: {:.2}ms/text", 
        batch_time.as_millis() as f64 / batch_texts.len() as f64
    );
    
    // Summary
    println!("\n=== Summary ===");
    println!("Sequential:  {:.2}ms/text", seq_time.as_millis() as f64 / batch_texts.len() as f64);
    println!("With Pool:   {:.2}ms/text", pool_time.as_millis() as f64 / batch_texts.len() as f64);
    println!("Batch:       {:.2}ms/text", batch_time.as_millis() as f64 / batch_texts.len() as f64);
    
    let speedup_seq = seq_time.as_millis() as f64 / batch_time.as_millis() as f64;
    println!("\nBatch vs Sequential: {:.1}x faster", speedup_seq);
    
    Ok(())
}
