use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use ferrumyx_ingestion::ironclaw_integration::{submit_ingestion_job};
use ferrumyx_ingestion::pipeline::IngestionJob;
use ferrumyx_ingestion::repository::IngestionRepository;

/// Initialize IronClaw integration for ingestion
pub async fn init_ironclaw_ingestion(
    repo: Arc<IngestionRepository>,
) -> anyhow::Result<()> {
    // Initialize scheduled ingestion jobs
    ferrumyx_ingestion::ironclaw_integration::init_scheduled_ingestion(repo.clone()).await?;

    info!("IronClaw ingestion integration initialized");

    Ok(())
}

/// API endpoint to submit ingestion job
pub async fn submit_ingestion_via_api(
    job: IngestionJob,
    repo: Arc<IngestionRepository>,
) -> anyhow::Result<uuid::Uuid> {
    submit_ingestion_job(job, repo).await
}