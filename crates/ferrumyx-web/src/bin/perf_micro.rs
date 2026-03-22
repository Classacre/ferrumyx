use ferrumyx_db::chunks::ChunkRepository;
use ferrumyx_db::entities::EntityRepository;
use ferrumyx_db::schema::{Chunk, Entity, EntityType};
use ferrumyx_db::Database;
use ferrumyx_ingestion::repository::IngestionRepository;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

fn median_ms(samples: &mut [u128]) -> u128 {
    samples.sort_unstable();
    samples[samples.len() / 2]
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tmp_root = std::env::temp_dir().join(format!("ferrumyx-perf-micro-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&tmp_root)?;
    let db_path: PathBuf = tmp_root.join("db");

    let db = Arc::new(Database::open(&db_path).await?);
    db.initialize().await?;
    let _ = db.create_vector_index().await;

    let entity_repo = EntityRepository::new(db.clone());
    let chunk_repo = ChunkRepository::new(db.clone());
    let ingestion_repo = IngestionRepository::new(db.clone());

    let entity_count = 6_000usize;
    let lookup_count = 3_000usize;
    let mut entities = Vec::with_capacity(entity_count);
    for idx in 0..entity_count {
        let mut entity = Entity::new(
            EntityType::Gene,
            format!("GENE_{idx:05}"),
            format!("HGNC:{idx:05}"),
            "bench".to_string(),
        );
        entity.synonyms = Some(format!("[\"G{idx}\",\"GENE{idx:05}\"]"));
        entities.push(entity);
    }
    entity_repo.insert_batch(&entities).await?;
    let lookup_ids: Vec<Uuid> = entities.iter().take(lookup_count).map(|e| e.id).collect();

    let mut n_plus_one_runs = Vec::new();
    for _ in 0..5 {
        let t0 = Instant::now();
        let mut found = 0usize;
        for id in &lookup_ids {
            if entity_repo.find_by_id(*id).await?.is_some() {
                found += 1;
            }
        }
        assert_eq!(found, lookup_count);
        n_plus_one_runs.push(t0.elapsed().as_millis());
    }

    let mut batch_runs = Vec::new();
    for _ in 0..5 {
        let t0 = Instant::now();
        let names = entity_repo.find_names_by_ids(&lookup_ids).await?;
        assert_eq!(names.len(), lookup_count);
        batch_runs.push(t0.elapsed().as_millis());
    }

    let entity_n_plus_one_ms = median_ms(&mut n_plus_one_runs);
    let entity_batch_ms = median_ms(&mut batch_runs);
    let entity_speedup = if entity_batch_ms == 0 {
        0.0
    } else {
        entity_n_plus_one_ms as f64 / entity_batch_ms as f64
    };

    let paper_id = Uuid::new_v4();
    let chunk_count = 8_000usize;
    let mut chunks = Vec::with_capacity(chunk_count);
    for idx in 0..chunk_count {
        let embedding = if idx % 2 == 0 {
            Some(vec![0.001_f32; 768])
        } else {
            None
        };
        chunks.push(Chunk {
            id: Uuid::new_v4(),
            paper_id,
            chunk_index: idx as i64,
            token_count: 64,
            content: format!("chunk content {}", idx),
            section: Some("bench".to_string()),
            page: None,
            created_at: chrono::Utc::now(),
            embedding,
            embedding_large: None,
        });
    }
    chunk_repo.insert_batch(&chunks).await?;

    let mut old_path_runs = Vec::new();
    for _ in 0..5 {
        let t0 = Instant::now();
        let all = chunk_repo.find_by_paper_id(paper_id).await?;
        let pending: Vec<(Uuid, String)> = all
            .into_iter()
            .filter(|c| c.embedding.is_none())
            .map(|c| (c.id, c.content))
            .collect();
        assert_eq!(pending.len(), chunk_count / 2);
        old_path_runs.push(t0.elapsed().as_millis());
    }

    let mut new_path_runs = Vec::new();
    for _ in 0..5 {
        let t0 = Instant::now();
        let pending = ingestion_repo
            .find_chunks_without_embeddings(paper_id)
            .await?;
        assert_eq!(pending.len(), chunk_count / 2);
        new_path_runs.push(t0.elapsed().as_millis());
    }

    let chunk_old_ms = median_ms(&mut old_path_runs);
    let chunk_new_ms = median_ms(&mut new_path_runs);
    let chunk_speedup = if chunk_new_ms == 0 {
        0.0
    } else {
        chunk_old_ms as f64 / chunk_new_ms as f64
    };

    let embedding_update_count = 200usize;
    let mut embedding_row_runs = Vec::new();
    let mut embedding_bulk_runs = Vec::new();
    for _ in 0..2 {
        let paper_row = Uuid::new_v4();
        let mut row_chunks = Vec::with_capacity(embedding_update_count);
        for idx in 0..embedding_update_count {
            row_chunks.push(Chunk {
                id: Uuid::new_v4(),
                paper_id: paper_row,
                chunk_index: idx as i64,
                token_count: 64,
                content: format!("row-update chunk {}", idx),
                section: Some("bench".to_string()),
                page: None,
                created_at: chrono::Utc::now(),
                embedding: None,
                embedding_large: None,
            });
        }
        chunk_repo.insert_batch(&row_chunks).await?;
        let row_updates: Vec<(Uuid, Vec<f32>)> = row_chunks
            .iter()
            .map(|c| (c.id, vec![0.002_f32; 768]))
            .collect();

        let t_row = Instant::now();
        for (chunk_id, embedding) in &row_updates {
            ingestion_repo
                .update_chunk_embedding(*chunk_id, embedding.clone())
                .await?;
        }
        embedding_row_runs.push(t_row.elapsed().as_millis());
        assert!(ingestion_repo
            .find_chunks_without_embeddings(paper_row)
            .await?
            .is_empty());

        let paper_bulk = Uuid::new_v4();
        let mut bulk_chunks = Vec::with_capacity(embedding_update_count);
        for idx in 0..embedding_update_count {
            bulk_chunks.push(Chunk {
                id: Uuid::new_v4(),
                paper_id: paper_bulk,
                chunk_index: idx as i64,
                token_count: 64,
                content: format!("bulk-update chunk {}", idx),
                section: Some("bench".to_string()),
                page: None,
                created_at: chrono::Utc::now(),
                embedding: None,
                embedding_large: None,
            });
        }
        chunk_repo.insert_batch(&bulk_chunks).await?;
        let bulk_updates: Vec<(Uuid, Vec<f32>)> = bulk_chunks
            .iter()
            .map(|c| (c.id, vec![0.002_f32; 768]))
            .collect();

        let t_bulk = Instant::now();
        let updated = ingestion_repo.bulk_update_embeddings(&bulk_updates).await?;
        assert_eq!(updated, embedding_update_count);
        embedding_bulk_runs.push(t_bulk.elapsed().as_millis());
        assert!(ingestion_repo
            .find_chunks_without_embeddings(paper_bulk)
            .await?
            .is_empty());
    }

    let embedding_row_ms = median_ms(&mut embedding_row_runs);
    let embedding_bulk_ms = median_ms(&mut embedding_bulk_runs);
    let embedding_speedup = if embedding_bulk_ms == 0 {
        0.0
    } else {
        embedding_row_ms as f64 / embedding_bulk_ms as f64
    };

    println!("=== Ferrumyx Micro Perf Benchmark ===");
    println!(
        "Entity lookup ({} ids): N+1={}ms, batch={}ms, speedup={:.2}x",
        lookup_count, entity_n_plus_one_ms, entity_batch_ms, entity_speedup
    );
    println!(
        "Missing embeddings ({} chunks): old_scan={}ms, db_filter={}ms, speedup={:.2}x",
        chunk_count, chunk_old_ms, chunk_new_ms, chunk_speedup
    );
    println!(
        "Embedding writes ({} chunks): per_row={}ms, bulk={}ms, speedup={:.2}x",
        embedding_update_count, embedding_row_ms, embedding_bulk_ms, embedding_speedup
    );
    println!("Temp benchmark DB: {}", db_path.display());

    Ok(())
}
