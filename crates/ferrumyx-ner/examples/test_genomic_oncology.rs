//! Test loading genomic and oncology models

use ferrumyx_ner::{NerModel, NerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    println!("=== Testing Genomic Model (SnowMed-568M - XLM-RoBERTa) ===\n");
    let genomic_config = NerConfig::genomic();
    println!("Model ID: {}", genomic_config.model_id);
    println!("Max length: {}", genomic_config.max_length);
    println!("Loading...");
    
    let start = std::time::Instant::now();
    match NerModel::new(genomic_config).await {
        Ok(model) => {
            println!("✅ Genomic model loaded in {:?}", start.elapsed());
            println!("   Architecture: {:?}", model.architecture());
            println!("   Model ID: {}", model.model_id());
            
            // Test extraction with cell line names (what this model detects)
            let test_texts = vec![
                "HeLa cells are widely used in research.",
                "The MCF-7 cell line is derived from breast cancer.",
                "We used A549 and HCT116 cells for the experiment.",
                "MDA-MB-231 is a triple-negative breast cancer cell line.",
            ];
            
            for test_text in test_texts {
                println!("\n   Testing: '{}'", test_text);
                match model.extract(test_text) {
                    Ok(entities) => {
                        println!("   Found {} entities:", entities.len());
                        for e in &entities {
                            println!("     {}: '{}' [score: {:.2}]", e.label, e.text, e.score);
                        }
                    }
                    Err(e) => println!("   ⚠️ Extraction error: {:?}", e),
                }
            }
            
            // Print label info
            println!("\n   Model labels: {:?}", model.labels());
        }
        Err(e) => {
            println!("❌ Failed to load genomic model: {:?}", e);
        }
    }
    
    println!("\n\n=== Testing Oncology Model (SuperMedical-355M - RoBERTa) ===\n");
    let oncology_config = NerConfig::oncology();
    println!("Model ID: {}", oncology_config.model_id);
    println!("Max length: {}", oncology_config.max_length);
    println!("Loading...");
    
    let start = std::time::Instant::now();
    match NerModel::new(oncology_config).await {
        Ok(model) => {
            println!("✅ Oncology model loaded in {:?}", start.elapsed());
            println!("   Architecture: {:?}", model.architecture());
            println!("   Model ID: {}", model.model_id());
            
            // Test extraction with various oncology texts
            let test_texts = vec![
                "Metastatic melanoma and non-small cell lung carcinoma show response to immunotherapy.",
                "The patient was diagnosed with breast cancer and treated with chemotherapy.",
                "Pembrolizumab is used for treating advanced melanoma.",
                "Lung cancer patients often have EGFR mutations.",
            ];
            
            for test_text in test_texts {
                println!("\n   Testing: '{}'", test_text);
                match model.extract(test_text) {
                    Ok(entities) => {
                        println!("   Found {} entities:", entities.len());
                        for e in &entities {
                            println!("     {}: '{}' [score: {:.2}]", e.label, e.text, e.score);
                        }
                    }
                    Err(e) => println!("   ⚠️ Extraction error: {:?}", e),
                }
            }
            
            // Print label info
            println!("\n   Model labels: {:?}", model.labels());
        }
        Err(e) => {
            println!("❌ Failed to load oncology model: {:?}", e);
        }
    }
    
    println!("\n\n=== All Tests Complete ===");
    Ok(())
}
