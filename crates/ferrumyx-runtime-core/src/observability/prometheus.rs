//! Prometheus metrics backend for observability

use crate::observability::traits::{Observer, ObserverEvent, ObserverMetric};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::time::Duration;

/// Prometheus observer for exporting metrics
pub struct PrometheusObserver {
    prometheus_handle: PrometheusHandle,
}

impl PrometheusObserver {
    /// Create a new Prometheus observer
    pub fn new() -> anyhow::Result<Self> {
        let builder = PrometheusBuilder::new();
        let handle = builder.install_recorder()?;

        Ok(Self {
            prometheus_handle: handle,
        })
    }

    /// Get Prometheus metrics as string
    pub fn metrics(&self) -> String {
        self.prometheus_handle.render()
    }
}

impl Observer for PrometheusObserver {
    fn record_event(&self, event: &ObserverEvent) {
        match event {
            ObserverEvent::AgentStart { provider, model } => {
                metrics::counter!("ferrumyx_agent_starts_total", "provider" => provider.clone(), "model" => model.clone()).increment(1);
            }
            ObserverEvent::LlmRequest { provider, model, message_count } => {
                metrics::counter!("ferrumyx_llm_requests_total", "provider" => provider.clone(), "model" => model.clone()).increment(1);
                metrics::histogram!("ferrumyx_llm_request_messages").record(*message_count as f64);
            }
            ObserverEvent::LlmResponse { provider, model, duration, success, .. } => {
                let labels = &[("provider", provider.as_str()), ("model", model.as_str()), ("success", if *success { "true" } else { "false" })];
                metrics::histogram!("ferrumyx_llm_response_duration_seconds", labels).record(duration.as_secs_f64());
                metrics::counter!("ferrumyx_llm_responses_total", labels).increment(1);
            }
            ObserverEvent::ToolCallStart { tool } => {
                metrics::counter!("ferrumyx_tool_calls_total", "tool" => tool.clone()).increment(1);
            }
            ObserverEvent::ToolCallEnd { tool, duration, success } => {
                let labels = &[("tool", tool.as_str()), ("success", if *success { "true" } else { "false" })];
                metrics::histogram!("ferrumyx_tool_call_duration_seconds", labels).record(duration.as_secs_f64());
            }
            ObserverEvent::ChannelMessage { channel, direction } => {
                metrics::counter!("ferrumyx_channel_messages_total", "channel" => channel.clone(), "direction" => direction.clone()).increment(1);
            }
            ObserverEvent::Error { component, message } => {
                metrics::counter!("ferrumyx_errors_total", "component" => component.clone(), "message" => message.clone()).increment(1);
            }
            ObserverEvent::AgentEnd { duration, tokens_used } => {
                metrics::histogram!("ferrumyx_agent_session_duration_seconds").record(duration.as_secs_f64());
                if let Some(tokens) = tokens_used {
                    metrics::histogram!("ferrumyx_agent_session_tokens").record(*tokens as f64);
                }
            }
            ObserverEvent::TurnComplete => {
                metrics::counter!("ferrumyx_turns_completed_total").increment(1);
            }
            ObserverEvent::HeartbeatTick => {
                metrics::counter!("ferrumyx_heartbeat_ticks_total").increment(1);
            }
        }
    }

    fn record_metric(&self, metric: &ObserverMetric) {
        match metric {
            ObserverMetric::RequestLatency(duration) => {
                metrics::histogram!("ferrumyx_request_latency_seconds").record(duration.as_secs_f64());
            }
            ObserverMetric::TokensUsed(tokens) => {
                metrics::counter!("ferrumyx_tokens_used_total").increment(*tokens);
            }
            ObserverMetric::ActiveJobs(count) => {
                metrics::gauge!("ferrumyx_active_jobs").set(*count as f64);
            }
            ObserverMetric::QueueDepth(depth) => {
                metrics::gauge!("ferrumyx_queue_depth").set(*depth as f64);
            }
        }
    }

    fn name(&self) -> &str {
        "prometheus"
    }
}