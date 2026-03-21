//! Chat endpoint handler proxying requests to the Ferrumyx Runtime Core GatewayChannel.

use crate::state::SharedState;
use axum::{
    body::Body,
    extract::{Json, State},
    response::{Html, IntoResponse, Response},
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const GATEWAY_BASE_URL: &str = "http://127.0.0.1:3002";
const GATEWAY_AUTH_TOKEN: &str = "Bearer ferrumyx-local-dev-token";
const AGENT_BOOT_MIN_INTERVAL_SECS: u64 = 12;
const AGENT_BOOT_WAIT_STEPS: usize = 12;
const AGENT_BOOT_WAIT_STEP_MS: u64 = 350;
const AGENT_AUTOBIND_DEFAULT: &str = "127.0.0.1:0";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocalThreadInfo {
    id: String,
    title: String,
    #[serde(default)]
    thread_type: String,
    #[serde(default)]
    updated_at: String,
}

#[derive(Debug, Default)]
struct LocalChatState {
    assistant_thread: Option<LocalThreadInfo>,
    threads: Vec<LocalThreadInfo>,
    turns: HashMap<String, Vec<TurnInfoLite>>,
    turn_counter: usize,
}

fn local_chat_state() -> &'static Mutex<LocalChatState> {
    static STATE: OnceLock<Mutex<LocalChatState>> = OnceLock::new();
    STATE.get_or_init(|| {
        let mut state = LocalChatState::default();
        state.assistant_thread = Some(LocalThreadInfo {
            id: "local-assistant".to_string(),
            title: "Assistant".to_string(),
            thread_type: "assistant".to_string(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        });
        Mutex::new(state)
    })
}

fn agent_boot_gate() -> &'static Mutex<Instant> {
    static GATE: OnceLock<Mutex<Instant>> = OnceLock::new();
    GATE.get_or_init(|| Mutex::new(Instant::now() - Duration::from_secs(60)))
}

fn agent_binary_names() -> Vec<&'static str> {
    if cfg!(windows) {
        vec!["ferrumyx.exe", "ferrumyx-agent.exe"]
    } else {
        vec!["ferrumyx", "ferrumyx-agent"]
    }
}

fn agent_binary_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let names = agent_binary_names();
    for name in &names {
        out.push(PathBuf::from(name));
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            for name in &names {
                out.push(parent.join(name));
            }
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        for name in &names {
            out.push(cwd.join("target").join("debug").join(name));
            out.push(cwd.join("target").join("release").join(name));
        }
    }

    out
}

fn open_agent_log_stdio(path: &str) -> Option<Stdio> {
    let p = PathBuf::from(path);
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(p)
        .ok()
        .map(Stdio::from)
}

fn apply_autoboot_env(cmd: &mut std::process::Command) {
    cmd.env("FERRUMYX_DISABLE_REPL", "1");
    cmd.env(
        "RUST_LOG",
        std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
    );
    // Prevent collisions with any user-launched web instance while still
    // starting the gateway on 3002 for chat proxying.
    cmd.env(
        "FERRUMYX_BIND",
        std::env::var("FERRUMYX_AGENT_BOOT_BIND")
            .unwrap_or_else(|_| AGENT_AUTOBIND_DEFAULT.to_string()),
    );
}

fn spawn_agent_via_cargo(cwd: &PathBuf) -> bool {
    if !cwd.join("Cargo.toml").exists() {
        return false;
    }

    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("run")
        .arg("-p")
        .arg("ferrumyx-agent")
        .arg("--bin")
        .arg("ferrumyx");
    cmd.current_dir(cwd);
    apply_autoboot_env(&mut cmd);
    if let Some(stdout) = open_agent_log_stdio("output/agent-autoboot.out.log") {
        cmd.stdout(stdout);
    } else {
        cmd.stdout(Stdio::null());
    }
    if let Some(stderr) = open_agent_log_stdio("output/agent-autoboot.err.log") {
        cmd.stderr(stderr);
    } else {
        cmd.stderr(Stdio::null());
    }

    match cmd.spawn() {
        Ok(child) => {
            tracing::info!(
                "Auto-started ferrumyx-agent via cargo run (pid={})",
                child.id()
            );
            true
        }
        Err(e) => {
            tracing::warn!("Failed to auto-start ferrumyx-agent via cargo run: {}", e);
            false
        }
    }
}

