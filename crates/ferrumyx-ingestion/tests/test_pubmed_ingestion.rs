//! Test ingestion pipeline with real PubMed search.
//!
//! Run with: cargo test --package ferrumyx-ingestion --test test_pubmed_ingestion -- --ignored --nocapture

use ferrumyx_ingestion::sources::pubmed::PubMedClient;
use ferrumyx_ingestion::sources::LiteratureSource;

#[tokio::test]
#[ignore] // Requires network access
async fn test_pubmed_search_kras() {
    let client = PubMedClient::new(None);
    
    let papers = client
        .search("KRAS[tiab] AND pancreatic cancer[tiab]", 5)
        .await
        .expect("PubMed search failed");
    
    println!("Found {} papers", papers.len());
    for paper in &papers {
        println!("\n---");
        println!("Title: {}", paper.title);
        println!("PMID: {:?}", paper.pmid);
        println!("Abstract: {:?}", paper.abstract_text.as_ref().map(|s| {
            if s.len() > 200 { &s[..200] } else { s }
        }));
    }
    
    assert!(!papers.is_empty(), "Should find at least one paper");
}
