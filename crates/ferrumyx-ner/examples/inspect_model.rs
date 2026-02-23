//! Inspect the NER model structure

use ferrumyx_ner::{NerModel, NerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    // Test with OpenMed disease NER model (has safetensors)
    let config = NerConfig::diseases();
    
    println!("Loading OpenMed disease NER model: {}...", config.model_id);
    let start = std::time::Instant::now();
    let model = NerModel::new(config).await?;
    println!("Model loaded in {:?}", start.elapsed());
    
    // Test extraction with biomedical texts
    let texts = vec![
        "The patient was diagnosed with diabetes mellitus.",
        "Patients with non-small cell lung carcinoma show resistance to therapy.",
        "Alzheimer's disease is characterized by amyloid plaques.",
        "KRAS G12D mutations are common in pancreatic cancer.",
        "The BRCA1 gene is associated with breast and ovarian cancer.",
    ];
    
    for text in texts {
        println!("\n--- Testing: {} ---", text);
        match model.extract(text) {
            Ok(entities) => {
                println!("Found {} entities:", entities.len());
                for e in &entities {
                    println!("  {} ({:?}): '{}' [score: {:.2}]", e.label, e.entity_type, e.text, e.score);
                }
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
    
    println!("\n\n=== Testing Genomic NER Model (XLM-RoBERTa) ===\n");
    let genomic_config = NerConfig::genomic();
    println!("Loading genomic NER model: {}...", genomic_config.model_id);
    let genomic_start = std::time::Instant::now();
    let genomic_model = NerModel::new(genomic_config).await?;
    println!("Model loaded in {:?}", genomic_start.elapsed());
    
    let genomic_texts = vec![
        "The patient was prescribed 500mg of metformin twice daily.",
        "KRAS G12D mutations confer resistance to EGFR inhibitors.",
    ];
    
    for text in genomic_texts {
        println!("\n--- Testing: {} ---", text);
        match genomic_model.extract(text) {
            Ok(entities) => {
                println!("Found {} entities:", entities.len());
                for e in &entities {
                    println!("  {} ({:?}): '{}' [score: {:.2}]", e.label, e.entity_type, e.text, e.score);
                }
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
    
    println!("\n\n=== Testing Oncology NER Model (RoBERTa) ===\n");
    let oncology_config = NerConfig::oncology();
    println!("Loading oncology NER model: {}...", oncology_config.model_id);
    let oncology_start = std::time::Instant::now();
    let oncology_model = NerModel::new(oncology_config).await?;
    println!("Model loaded in {:?}", oncology_start.elapsed());
    
    let oncology_texts = vec![
        "The tumor showed high expression of HER2 in breast cancer patients.",
        "Metastatic melanoma responds to checkpoint inhibitors.",
    ];
    
    for text in oncology_texts {
        println!("\n--- Testing: {} ---", text);
        match oncology_model.extract(text) {
            Ok(entities) => {
                println!("Found {} entities:", entities.len());
                for e in &entities {
                    println!("  {} ({:?}): '{}' [score: {:.2}]", e.label, e.entity_type, e.text, e.score);
                }
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
    
    println!("\nDone!");
    Ok(())
}
