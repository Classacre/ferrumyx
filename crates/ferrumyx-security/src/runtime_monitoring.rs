//! Runtime application security monitoring and threat detection

use crate::audit::{AuditManager, AuditEvent, AuditEventType};
use crate::correlation_engine::CorrelationEngine;
use crate::incident_response::IncidentResponseEngine;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use regex::Regex;

/// Runtime security monitor for application-level threat detection
pub struct RuntimeSecurityMonitor {
    /// Audit manager for logging security events
    audit_manager: Arc<AuditManager>,
    /// Threat detection engine
    threat_detector: Arc<ThreatDetectionEngine>,
    /// Correlation engine for event analysis
    correlation_engine: Arc<CorrelationEngine>,
    /// Incident response engine
    incident_response: Arc<IncidentResponseEngine>,
    /// Request monitoring state
    request_monitor: Arc<RequestMonitor>,
    /// Security event buffer
    event_buffer: Arc<RwLock<VecDeque<SecurityEvent>>>,
    /// Active monitoring rules
    monitoring_rules: Arc<RwLock<Vec<MonitoringRule>>>,
}

impl RuntimeSecurityMonitor {
    /// Create new runtime security monitor
    pub async fn new(
        audit_manager: Arc<AuditManager>,
        correlation_engine: Arc<CorrelationEngine>,
        incident_response: Arc<IncidentResponseEngine>,
    ) -> anyhow::Result<Self> {
        let threat_detector = Arc::new(ThreatDetectionEngine::new()?);
        let request_monitor = Arc::new(RequestMonitor::new());

        // Initialize default monitoring rules
        let monitoring_rules = Self::load_default_rules();

        Ok(Self {
            audit_manager,
            threat_detector,
            correlation_engine,
            incident_response,
            request_monitor,
            event_buffer: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            monitoring_rules: Arc::new(RwLock::new(monitoring_rules)),
        })
    }

    /// Load default monitoring rules
    fn load_default_rules() -> Vec<MonitoringRule> {
        vec![
            MonitoringRule {
                id: "sql_injection_detection".to_string(),
                name: "SQL Injection Detection".to_string(),
                pattern_type: PatternType::Regex,
                pattern: r"(?i)(union\s+select|select\s+.*\s+from|insert\s+into|update\s+.*\s+set|delete\s+from).*('|(\\x27)|(\\x2D\\x2D)|#)".to_string(),
                severity: Severity::High,
                action: SecurityAction::Block,
                enabled: true,
            },
            MonitoringRule {
                id: "xss_detection".to_string(),
                name: "Cross-Site Scripting Detection".to_string(),
                pattern_type: PatternType::Regex,
                pattern: r"(?i)<script[^>]*>.*?</script>|<.*?javascript:|<.*?on\w+\s*=|<.*?vbscript:|&lt;script".to_string(),
                severity: Severity::High,
                action: SecurityAction::Block,
                enabled: true,
            },
            MonitoringRule {
                id: "path_traversal".to_string(),
                name: "Path Traversal Detection".to_string(),
                pattern_type: PatternType::Regex,
                pattern: r"(?:\.\./|\.\.\\|%2e%2e%2f|%2e%2e%5c)".to_string(),
                severity: Severity::Critical,
                action: SecurityAction::Block,
                enabled: true,
            },
            MonitoringRule {
                id: "command_injection".to_string(),
                name: "Command Injection Detection".to_string(),
                pattern_type: PatternType::Regex,
                pattern: r"(?i)(;|\\||&|\\$\\(|`|\\$\\{).*?(cat|ls|rm|cp|mv|chmod|chown|wget|curl|python|perl|bash|sh)".to_string(),
                severity: Severity::Critical,
                action: SecurityAction::Block,
                enabled: true,
            },
            MonitoringRule {
                id: "suspicious_user_agent".to_string(),
                name: "Suspicious User Agent Detection".to_string(),
                pattern_type: PatternType::Regex,
                pattern: r"(?i)(sqlmap|nmap|nikto|dirbuster|acunetix|openvas|zaproxy|burpsuite|owasp)".to_string(),
                severity: Severity::Medium,
                action: SecurityAction::Flag,
                enabled: true,
            },
            MonitoringRule {
                id: "rate_limiting_bypass".to_string(),
                name: "Rate Limiting Bypass Detection".to_string(),
                pattern_type: PatternType::Custom,
                pattern: "rate_limit_bypass".to_string(),
                severity: Severity::Medium,
                action: SecurityAction::Throttle,
                enabled: true,
            },
        ]
    }

