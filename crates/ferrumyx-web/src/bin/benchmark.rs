use ferrumyx_ingestion::pipeline::{IngestionJob, run_ingestion, IngestionSourceSpec};
use ferrumyx_ingestion::repository::IngestionRepository;
use ferrumyx_kg::scoring::compute_target_scores;
use ferrumyx_molecules::pipeline::MoleculesPipeline;

use ferrumyx_web::state::AppState;
use std::time::Instant;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    // 1. Setup DB
    let state = AppState::new_without_db().await?;
    let db = state.db.clone();
    
    // 2. Ingestion with scihub
    let mut job = IngestionJob::default();
    job.enable_scihub_fallback = true;
    job.gene = "BRCA1".to_string();
    job.cancer_type = "ovarian cancer".to_string();
    job.max_results = 5; // A bit more for realistic benchmarking
    job.sources = vec![IngestionSourceSpec::PubMed, IngestionSourceSpec::EuropePmc];
    
    let repo = Arc::new(IngestionRepository::new(db.clone()));
    
    println!("=== Starting Benchmark ===");
    let start_total = Instant::now();
    
    let start_ing = Instant::now();
    let result = run_ingestion(job, repo, None).await;
    println!("Ingestion (+ NER) took: {:.2?} (Inserted {} papers out of {} found)", start_ing.elapsed(), result.papers_inserted, result.papers_found);
    
    let start_kg = Instant::now();
    let scored_targets = compute_target_scores(db.clone()).await.unwrap_or(0);
    println!("KG Fact Scoring computation took: {:.2?} (Scored {} targets)", start_kg.elapsed(), scored_targets);
    
    let top_gene = "P38398"; // BRCA1 Uniprot ID
    
    let start_mol = Instant::now();
    // Molecules Pipeline
    let pipeline = MoleculesPipeline::new(".kilocode/cache");
    match pipeline.run(top_gene).await {
        Ok(res) => println!("Molecules Pipeline (Fetch PDB, Pocket, Ligand, Vina, Scoring) took: {:.2?} (Generated {} molecules)", start_mol.elapsed(), res.len()),
        Err(e) => println!("Molecules Pipeline Error: {}", e),
    }

    println!("Total workflow took: {:.2?}", start_total.elapsed());
    
    Ok(())
}
