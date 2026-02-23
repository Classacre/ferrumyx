//! Test the NER model with OpenMed biomedical models

use ferrumyx_ner::{NerModel, NerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    // Test OpenMed disease NER
    println!("=== OpenMed Disease NER ===\n");
    
    let disease_model = NerModel::new(NerConfig::diseases()).await?;
    println!("Loaded model: {}", disease_model.model_id());
    println!("Labels: {:?}", disease_model.labels());
    
    let texts = vec![
        "The patient was diagnosed with non-small cell lung carcinoma.",
        "Patients with diabetes mellitus show elevated blood glucose.",
        "Alzheimer's disease is characterized by amyloid plaques.",
    ];
    
    for text in &texts {
        println!("\nText: {}", text);
        let entities = disease_model.extract(text)?;
        println!("Found {} entities", entities.len());
        for e in &entities {
            println!("  {} ({:?}): '{}' [score: {:.2}]", e.label, e.entity_type, e.text, e.score);
        }
    }
    
    // Test OpenMed genomic NER
    println!("\n\n=== OpenMed Genomic NER ===\n");
    
    let genomic_model = NerModel::new(NerConfig::genomic()).await?;
    println!("Loaded model: {}", genomic_model.model_id());
    println!("Labels: {:?}", genomic_model.labels());
    
    let genomic_texts = vec![
        "HeLa cells are widely used in cancer research.",
        "The MCF-7 cell line is derived from breast cancer.",
        "We used A549 and HCT116 cells for the experiment.",
        "MDA-MB-231 is a triple-negative breast cancer cell line.",
    ];
    
    for text in &genomic_texts {
        println!("\nText: {}", text);
        let entities = genomic_model.extract(text)?;
        println!("Found {} entities", entities.len());
        for e in &entities {
            println!("  {} ({:?}): '{}' [score: {:.2}]", e.label, e.entity_type, e.text, e.score);
        }
    }
    
    // Test OpenMed oncology NER
    println!("\n\n=== OpenMed Oncology NER ===\n");
    
    let oncology_model = NerModel::new(NerConfig::oncology()).await?;
    println!("Loaded model: {}", oncology_model.model_id());
    println!("Labels: {:?}", oncology_model.labels());
    
    let oncology_texts = vec![
        "The patient was diagnosed with stage III non-small cell lung carcinoma.",
        "Metastatic breast cancer with HER2 amplification was confirmed.",
        "The tumor showed evidence of vascular invasion and lymph node metastasis.",
    ];
    
    for text in &oncology_texts {
        println!("\nText: {}", text);
        let entities = oncology_model.extract(text)?;
        println!("Found {} entities", entities.len());
        for e in &entities {
            println!("  {} ({:?}): '{}' [score: {:.2}]", e.label, e.entity_type, e.text, e.score);
        }
    }
    
    // Test OpenMed pharmaceutical NER
    println!("\n\n=== OpenMed Pharmaceutical NER ===\n");
    
    let pharma_model = NerModel::new(NerConfig::pharmaceuticals()).await?;
    println!("Loaded model: {}", pharma_model.model_id());
    println!("Labels: {:?}", pharma_model.labels());
    
    let pharma_texts = vec![
        "The patient was prescribed 500mg of metformin twice daily.",
        "Treatment with erlotinib showed efficacy in lung cancer.",
    ];
    
    for text in &pharma_texts {
        println!("\nText: {}", text);
        let entities = pharma_model.extract(text)?;
        println!("Found {} entities", entities.len());
        for e in &entities {
            println!("  {} ({:?}): '{}' [score: {:.2}]", e.label, e.entity_type, e.text, e.score);
        }
    }
    
    println!("\nDone!");
    Ok(())
}
