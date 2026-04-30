
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, error};
use uuid::Uuid;

use crate::pipeline::{IngestionJob as PipelineJob, IngestionResult, IngestionProgress, run_ingestion};
use crate::repository::IngestionRepository;

/// IronClaw-compatible ingestion job wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IronClawIngestionJob {
    pub pipeline_job: PipelineJob,
    pub job_id: Uuid,
}

impl IronClawIngestionJob {
    pub fn new(pipeline_job: PipelineJob) -> Self {
        Self {
            pipeline_job,
            job_id: Uuid::new_v4(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.job_id
    }

    pub fn name(&self) -> &str {
        "ferrumyx_ingestion"
    }

    pub async fn execute(&self, repo: Arc<IngestionRepository>) -> anyhow::Result<IngestionResult> {
        info!("Starting IronClaw ingestion job {}", self.job_id);

        // Create progress sender for broadcast channel
        let (progress_tx, _) = broadcast::channel(100);

        // Run ingestion with progress tracking
        let result = run_ingestion_with_workers(
            self.pipeline_job.clone(),
            repo,
            Some(progress_tx),
            None, // For now, no worker pool integration
        ).await?;

        info!("Completed IronClaw ingestion job {}", self.job_id);
        Ok(result)
    }
}



/// Modified run_ingestion to use IronClaw worker pool for parallel processing
pub async fn run_ingestion_with_workers(
    job: PipelineJob,
    repo: Arc<IngestionRepository>,
    progress_tx: Option<broadcast::Sender<IngestionProgress>>,
    worker_pool: Arc<WorkerPool>,
) -> anyhow::Result<IngestionResult> {
    // For now, delegate to existing run_ingestion, but we can modify internal parallel processing
    // to use worker_pool for paper processing tasks
    run_ingestion(job, repo, progress_tx).await
}

/// API handler for submitting ingestion jobs
pub async fn submit_ingestion_job(
    pipeline_job: PipelineJob,
    repo: Arc<IngestionRepository>,
) -> anyhow::Result<Uuid> {
    let job = IronClawIngestionJob::new(pipeline_job);
    let job_id = job.id();

    // For now, execute immediately. In a full IronClaw integration,
    // this would submit to the scheduler with audit logging and channels
    info!("Submitting ingestion job {}", job_id);

    tokio::spawn(async move {
        if let Err(e) = job.execute(repo).await {
            error!("Ingestion job {} failed: {}", job_id, e);
        }
    });

    Ok(job_id)
}

/// Initialize scheduled ingestion jobs (placeholder)
pub async fn init_scheduled_ingestion(
    _repo: Arc<IngestionRepository>,
) -> anyhow::Result<()> {
    // Example scheduled jobs - in full integration, these would be registered
    // with IronClaw's scheduler
    let scheduled_configs = vec![
        (PipelineJob {
            gene: "KRAS".to_string(),
            mutation: Some("G12D".to_string()),
            cancer_type: "pancreatic cancer".to_string(),
            max_results: 50,
            ..Default::default()
        }, "0 2 * * *"), // Daily at 2 AM
        (PipelineJob {
            gene: "EGFR".to_string(),
            mutation: Some("L858R".to_string()),
            cancer_type: "lung cancer".to_string(),
            max_results: 50,
            ..Default::default()
        }, "0 3 * * *"), // Daily at 3 AM
    ];

    for (job, cron) in scheduled_configs {
        info!(
            "Would schedule ingestion for gene {} with cron: {}",
            job.gene,
            cron
        );
        // In full IronClaw integration:
        // scheduler.register_job(config).await?;
    }

    Ok(())
}