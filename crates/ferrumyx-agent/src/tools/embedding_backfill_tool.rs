use async_trait::async_trait;
use ferrumyx_db::Database;
use ferrumyx_ingestion::embedding::{
    embed_pending_chunks_for_papers, EmbeddingClient, EmbeddingConfig as IngestionEmbeddingConfig,
};
use ferrumyx_ingestion::repository::IngestionRepository;
use ferrumyx_runtime::context::JobContext;
use ferrumyx_runtime::tools::{Tool, ToolError, ToolOutput};
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

use super::ingestion_tool::{
    load_runtime_defaults, resolve_embedding_runtime, ResolvedEmbeddingRuntime,
};
use super::runtime_profile::RuntimeProfile;

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct BackfillEmbeddingsReport {
    pub papers_processed: usize,
    pub papers_selected: usize,
    pub papers_embedded: usize,
    pub chunks_embedded: usize,
    pub scan_limit: Option<usize>,
    pub throughput_chunk_cap: Option<usize>,
    pub errors: Vec<String>,
}

pub(crate) async fn backfill_embeddings_for_papers(
    repo: Arc<IngestionRepository>,
    embedding_cfg: IngestionEmbeddingConfig,
    paper_ids: Vec<Uuid>,
    scan_limit: Option<usize>,
) -> anyhow::Result<BackfillEmbeddingsReport> {
    let client = EmbeddingClient::new(embedding_cfg);
    let target_paper_ids = resolve_backfill_targets(&repo, paper_ids, scan_limit).await?;
    let mut report = BackfillEmbeddingsReport {
        papers_processed: target_paper_ids.len(),
        papers_selected: target_paper_ids.len(),
        papers_embedded: 0,
        chunks_embedded: 0,
        scan_limit,
        throughput_chunk_cap: resolve_throughput_embedding_chunk_cap(),
        errors: Vec::new(),
    };

    if target_paper_ids.is_empty() {
        return Ok(report);
    }

    let mut papers_with_pending = 0usize;
    for paper_id in &target_paper_ids {
        if !repo.find_chunks_without_embeddings(*paper_id).await?.is_empty() {
            papers_with_pending += 1;
        }
    }

    match embed_pending_chunks_for_papers(&client, repo.as_ref(), &target_paper_ids).await {
        Ok(embedded) => {
            report.chunks_embedded = embedded;
            if embedded > 0 {
                report.papers_embedded = papers_with_pending;
            }
        }
        Err(e) => {
            report
                .errors
                .push(format!("global embed backfill failed: {e}"));
        }
    }

    Ok(report)
}

#[derive(Debug, Default, Clone)]
struct BackfillInput {
    paper_ids: Vec<Uuid>,
    scan_limit: Option<usize>,
}

pub struct BackfillEmbeddingsTool {
    db: Arc<Database>,
}

impl BackfillEmbeddingsTool {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl Tool for BackfillEmbeddingsTool {
    fn name(&self) -> &str {
        "backfill_embeddings"
    }

    fn description(&self) -> &str {
        "Backfills pending embeddings for papers by paper_id list and/or bounded scan."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "paper_ids": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional list of paper UUIDs to backfill"
                },
                "scan_limit": {
                    "type": "integer",
                    "description": "Optional bounded paper scan for pending embeddings"
                }
            }
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let defaults = load_runtime_defaults();
        let profile = RuntimeProfile::detect_and_prepare();
        let input = parse_input(&params)?;
        let effective_scan_limit = input
            .scan_limit
            .or(Some(defaults.max_results))
            .map(|n| n.clamp(1, 5_000));
        let requested_max_results = input
            .paper_ids
            .len()
            .max(effective_scan_limit.unwrap_or(defaults.max_results))
            .max(1);
        let perf_mode = match defaults.perf_mode.as_str() {
            "throughput" => "throughput",
            "balanced" => "balanced",
            "safe" => "safe",
            _ => "auto",
        };
        let resolved = resolve_embedding_runtime(
            &defaults,
            &profile,
            perf_mode,
            requested_max_results,
        );
        let ResolvedEmbeddingRuntime {
            cfg: embedding_cfg,
            speed_mode,
            async_backfill_enabled,
            throughput_chunk_cap,
            ..
        } = resolved;
        let Some(embedding_cfg) = embedding_cfg else {
            return Err(ToolError::ExecutionFailed(
                "embedding is not configured for this runtime".to_string(),
            ));
        };

        let started = std::time::Instant::now();
        let repo = Arc::new(IngestionRepository::new(self.db.clone()));
        let report = backfill_embeddings_for_papers(
            repo,
            embedding_cfg,
            input.paper_ids,
            effective_scan_limit,
        )
        .await
        .map_err(|e| ToolError::ExecutionFailed(format!("backfill failed: {e}")))?;

        Ok(ToolOutput::success(
            json!({
                "status": "ok",
                "async_backfill_enabled": async_backfill_enabled,
                "embedding_speed_mode": speed_mode,
                "throughput_chunk_cap": throughput_chunk_cap,
                "scan_limit": report.scan_limit,
                "papers_processed": report.papers_processed,
                "papers_selected": report.papers_selected,
                "papers_embedded": report.papers_embedded,
                "chunks_embedded": report.chunks_embedded,
                "errors": report.errors,
            }),
            started.elapsed(),
        ))
    }
}

async fn resolve_backfill_targets(
    repo: &IngestionRepository,
    paper_ids: Vec<Uuid>,
    scan_limit: Option<usize>,
) -> anyhow::Result<Vec<Uuid>> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for paper_id in paper_ids {
        if seen.insert(paper_id) {
            out.push(paper_id);
        }
    }

    if let Some(limit) = scan_limit {
        for paper_id in repo.pending_embedding_paper_ids(limit).await? {
            if seen.insert(paper_id) {
                out.push(paper_id);
            }
        }
    }

    Ok(out)
}

fn parse_input(params: &serde_json::Value) -> Result<BackfillInput, ToolError> {
    let mut paper_ids = Vec::new();
    if let Some(raw_ids) = params.get("paper_ids").and_then(|v| v.as_array()) {
        for value in raw_ids {
            let raw = value
                .as_str()
                .ok_or_else(|| ToolError::InvalidParameters("paper_ids must be strings".to_string()))?;
            let paper_id = Uuid::parse_str(raw).map_err(|e| {
                ToolError::InvalidParameters(format!("invalid paper_id '{raw}': {e}"))
            })?;
            paper_ids.push(paper_id);
        }
    }

    let scan_limit = params
        .get("scan_limit")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .filter(|n| *n > 0);

    Ok(BackfillInput {
        paper_ids,
        scan_limit,
    })
}

fn resolve_throughput_embedding_chunk_cap() -> Option<usize> {
    std::env::var("FERRUMYX_EMBED_THROUGHPUT_MAX_CHUNKS_PER_PAPER")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map(|v| v.clamp(1, 4_096))
}
