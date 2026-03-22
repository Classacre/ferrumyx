use anyhow::{bail, Context};
use ferrumyx_db::{
    validate_contribution_package, ChunkRepository, Database, EntityMentionRepository,
    EntityRepository, KgFactRepository, PaperRepository, TargetScoreRepository,
};
use serde::de::DeserializeOwned;
use std::fs::File;
use std::future::Future;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

const BATCH_SIZE: usize = 1_000;

#[derive(Debug, Default)]
struct ImportSummary {
    papers: usize,
    entities: usize,
    kg_facts: usize,
    target_scores: usize,
    chunks: usize,
    entity_mentions: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (package_dir, data_dir) = parse_args()?;

    let report = validate_contribution_package(&package_dir)
        .with_context(|| format!("failed to validate package at {}", package_dir.display()))?;
    if !report.valid {
        bail!(
            "package validation failed for {} (manifest/signature/artifact checks did not pass)",
            package_dir.display()
        );
    }

    let db = Database::open(&data_dir)
        .await
        .with_context(|| format!("failed to open database at {}", data_dir.display()))?;
    db.initialize()
        .await
        .context("failed to initialize database tables")?;
    let db = Arc::new(db);

    let papers_repo = PaperRepository::new(db.clone());
    let entities_repo = EntityRepository::new(db.clone());
    let kg_repo = KgFactRepository::new(db.clone());
    let scores_repo = TargetScoreRepository::new(db.clone());
    let chunks_repo = ChunkRepository::new(db.clone());
    let mentions_repo = EntityMentionRepository::new(db);

    let mut summary = ImportSummary::default();

    summary.papers = import_jsonl_batch(&package_dir.join("papers.jsonl"), |rows| {
        let repo = papers_repo.clone();
        Box::pin(async move {
            repo.insert_batch(rows).await?;
            Ok(rows.len())
        })
    })
    .await?;

    summary.entities = import_jsonl_batch(&package_dir.join("entities.jsonl"), |rows| {
        let repo = entities_repo.clone();
        Box::pin(async move {
            repo.insert_batch(rows).await?;
            Ok(rows.len())
        })
    })
    .await?;

    summary.kg_facts = import_jsonl_batch(&package_dir.join("kg_facts.jsonl"), |rows| {
        let repo = kg_repo.clone();
        Box::pin(async move {
            repo.insert_batch(rows).await?;
            Ok(rows.len())
        })
    })
    .await?;

    summary.target_scores = import_jsonl_batch(&package_dir.join("target_scores.jsonl"), |rows| {
        let repo = scores_repo.clone();
        Box::pin(async move { Ok(repo.upsert_batch(rows).await?) })
    })
    .await?;

    summary.chunks = import_jsonl_batch(&package_dir.join("chunks.jsonl"), |rows| {
        let repo = chunks_repo.clone();
        Box::pin(async move {
            repo.insert_batch(rows).await?;
            Ok(rows.len())
        })
    })
    .await?;

    summary.entity_mentions =
        import_jsonl_batch(&package_dir.join("entity_mentions.jsonl"), |rows| {
            let repo = mentions_repo.clone();
            Box::pin(async move {
                repo.insert_batch(rows).await?;
                Ok(rows.len())
            })
        })
        .await?;

    let final_counts = fetch_final_counts(
        &papers_repo,
        &entities_repo,
        &kg_repo,
        &scores_repo,
        &chunks_repo,
        &mentions_repo,
    )
    .await?;

    println!("federation_import package_dir={}", package_dir.display());
    println!("loaded_rows papers={} entities={} kg_facts={} target_scores={} chunks={} entity_mentions={}",
        summary.papers,
        summary.entities,
        summary.kg_facts,
        summary.target_scores,
        summary.chunks,
        summary.entity_mentions
    );
    println!(
        "db_counts papers={} entities={} kg_facts={} target_scores={} chunks={} entity_mentions={}",
        final_counts.papers,
        final_counts.entities,
        final_counts.kg_facts,
        final_counts.target_scores,
        final_counts.chunks,
        final_counts.entity_mentions
    );

    Ok(())
}

fn parse_args() -> anyhow::Result<(PathBuf, PathBuf)> {
    let mut args = std::env::args_os().skip(1);

    let package_dir = args
        .next()
        .map(PathBuf::from)
        .context("usage: federation_import <package_dir> [data_dir]")?;
    let explicit_data_dir = args.next().map(PathBuf::from);
    if args.next().is_some() {
        bail!("usage: federation_import <package_dir> [data_dir]");
    }

    let data_dir = explicit_data_dir.unwrap_or_else(|| {
        std::env::var("FERRUMYX_DATA_DIR")
            .ok()
            .and_then(|s| {
                let trimmed = s.trim().to_string();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(trimmed))
                }
            })
            .unwrap_or_else(|| PathBuf::from("./data"))
    });

    Ok((package_dir, data_dir))
}

async fn import_jsonl_batch<T, F>(path: &Path, mut sink: F) -> anyhow::Result<usize>
where
    T: DeserializeOwned,
    F: for<'a> FnMut(&'a [T]) -> Pin<Box<dyn Future<Output = anyhow::Result<usize>> + 'a>>,
{
    if !path.exists() {
        return Ok(0);
    }

    let file =
        File::open(path).with_context(|| format!("failed to open artifact {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut batch: Vec<T> = Vec::with_capacity(BATCH_SIZE);
    let mut total = 0usize;

    for (line_no, line) in reader.lines().enumerate() {
        let line = line.with_context(|| {
            format!(
                "failed to read line {} from {}",
                line_no + 1,
                path.display()
            )
        })?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row: T = serde_json::from_str(trimmed).with_context(|| {
            format!(
                "failed to parse JSON at line {} in {}",
                line_no + 1,
                path.display()
            )
        })?;
        batch.push(row);

        if batch.len() >= BATCH_SIZE {
            total += flush_batch(&mut batch, &mut sink).await?;
        }
    }

    total += flush_batch(&mut batch, &mut sink).await?;
    Ok(total)
}

async fn flush_batch<T, F>(batch: &mut Vec<T>, sink: &mut F) -> anyhow::Result<usize>
where
    F: for<'a> FnMut(&'a [T]) -> Pin<Box<dyn Future<Output = anyhow::Result<usize>> + 'a>>,
{
    if batch.is_empty() {
        return Ok(0);
    }

    let len = batch.len();
    let rows = std::mem::take(batch);
    let imported = sink(&rows).await?;
    if imported != len {
        Ok(imported)
    } else {
        Ok(len)
    }
}

#[derive(Debug, Default)]
struct FinalCounts {
    papers: u64,
    entities: u64,
    kg_facts: u64,
    target_scores: u64,
    chunks: u64,
    entity_mentions: u64,
}

async fn fetch_final_counts(
    papers_repo: &PaperRepository,
    entities_repo: &EntityRepository,
    kg_repo: &KgFactRepository,
    scores_repo: &TargetScoreRepository,
    chunks_repo: &ChunkRepository,
    mentions_repo: &EntityMentionRepository,
) -> anyhow::Result<FinalCounts> {
    Ok(FinalCounts {
        papers: papers_repo.count().await?,
        entities: entities_repo.count().await?,
        kg_facts: kg_repo.count().await?,
        target_scores: scores_repo.count().await?,
        chunks: chunks_repo.count().await?,
        entity_mentions: mentions_repo.count().await?,
    })
}
