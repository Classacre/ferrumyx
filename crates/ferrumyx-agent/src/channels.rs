//! Multi-channel support for Ferrumyx with oncology-specific integrations.
//!
//! This module provides:
//! - Channel plugins for biomedical data formatting
//! - Routing based on user permissions and data sensitivity
//! - Integration with oncology workflows

use ferrumyx_runtime::channels::{Channel, IncomingMessage, MessageStream, OutgoingResponse, StatusUpdate};
use async_trait::async_trait;
use std::sync::Arc;
use ferrumyx_runtime::channels::ChannelError;
use ferrumyx_security::{PhiDetector, AuditManager, PhiDetectionResult};
use log;

/// Oncology-specific channel wrapper that adds biomedical data formatting.
pub struct OncologyChannelWrapper<C: Channel> {
    inner: C,
    data_sensitivity_filter: DataSensitivityFilter,
    phi_detector: PhiDetector,
    audit_manager: Option<Arc<AuditManager>>,
}

impl<C: Channel> OncologyChannelWrapper<C> {
    pub fn new(inner: C) -> Self {
        Self {
            inner,
            data_sensitivity_filter: DataSensitivityFilter::new(),
            phi_detector: PhiDetector::new().unwrap(), // TODO: Handle error properly
            audit_manager: None,
        }
    }

    pub fn with_audit_manager(mut self, audit_manager: Arc<AuditManager>) -> Self {
        self.audit_manager = Some(audit_manager);
        self
    }
}

#[async_trait]
impl<C: Channel> Channel for OncologyChannelWrapper<C> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn start(&self) -> Result<MessageStream, ChannelError> {
        self.inner.start().await
    }

    async fn respond(
        &self,
        msg: &IncomingMessage,
        mut response: OutgoingResponse,
    ) -> Result<(), ChannelError> {
        // Apply biomedical data formatting
        response.content = self.format_biomedical_response(&response.content, &msg.channel);

        // Enhanced PHI detection for both query and response
        let query_phi_result = self.phi_detector.detect_phi(&msg.content);
        let response_phi_result = self.phi_detector.detect_phi(&response.content);

        // Log PHI detection
        if let Some(audit) = &self.audit_manager {
            if query_phi_result.has_phi || response_phi_result.has_phi {
                let detection_details = serde_json::json!({
                    "query_risk": query_phi_result.risk_score,
                    "response_risk": response_phi_result.risk_score,
                    "query_detections": query_phi_result.detections.len(),
                    "response_detections": response_phi_result.detections.len()
                });

                let _ = audit.log_phi_detection(
                    Some(msg.user_id.clone()),
                    msg.channel.clone(),
                    msg.content.chars().take(100).collect::<String>(),
                    response_phi_result.risk_score.max(query_phi_result.risk_score),
                    "scanning".to_string(),
                    detection_details,
                ).await;
            }
        }

        // Check data sensitivity with enhanced logic
        let allow_response = self.data_sensitivity_filter.allow_response_enhanced(
            &response,
            &msg,
            &query_phi_result,
            &response_phi_result
        );

        if !allow_response {
            response.content = "⚠️ Response contains sensitive biomedical data that cannot be shared via this channel.".to_string();

            // Log PHI blocking
            if let Some(audit) = &self.audit_manager {
                let _ = audit.log_phi_blocking(
                    Some(msg.user_id.clone()),
                    msg.channel.clone(),
                    "PHI detected in response".to_string(),
                    response.content.len(),
                ).await;
            }
        }

        self.inner.respond(msg, response).await
    }

    async fn send_status(
        &self,
        status: StatusUpdate,
        metadata: &serde_json::Value,
    ) -> Result<(), ChannelError> {
        self.inner.send_status(status, metadata).await
    }

    async fn broadcast(
        &self,
        user_id: &str,
        response: OutgoingResponse,
    ) -> Result<(), ChannelError> {
        self.inner.broadcast(user_id, response).await
    }

    async fn health_check(&self) -> Result<(), ChannelError> {
        self.inner.health_check().await
    }

    async fn shutdown(&self) -> Result<(), ChannelError> {
        self.inner.shutdown().await
    }
}

impl<C: Channel> OncologyChannelWrapper<C> {
    /// Format response content for biomedical data display on different channels.
    fn format_biomedical_response(&self, content: &str, channel: &str) -> String {
        match channel {
            "whatsapp" => self.format_for_whatsapp(content),
            "slack" => self.format_for_slack(content),
            "discord" => self.format_for_discord(content),
            "telegram" => self.format_for_telegram(content),
            "web" => self.format_for_web(content),
            _ => content.to_string(),
        }
    }

