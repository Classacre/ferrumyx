use ferrumyx_kg::compute_target_scores;
use ferrumyx_web::state::AppState;
use std::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let state = AppState::new_without_db().await?;
    let db = state.db.clone();

    println!("=== Recompute Target Scores ===");
    let started = Instant::now();
    let upserted = compute_target_scores(db).await?;
    println!(
        "target_scores_upserted={} duration_ms={}",
        upserted,
        started.elapsed().as_millis()
    );
    Ok(())
}
