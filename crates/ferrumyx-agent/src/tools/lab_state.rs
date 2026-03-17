use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Serialize)]
pub struct RetrievalCheckpoint {
    pub at: DateTime<Utc>,
    pub summary: String,
    pub selected_gene: String,
    pub selected_cancer: String,
    pub papers_found_raw: Option<u64>,
    pub papers_found_unique: Option<u64>,
    pub papers_inserted: Option<u64>,
    pub papers_duplicate: Option<u64>,
    pub chunks_inserted: Option<u64>,
    pub novelty_ratio: Option<f64>,
    pub duplicate_pressure: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationCheckpoint {
    pub at: DateTime<Utc>,
    pub top_target: Option<String>,
    pub top_score: Option<f64>,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LabRunState {
    pub run_id: String,
    pub objective: String,
    pub research_question: String,
    pub cancer_type: Option<String>,
    pub target_gene: Option<String>,
    pub stage: String,
    pub next_action: Option<String>,
    pub planner_notes: Vec<String>,
    pub retrieval_history: Vec<RetrievalCheckpoint>,
    pub validation_history: Vec<ValidationCheckpoint>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

static RUN_STATES: OnceLock<Mutex<HashMap<String, LabRunState>>> = OnceLock::new();

fn run_states() -> &'static Mutex<HashMap<String, LabRunState>> {
    RUN_STATES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn touch(state: &mut LabRunState) {
    state.updated_at = Utc::now();
}

fn lab_state_path() -> PathBuf {
    std::env::var("FERRUMYX_LAB_STATE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("output/lab_runs.json"))
}

fn persist_states_guarded(guard: &HashMap<String, LabRunState>) {
    #[derive(Serialize)]
    struct PersistedRuns<'a> {
        updated_at: DateTime<Utc>,
        run_count: usize,
        runs: &'a [LabRunState],
    }

    let mut runs: Vec<LabRunState> = guard.values().cloned().collect();
    runs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    let payload = PersistedRuns {
        updated_at: Utc::now(),
        run_count: runs.len(),
        runs: &runs,
    };

    let path = lab_state_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let tmp = path.with_extension("json.tmp");
    if let Ok(serialized) = serde_json::to_vec_pretty(&payload) {
        if fs::write(&tmp, serialized).is_ok() {
            if path.exists() {
                let _ = fs::remove_file(&path);
            }
            let _ = fs::rename(&tmp, &path);
        }
    }
}

pub fn create_run(
    objective: String,
    research_question: String,
    cancer_type: Option<String>,
    target_gene: Option<String>,
    planner_notes: Vec<String>,
) -> LabRunState {
    let now = Utc::now();
    let run_id = format!("lab-{}", uuid::Uuid::new_v4().simple());
    let state = LabRunState {
        run_id: run_id.clone(),
        objective,
        research_question,
        cancer_type,
        target_gene,
        stage: "planned".to_string(),
        next_action: Some("lab_retriever".to_string()),
        planner_notes,
        retrieval_history: Vec::new(),
        validation_history: Vec::new(),
        created_at: now,
        updated_at: now,
    };

    if let Ok(mut guard) = run_states().lock() {
        guard.insert(run_id, state.clone());
        persist_states_guarded(&guard);
    }
    state
}

pub fn get_run(run_id: &str) -> Option<LabRunState> {
    let guard = run_states().lock().ok()?;
    guard.get(run_id).cloned()
}

pub fn set_stage(run_id: &str, stage: &str, next_action: Option<String>) -> Option<LabRunState> {
    let mut guard = run_states().lock().ok()?;
    let state = guard.get_mut(run_id)?;
    state.stage = stage.to_string();
    state.next_action = next_action;
    touch(state);
    let snapshot = state.clone();
    persist_states_guarded(&guard);
    Some(snapshot)
}

pub fn append_retrieval(
    run_id: &str,
    summary: String,
    selected_gene: String,
    selected_cancer: String,
    papers_found_raw: Option<u64>,
    papers_found_unique: Option<u64>,
    papers_inserted: Option<u64>,
    papers_duplicate: Option<u64>,
    chunks_inserted: Option<u64>,
) -> Option<LabRunState> {
    let mut guard = run_states().lock().ok()?;
    let state = guard.get_mut(run_id)?;
    let denominator = papers_found_unique.or(papers_found_raw).unwrap_or(0);
    let novelty_ratio = if denominator > 0 {
        papers_inserted.map(|v| v as f64 / denominator as f64)
    } else {
        None
    };
    let duplicate_pressure = if denominator > 0 {
        papers_duplicate.map(|v| v as f64 / denominator as f64)
    } else {
        None
    };
    state.retrieval_history.push(RetrievalCheckpoint {
        at: Utc::now(),
        summary,
        selected_gene,
        selected_cancer,
        papers_found_raw,
        papers_found_unique,
        papers_inserted,
        papers_duplicate,
        chunks_inserted,
        novelty_ratio,
        duplicate_pressure,
    });
    state.stage = "retrieved".to_string();
    state.next_action = Some("lab_validator".to_string());
    touch(state);
    let snapshot = state.clone();
    persist_states_guarded(&guard);
    Some(snapshot)
}

pub fn append_validation(
    run_id: &str,
    top_target: Option<String>,
    top_score: Option<f64>,
    recommendation: String,
    next_action: Option<String>,
) -> Option<LabRunState> {
    let mut guard = run_states().lock().ok()?;
    let state = guard.get_mut(run_id)?;
    state.validation_history.push(ValidationCheckpoint {
        at: Utc::now(),
        top_target,
        top_score,
        recommendation,
    });
    state.stage = "validated".to_string();
    state.next_action = next_action;
    touch(state);
    let snapshot = state.clone();
    persist_states_guarded(&guard);
    Some(snapshot)
}

pub fn list_runs(limit: usize) -> Vec<LabRunState> {
    let guard = match run_states().lock() {
        Ok(g) => g,
        Err(_) => return Vec::new(),
    };
    let mut runs: Vec<LabRunState> = guard.values().cloned().collect();
    runs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    runs.truncate(limit.max(1));
    runs
}