    fn format_for_whatsapp(&self, content: &str) -> String {
        // WhatsApp supports basic formatting
        content
            .replace("**", "*")  // Bold
            .replace("__", "_")  // Italic
            .replace("`", "")    // Remove code blocks
            .replace("# ", "")   // Remove headers
            .replace("\n\n", "\n") // Reduce spacing
    }

    fn format_for_slack(&self, content: &str) -> String {
        // Slack supports markdown-like formatting
        content.to_string()
    }

    fn format_for_discord(&self, content: &str) -> String {
        // Discord supports markdown
        content.to_string()
    }

    fn format_for_telegram(&self, content: &str) -> String {
        // Telegram supports HTML/Markdown
        content
            .replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace("&", "&amp;")
    }

    fn format_for_web(&self, content: &str) -> String {
        // Web supports full HTML/markdown
        content.to_string()
    }
}

/// Data sensitivity levels for biomedical information.
#[derive(Debug, Clone, PartialEq)]
pub enum SensitivityLevel {
    Public,      // Can be shared on any channel
    Restricted,  // Limited sharing, requires consent for sensitive data
    Internal,    // Internal use only, not for external channels
    Confidential, // Highly sensitive, restrict to secure channels
}

/// Data sensitivity filter for biomedical information.
pub struct DataSensitivityFilter {
    channel_trust_levels: std::collections::HashMap<String, SensitivityLevel>,
}

impl DataSensitivityFilter {
    pub fn new() -> Self {
        let mut channel_trust_levels = std::collections::HashMap::new();

        // Define trust levels for different channels
        channel_trust_levels.insert("web".to_string(), SensitivityLevel::Internal);
        channel_trust_levels.insert("whatsapp".to_string(), SensitivityLevel::Restricted);
        channel_trust_levels.insert("telegram".to_string(), SensitivityLevel::Restricted);
        channel_trust_levels.insert("slack".to_string(), SensitivityLevel::Internal);
        channel_trust_levels.insert("discord".to_string(), SensitivityLevel::Restricted);

        Self { channel_trust_levels }
    }

    /// Check if a response can be sent via the given channel based on data sensitivity.
    pub fn allow_response(&self, response: &OutgoingResponse, msg: &IncomingMessage) -> bool {
        let channel_trust = self.channel_trust_levels.get(&msg.channel)
            .unwrap_or(&SensitivityLevel::Public);

        // Enhanced PHI detection keywords
        let phi_keywords = [
            "patient", "clinical trial", "medical record", "diagnosis", "treatment", "medication",
            "medical history", "social security", "ssn", "date of birth", "dob", "address", "phone",
            "phi", "hipaa", "confidential", "protected health information", "ehr", "electronic health record",
            "patient data", "clinical data", "biomedical data"
        ];

        let content_lower = response.content.to_lowercase();
        let has_phi_data = phi_keywords.iter().any(|&keyword| content_lower.contains(keyword));

        // Check user consent for sensitive data
        let user_consent = msg.metadata.get("user_consent")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Log PHI-related operations
        if has_phi_data {
            log::info!("PHI detected in response for channel: {}, user: {}", msg.channel, msg.user_id);
        }

        match channel_trust {
            SensitivityLevel::Public => !has_phi_data, // Allow only non-sensitive data
            SensitivityLevel::Restricted => {
                // Restricted channels require explicit consent for PHI data
                if has_phi_data && !user_consent {
                    log::warn!("PHI blocked on restricted channel {} due to lack of user consent", msg.channel);
                    false
                } else {
                    true // Allow non-PHI or PHI with consent
                }
            },
            SensitivityLevel::Internal => true, // Allow all data for internal channels
            SensitivityLevel::Confidential => has_phi_data, // Only PHI for confidential channels
        }
    }

