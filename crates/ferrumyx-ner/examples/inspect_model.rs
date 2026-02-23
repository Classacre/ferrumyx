//! Inspect the NER model structure

use ferrumyx_ner::{NerModel, NerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    let config = NerConfig {
        model_id: "dslim/bert-base-NER".to_string(),
        use_gpu: false,
        max_length: 512,
    };
    
    println!("Loading model to inspect...");
    let model = NerModel::new(config).await?;
    
    // Test extraction with various texts
    let texts = vec![
        "John Smith works at Google",
        "Apple Inc. is headquartered in Cupertino, California.",
        "Barack Obama was born in Hawaii.",
        "Microsoft CEO Satya Nadella announced new products.",
        "The Eiffel Tower is located in Paris, France.",
        "Elon Musk founded SpaceX.",
        "Dr. Jane Smith at Johns Hopkins Hospital.",
        "KRAS mutations are common in pancreatic cancer.",
        "The BRCA1 gene is associated with breast cancer.",
    ];
    
    for text in texts {
        println!("\n--- Testing: {} ---", text);
        let entities = model.extract(text)?;
        println!("Found {} entities:", entities.len());
        for e in &entities {
            println!("  {:?}: '{}' (score: {:.2})", e.entity_type, e.text, e.score);
        }
    }
    
    Ok(())
}
