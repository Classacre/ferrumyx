//! Real-time security event correlation and alerting engine

use crate::audit::{AuditManager, AuditEvent, AuditEventType};
use crate::runtime_monitoring::Severity;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;

/// Real-time event correlation engine
pub struct CorrelationEngine {
    /// Audit manager for event retrieval
    audit_manager: Arc<AuditManager>,
    /// Active correlation rules
    correlation_rules: Arc<RwLock<Vec<CorrelationRule>>>,
    /// Event buffer for correlation
    event_buffer: Arc<RwLock<VecDeque<SecurityEvent>>>,
    /// Active alerts
    active_alerts: Arc<RwLock<HashMap<String, Alert>>>,
    /// Alert handlers (simplified for now)
    alert_handlers: Arc<RwLock<Vec<Box<dyn Fn(&Alert) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + '_>> + Send + Sync>>>>,
}

impl CorrelationEngine {
    /// Create new correlation engine
    pub async fn new(audit_manager: Arc<AuditManager>) -> anyhow::Result<Self> {
        let correlation_rules = Self::load_default_rules();

        Ok(Self {
            audit_manager,
            correlation_rules: Arc::new(RwLock::new(correlation_rules)),
            event_buffer: Arc::new(RwLock::new(VecDeque::with_capacity(10000))),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_handlers: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Load default correlation rules
    fn load_default_rules() -> Vec<CorrelationRule> {
        vec![
            CorrelationRule {
                id: "brute_force_attack".to_string(),
                name: "Brute Force Attack Detection".to_string(),
                conditions: vec![
                    Condition::EventCount {
                        event_type: AuditEventType::Authentication,
                        count: 5,
                        time_window: Duration::minutes(5),
                        success: Some(false),
                    },
                    Condition::SourceCorrelation {
                        correlation_type: CorrelationType::SameIP,
                        time_window: Duration::minutes(5),
                    },
                ],
                severity: Severity::High,
                alert_message: "Potential brute force attack detected".to_string(),
                enabled: true,
            },
            CorrelationRule {
                id: "phi_data_exfiltration".to_string(),
                name: "PHI Data Exfiltration Detection".to_string(),
                conditions: vec![
                    Condition::EventCount {
                        event_type: AuditEventType::PhiAccess,
                        count: 3,
                        time_window: Duration::hours(1),
                        success: Some(true),
                    },
                    Condition::EventCount {
                        event_type: AuditEventType::DataAccess,
                        count: 10,
                        time_window: Duration::hours(1),
                        success: Some(true),
                    },
                    Condition::UnusualVolume {
                        baseline_multiplier: 3.0,
                        time_window: Duration::hours(1),
                    },
                ],
                severity: Severity::Critical,
                alert_message: "Potential PHI data exfiltration detected".to_string(),
                enabled: true,
            },
            CorrelationRule {
                id: "insider_threat".to_string(),
                name: "Insider Threat Detection".to_string(),
                conditions: vec![
                    Condition::BehavioralAnomaly {
                        anomaly_types: vec![
                            "high_risk_activity_burst".to_string(),
                            "unusual_timing_pattern".to_string(),
                        ],
                        time_window: Duration::hours(2),
                    },
                    Condition::PrivilegeEscalation {
                        time_window: Duration::hours(1),
                    },
                ],
                severity: Severity::High,
                alert_message: "Potential insider threat detected".to_string(),
                enabled: true,
            },
            CorrelationRule {
                id: "dDoS_attack".to_string(),
                name: "DDoS Attack Detection".to_string(),
                conditions: vec![
                    Condition::EventCount {
                        event_type: AuditEventType::Security,
                        count: 100,
                        time_window: Duration::minutes(1),
                        success: Some(false),
                    },
                    Condition::SourceDiversity {
                        unique_sources: 50,
                        time_window: Duration::minutes(1),
                    },
                ],
                severity: Severity::Critical,
                alert_message: "Potential DDoS attack detected".to_string(),
                enabled: true,
            },
            CorrelationRule {
                id: "malware_infection".to_string(),
                name: "Malware Infection Indicators".to_string(),
                conditions: vec![
                    Condition::EventSequence {
                        events: vec![
                            EventPattern {
                                event_type: AuditEventType::Security,
                                resource_pattern: Some("malware_signatures".to_string()),
                            },
                            EventPattern {
                                event_type: AuditEventType::DataModification,
                                resource_pattern: Some("system_files".to_string()),
                            },
                        ],
                        time_window: Duration::minutes(10),
                    },
                    Condition::AnomalyDetection {
                        metric: "file_modifications".to_string(),
                        threshold: 5.0,
                        time_window: Duration::hours(1),
                    },
                ],
                severity: Severity::Critical,
                alert_message: "Potential malware infection detected".to_string(),
                enabled: true,
            },
        ]
    }

    /// Process security event for correlation
    pub async fn process_event(&self, event: SecurityEvent) -> anyhow::Result<()> {
        // Add event to buffer
        {
            let mut buffer = self.event_buffer.write().await;
            buffer.push_back(event.clone());

            // Maintain buffer size (last 24 hours)
            let cutoff = Utc::now() - Duration::hours(24);
            while buffer.front().map_or(false, |e| e.timestamp < cutoff) {
                buffer.pop_front();
            }
        }

        // Check correlation rules
        self.check_correlations().await?;

        Ok(())
    }

    /// Check all correlation rules against recent events
    async fn check_correlations(&self) -> anyhow::Result<()> {
        let rules = self.correlation_rules.read().await.clone();
        let events = self.event_buffer.read().await.clone();

        for rule in rules {
            if !rule.enabled {
                continue;
            }

            if let Some(correlation) = self.evaluate_rule(&rule, &events).await {
                self.trigger_alert(&rule, correlation).await?;
            }
        }

        Ok(())
    }

    /// Evaluate correlation rule against events
    async fn evaluate_rule(&self, rule: &CorrelationRule, events: &VecDeque<SecurityEvent>) -> Option<CorrelationResult> {
        let mut condition_results = Vec::new();
        let mut overall_confidence = 0.0;

        for condition in &rule.conditions {
            let result = self.evaluate_condition(condition, events).await;
            condition_results.push(result);

            if let Some(confidence) = result.confidence {
                overall_confidence += confidence;
            } else {
                // If any condition fails, the rule doesn't match
                return None;
            }
        }

        if condition_results.is_empty() {
            return None;
        }

        overall_confidence /= condition_results.len() as f64;

        Some(CorrelationResult {
            rule_id: rule.id.clone(),
            confidence: overall_confidence,
            matched_conditions: condition_results,
            evidence: self.gather_evidence(&condition_results, events),
        })
    }

    /// Evaluate individual correlation condition
    async fn evaluate_condition(&self, condition: &Condition, events: &VecDeque<SecurityEvent>) -> ConditionResult {
        match condition {
            Condition::EventCount { event_type, count, time_window, success } => {
                let cutoff = Utc::now() - *time_window;
                let matching_events = events.iter()
                    .filter(|e| e.timestamp > cutoff)
                    .filter(|e| e.event_type == *event_type)
                    .filter(|e| success.map_or(true, |s| e.success == s))
                    .count();

                ConditionResult {
                    condition_type: "event_count".to_string(),
                    matched: matching_events >= *count,
                    confidence: if matching_events >= *count {
                        Some((matching_events as f64 / *count as f64).min(1.0))
                    } else {
                        None
                    },
                    evidence: format!("Found {} matching events, required {}", matching_events, count),
                }
            }
            Condition::SourceCorrelation { correlation_type, time_window } => {
                let cutoff = Utc::now() - *time_window;
                let recent_events: Vec<_> = events.iter()
                    .filter(|e| e.timestamp > cutoff)
                    .collect();

                let correlation_strength = match correlation_type {
                    CorrelationType::SameIP => self.calculate_ip_correlation(&recent_events),
                    CorrelationType::SameUser => self.calculate_user_correlation(&recent_events),
                    CorrelationType::SameResource => self.calculate_resource_correlation(&recent_events),
                };

                ConditionResult {
                    condition_type: "source_correlation".to_string(),
                    matched: correlation_strength > 0.7,
                    confidence: if correlation_strength > 0.7 { Some(correlation_strength) } else { None },
                    evidence: format!("Correlation strength: {:.2}", correlation_strength),
                }
            }
            Condition::UnusualVolume { baseline_multiplier, time_window } => {
                let cutoff = Utc::now() - *time_window;
                let recent_count = events.iter()
                    .filter(|e| e.timestamp > cutoff)
                    .count();

                // Simplified baseline calculation (would use historical data in production)
                let baseline = 10.0; // Average events per time window
                let is_unusual = recent_count as f64 > baseline * baseline_multiplier;

                ConditionResult {
                    condition_type: "unusual_volume".to_string(),
                    matched: is_unusual,
                    confidence: if is_unusual {
                        Some((recent_count as f64 / baseline).min(2.0) / 2.0)
                    } else {
                        None
                    },
                    evidence: format!("Event count: {}, baseline: {}", recent_count, baseline),
                }
            }
            Condition::BehavioralAnomaly { anomaly_types, time_window } => {
                let cutoff = Utc::now() - *time_window;
                let anomalies_found = events.iter()
                    .filter(|e| e.timestamp > cutoff)
                    .filter(|e| anomaly_types.contains(&e.details.get("anomaly_type")
                        .and_then(|v| v.as_str()).unwrap_or("").to_string()))
                    .count();

                ConditionResult {
                    condition_type: "behavioral_anomaly".to_string(),
                    matched: anomalies_found > 0,
                    confidence: if anomalies_found > 0 {
                        Some((anomalies_found as f64 / 3.0).min(1.0)) // Normalize by expected max
                    } else {
                        None
                    },
                    evidence: format!("Found {} behavioral anomalies", anomalies_found),
                }
            }
            Condition::PrivilegeEscalation { time_window } => {
                let cutoff = Utc::now() - *time_window;
                let escalation_events = events.iter()
                    .filter(|e| e.timestamp > cutoff)
                    .filter(|e| e.event_type == AuditEventType::Authorization)
                    .filter(|e| e.details.get("privilege_escalation")
                        .and_then(|v| v.as_bool()).unwrap_or(false))
                    .count();

                ConditionResult {
                    condition_type: "privilege_escalation".to_string(),
                    matched: escalation_events > 0,
                    confidence: if escalation_events > 0 { Some(0.9) } else { None },
                    evidence: format!("Found {} privilege escalation events", escalation_events),
                }
            }
            Condition::EventSequence { events: expected_sequence, time_window } => {
                let matched = self.detect_event_sequence(expected_sequence, events, *time_window);
                ConditionResult {
                    condition_type: "event_sequence".to_string(),
                    matched,
                    confidence: if matched { Some(0.95) } else { None },
                    evidence: "Event sequence matched".to_string(),
                }
            }
            Condition::SourceDiversity { unique_sources, time_window } => {
                let cutoff = Utc::now() - *time_window;
                let sources: std::collections::HashSet<_> = events.iter()
                    .filter(|e| e.timestamp > cutoff)
                    .filter_map(|e| e.source_ip.clone())
                    .collect();

                let matched = sources.len() >= *unique_sources;
                ConditionResult {
                    condition_type: "source_diversity".to_string(),
                    matched,
                    confidence: if matched {
                        Some((sources.len() as f64 / *unique_sources as f64).min(1.0))
                    } else {
                        None
                    },
                    evidence: format!("Found {} unique sources, required {}", sources.len(), unique_sources),
                }
            }
            Condition::AnomalyDetection { metric, threshold, time_window } => {
                // Simplified anomaly detection (would use statistical methods in production)
                let cutoff = Utc::now() - *time_window;
                let metric_value = events.iter()
                    .filter(|e| e.timestamp > cutoff)
                    .filter(|e| e.details.get("metric")
                        .and_then(|v| v.as_str()) == Some(metric))
                    .filter_map(|e| e.details.get("value").and_then(|v| v.as_f64()))
                    .sum::<f64>();

                let matched = metric_value > *threshold;
                ConditionResult {
                    condition_type: "anomaly_detection".to_string(),
                    matched,
                    confidence: if matched { Some((metric_value / threshold).min(2.0) / 2.0) } else { None },
                    evidence: format!("Metric {} value: {:.2}, threshold: {:.2}", metric, metric_value, threshold),
                }
            }
        }
    }

    /// Calculate IP address correlation strength
    fn calculate_ip_correlation(&self, events: &[&SecurityEvent]) -> f64 {
        if events.is_empty() {
            return 0.0;
        }

        let mut ip_counts = HashMap::new();
        for event in events {
            if let Some(ip) = &event.source_ip {
                *ip_counts.entry(ip.clone()).or_insert(0) += 1;
            }
        }

        let max_count = ip_counts.values().max().unwrap_or(&0);
        *max_count as f64 / events.len() as f64
    }

    /// Calculate user correlation strength
    fn calculate_user_correlation(&self, events: &[&SecurityEvent]) -> f64 {
        if events.is_empty() {
            return 0.0;
        }

        let mut user_counts = HashMap::new();
        for event in events {
            if let Some(user) = &event.user_id {
                *user_counts.entry(user.clone()).or_insert(0) += 1;
            }
        }

        let max_count = user_counts.values().max().unwrap_or(&0);
        *max_count as f64 / events.len() as f64
    }

    /// Calculate resource correlation strength
    fn calculate_resource_correlation(&self, events: &[&SecurityEvent]) -> f64 {
        if events.is_empty() {
            return 0.0;
        }

        let mut resource_counts = HashMap::new();
        for event in events {
            *resource_counts.entry(event.resource.clone()).or_insert(0) += 1;
        }

        let max_count = resource_counts.values().max().unwrap_or(&0);
        *max_count as f64 / events.len() as f64
    }

    /// Detect event sequence
    fn detect_event_sequence(&self, sequence: &[EventPattern], events: &VecDeque<SecurityEvent>, time_window: Duration) -> bool {
        if sequence.is_empty() {
            return false;
        }

        let cutoff = Utc::now() - time_window;
        let recent_events: Vec<_> = events.iter()
            .filter(|e| e.timestamp > cutoff)
            .collect();

        if recent_events.len() < sequence.len() {
            return false;
        }

        // Simple sequence matching (would use more sophisticated pattern matching in production)
        let mut sequence_index = 0;
        for event in recent_events {
            if sequence_index < sequence.len() &&
               event.event_type == sequence[sequence_index].event_type {
                sequence_index += 1;
                if sequence_index == sequence.len() {
                    return true;
                }
            }
        }

        false
    }

    /// Gather evidence for correlation
    fn gather_evidence(&self, condition_results: &[ConditionResult], events: &VecDeque<SecurityEvent>) -> Vec<String> {
        let mut evidence = Vec::new();

        for result in condition_results {
            evidence.push(result.evidence.clone());
        }

        // Add summary of recent events
        let recent_events = events.iter()
            .rev()
            .take(5)
            .map(|e| format!("{}: {} on {}", e.event_type, e.action, e.resource))
            .collect::<Vec<_>>();

        evidence.push(format!("Recent events: {}", recent_events.join(", ")));

        evidence
    }

    /// Trigger alert for correlation match
    async fn trigger_alert(&self, rule: &CorrelationRule, correlation: CorrelationResult) -> anyhow::Result<()> {
        let alert_id = format!("{}_{}", rule.id, Utc::now().timestamp());

        let alert = Alert {
            id: alert_id.clone(),
            rule_id: rule.id.clone(),
            title: rule.name.clone(),
            message: rule.alert_message.clone(),
            severity: rule.severity.clone(),
            confidence: correlation.confidence,
            evidence: correlation.evidence,
            timestamp: Utc::now(),
            status: AlertStatus::Active,
        };

        // Store active alert
        {
            let mut alerts = self.active_alerts.write().await;
            alerts.insert(alert_id.clone(), alert.clone());
        }

        // Log alert event
        self.log_alert_event(&alert).await?;

        // Notify alert handlers
        self.notify_handlers(&alert).await;

        Ok(())
    }

    /// Log alert as audit event
    async fn log_alert_event(&self, alert: &Alert) -> anyhow::Result<()> {
        let mut details = std::collections::HashMap::new();
        details.insert("alert_id".to_string(), serde_json::Value::String(alert.id.clone()));
        details.insert("rule_id".to_string(), serde_json::Value::String(alert.rule_id.clone()));
        details.insert("severity".to_string(), serde_json::Value::String(format!("{:?}", alert.severity)));
        details.insert("confidence".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(alert.confidence).unwrap()));
        details.insert("evidence".to_string(), serde_json::Value::Array(
            alert.evidence.iter().map(|e| serde_json::Value::String(e.clone())).collect()
        ));

        let event = AuditEvent {
            id: Uuid::new_v4(),
            event_type: AuditEventType::Security,
            user_id: None,
            resource: "correlation_engine".to_string(),
            action: "alert_triggered".to_string(),
            data_class: "security_alert".to_string(),
            timestamp: alert.timestamp,
            ip_address: None,
            user_agent: None,
            success: true,
            details,
            hash: String::new(),
        };

        self.audit_manager.log_event(event).await
    }

    /// Notify alert handlers
    async fn notify_handlers(&self, alert: &Alert) {
        let handlers = self.alert_handlers.read().await;
        for handler in handlers.iter() {
            let future = handler(alert);
            if let Err(e) = future.await {
                tracing::error!("Alert handler failed: {}", e);
            }
        }
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        self.active_alerts.read().await.values().cloned().collect()
    }

    /// Acknowledge alert
    pub async fn acknowledge_alert(&self, alert_id: &str) -> anyhow::Result<()> {
        let mut alerts = self.active_alerts.write().await;
        if let Some(alert) = alerts.get_mut(alert_id) {
            alert.status = AlertStatus::Acknowledged;
        }
        Ok(())
    }

    /// Resolve alert
    pub async fn resolve_alert(&self, alert_id: &str) -> anyhow::Result<()> {
        let mut alerts = self.active_alerts.write().await;
        if let Some(alert) = alerts.get_mut(alert_id) {
            alert.status = AlertStatus::Resolved;
        }
        Ok(())
    }

    /// Add alert handler
    pub async fn add_alert_handler(&self, handler: Box<dyn Fn(&Alert) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + '_>> + Send + Sync>) {
        self.alert_handlers.write().await.push(handler);
    }

    /// Get correlation statistics
    pub async fn get_correlation_stats(&self) -> CorrelationStats {
        let alerts = self.active_alerts.read().await;
        let buffer = self.event_buffer.read().await;

        let total_events = buffer.len();
        let active_alerts = alerts.len();
        let critical_alerts = alerts.values().filter(|a| matches!(a.severity, Severity::Critical)).count();
        let high_alerts = alerts.values().filter(|a| matches!(a.severity, Severity::High)).count();

        let alerts_by_rule = alerts.values()
            .fold(HashMap::new(), |mut acc, alert| {
                *acc.entry(alert.rule_id.clone()).or_insert(0) += 1;
                acc
            });

        CorrelationStats {
            total_events,
            active_alerts,
            critical_alerts,
            high_alerts,
            alerts_by_rule,
            last_update: Utc::now(),
        }
    }
}

// Alert handler trait removed for simplicity

/// Security event for correlation
#[derive(Debug, Clone)]
pub struct SecurityEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub user_id: Option<String>,
    pub resource: String,
    pub action: String,
    pub success: bool,
    pub source_ip: Option<String>,
    pub details: HashMap<String, serde_json::Value>,
}

/// Correlation rule
#[derive(Debug, Clone)]
pub struct CorrelationRule {
    pub id: String,
    pub name: String,
    pub conditions: Vec<Condition>,
    pub severity: Severity,
    pub alert_message: String,
    pub enabled: bool,
}

/// Correlation condition
#[derive(Debug, Clone)]
pub enum Condition {
    EventCount {
        event_type: AuditEventType,
        count: usize,
        time_window: Duration,
        success: Option<bool>,
    },
    SourceCorrelation {
        correlation_type: CorrelationType,
        time_window: Duration,
    },
    UnusualVolume {
        baseline_multiplier: f64,
        time_window: Duration,
    },
    BehavioralAnomaly {
        anomaly_types: Vec<String>,
        time_window: Duration,
    },
    PrivilegeEscalation {
        time_window: Duration,
    },
    EventSequence {
        events: Vec<EventPattern>,
        time_window: Duration,
    },
    SourceDiversity {
        unique_sources: usize,
        time_window: Duration,
    },
    AnomalyDetection {
        metric: String,
        threshold: f64,
        time_window: Duration,
    },
}

/// Correlation type
#[derive(Debug, Clone)]
pub enum CorrelationType {
    SameIP,
    SameUser,
    SameResource,
}

/// Event pattern for sequence matching
#[derive(Debug, Clone)]
pub struct EventPattern {
    pub event_type: AuditEventType,
    pub resource_pattern: Option<String>,
}

/// Correlation result
#[derive(Debug, Clone)]
pub struct CorrelationResult {
    pub rule_id: String,
    pub confidence: f64,
    pub matched_conditions: Vec<ConditionResult>,
    pub evidence: Vec<String>,
}

/// Condition evaluation result
#[derive(Debug, Clone)]
pub struct ConditionResult {
    pub condition_type: String,
    pub matched: bool,
    pub confidence: Option<f64>,
    pub evidence: String,
}

/// Security alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub rule_id: String,
    pub title: String,
    pub message: String,
    pub severity: Severity,
    pub confidence: f64,
    pub evidence: Vec<String>,
    pub timestamp: DateTime<Utc>,
    pub status: AlertStatus,
}

/// Alert status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertStatus {
    Active,
    Acknowledged,
    Resolved,
}

/// Correlation statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CorrelationStats {
    pub total_events: usize,
    pub active_alerts: usize,
    pub critical_alerts: usize,
    pub high_alerts: usize,
    pub alerts_by_rule: HashMap<String, usize>,
    pub last_update: DateTime<Utc>,
}