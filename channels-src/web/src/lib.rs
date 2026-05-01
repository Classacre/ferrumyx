//! Web HTTP channel for Ferrumyx Runtime Core.
//!
//! This WASM component implements the channel interface for handling
//! web-based chat interactions via HTTP API endpoints.
//!
//! # Features
//!
//! - HTTP-based message receiving
//! - JSON API for chat messages
//! - User session management
//! - Response formatting for web display

// Generate bindings from the WIT file
wit_bindgen::generate!({
    world: "sandboxed-channel",
    path: "../channel.wit",
});

use serde::{Deserialize, Serialize};

// Re-export generated types
use exports::near::agent::channel::{
    AgentResponse, ChannelConfig, Guest, HttpEndpointConfig, IncomingHttpRequest,
    OutgoingHttpResponse, StatusUpdate,
};
use near::agent::channel_host::{self, EmittedMessage};

/// Web message payload.
#[derive(Debug, Deserialize)]
struct WebMessage {
    /// User identifier
    user_id: String,
    /// Optional user name
    user_name: Option<String>,
    /// Message content
    content: String,
    /// Optional session ID
    session_id: Option<String>,
}

/// Web response payload.
#[derive(Debug, Serialize)]
struct WebResponse {
    /// Response content
    content: String,
    /// Optional metadata
    metadata: Option<serde_json::Value>,
}

/// Channel configuration.
#[derive(Debug, Deserialize)]
struct WebConfig {
    #[serde(default)]
    owner_id: Option<String>,
    #[serde(default)]
    dm_policy: Option<String>,
    #[serde(default)]
    allow_from: Option<Vec<String>>,
}

/// Workspace paths for persistence.
const OWNER_ID_PATH: &str = "state/owner_id";
const DM_POLICY_PATH: &str = "state/dm_policy";
const ALLOW_FROM_PATH: &str = "state/allow_from";

struct WebChannel;

impl Guest for WebChannel {
    fn on_start(config_json: String) -> Result<ChannelConfig, String> {
        let config: WebConfig = match serde_json::from_str(&config_json) {
            Ok(c) => c,
            Err(e) => {
                channel_host::log(
                    channel_host::LogLevel::Warn,
                    &format!("Failed to parse Web config, using defaults: {}", e),
                );
                WebConfig {
                    owner_id: None,
                    dm_policy: None,
                    allow_from: None,
                }
            }
        };

        channel_host::log(
            channel_host::LogLevel::Info,
            "Web channel starting",
        );

        // Persist permission config
        if let Some(ref owner_id) = config.owner_id {
            let _ = channel_host::workspace_write(OWNER_ID_PATH, owner_id);
        } else {
            let _ = channel_host::workspace_write(OWNER_ID_PATH, "");
        }

        let dm_policy = config.dm_policy.as_deref().unwrap_or("open");
        let _ = channel_host::workspace_write(DM_POLICY_PATH, dm_policy);

        let allow_from_json = serde_json::to_string(&config.allow_from.unwrap_or_default())
            .unwrap_or_else(|_| "[]".to_string());
        let _ = channel_host::workspace_write(ALLOW_FROM_PATH, &allow_from_json);

        Ok(ChannelConfig {
            display_name: "Web".to_string(),
            http_endpoints: vec![
                HttpEndpointConfig {
                    path: "/api/chat".to_string(),
                    methods: vec!["POST".to_string()],
                    require_secret: false,
                },
                HttpEndpointConfig {
                    path: "/api/messages".to_string(),
                    methods: vec!["POST".to_string()],
                    require_secret: false,
                },
            ],
            poll: None,
        })
    }

    fn on_http_request(req: IncomingHttpRequest) -> OutgoingHttpResponse {
        channel_host::log(
            channel_host::LogLevel::Debug,
            &format!("Received {} request to {}", req.method, req.path),
        );

        if req.method != "POST" {
            return json_response(405, serde_json::json!({"error": "Method not allowed"}));
        }

        match req.path.as_str() {
            "/api/chat" | "/api/messages" => handle_chat_message(&req),
            _ => json_response(404, serde_json::json!({"error": "Not found"})),
        }
    }

    fn on_poll() {
        // Web channel doesn't use polling
    }

    fn on_respond(response: AgentResponse) -> Result<(), String> {
        channel_host::log(
            channel_host::LogLevel::Debug,
            &format!("Sending response for message: {}", response.message_id),
        );

        // For web channel, responses are sent back via HTTP
        // The metadata should contain connection info for the response
        let web_response = WebResponse {
            content: response.content,
            metadata: Some(serde_json::json!({
                "message_id": response.message_id,
                "thread_id": response.thread_id
            })),
        };

        let response_json = serde_json::to_string(&web_response)
            .map_err(|e| format!("Failed to serialize response: {}", e))?;

        // Store response in workspace for retrieval
        let response_key = format!("responses/{}", response.message_id);
        let _ = channel_host::workspace_write(&response_key, &response_json);

        Ok(())
    }

    fn on_status(_update: StatusUpdate) {}

    fn on_broadcast(_user_id: String, _response: AgentResponse) -> Result<(), String> {
        Err("broadcast not implemented for web channel".to_string())
    }

    fn on_shutdown() {
        channel_host::log(
            channel_host::LogLevel::Info,
            "Web channel shutting down",
        );
    }
}

/// Handle incoming chat message.
fn handle_chat_message(req: &IncomingHttpRequest) -> OutgoingHttpResponse {
    let body_str = match std::str::from_utf8(&req.body) {
        Ok(s) => s,
        Err(_) => {
            return json_response(400, serde_json::json!({"error": "Invalid UTF-8 body"}));
        }
    };

    let message: WebMessage = match serde_json::from_str(body_str) {
        Ok(m) => m,
        Err(e) => {
            return json_response(400, serde_json::json!({
                "error": format!("Invalid JSON: {}", e)
            }));
        }
    };

    // Basic permission check
    if !check_sender_permission(&message.user_id) {
        return json_response(403, serde_json::json!({"error": "Access denied"}));
    }

    // Emit message to agent
    let session_id = message.session_id.clone();
    channel_host::emit_message(&EmittedMessage {
        user_id: message.user_id.clone(),
        user_name: message.user_name,
        content: message.content,
        thread_id: message.session_id,
        metadata_json: serde_json::json!({
            "channel": "web",
            "session_id": session_id
        }).to_string(),
        attachments: vec![],
    });

    // Return immediate acknowledgment
    json_response(200, serde_json::json!({
        "status": "received",
        "user_id": message.user_id
    }))
}

/// Check if sender is permitted.
fn check_sender_permission(user_id: &str) -> bool {
    // Owner check
    let owner_id = channel_host::workspace_read(OWNER_ID_PATH).filter(|s| !s.is_empty());
    if let Some(ref owner) = owner_id {
        return user_id == owner;
    }

    // DM policy
    let dm_policy = channel_host::workspace_read(DM_POLICY_PATH).unwrap_or_else(|| "open".to_string());

    if dm_policy == "open" {
        return true;
    }

    // Allow list check
    let allow_from: Vec<String> = channel_host::workspace_read(ALLOW_FROM_PATH)
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    allow_from.contains(&"*".to_string()) || allow_from.contains(&user_id.to_string())
}

/// Create a JSON HTTP response.
fn json_response(status: u16, value: serde_json::Value) -> OutgoingHttpResponse {
    let body = serde_json::to_vec(&value).unwrap_or_default();
    let headers = serde_json::json!({"Content-Type": "application/json"});

    OutgoingHttpResponse {
        status,
        headers_json: headers.to_string(),
        body,
    }
}

// Export the component
export!(WebChannel);