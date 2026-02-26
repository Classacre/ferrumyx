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
    job.mutation = None; // Important: Clear the default G12D mutation restriction
    job.cancer_type = "ovarian cancer".to_string();
    job.max_results = 20; // Enough to test the pipeline
    job.sources = vec![IngestionSourceSpec::PubMed, IngestionSourceSpec::EuropePmc];
    
    let repo = Arc::new(IngestionRepository::new(db.clone()));
    
    println!("=== Starting Benchmark ===");
    let start_total = Instant::now();
    
    let start_ing = Instant::now();
    let result = run_ingestion(job, repo, None).await;
    println!("Ingestion (+ NER) took: {:.2?} (Inserted {} papers out of {} found)", start_ing.elapsed(), result.papers_inserted, result.papers_found);
    
    let start_kg = Instant::now();
    
    // Simulate KG Extraction Phase:
    // Read chunks from DB, extract facts, and write them back
    let chunk_repo = ferrumyx_db::chunks::ChunkRepository::new(db.clone());
    let kg_repo = ferrumyx_db::kg_facts::KgFactRepository::new(db.clone());
    
    let chunks = chunk_repo.list(0, 1000).await.unwrap_or_default();
    let mut db_facts = Vec::new();
    let dummy_uuid = uuid::Uuid::new_v4();
    
    for chunk in chunks {
        let extracted = ferrumyx_kg::extraction::build_facts("BRCA1", &chunk.content);
        for fact in extracted {
            db_facts.push(ferrumyx_db::schema::KgFact::new(
                chunk.paper_id,
                dummy_uuid, // Not fully resolved in extraction yet
                fact.subject,
                fact.fact_type, // e.g. gene_cancer, gene_mutation
                dummy_uuid,
                fact.object,
            ));
        }
    }
    
    if !db_facts.is_empty() {
        let _ = kg_repo.insert_batch(&db_facts).await;
    }

    let scored_targets = compute_target_scores(db.clone()).await.unwrap_or(0);
    println!("KG Fact Extraction & Scoring took: {:.2?} (Scored {} targets, extracted {} facts)", start_kg.elapsed(), scored_targets, db_facts.len());
    
    let top_gene = "P38398"; // BRCA1 Uniprot ID
    
    let start_mol = Instant::now();
    // Molecules Pipeline
    let pipeline = MoleculesPipeline::new(".kilocode/cache");
    match pipeline.run(top_gene).await {
        Ok(res) => {
            println!("Molecules Pipeline (Fetch PDB, Pocket, Ligand, Vina, Scoring) took: {:.2?} (Generated {} molecules)", start_mol.elapsed(), res.len());
            for (i, m) in res.iter().enumerate() {
                println!("  Molecule {}: SMILES={}, Score={:.4}", i+1, m.molecule.smiles, m.composite_score);
            }
        },
        Err(e) => println!("Molecules Pipeline Error: {}", e),
    }

    println!("Total workflow took: {:.2?}", start_total.elapsed());
    
    Ok(())
}