fn spawn_agent_process() -> bool {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    for candidate in agent_binary_candidates() {
        let looks_like_path = candidate.components().count() > 1;
        if looks_like_path && !candidate.exists() {
            continue;
        }

        let mut cmd = std::process::Command::new(&candidate);
        cmd.current_dir(&cwd);
        apply_autoboot_env(&mut cmd);
        if let Some(stdout) = open_agent_log_stdio("output/agent-autoboot.out.log") {
            cmd.stdout(stdout);
        } else {
            cmd.stdout(Stdio::null());
        }
        if let Some(stderr) = open_agent_log_stdio("output/agent-autoboot.err.log") {
            cmd.stderr(stderr);
        } else {
            cmd.stderr(Stdio::null());
        }

        match cmd.spawn() {
            Ok(child) => {
                tracing::info!(
                    "Auto-started ferrumyx-agent from '{}' (pid={})",
                    candidate.display(),
                    child.id()
                );
                return true;
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to spawn ferrumyx-agent from '{}': {}",
                    candidate.display(),
                    e
                );
            }
        }
    }

    spawn_agent_via_cargo(&cwd)
}

async fn gateway_online(client: &Client) -> bool {
    let gateway_url = format!("{GATEWAY_BASE_URL}/api/chat/threads");
    match client
        .get(gateway_url)
        .header("Authorization", GATEWAY_AUTH_TOKEN)
        .timeout(Duration::from_millis(900))
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

async fn ensure_gateway_online(client: &Client) -> bool {
    if gateway_online(client).await {
        return true;
    }

    let should_attempt = {
        let mut last = agent_boot_gate()
            .lock()
            .expect("agent boot gate mutex poisoned");
        if last.elapsed() < Duration::from_secs(AGENT_BOOT_MIN_INTERVAL_SECS) {
            false
        } else {
            *last = Instant::now();
            true
        }
    };

    if should_attempt {
        let _ = spawn_agent_process();
    }

    for _ in 0..AGENT_BOOT_WAIT_STEPS {
        tokio::time::sleep(Duration::from_millis(AGENT_BOOT_WAIT_STEP_MS)).await;
        if gateway_online(client).await {
            return true;
        }
    }

    false
}

fn local_threads_payload() -> Value {
    let state = local_chat_state()
        .lock()
        .expect("local chat mutex poisoned");
    json!({
        "assistant_thread": state.assistant_thread,
        "active_thread": state.assistant_thread.as_ref().map(|t| t.id.clone()),
        "threads": state.threads,
    })
}

fn local_create_thread() -> LocalThreadInfo {
    let mut state = local_chat_state()
        .lock()
        .expect("local chat mutex poisoned");
    let thread = LocalThreadInfo {
        id: format!("local-{}", uuid::Uuid::new_v4()),
        title: "Local Thread".to_string(),
        thread_type: "user".to_string(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    state.threads.insert(0, thread.clone());
    state.turns.entry(thread.id.clone()).or_default();
    thread
}

fn local_thread_history(thread_id: &str, limit: usize) -> Value {
    let state = local_chat_state()
        .lock()
        .expect("local chat mutex poisoned");
    let turns = state
        .turns
        .get(thread_id)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .rev()
        .take(limit)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    json!({ "turns": turns })
}

fn local_append_turn(thread_id: &str, user_input: String, response: String) {
    let mut state = local_chat_state()
        .lock()
        .expect("local chat mutex poisoned");
    state.turn_counter += 1;
    let turn = TurnInfoLite {
        turn_number: state.turn_counter,
        user_input,
        response: Some(response),
    };
    state
        .turns
        .entry(thread_id.to_string())
        .or_default()
        .push(turn);
    if let Some(thread) = state.threads.iter_mut().find(|t| t.id == thread_id) {
        thread.updated_at = chrono::Utc::now().to_rfc3339();
    }
}

pub async fn chat_page(State(_state): State<SharedState>) -> Html<String> {
    let html = include_str!("../../templates/chat.html");
    // Swap the nav_chat active class logically using JS or template injection,
    // but the actual nav logic is in main.js, so just returning the HTML with the NAV payload works.
    let final_html = html.replace("{NAV_HTML}", crate::handlers::dashboard::NAV_HTML);
    Html(final_html)
}

#[derive(serde::Deserialize)]
pub struct ChatRequest {
    message: String,
    thread_id: Option<String>,
}

pub async fn chat_submit(
    State(_state): State<SharedState>,
    Json(payload): Json<ChatRequest>,
) -> impl IntoResponse {
    let client = Client::new();
    let _ = ensure_gateway_online(&client).await;
    let outbound_message = maybe_augment_message_for_lab_roles(&payload.message);

    // Resolve a concrete thread so we can poll async completion reliably.
    let thread_id = match payload.thread_id.clone() {
        Some(t) => Some(t),
        None => resolve_or_create_thread_id(&client).await,
    };

    let pre_turn_marker = if let Some(ref tid) = thread_id {
        fetch_turn_marker(&client, tid).await.unwrap_or(0)
    } else {
        0
    };

    let gateway_url = format!("{GATEWAY_BASE_URL}/api/chat/send");
    let res = client
        .post(gateway_url)
        .header("Authorization", GATEWAY_AUTH_TOKEN)
        .json(&json!({
            "content": outbound_message,
            "thread_id": thread_id
        }))
        .send()
        .await;

    match res {
        Ok(r) => {
            if !r.status().is_success() {
                return (
                    axum::http::StatusCode::BAD_GATEWAY,
                    "Agent gateway returned error status",
                )
                    .into_response();
            }

            if r.json::<SendAck>().await.is_err() {
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "Invalid response from agent",
                )
                    .into_response();
            }

            if let Some(ref tid) = thread_id {
                if let Some(response) = poll_for_response(&client, tid, pre_turn_marker).await {
                    return axum::Json::<Value>(json!({
                        "status": "success",
                        "thread_id": tid,
                        "before_turn_marker": pre_turn_marker,
                        "response": response
                    }))
                    .into_response();
                }
            }

            axum::Json::<Value>(json!({
                "status": "accepted",
                "thread_id": thread_id,
                "before_turn_marker": pre_turn_marker,
                "response": "Task accepted and processing in background."
            }))
            .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to contact Ferrumyx Runtime Core Agent: {}", e);
            let fallback_thread = if let Some(existing) = payload.thread_id.clone() {
                existing
            } else {
                resolve_or_create_thread_id(&client)
                    .await
                    .unwrap_or_else(|| local_create_thread().id)
            };
            let fallback_response = "Agent gateway is currently offline. Start `ferrumyx` (Ferrumyx agent runtime) and retry this request.".to_string();
            local_append_turn(
                &fallback_thread,
                payload.message.clone(),
                fallback_response.clone(),
            );
            axum::Json::<Value>(json!({
                "status": "success",
                "thread_id": fallback_thread,
                "response": fallback_response
            }))
            .into_response()
        }
    }
}

fn maybe_augment_message_for_lab_roles(message: &str) -> String {
    let raw = message.trim();
    if raw.is_empty() {
        return message.to_string();
    }
    let lower = raw.to_ascii_lowercase();
    let wants_autonomous = lower.contains("autonomous")
        || lower.contains("overnight")
        || lower.contains("research run")
        || (lower.contains("run") && lower.contains("workflow"))
        || (lower.contains("agent") && lower.contains("team"));
    let already_mentions_roles = lower.contains("lab_planner")
        || lower.contains("lab_retriever")
        || lower.contains("lab_validator")
        || lower.contains("run_lab_autoresearch")
        || lower.contains("lab_run_status");
    if !wants_autonomous || already_mentions_roles {
        return message.to_string();
    }

    format!(
        "{}\n\n[System execution hint for autonomous lab mode: prefer `run_lab_autoresearch` for dynamic planner->retriever->validator cycling, then call `lab_run_status` with run_id for periodic updates. If running manually, use `lab_planner` -> `lab_retriever` -> `lab_validator`, pass run_id between calls, and include brief progress updates after each role.]",
        message
    )
}

#[cfg(test)]
mod tests {
    use super::maybe_augment_message_for_lab_roles;

    #[test]
    fn augments_autonomous_prompt() {
        let input = "Please run an autonomous research run for lung cancer.";
        let out = maybe_augment_message_for_lab_roles(input);
        assert!(out.contains("run_lab_autoresearch"));
        assert!(out.contains("lab_validator"));
    }

    #[test]
    fn does_not_double_augment() {
        let input = "Use lab_planner then lab_retriever.";
        let out = maybe_augment_message_for_lab_roles(input);
        assert_eq!(out, input);
    }
}

#[derive(Debug, Deserialize)]
struct SendAck {
    #[allow(dead_code)]
    message_id: String,
    #[allow(dead_code)]
    status: String,
}

#[derive(Debug, Deserialize)]
struct ThreadInfoLite {
    id: String,
}

#[derive(Debug, Deserialize)]
struct ThreadsResponse {
    assistant_thread: Option<ThreadInfoLite>,
    active_thread: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct TurnInfoLite {
    turn_number: usize,
    user_input: String,
    response: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HistoryResponse {
    turns: Vec<TurnInfoLite>,
}

#[derive(Debug, Deserialize)]
pub struct ChatHistoryQuery {
    thread_id: String,
    limit: Option<usize>,
}

async fn resolve_assistant_thread_id(client: &Client) -> Option<String> {
    let url = format!("{GATEWAY_BASE_URL}/api/chat/threads");
    let resp = client
        .get(url)
        .header("Authorization", GATEWAY_AUTH_TOKEN)
        .send()
        .await
        .ok()?;
    let threads = resp.json::<ThreadsResponse>().await.ok()?;
    if let Some(assistant) = threads.assistant_thread {
        return Some(assistant.id);
    }
    threads.active_thread
}

fn threads_payload_has_any_thread(payload: &Value) -> bool {
    let has_assistant = payload
        .get("assistant_thread")
        .and_then(|v| v.get("id").and_then(Value::as_str))
        .is_some();
    let has_active = payload
        .get("active_thread")
        .and_then(Value::as_str)
        .is_some_and(|s| !s.trim().is_empty());
    let has_threads = payload
        .get("threads")
        .and_then(Value::as_array)
        .is_some_and(|arr| !arr.is_empty());
    has_assistant || has_active || has_threads
}

async fn fetch_gateway_threads_value(client: &Client) -> Option<Value> {
    let gateway_url = format!("{GATEWAY_BASE_URL}/api/chat/threads");
    let resp = client
        .get(gateway_url)
        .header("Authorization", GATEWAY_AUTH_TOKEN)
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.json::<Value>().await.ok()
}

async fn ensure_gateway_threads_nonempty(client: &Client) -> Option<Value> {
    let first = fetch_gateway_threads_value(client).await?;
    if threads_payload_has_any_thread(&first) {
        return Some(first);
    }

    let mut created_thread_id: Option<String> = None;
    let create_url = format!("{GATEWAY_BASE_URL}/api/chat/thread/new");
    if let Ok(resp) = client
        .post(create_url)
        .header("Authorization", GATEWAY_AUTH_TOKEN)
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(payload) = resp.json::<Value>().await {
                created_thread_id = payload
                    .get("id")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string());
            }
        }
    }

    if let Some(second) = fetch_gateway_threads_value(client).await {
        if threads_payload_has_any_thread(&second) {
            return Some(second);
        }
    }

    // Gateway returned an empty thread surface even after creation attempt.
    // Fall back so the web UI can auto-initialize a local thread and keep history.
    if let Some(thread_id) = created_thread_id {
        return Some(json!({
            "assistant_thread": Value::Null,
            "active_thread": thread_id,
            "threads": [{
                "id": thread_id,
                "title": "Thread",
                "thread_type": "user",
                "updated_at": chrono::Utc::now().to_rfc3339(),
            }]
        }));
    }

    None
}

async fn resolve_or_create_thread_id(client: &Client) -> Option<String> {
    if let Some(existing) = resolve_assistant_thread_id(client).await {
        return Some(existing);
    }

    let url = format!("{GATEWAY_BASE_URL}/api/chat/thread/new");
    let resp = client
        .post(url)
        .header("Authorization", GATEWAY_AUTH_TOKEN)
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let thread = resp.json::<ThreadInfoLite>().await.ok()?;
    Some(thread.id)
}

async fn fetch_turn_marker(client: &Client, thread_id: &str) -> Option<usize> {
    let url = format!("{GATEWAY_BASE_URL}/api/chat/history?thread_id={thread_id}&limit=50");
    let resp = client
        .get(url)
        .header("Authorization", GATEWAY_AUTH_TOKEN)
        .send()
        .await
        .ok()?;
    let history = resp.json::<HistoryResponse>().await.ok()?;
    Some(
        history
            .turns
            .iter()
            .map(|t| t.turn_number)
            .max()
            .unwrap_or(0),
    )
}

async fn poll_for_response(
    client: &Client,
    thread_id: &str,
    before_turn_marker: usize,
) -> Option<String> {
    let mut attempts = 0u32;
    while attempts < 40 {
        attempts += 1;
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        let url = format!("{GATEWAY_BASE_URL}/api/chat/history?thread_id={thread_id}&limit=40");
        let resp = match client
            .get(url)
            .header("Authorization", GATEWAY_AUTH_TOKEN)
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => continue,
        };

        let history = match resp.json::<HistoryResponse>().await {
            Ok(h) => h,
            Err(_) => continue,
        };
        if let Some(done) = history
            .turns
            .iter()
            .rev()
            .filter(|t| t.turn_number > before_turn_marker)
            .find_map(|t| t.response.as_ref().filter(|s| !s.trim().is_empty()))
        {
            return Some(done.to_string());
        }
    }

    None
}

