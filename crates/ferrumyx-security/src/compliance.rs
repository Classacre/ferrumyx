//! Continuous compliance monitoring

use crate::audit::{AuditManager, AuditEventType};
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time;

/// Compliance monitor for continuous compliance checking
pub struct ComplianceMonitor {
    audit_manager: Arc<AuditManager>,
    rules: Arc<RwLock<Vec<ComplianceRule>>>,
    last_check: Arc<RwLock<DateTime<Utc>>>,
    alerts: Arc<RwLock<Vec<ComplianceAlert>>>,
}

impl ComplianceMonitor {
    /// Create new compliance monitor
    pub async fn new(audit_manager: Arc<AuditManager>) -> anyhow::Result<Self> {
        let rules = Self::load_default_rules();
        let last_check = Utc::now();

        Ok(Self {
            audit_manager,
            rules: Arc::new(RwLock::new(rules)),
            last_check: Arc::new(RwLock::new(last_check)),
            alerts: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Load default compliance rules
    fn load_default_rules() -> Vec<ComplianceRule> {
        vec![
            ComplianceRule {
                id: "audit_log_integrity".to_string(),
                name: "Audit Log Integrity".to_string(),
                description: "Verify audit logs are complete and tamper-proof".to_string(),
                check_interval: Duration::hours(1),
                rule_type: RuleType::AuditIntegrity,
                severity: Severity::Critical,
                enabled: true,
            },
            ComplianceRule {
                id: "phi_access_controls".to_string(),
                name: "PHI Access Controls".to_string(),
                description: "Monitor PHI data access patterns for anomalies".to_string(),
                check_interval: Duration::hours(4),
                rule_type: RuleType::PhiAccess,
                severity: Severity::High,
                enabled: true,
            },
            ComplianceRule {
                id: "encryption_key_rotation".to_string(),
                name: "Encryption Key Rotation".to_string(),
                description: "Ensure encryption keys are rotated regularly".to_string(),
                check_interval: Duration::days(30),
                rule_type: RuleType::EncryptionKeyRotation,
                severity: Severity::Medium,
                enabled: true,
            },
            ComplianceRule {
                id: "access_pattern_analysis".to_string(),
                name: "Access Pattern Analysis".to_string(),
                description: "Detect unusual access patterns that may indicate security issues".to_string(),
                check_interval: Duration::hours(2),
                rule_type: RuleType::AccessPatterns,
                severity: Severity::Medium,
                enabled: true,
            },
            ComplianceRule {
                id: "data_retention_compliance".to_string(),
                name: "Data Retention Compliance".to_string(),
                description: "Ensure data is retained and disposed according to policy".to_string(),
                check_interval: Duration::days(7),
                rule_type: RuleType::DataRetention,
                severity: Severity::Low,
                enabled: true,
            },
        ]
    }

    /// Start continuous compliance monitoring
    pub async fn start_monitoring(&self) -> anyhow::Result<()> {
        let rules = self.rules.read().await.clone();

        for rule in rules {
            if rule.enabled {
                let audit_manager = self.audit_manager.clone();
                let alerts = self.alerts.clone();
                let last_check = self.last_check.clone();

                tokio::spawn(async move {
                    let mut interval = time::interval(rule.check_interval.to_std().unwrap());

                    loop {
                        interval.tick().await;

                        match Self::check_rule(&rule, &audit_manager).await {
                            Ok(violations) => {
                                if !violations.is_empty() {
                                    let alert = ComplianceAlert {
                                        rule_id: rule.id.clone(),
                                        timestamp: Utc::now(),
                                        violations,
                                        severity: rule.severity.clone(),
                                        status: AlertStatus::Active,
                                    };

                                    alerts.write().await.push(alert);
                                }
                            }
                            Err(e) => {
                                tracing::error!("Compliance check failed for rule {}: {}", rule.id, e);
                            }
                        }

                        *last_check.write().await = Utc::now();
                    }
                });
            }
        }

        Ok(())
    }

    /// Check a specific compliance rule
    async fn check_rule(rule: &ComplianceRule, audit_manager: &AuditManager) -> anyhow::Result<Vec<ComplianceViolation>> {
        match rule.rule_type {
            RuleType::AuditIntegrity => Self::check_audit_integrity(audit_manager).await,
            RuleType::PhiAccess => Self::check_phi_access(audit_manager).await,
            RuleType::EncryptionKeyRotation => Self::check_key_rotation().await,
            RuleType::AccessPatterns => Self::check_access_patterns(audit_manager).await,
            RuleType::DataRetention => Self::check_data_retention(audit_manager).await,
        }
    }

    /// Check audit log integrity
    async fn check_audit_integrity(audit_manager: &AuditManager) -> anyhow::Result<Vec<ComplianceViolation>> {
        let end_time = Utc::now();
        let start_time = end_time - Duration::hours(24);

        let integrity_report = audit_manager.verify_integrity(start_time, end_time).await?;

        let mut violations = Vec::new();

        if integrity_report.integrity_score < 95.0 {
            violations.push(ComplianceViolation {
                violation_type: "audit_integrity_low".to_string(),
                description: format!("Audit integrity score is {:.1}%", integrity_report.integrity_score),
                severity: Severity::Critical,
                evidence: format!("Found {} violations in {} events", integrity_report.violations.len(), integrity_report.total_events),
            });
        }

        Ok(violations)
    }

    /// Check PHI access patterns
    async fn check_phi_access(audit_manager: &AuditManager) -> anyhow::Result<Vec<ComplianceViolation>> {
        let end_time = Utc::now();
        let start_time = end_time - Duration::hours(24);

        let phi_events = audit_manager.get_events_for_compliance(
            start_time,
            end_time,
            Some(vec![AuditEventType::PhiAccess])
        ).await?;

        let mut violations = Vec::new();

        // Check for unauthorized PHI access
        let unauthorized_access = phi_events.iter()
            .filter(|e| !e.success)
            .count();

        if unauthorized_access > 5 {
            violations.push(ComplianceViolation {
                violation_type: "excessive_phi_access_denials".to_string(),
                description: format!("{} unauthorized PHI access attempts in 24 hours", unauthorized_access),
                severity: Severity::High,
                evidence: "Multiple failed PHI access attempts detected".to_string(),
            });
        }

        // Check for unusual access patterns (same user accessing many PHI records)
        let mut user_access_counts = HashMap::new();
        for event in &phi_events {
            if let Some(user_id) = &event.user_id {
                *user_access_counts.entry(user_id.clone()).or_insert(0) += 1;
            }
        }

        for (user_id, count) in user_access_counts {
            if count > 100 { // Threshold for suspicious activity
                violations.push(ComplianceViolation {
                    violation_type: "unusual_phi_access_pattern".to_string(),
                    description: format!("User {} accessed {} PHI records in 24 hours", user_id, count),
                    severity: Severity::Medium,
                    evidence: "Excessive PHI record access by single user".to_string(),
                });
            }
        }

        Ok(violations)
    }

    /// Check encryption key rotation
    async fn check_key_rotation() -> anyhow::Result<Vec<ComplianceViolation>> {
        // In a real implementation, this would check key metadata
        // For now, we'll simulate checking key age

        let mut violations = Vec::new();

        // Simulate checking if keys are older than 90 days
        let key_age_days = 45; // This would come from actual key metadata

        if key_age_days > 90 {
            violations.push(ComplianceViolation {
                violation_type: "encryption_key_overdue_rotation".to_string(),
                description: format!("Encryption keys are {} days old, rotation required", key_age_days),
                severity: Severity::Medium,
                evidence: "Keys exceed maximum age limit of 90 days".to_string(),
            });
        }

        Ok(violations)
    }

    /// Check access patterns for anomalies
    async fn check_access_patterns(audit_manager: &AuditManager) -> anyhow::Result<Vec<ComplianceViolation>> {
        let end_time = Utc::now();
        let start_time = end_time - Duration::hours(24);

        let events = audit_manager.get_events_for_compliance(
            start_time,
            end_time,
            None
        ).await?;

        let mut violations = Vec::new();

        // Check for brute force attempts (multiple failed authentications)
        let failed_auth_attempts = events.iter()
            .filter(|e| matches!(e.event_type, AuditEventType::Authentication) && !e.success)
            .count();

        if failed_auth_attempts > 10 {
            violations.push(ComplianceViolation {
                violation_type: "potential_brute_force_attack".to_string(),
                description: format!("{} failed authentication attempts in 24 hours", failed_auth_attempts),
                severity: Severity::High,
                evidence: "Unusual number of failed authentication events".to_string(),
            });
        }

        // Check for data exfiltration patterns (large data exports)
        let data_access_events = events.iter()
            .filter(|e| matches!(e.event_type, AuditEventType::DataAccess))
            .count();

        if data_access_events > 1000 {
            violations.push(ComplianceViolation {
                violation_type: "high_data_access_volume".to_string(),
                description: format!("{} data access events in 24 hours", data_access_events),
                severity: Severity::Medium,
                evidence: "Unusually high volume of data access events".to_string(),
            });
        }

        Ok(violations)
    }

    /// Check data retention compliance
    async fn check_data_retention(audit_manager: &AuditManager) -> anyhow::Result<Vec<ComplianceViolation>> {
        // This would check if old data is properly archived/deleted
        // For simulation, we'll check if there are very old audit events that should be archived

        let mut violations = Vec::new();

        // Check for events older than 7 years (typical retention period)
        let seven_years_ago = Utc::now() - Duration::days(365 * 7);

        // In a real implementation, this would query for old records
        // For now, we'll assume some old records exist
        let old_records_count = 0; // This would be queried from database

        if old_records_count > 0 {
            violations.push(ComplianceViolation {
                violation_type: "data_retention_violation".to_string(),
                description: format!("{} records older than retention period still exist", old_records_count),
                severity: Severity::Low,
                evidence: "Data retention policy not properly enforced".to_string(),
            });
        }

        Ok(violations)
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<ComplianceAlert> {
        self.alerts.read().await.clone()
    }

    /// Acknowledge alert
    pub async fn acknowledge_alert(&self, alert_id: &str) -> anyhow::Result<()> {
        let mut alerts = self.alerts.write().await;
        if let Some(alert) = alerts.iter_mut().find(|a| a.rule_id == alert_id) {
            alert.status = AlertStatus::Acknowledged;
        }
        Ok(())
    }

    /// Get compliance status summary
    pub async fn get_compliance_status(&self) -> ComplianceStatus {
        let alerts = self.get_active_alerts().await;
        let last_check = *self.last_check.read().await;

        let critical_alerts = alerts.iter().filter(|a| matches!(a.severity, Severity::Critical)).count();
        let high_alerts = alerts.iter().filter(|a| matches!(a.severity, Severity::High)).count();
        let medium_alerts = alerts.iter().filter(|a| matches!(a.severity, Severity::Medium)).count();
        let low_alerts = alerts.iter().filter(|a| matches!(a.severity, Severity::Low)).count();

        let overall_score = if critical_alerts > 0 {
            20.0 // Critical issues present
        } else if high_alerts > 0 {
            60.0 // High severity issues
        } else if medium_alerts > 0 {
            80.0 // Medium severity issues
        } else if low_alerts > 0 {
            95.0 // Only low severity issues
        } else {
            100.0 // No active alerts
        };

        ComplianceStatus {
            overall_score,
            critical_alerts,
            high_alerts,
            medium_alerts,
            low_alerts,
            last_check,
            status: if overall_score >= 80.0 {
                ComplianceLevel::Compliant
            } else if overall_score >= 60.0 {
                ComplianceLevel::Warning
            } else {
                ComplianceLevel::NonCompliant
            },
        }
    }
}

/// Compliance rule definition
#[derive(Debug, Clone)]
pub struct ComplianceRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub check_interval: Duration,
    pub rule_type: RuleType,
    pub severity: Severity,
    pub enabled: bool,
}

/// Types of compliance rules
#[derive(Debug, Clone)]
pub enum RuleType {
    AuditIntegrity,
    PhiAccess,
    EncryptionKeyRotation,
    AccessPatterns,
    DataRetention,
}

/// Severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Compliance violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceViolation {
    pub violation_type: String,
    pub description: String,
    pub severity: Severity,
    pub evidence: String,
}

/// Compliance alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceAlert {
    pub rule_id: String,
    pub timestamp: DateTime<Utc>,
    pub violations: Vec<ComplianceViolation>,
    pub severity: Severity,
    pub status: AlertStatus,
}

/// Alert status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertStatus {
    Active,
    Acknowledged,
    Resolved,
}

/// Compliance status summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
    pub overall_score: f64,
    pub critical_alerts: usize,
    pub high_alerts: usize,
    pub medium_alerts: usize,
    pub low_alerts: usize,
    pub last_check: DateTime<Utc>,
    pub status: ComplianceLevel,
}

/// Compliance levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplianceLevel {
    Compliant,
    Warning,
    NonCompliant,
}