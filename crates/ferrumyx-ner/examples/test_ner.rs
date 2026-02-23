//! Test NER model loading and extraction

use ferrumyx_ner::{NerModel, NerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    println!("Loading NER model...");
    let start = std::time::Instant::now();
    
    // Use a simple BERT NER model that works with Candle
    let config = NerConfig {
        model_id: "dslim/bert-base-NER".to_string(),
        ..Default::default()
    };
    let model = NerModel::new(config).await?;
    
    println!("Model loaded in {:?}", start.elapsed());
    
    // Test extraction with simpler text
    let text = "John Smith works at Google";
    
    println!("\nExtracting entities from: {}", text);
    let start = std::time::Instant::now();
    
    let entities = model.extract(text)?;
    
    println!("Extraction took {:?}", start.elapsed());
    println!("\nFound {} entities:", entities.len());
    
    for e in &entities {
        println!("  - {} ({:?}): '{}' [score: {:.2}]", 
            e.label, e.entity_type, e.text, e.score);
    }
    
    // Test with more text
    let text2 = "Apple Inc. is headquartered in Cupertino, California.";
    println!("\nExtracting entities from: {}", text2);
    let entities2 = model.extract(text2)?;
    println!("Found {} entities:", entities2.len());
    for e in &entities2 {
        println!("  - {} ({:?}): '{}' [score: {:.2}]", 
            e.label, e.entity_type, e.text, e.score);
    }
    
    Ok(())
}