pub async fn chat_history(
    State(_state): State<SharedState>,
    axum::extract::Query(query): axum::extract::Query<ChatHistoryQuery>,
) -> impl IntoResponse {
    let client = Client::new();
    let _ = ensure_gateway_online(&client).await;
    let limit = query.limit.unwrap_or(40).clamp(1, 200);
    let url = format!(
        "{GATEWAY_BASE_URL}/api/chat/history?thread_id={}&limit={}",
        query.thread_id, limit
    );

    match client
        .get(url)
        .header("Authorization", GATEWAY_AUTH_TOKEN)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
            Ok(v) => axum::Json(v).into_response(),
            Err(_) => (
                axum::http::StatusCode::BAD_GATEWAY,
                "Invalid response from agent",
            )
                .into_response(),
        },
        Ok(_) => axum::Json(local_thread_history(&query.thread_id, limit)).into_response(),
        Err(e) => {
            tracing::error!("Failed to contact Ferrumyx Runtime Core Agent history endpoint: {}", e);
            axum::Json(local_thread_history(&query.thread_id, limit)).into_response()
        }
    }
}

pub async fn chat_threads(State(_state): State<SharedState>) -> impl IntoResponse {
    let client = Client::new();
    let _ = ensure_gateway_online(&client).await;
    match ensure_gateway_threads_nonempty(&client).await {
        Some(v) => axum::Json(v).into_response(),
        None => axum::Json(local_threads_payload()).into_response(),
    }
}