    /// Monitor incoming request
    pub async fn monitor_request(&self, request: &SecurityRequest) -> SecurityDecision {
        let mut threats = Vec::new();

        // Run threat detection
        for rule in self.monitoring_rules.read().await.iter() {
            if !rule.enabled {
                continue;
            }

            let threat_level = self.threat_detector.detect_threat(request, rule).await;

            if threat_level.confidence > 0.0 {
                threats.push(DetectedThreat {
                    rule_id: rule.id.clone(),
                    threat_type: rule.id.clone(),
                    confidence: threat_level.confidence,
                    severity: rule.severity.clone(),
                    evidence: threat_level.evidence.clone(),
                });
            }
        }

        // Update request monitor
        self.request_monitor.record_request(request).await;

        // Check for rate limiting violations
        if let Some(rate_violation) = self.request_monitor.check_rate_limits(request).await {
            threats.push(rate_violation);
        }

        // Make security decision
        let decision = self.make_security_decision(&threats);

        // Log security event
        self.log_security_event(request, &threats, &decision).await;

        // Trigger incident response if needed
        if matches!(decision.action, SecurityAction::Block | SecurityAction::Quarantine) {
            self.incident_response.handle_incident(
                &Incident {
                    id: uuid::Uuid::new_v4(),
                    timestamp: Utc::now(),
                    incident_type: "security_threat_detected".to_string(),
                    severity: Severity::High,
                    description: format!("Security threat detected: {:?}", threats),
                    source: request.source.clone(),
                    details: {
                        let mut details = HashMap::new();
                        details.insert("threats".to_string(), serde_json::to_value(&threats).unwrap_or_default());
                        details
                    },
                    status: IncidentStatus::Active,
                }
            ).await;
        }

        decision
    }

    /// Make security decision based on detected threats
    fn make_security_decision(&self, threats: &[DetectedThreat]) -> SecurityDecision {
        if threats.is_empty() {
            return SecurityDecision {
                action: SecurityAction::Allow,
                confidence: 1.0,
                reason: "No threats detected".to_string(),
            };
        }

        // Find highest severity threat
        let highest_severity = threats.iter()
            .max_by_key(|t| match t.severity {
                Severity::Low => 1,
                Severity::Medium => 2,
                Severity::High => 3,
                Severity::Critical => 4,
            })
            .map(|t| &t.severity)
            .unwrap();

        // Calculate overall confidence
        let avg_confidence = threats.iter()
            .map(|t| t.confidence)
            .sum::<f64>() / threats.len() as f64;

        let action = match highest_severity {
            Severity::Critical => SecurityAction::Block,
            Severity::High => SecurityAction::Block,
            Severity::Medium => SecurityAction::Flag,
            Severity::Low => SecurityAction::Allow,
        };

        SecurityDecision {
            action,
            confidence: avg_confidence,
            reason: format!("Detected {} threats with highest severity {:?}", threats.len(), highest_severity),
        }
    }