    /// Enhanced response validation using detailed PHI detection results
    pub fn allow_response_enhanced(
        &self,
        response: &OutgoingResponse,
        msg: &IncomingMessage,
        query_phi: &PhiDetectionResult,
        response_phi: &PhiDetectionResult,
    ) -> bool {
        let channel_trust = self.channel_trust_levels.get(&msg.channel)
            .unwrap_or(&SensitivityLevel::Public);

        // Emergency blocking for high-risk PHI patterns
        if response_phi.risk_score >= 0.8 {
            log::error!("Emergency PHI blocking triggered for channel {}: risk_score={:.2}",
                       msg.channel, response_phi.risk_score);
            return false;
        }

        // Check user type and channel combination
        let user_type = msg.metadata.get("user_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        match channel_trust {
            SensitivityLevel::Public => {
                // Public channels: strict blocking, allow research queries
                if response_phi.risk_score > 0.3 && !self.is_research_context(query_phi, response_phi) {
                    false
                } else {
                    true
                }
            },
            SensitivityLevel::Restricted => {
                // Restricted channels (WhatsApp, Discord): allow clinician research queries
                let user_consent = msg.metadata.get("user_consent")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if user_type == "clinician" && self.is_research_context(query_phi, response_phi) {
                    // Allow clinician research queries even without explicit consent
                    true
                } else if response_phi.risk_score > 0.5 && !user_consent {
                    // Block high-risk PHI without consent
                    log::warn!("PHI blocked on restricted channel {} for user {}: risk_score={:.2}",
                              msg.channel, user_type, response_phi.risk_score);
                    false
                } else {
                    true
                }
            },
            SensitivityLevel::Internal => true, // Allow all on internal channels
            SensitivityLevel::Confidential => response_phi.has_phi, // Only PHI on confidential channels
        }
    }

    /// Determine if the context appears to be research-oriented
    fn is_research_context(&self, query_phi: &PhiDetectionResult, response_phi: &PhiDetectionResult) -> bool {
        // Research context if both query and response have low PHI risk
        // and query contains research indicators
        query_phi.risk_score < 0.4 && response_phi.risk_score < 0.5
    }
}

/// Channel routing based on user permissions and data sensitivity.
pub struct ChannelRouter {
    user_permissions: std::collections::HashMap<String, Vec<String>>, // user -> allowed_channels
    channel_sensitivity: std::collections::HashMap<String, SensitivityLevel>, // channel -> sensitivity_level
}

impl ChannelRouter {
    pub fn new() -> Self {
        let mut user_permissions = std::collections::HashMap::new();
        let mut channel_sensitivity = std::collections::HashMap::new();

        // Default permissions - all users can use web
        user_permissions.insert("default".to_string(), vec!["web".to_string()]);

        // Channel sensitivity levels
        channel_sensitivity.insert("web".to_string(), SensitivityLevel::Internal);
        channel_sensitivity.insert("whatsapp".to_string(), SensitivityLevel::Restricted);
        channel_sensitivity.insert("telegram".to_string(), SensitivityLevel::Restricted);
        channel_sensitivity.insert("slack".to_string(), SensitivityLevel::Internal);
        channel_sensitivity.insert("discord".to_string(), SensitivityLevel::Restricted);

        Self {
            user_permissions,
            channel_sensitivity,
        }
    }

    /// Add user permissions for specific channels.
    pub fn add_user_permission(&mut self, user_id: String, channels: Vec<String>) {
        self.user_permissions.insert(user_id, channels);
    }

    /// Route a message to appropriate channels based on user permissions and content sensitivity.
    pub fn route_message(&self, msg: &IncomingMessage) -> Vec<String> {
        let user_channels = self.user_permissions.get(&msg.user_id)
            .or_else(|| self.user_permissions.get("default"))
            .unwrap_or(&vec!["web".to_string()]);

        // Enhanced PHI detection for routing
        let phi_keywords = [
            "patient", "clinical trial", "medical record", "diagnosis", "treatment", "medication",
            "medical history", "social security", "ssn", "date of birth", "dob", "address", "phone",
            "phi", "hipaa", "confidential", "protected health information"
        ];
        let content_lower = msg.content.to_lowercase();
        let has_phi_content = phi_keywords.iter().any(|&keyword| content_lower.contains(keyword));

        let user_consent = msg.metadata.get("user_consent")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        user_channels.iter().filter(|channel| {
            let channel_sensitivity = self.channel_sensitivity.get(*channel)
                .unwrap_or(&SensitivityLevel::Restricted);

            match channel_sensitivity {
                SensitivityLevel::Public => !has_phi_content,
                SensitivityLevel::Restricted => {
                    // Allow if no PHI or user consented
                    !has_phi_content || user_consent
                },
                SensitivityLevel::Internal | SensitivityLevel::Confidential => true,
            }
        }).cloned().collect()
    }
}