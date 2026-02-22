//! Test end-to-end ingestion pipeline.
//!
//! Requires database connection. Run with:
//! ```bash
//! cargo test --package ferrumyx-ingestion --test test_ingestion_e2e -- --ignored --nocapture
//! ```

use std::sync::Arc;
use sqlx::postgres::PgPoolOptions;
use ferrumyx_ingestion::pipeline::{IngestionJob, IngestionSourceSpec, run_ingestion};
use ferrumyx_ingestion::pg_repository::PgIngestionRepository;
use ferrumyx_ingestion::embedding::{EmbeddingConfig, EmbeddingBackend};

#[tokio::test(flavor = "multi_thread")]
#[ignore] // Requires database connection
async fn test_ingestion_kras_pancreatic() {
    // Initialize logging (optional)
    // let _ = tracing_subscriber::fmt::try_init();
    
    // Connect to database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrumyx:ferrumyx@localhost:5432/ferrumyx?sslmode=disable".to_string());
    
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");
    
    let repo = Arc::new(PgIngestionRepository::new(pool));
    
    // Get initial counts
    let initial_papers = repo.paper_count().await.unwrap();
    let initial_chunks = repo.chunk_count().await.unwrap();
    println!("Initial: {} papers, {} chunks", initial_papers, initial_chunks);
    
    // Configure ingestion job
    let job = IngestionJob {
        gene: "KRAS".to_string(),
        mutation: Some("G12D".to_string()),
        cancer_type: "pancreatic cancer".to_string(),
        max_results: 10,
        sources: vec![IngestionSourceSpec::PubMed],
        pubmed_api_key: None,
        embedding_cfg: Some(EmbeddingConfig {
            backend: EmbeddingBackend::RustNative,
            model: "NeuML/pubmedbert-base-embeddings".to_string(),
            dim: 768,
            batch_size: 8,
            ..Default::default()
        }),
    };
    
    // Run ingestion
    let result = run_ingestion(job, repo.clone(), None).await;
    
    println!("\n=== Ingestion Result ===");
    println!("Job ID: {}", result.job_id);
    println!("Query: {}", result.query);
    println!("Papers found: {}", result.papers_found);
    println!("Papers inserted: {}", result.papers_inserted);
    println!("Duplicates skipped: {}", result.papers_duplicate);
    println!("Chunks inserted: {}", result.chunks_inserted);
    println!("Chunks embedded: {}", result.chunks_embedded);
    println!("Duration: {}ms", result.duration_ms);
    if !result.errors.is_empty() {
        println!("Errors: {:?}", result.errors);
    }
    
    // Verify database state
    let final_papers = repo.paper_count().await.unwrap();
    let final_chunks = repo.chunk_count().await.unwrap();
    println!("\nFinal: {} papers, {} chunks", final_papers, final_chunks);
    
    // Should have inserted at least some papers
    assert!(result.papers_found > 0, "Should find papers from PubMed");
    assert!(result.duration_ms > 0, "Should have taken some time");
}