    /// Log security event
    async fn log_security_event(
        &self,
        request: &SecurityRequest,
        threats: &[DetectedThreat],
        decision: &SecurityDecision,
    ) {
        let mut details = std::collections::HashMap::new();
        details.insert("request_path".to_string(), serde_json::Value::String(request.path.clone()));
        details.insert("request_method".to_string(), serde_json::Value::String(request.method.clone()));
        details.insert("source_ip".to_string(), serde_json::Value::String(request.source.clone()));
        details.insert("threats_detected".to_string(), serde_json::to_value(&threats).unwrap_or_default());
        details.insert("decision".to_string(), serde_json::to_value(decision).unwrap_or_default());

        let event = AuditEvent {
            id: uuid::Uuid::new_v4(),
            event_type: AuditEventType::Security,
            user_id: request.user_id.clone(),
            resource: request.path.clone(),
            action: "monitor".to_string(),
            data_class: "security_monitoring".to_string(),
            timestamp: Utc::now(),
            ip_address: Some(request.source.clone()),
            user_agent: request.user_agent.clone(),
            success: matches!(decision.action, SecurityAction::Allow),
            details,
            hash: String::new(),
        };

        if let Err(e) = self.audit_manager.log_event(event).await {
            tracing::error!("Failed to log security event: {}", e);
        }
    }

    /// Get monitoring statistics
    pub async fn get_monitoring_stats(&self) -> MonitoringStats {
        let events = self.event_buffer.read().await;
        let rules = self.monitoring_rules.read().await;

        let total_requests = events.len();
        let blocked_requests = events.iter()
            .filter(|e| matches!(e.decision.action, SecurityAction::Block))
            .count();
        let flagged_requests = events.iter()
            .filter(|e| matches!(e.decision.action, SecurityAction::Flag))
            .count();

        let threats_by_type = events.iter()
            .flat_map(|e| &e.threats)
            .fold(HashMap::new(), |mut acc, threat| {
                *acc.entry(threat.threat_type.clone()).or_insert(0) += 1;
                acc
            });

        MonitoringStats {
            total_requests,
            blocked_requests,
            flagged_requests,
            allowed_requests: total_requests - blocked_requests - flagged_requests,
            threats_by_type,
            active_rules: rules.iter().filter(|r| r.enabled).count(),
            last_update: Utc::now(),
        }
    }

    /// Add custom monitoring rule
    pub async fn add_monitoring_rule(&self, rule: MonitoringRule) {
        self.monitoring_rules.write().await.push(rule);
    }

    /// Remove monitoring rule
    pub async fn remove_monitoring_rule(&self, rule_id: &str) {
        self.monitoring_rules.write().await.retain(|r| r.id != rule_id);
    }
}

/// Threat detection engine
pub struct ThreatDetectionEngine {
    regex_cache: RwLock<HashMap<String, Regex>>,
}

impl ThreatDetectionEngine {
    /// Create new threat detection engine
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            regex_cache: RwLock::new(HashMap::new()),
        })
    }

    /// Detect threats in request using rule
    pub async fn detect_threat(&self, request: &SecurityRequest, rule: &MonitoringRule) -> ThreatLevel {
        match rule.pattern_type {
            PatternType::Regex => self.detect_regex_threat(request, rule).await,
            PatternType::Custom => self.detect_custom_threat(request, rule).await,
        }
    }

    /// Detect regex-based threats
    async fn detect_regex_threat(&self, request: &SecurityRequest, rule: &MonitoringRule) -> ThreatLevel {
        let regex = {
            let mut cache = self.regex_cache.write().await;
            if let Some(regex) = cache.get(&rule.pattern) {
                regex.clone()
            } else {
                match Regex::new(&rule.pattern) {
                    Ok(regex) => {
                        cache.insert(rule.pattern.clone(), regex.clone());
                        regex
                    }
                    Err(_) => return ThreatLevel { confidence: 0.0, evidence: Vec::new() },
                }
            }
        };

        let mut evidence = Vec::new();
        let mut confidence = 0.0;

        // Check various request components
        let check_locations = vec![
            ("path", &request.path),
            ("query", &request.query_string),
            ("body", &request.body),
            ("headers", &request.headers),
        ];

        for (location, content) in check_locations {
            if regex.is_match(content) {
                confidence = 0.9; // High confidence for regex matches
                evidence.push(format!("Pattern matched in {}: {}", location, rule.name));
            }
        }

        ThreatLevel { confidence, evidence }
    }

    /// Detect custom threats (placeholder for ML-based detection)
    async fn detect_custom_threat(&self, request: &SecurityRequest, rule: &MonitoringRule) -> ThreatLevel {
        match rule.pattern.as_str() {
            "rate_limit_bypass" => {
                // This would be handled by RequestMonitor
                ThreatLevel { confidence: 0.0, evidence: Vec::new() }
            }
            _ => ThreatLevel { confidence: 0.0, evidence: Vec::new() },
        }
    }
}

