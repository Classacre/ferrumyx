//! Example: Full Paper Pipeline with Real Data
//!
//! This example demonstrates fetching real full papers from Europe PMC
//! and extracting entities using trie-based NER.

use ferrumyx_ingestion::sources::europepmc::EuropePmcClient;
use ferrumyx_ingestion::sources::LiteratureSource;
use ferrumyx_ner::trie_ner::TrieNer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║     Ferrumyx: Full Paper Pipeline with Real Data             ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // Step 1: Initialize NER
    println!("🔧 Step 1: Initializing Entity Recognition");
    let ner = TrieNer::with_embedded_subset();
    let stats = ner.stats();
    println!("   ✓ Loaded {} genes, {} diseases, {} chemicals",
        stats.gene_count, stats.disease_count, stats.chemical_count);
    println!("   ✓ Total patterns: {}\n", stats.total_patterns);

    // Step 2: Search Europe PMC for real papers
    println!("🔍 Step 2: Searching Europe PMC");
    println!("   Query: \"EGFR\" AND \"lung cancer\"");
    
    let client = EuropePmcClient::new();
    let query = "EGFR AND \"lung cancer\"";
    
    match client.search(query, 5).await {
        Ok(papers) => {
            println!("   ✓ Found {} papers\n", papers.len());
            
            // Step 3: Process each paper
            println!("📚 Step 3: Processing Papers");
            
            let mut total_entities = 0;
            let mut all_genes: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut all_diseases: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut all_chemicals: std::collections::HashSet<String> = std::collections::HashSet::new();
            
            for (i, paper) in papers.iter().enumerate() {
                println!("\n   Paper {}:", i + 1);
                println!("   Title: {}", paper.title);
                if let Some(ref pmid) = paper.pmid {
                    println!("   PMID: {}", pmid);
                }
                if let Some(ref pmcid) = paper.pmcid {
                    println!("   PMCID: {}", pmcid);
                }
                
                // Try to get full text
                let text_to_process = if let Some(ref pmcid) = paper.pmcid {
                    match client.fetch_full_text(pmcid).await {
                        Ok(Some(full_text)) => {
                            println!("   Source: Full text XML ({} chars)", full_text.len());
                            full_text
                        }
                        _ => {
                            if let Some(ref abstract_text) = paper.abstract_text {
                                println!("   Source: Abstract only ({} chars)", abstract_text.len());
                                abstract_text.clone()
                            } else {
                                println!("   Source: No text available");
                                continue;
                            }
                        }
                    }
                } else if let Some(ref abstract_text) = paper.abstract_text {
                    println!("   Source: Abstract only ({} chars)", abstract_text.len());
                    abstract_text.clone()
                } else {
                    println!("   Source: No text available");
                    continue;
                };
                
                // Extract entities
                let entities = ner.extract(&text_to_process);
                
                if entities.is_empty() {
                    println!("   Entities: None found");
                } else {
                    // Group by type
                    let mut genes: Vec<&str> = Vec::new();
                    let mut diseases: Vec<&str> = Vec::new();
                    let mut chemicals: Vec<&str> = Vec::new();
                    
                    for entity in &entities {
                        match entity.label {
                            ferrumyx_ner::entity_types::EntityType::Gene => {
                                genes.push(&entity.text);
                                all_genes.insert(entity.text.clone());
                            }
                            ferrumyx_ner::entity_types::EntityType::Disease => {
                                diseases.push(&entity.text);
                                all_diseases.insert(entity.text.clone());
                            }
                            ferrumyx_ner::entity_types::EntityType::Chemical => {
                                chemicals.push(&entity.text);
                                all_chemicals.insert(entity.text.clone());
                            }
                            _ => {}
                        }
                    }
                    
                    // Remove duplicates
                    genes.sort();
                    genes.dedup();
                    diseases.sort();
                    diseases.dedup();
                    chemicals.sort();
                    chemicals.dedup();
                    
                    println!("   Entities found: {}", entities.len());
                    
                    if !genes.is_empty() {
                        println!("   Genes ({}): {}", genes.len(), genes.join(", "));
                    }
                    if !diseases.is_empty() {
                        println!("   Diseases ({}): {}", diseases.len(), diseases.join(", "));
                    }
                    if !chemicals.is_empty() {
                        println!("   Chemicals ({}): {}", chemicals.len(), chemicals.join(", "));
                    }
                    
                    total_entities += entities.len();
                }
            }
            
            // Step 4: Summary
            println!("\n📊 Step 4: Summary");
            println!("   Papers processed: {}", papers.len());
            println!("   Total entity mentions: {}", total_entities);
            println!("   Unique genes: {} - {:?}", all_genes.len(), all_genes);
            println!("   Unique diseases: {} - {:?}", all_diseases.len(), all_diseases);
            println!("   Unique chemicals: {} - {:?}", all_chemicals.len(), all_chemicals);
            
            println!("\n✅ Pipeline Complete!");
            println!("\nℹ️  Note: Results are from dictionary-based NER using:");
            println!("      - HGNC gene symbols");
            println!("      - MeSH disease terms");
            println!("      - ChEMBL drug names");
        }
        Err(e) => {
            eprintln!("   ✗ Error: {}", e);
            eprintln!("\n   This example requires internet access to Europe PMC API.");
        }
    }
    
    Ok(())
}