pub async fn chat_thread_new(State(_state): State<SharedState>) -> impl IntoResponse {
    let client = Client::new();
    let _ = ensure_gateway_online(&client).await;
    let gateway_url = format!("{GATEWAY_BASE_URL}/api/chat/thread/new");

    match client
        .post(gateway_url)
        .header("Authorization", GATEWAY_AUTH_TOKEN)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
            Ok(v) => axum::Json(v).into_response(),
            Err(_) => (
                axum::http::StatusCode::BAD_GATEWAY,
                "Invalid response from agent",
            )
                .into_response(),
        },
        Ok(_) => axum::Json(json!(local_create_thread())).into_response(),
        Err(e) => {
            tracing::error!(
                "Failed to contact Ferrumyx Runtime Core Agent new-thread endpoint: {}",
                e
            );
            axum::Json(json!(local_create_thread())).into_response()
        }
    }
}

fn offline_sse_response(message: &str) -> Response {
    let sanitized = message.replace('\n', " ").replace('"', "'");
    let payload = format!(
        "retry: 8000\nevent: status\ndata: {{\"message\":\"{}\"}}\n\n",
        sanitized
    );
    let mut out = Response::new(Body::from(payload));
    *out.status_mut() = axum::http::StatusCode::OK;
    let headers = out.headers_mut();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("text/event-stream"),
    );
    headers.insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-cache"),
    );
    headers.insert(
        axum::http::header::CONNECTION,
        axum::http::HeaderValue::from_static("keep-alive"),
    );
    out
}