/// Request monitor for rate limiting and pattern analysis
pub struct RequestMonitor {
    request_counts: RwLock<HashMap<String, Vec<DateTime<Utc>>>>,
}

impl RequestMonitor {
    /// Create new request monitor
    pub fn new() -> Self {
        Self {
            request_counts: RwLock::new(HashMap::new()),
        }
    }

    /// Record request for rate limiting
    pub async fn record_request(&self, request: &SecurityRequest) {
        let mut counts = self.request_counts.write().await;
        let key = format!("{}:{}", request.source, request.path);

        let now = Utc::now();
        counts.entry(key).or_insert_with(Vec::new).push(now);

        // Clean old entries (older than 1 minute)
        let cutoff = now - Duration::minutes(1);
        counts.values_mut().for_each(|times| {
            times.retain(|&time| time > cutoff);
        });

        // Remove empty entries
        counts.retain(|_, times| !times.is_empty());
    }

    /// Check rate limits
    pub async fn check_rate_limits(&self, request: &SecurityRequest) -> Option<DetectedThreat> {
        let counts = self.request_counts.read().await;
        let key = format!("{}:{}", request.source, request.path);

        if let Some(times) = counts.get(&key) {
            let recent_requests = times.iter()
                .filter(|&&time| Utc::now() - time < Duration::minutes(1))
                .count();

            // Rate limit: 100 requests per minute per IP+path
            if recent_requests > 100 {
                return Some(DetectedThreat {
                    rule_id: "rate_limiting_bypass".to_string(),
                    threat_type: "rate_limit_exceeded".to_string(),
                    confidence: 0.95,
                    severity: Severity::Medium,
                    evidence: vec![format!("{} requests in last minute from {}", recent_requests, request.source)],
                });
            }
        }

        None
    }
}

/// Security request representation
#[derive(Debug, Clone)]
pub struct SecurityRequest {
    pub method: String,
    pub path: String,
    pub query_string: String,
    pub headers: String,
    pub body: String,
    pub source: String,
    pub user_agent: Option<String>,
    pub user_id: Option<String>,
}

/// Security decision
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecurityDecision {
    pub action: SecurityAction,
    pub confidence: f64,
    pub reason: String,
}

/// Security actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityAction {
    Allow,
    Flag,
    Throttle,
    Block,
    Quarantine,
}

/// Security severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Pattern types for detection
#[derive(Debug, Clone)]
pub enum PatternType {
    Regex,
    Custom,
}

/// Monitoring rule
#[derive(Debug, Clone)]
pub struct MonitoringRule {
    pub id: String,
    pub name: String,
    pub pattern_type: PatternType,
    pub pattern: String,
    pub severity: Severity,
    pub action: SecurityAction,
    pub enabled: bool,
}

/// Detected threat
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DetectedThreat {
    pub rule_id: String,
    pub threat_type: String,
    pub confidence: f64,
    pub severity: Severity,
    pub evidence: Vec<String>,
}

/// Threat level assessment
#[derive(Debug, Clone)]
pub struct ThreatLevel {
    pub confidence: f64,
    pub evidence: Vec<String>,
}

/// Security event
#[derive(Debug, Clone)]
pub struct SecurityEvent {
    pub timestamp: DateTime<Utc>,
    pub request: SecurityRequest,
    pub threats: Vec<DetectedThreat>,
    pub decision: SecurityDecision,
}

/// Monitoring statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MonitoringStats {
    pub total_requests: usize,
    pub blocked_requests: usize,
    pub flagged_requests: usize,
    pub allowed_requests: usize,
    pub threats_by_type: HashMap<String, usize>,
    pub active_rules: usize,
    pub last_update: DateTime<Utc>,
}

// Re-export incident types
pub use crate::incident_response::{Incident, IncidentStatus};