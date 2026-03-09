//! Chat endpoint handler proxying requests to the IronClaw GatewayChannel.

use axum::{
    extract::{State, Json},
    response::{IntoResponse, Html},
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::state::SharedState;

const GATEWAY_BASE_URL: &str = "http://127.0.0.1:3002";
const GATEWAY_AUTH_TOKEN: &str = "Bearer ferrumyx-local-dev-token";

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

    // Resolve a concrete thread so we can poll async completion reliably.
    let thread_id = match payload.thread_id.clone() {
        Some(t) => Some(t),
        None => resolve_assistant_thread_id(&client).await,
    };

    let pre_turn_marker = if let Some(ref tid) = thread_id {
        fetch_turn_marker(&client, tid).await.unwrap_or(0)
    } else {
        0
    };

    let gateway_url = format!("{GATEWAY_BASE_URL}/api/chat/send");
    let res = client.post(gateway_url)
        .header("Authorization", GATEWAY_AUTH_TOKEN)
        .json(&json!({
            "content": payload.message,
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
                        "response": response
                    }))
                    .into_response();
                }
            }

            axum::Json::<Value>(json!({
                "status": "accepted",
                "thread_id": thread_id,
                "response": "Task accepted and processing in background."
            }))
            .into_response()
        },
        Err(e) => {
            tracing::error!("Failed to contact IronClaw Agent: {}", e);
            (axum::http::StatusCode::SERVICE_UNAVAILABLE, "Agent offline").into_response()
        }
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

async fn poll_for_response(client: &Client, thread_id: &str, before_turn_marker: usize) -> Option<String> {
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
