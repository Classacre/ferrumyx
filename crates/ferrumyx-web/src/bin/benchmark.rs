use ferrumyx_ingestion::pipeline::{run_ingestion, IngestionJob, IngestionSourceSpec};
use ferrumyx_ingestion::repository::IngestionRepository;
use ferrumyx_kg::scoring::compute_target_scores;
use ferrumyx_molecules::pipeline::MoleculesPipeline;

use ferrumyx_web::state::AppState;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::broadcast;
use tokio::time::{timeout, Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // 1. Setup DB
    let state = AppState::new_without_db().await?;
    let db = state.db.clone();

    // 2. Ingestion benchmark
    let mut job = IngestionJob::default();
    let enable_scihub = std::env::var("FERRUMYX_BENCH_ENABLE_SCIHUB")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    job.enable_scihub_fallback = enable_scihub;
    job.gene = "BRCA1".to_string();
    job.mutation = None; // Important: Clear the default G12D mutation restriction
    job.cancer_type = "ovarian cancer".to_string();
    job.max_results = std::env::var("FERRUMYX_BENCH_MAX_RESULTS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(8)
        .clamp(1, 50);
    job.sources = vec![IngestionSourceSpec::PubMed, IngestionSourceSpec::EuropePmc];
    job.source_timeout_secs = Some(
        std::env::var("FERRUMYX_BENCH_SOURCE_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(20)
            .clamp(5, 120),
    );
    job.full_text_step_timeout_secs = Some(
        std::env::var("FERRUMYX_BENCH_FULLTEXT_STEP_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(12)
            .clamp(5, 120),
    );
    job.full_text_enabled = std::env::var("FERRUMYX_BENCH_FULL_TEXT")
        .ok()
        .is_none_or(|v| v == "1" || v.eq_ignore_ascii_case("true"));

    let repo = Arc::new(IngestionRepository::new(db.clone()));

    println!("=== Starting Benchmark ===");
    let start_total = Instant::now();

    let start_ing = Instant::now();
    let (tx, mut rx) = broadcast::channel(1024);
    let ingest_timeout = Duration::from_secs(
        std::env::var("FERRUMYX_BENCH_INGEST_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(900)
            .clamp(60, 7200),
    );
    let ingest_repo = repo.clone();
    let ingest_task = tokio::spawn(async move { run_ingestion(job, ingest_repo, Some(tx)).await });
    let result = loop {
        if ingest_task.is_finished() {
            break ingest_task.await?;
        }
        match timeout(Duration::from_secs(10), rx.recv()).await {
            Ok(Ok(p)) => {
                println!(
                    "[ingestion:{}] {} (found={}, inserted={}, chunks={})",
                    p.stage, p.message, p.papers_found, p.papers_inserted, p.chunks_inserted
                );
            }
            Ok(Err(_)) => {}
            Err(_) => {
                println!(
                    "[ingestion] still running... elapsed={:.1}s",
                    start_ing.elapsed().as_secs_f32()
                );
            }
        }
        if start_ing.elapsed() > ingest_timeout {
            ingest_task.abort();
            anyhow::bail!("ingestion benchmark timeout after {:?}", ingest_timeout);
        }
    };
    println!(
        "Ingestion (+ NER) took: {:.2?} (Inserted {} papers out of {} found)",
        start_ing.elapsed(),
        result.papers_inserted,
        result.papers_found
    );

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
    println!(
        "KG Fact Extraction & Scoring took: {:.2?} (Scored {} targets, extracted {} facts)",
        start_kg.elapsed(),
        scored_targets,
        db_facts.len()
    );

    let top_gene = "P38398"; // BRCA1 Uniprot ID

    let skip_molecules = std::env::var("FERRUMYX_BENCH_SKIP_MOLECULES")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    if !skip_molecules {
        let start_mol = Instant::now();
        // Molecules Pipeline
        let pipeline = MoleculesPipeline::new(".kilocode/cache");
        match pipeline.run(top_gene).await {
            Ok(res) => {
                println!("Molecules Pipeline (Fetch PDB, Pocket, Ligand, Vina, Scoring) took: {:.2?} (Generated {} molecules)", start_mol.elapsed(), res.len());
                for (i, m) in res.iter().enumerate() {
                    println!(
                        "  Molecule {}: SMILES={}, Score={:.4}",
                        i + 1,
                        m.molecule.smiles,
                        m.composite_score
                    );
                }
            }
            Err(e) => println!("Molecules Pipeline Error: {}", e),
        }
    } else {
        println!("Molecules pipeline skipped (FERRUMYX_BENCH_SKIP_MOLECULES=1)");
    }

    println!("Total workflow took: {:.2?}", start_total.elapsed());

    Ok(())
}