pub async fn chat_events_proxy(State(_state): State<SharedState>) -> impl IntoResponse {
    let client = Client::new();
    let _ = ensure_gateway_online(&client).await;
    let gateway_url = format!("{GATEWAY_BASE_URL}/api/chat/events");

    match client
        .get(gateway_url)
        .header("Authorization", GATEWAY_AUTH_TOKEN)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            let stream = resp.bytes_stream();
            let mut out = Response::new(Body::from_stream(stream));
            *out.status_mut() = axum::http::StatusCode::OK;
            let headers = out.headers_mut();
            headers.insert(
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static("text/event-stream"),
            );
            headers.insert(
                axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-cache"),
            );
            headers.insert(
                axum::http::header::CONNECTION,
                axum::http::HeaderValue::from_static("keep-alive"),
            );
            out
        }
        Ok(_) => offline_sse_response("Agent gateway returned error status"),
        Err(e) => {
            tracing::error!("Failed to proxy Ferrumyx Runtime Core SSE events: {}", e);
            offline_sse_response("Agent offline")
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ChatLabMonitorQuery {
    run_id: Option<String>,
    limit: Option<usize>,
}

fn lab_monitor_path() -> PathBuf {
    std::env::var("FERRUMYX_LAB_STATE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("output/lab_runs.json"))
}

fn read_lab_monitor_snapshot() -> Value {
    let path = lab_monitor_path();
    let content = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return json!({ "updated_at": Value::Null, "run_count": 0, "runs": [] }),
    };
    serde_json::from_str::<Value>(&content)
        .unwrap_or_else(|_| json!({ "updated_at": Value::Null, "run_count": 0, "runs": [] }))
}

pub async fn chat_lab_monitor(
    State(_state): State<SharedState>,
    axum::extract::Query(query): axum::extract::Query<ChatLabMonitorQuery>,
) -> impl IntoResponse {
    let snapshot = read_lab_monitor_snapshot();
    let mut runs = snapshot
        .get("runs")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if let Some(run_id) = query
        .run_id
        .as_ref()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
    {
        let found = runs
            .iter()
            .find(|run| {
                run.get("run_id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| id == run_id)
            })
            .cloned();

        return axum::Json(json!({
            "status": "ok",
            "source": "persisted_lab_state",
            "run_id": run_id,
            "found": found.is_some(),
            "run": found,
            "updated_at": snapshot.get("updated_at").cloned().unwrap_or(Value::Null)
        }))
        .into_response();
    }

    let limit = query.limit.unwrap_or(8).clamp(1, 30);
    runs.truncate(limit);
    axum::Json(json!({
        "status": "ok",
        "source": "persisted_lab_state",
        "updated_at": snapshot.get("updated_at").cloned().unwrap_or(Value::Null),
        "run_count": snapshot.get("run_count").cloned().unwrap_or(json!(runs.len())),
        "runs": runs
    }))
    .into_response()
}

