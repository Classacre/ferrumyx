//! Chat endpoint handler proxying requests to the IronClaw GatewayChannel.

use axum::{
    extract::{State, Json},
    response::{IntoResponse, Html},
};
use reqwest::Client;
use serde_json::{json, Value};
use crate::state::SharedState;

pub async fn chat_page(State(_state): State<SharedState>) -> Html<String> {
    let html = include_str!("../../templates/chat.html");
    // Swap the nav_chat active class logically using JS or template injection,
    // but the actual nav logic is in main.js, so just returning the raw HTML is usually fine.
    Html(html.to_string())
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
    
    // Connect to the local GatewayChannel on port 3002
    let gateway_url = "http://127.0.0.1:3002/api/chat/send";
    
    let res = client.post(gateway_url)
        .header("Authorization", "Bearer ferrumyx-local-dev-token")
        .json(&json!({
            "content": payload.message,
            "thread_id": payload.thread_id
        }))
        .send()
        .await;

    match res {
        Ok(r) => {
            if let Ok(json_resp) = r.json::<Value>().await {
                axum::Json::<Value>(json_resp).into_response()
            } else {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Invalid response from agent").into_response()
            }
        },
        Err(e) => {
            tracing::error!("Failed to contact IronClaw Agent: {}", e);
            (axum::http::StatusCode::SERVICE_UNAVAILABLE, "Agent offline").into_response()
        }
    }
}
