//! Automated compliance reporting

use crate::audit::{AuditManager, AuditIntegrityReport};
use crate::compliance::{ComplianceMonitor, ComplianceStatus};
use crate::monitoring::{MonitoringStatus, SecurityAssessment};
use crate::phi::{PhiDetector, PhiTestResults};
use crate::scanner::{VulnerabilityScanner, VulnerabilityReport};
use crate::tests::{SecurityTestReport, SecurityTestRunner};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Compliance reporter for generating automated reports
pub struct ComplianceReporter {
    audit_manager: AuditManager,
    compliance_monitor: ComplianceMonitor,
    phi_detector: PhiDetector,
    vuln_scanner: VulnerabilityScanner,
    test_runner: SecurityTestRunner,
}

impl ComplianceReporter {
    /// Create new compliance reporter
    pub async fn new() -> anyhow::Result<Self> {
        let audit_manager = AuditManager::new().await?;
        let compliance_audit_manager = AuditManager::new().await?;
        let compliance_monitor = ComplianceMonitor::new(Arc::new(compliance_audit_manager)).await?;
        let phi_detector = PhiDetector::new()?;
        let vuln_scanner = VulnerabilityScanner::new().await?;
        let test_runner = SecurityTestRunner::new().await?;

        Ok(Self {
            audit_manager,
            compliance_monitor,
            phi_detector,
            vuln_scanner,
            test_runner,
        })
    }

    /// Generate comprehensive compliance report
    pub async fn generate_comprehensive_report(&self, period_days: i64) -> anyhow::Result<ComprehensiveComplianceReport> {
        let report_start = chrono::Utc::now() - chrono::Duration::days(period_days);
        let report_end = chrono::Utc::now();

        // Gather all report components
        let audit_integrity = self.audit_manager.verify_integrity(report_start, report_end).await?;
        let compliance_status = self.compliance_monitor.get_compliance_status().await;
        let phi_test_results = self.phi_detector.test_phi_detection();
        let vuln_report = self.vuln_scanner.run_full_scan().await?;
        let security_test_report = self.test_runner.run_comprehensive_tests().await?;

        // Calculate overall compliance score
        let overall_score = self.calculate_overall_compliance_score(
            &audit_integrity,
            &compliance_status,
            &phi_test_results,
            &vuln_report,
            &security_test_report,
        );

        // Generate compliance level
        let compliance_level = self.determine_compliance_level(overall_score);

        // Generate recommendations
        let recommendations = self.generate_comprehensive_recommendations(
            &audit_integrity,
            &compliance_status,
            &phi_test_results,
            &vuln_report,
            &security_test_report,
        );

        let phi_leaks_prevented = self.calculate_phi_leaks_prevented();
        let phi_detection_accuracy = phi_test_results.accuracy;
        let vuln_risk_score = vuln_report.risk_score;
        let vuln_critical_count = vuln_report.critical_vulnerabilities;

        Ok(ComprehensiveComplianceReport {
            report_id: uuid::Uuid::new_v4(),
            generated_at: chrono::Utc::now(),
            report_period: ReportPeriod {
                start: report_start,
                end: report_end,
            },
            overall_compliance_score: overall_score,
            compliance_level,
            audit_integrity_report: audit_integrity,
            compliance_status,
            phi_protection_status: PhiProtectionStatus {
                test_results: phi_test_results,
                phi_leaks_prevented,
                detection_accuracy: phi_detection_accuracy,
            },
            vulnerability_assessment: VulnerabilityAssessment {
                scan_report: vuln_report,
                risk_level: self.determine_risk_level(vuln_risk_score),
                critical_vulnerabilities: vuln_critical_count,
                remediation_priority: self.calculate_remediation_priority(&Default::default()), // Simplified
            },
            security_test_results: security_test_report,
            recommendations,
            next_review_date: chrono::Utc::now() + chrono::Duration::days(30),
        })
    }

    /// Generate daily compliance summary
    pub async fn generate_daily_summary(&self) -> anyhow::Result<DailyComplianceSummary> {
        let today = chrono::Utc::now().date_naive();
        let start_of_day = today.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end_of_day = (today + chrono::Duration::days(1)).and_hms_opt(0, 0, 0).unwrap().and_utc();

        let audit_events = self.audit_manager.get_events_for_compliance(
            start_of_day,
            end_of_day,
            None,
        ).await?;

        let compliance_status = self.compliance_monitor.get_compliance_status().await;
        let active_alerts = self.compliance_monitor.get_active_alerts().await;

        // Calculate daily metrics
        let total_events = audit_events.len();
        let security_events = audit_events.iter()
            .filter(|e| matches!(e.event_type, crate::audit::AuditEventType::Security))
            .count();
        let failed_events = audit_events.iter()
            .filter(|e| !e.success)
            .count();

        let events_by_type = self.categorize_events_by_type(&audit_events);
        let risk_indicators = self.identify_risk_indicators(&audit_events);

        Ok(DailyComplianceSummary {
            date: today,
            total_audit_events: total_events,
            security_events,
            failed_events,
            events_by_type,
            risk_indicators,
            compliance_status: compliance_status.status,
            active_alerts: active_alerts.len(),
            critical_alerts: active_alerts.iter()
                .filter(|a| matches!(a.severity, crate::compliance::Severity::Critical))
                .count(),
        })
    }

    /// Generate HIPAA compliance report
    pub async fn generate_hipaa_report(&self) -> anyhow::Result<HipaaComplianceReport> {
        let phi_test_results = self.phi_detector.test_phi_detection();
        let audit_integrity = self.audit_manager.verify_integrity(
            chrono::Utc::now() - chrono::Duration::days(90),
            chrono::Utc::now()
        ).await?;

        let phi_access_events = self.audit_manager.get_events_for_compliance(
            chrono::Utc::now() - chrono::Duration::days(30),
            chrono::Utc::now(),
            Some(vec![crate::audit::AuditEventType::PhiAccess])
        ).await?;

        // HIPAA-specific checks
        let privacy_rule_compliance = self.assess_privacy_rule_compliance(&phi_access_events);
        let security_rule_compliance = self.assess_security_rule_compliance(&audit_integrity);
        let breach_notification_compliance = self.assess_breach_notification_compliance();

        let overall_hipaa_score = (privacy_rule_compliance + security_rule_compliance + breach_notification_compliance) / 3.0;

        Ok(HipaaComplianceReport {
            report_date: chrono::Utc::now(),
            hipaa_version: "HIPAA Security Rule 2003".to_string(),
            overall_compliance_score: overall_hipaa_score,
            privacy_rule_score: privacy_rule_compliance,
            security_rule_score: security_rule_compliance,
            breach_notification_score: breach_notification_compliance,
            phi_protection_effectiveness: phi_test_results.accuracy * 100.0,
            audit_trail_integrity: audit_integrity.integrity_score,
            risk_assessment: self.generate_hipaa_risk_assessment(overall_hipaa_score),
            required_actions: self.generate_hipaa_required_actions(overall_hipaa_score),
        })
    }

    /// Calculate overall compliance score
    fn calculate_overall_compliance_score(
        &self,
        audit_integrity: &AuditIntegrityReport,
        compliance_status: &ComplianceStatus,
        phi_tests: &PhiTestResults,
        vuln_report: &VulnerabilityReport,
        security_tests: &SecurityTestReport,
    ) -> f64 {
        let weights = ComplianceWeights {
            audit_integrity: 0.25,
            compliance_status: 0.25,
            phi_protection: 0.20,
            vulnerability_management: 0.15,
            security_testing: 0.15,
        };

        let audit_score = audit_integrity.integrity_score;
        let compliance_score = match compliance_status.status {
            crate::compliance::ComplianceLevel::Compliant => 100.0,
            crate::compliance::ComplianceLevel::Warning => 75.0,
            crate::compliance::ComplianceLevel::NonCompliant => 50.0,
        };
        let phi_score = phi_tests.accuracy * 100.0;
        let vuln_score = vuln_report.risk_score;
        let test_score = security_tests.success_rate;

        (audit_score * weights.audit_integrity +
         compliance_score * weights.compliance_status +
         phi_score * weights.phi_protection +
         vuln_score * weights.vulnerability_management +
         test_score * weights.security_testing)
    }

    /// Determine compliance level
    fn determine_compliance_level(&self, score: f64) -> ComplianceLevel {
        match score {
            s if s >= 90.0 => ComplianceLevel::FullyCompliant,
            s if s >= 80.0 => ComplianceLevel::MostlyCompliant,
            s if s >= 70.0 => ComplianceLevel::PartiallyCompliant,
            s if s >= 60.0 => ComplianceLevel::NeedsImprovement,
            _ => ComplianceLevel::NonCompliant,
        }
    }

    /// Generate comprehensive recommendations
    fn generate_comprehensive_recommendations(
        &self,
        audit_integrity: &AuditIntegrityReport,
        compliance_status: &ComplianceStatus,
        phi_tests: &PhiTestResults,
        vuln_report: &VulnerabilityReport,
        security_tests: &SecurityTestReport,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if audit_integrity.integrity_score < 95.0 {
            recommendations.push("Improve audit trail integrity - address integrity violations".to_string());
        }

        if !matches!(compliance_status.status, crate::compliance::ComplianceLevel::Compliant) {
            recommendations.push("Address compliance violations to achieve full compliance status".to_string());
        }

        if phi_tests.accuracy < 0.9 {
            recommendations.push("Enhance PHI detection accuracy through algorithm improvements".to_string());
        }

        if vuln_report.risk_score < 80.0 {
            recommendations.push(format!("Remediate {} identified vulnerabilities", vuln_report.total_findings));
        }

        if security_tests.success_rate < 90.0 {
            recommendations.push("Fix failed security tests and improve test coverage".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("Maintain current security posture - all systems operating within acceptable parameters".to_string());
        }

        recommendations
    }

    /// Categorize events by type
    fn categorize_events_by_type(&self, events: &[crate::audit::AuditEvent]) -> HashMap<String, usize> {
        let mut categories = HashMap::new();

        for event in events {
            let category = match event.event_type {
                crate::audit::AuditEventType::Authentication => "Authentication",
                crate::audit::AuditEventType::Authorization => "Authorization",
                crate::audit::AuditEventType::DataAccess => "Data Access",
                crate::audit::AuditEventType::DataModification => "Data Modification",
                crate::audit::AuditEventType::Security => "Security",
                crate::audit::AuditEventType::Compliance => "Compliance",
                crate::audit::AuditEventType::PhiAccess => "PHI Access",
                crate::audit::AuditEventType::PhiDetection => "PHI Detection",
                crate::audit::AuditEventType::PhiBlocking => "PHI Blocking",
            };

            *categories.entry(category.to_string()).or_insert(0) += 1;
        }

        categories
    }

    /// Identify risk indicators
    fn identify_risk_indicators(&self, events: &[crate::audit::AuditEvent]) -> Vec<RiskIndicator> {
        let mut indicators = Vec::new();

        let failed_auth_attempts = events.iter()
            .filter(|e| matches!(e.event_type, crate::audit::AuditEventType::Authentication) && !e.success)
            .count();

        if failed_auth_attempts > 5 {
            indicators.push(RiskIndicator {
                indicator_type: "Failed Authentication Attempts".to_string(),
                severity: RiskSeverity::Medium,
                count: failed_auth_attempts,
                description: "Multiple failed authentication attempts detected".to_string(),
            });
        }

        let phi_access_events = events.iter()
            .filter(|e| matches!(e.event_type, crate::audit::AuditEventType::PhiAccess))
            .count();

        if phi_access_events > 10 {
            indicators.push(RiskIndicator {
                indicator_type: "High PHI Access Volume".to_string(),
                severity: RiskSeverity::Low,
                count: phi_access_events,
                description: "High volume of PHI access events".to_string(),
            });
        }

        indicators
    }

    /// Calculate PHI leaks prevented (placeholder)
    fn calculate_phi_leaks_prevented(&self) -> usize {
        // In a real implementation, this would track actual PHI blocking events
        0
    }

    /// Determine risk level from vulnerability score
    fn determine_risk_level(&self, score: f64) -> RiskLevel {
        match score {
            s if s >= 80.0 => RiskLevel::Low,
            s if s >= 60.0 => RiskLevel::Medium,
            s if s >= 40.0 => RiskLevel::High,
            _ => RiskLevel::Critical,
        }
    }

    /// Calculate remediation priority
    fn calculate_remediation_priority(&self, report: &VulnerabilityReport) -> RemediationPriority {
        if report.critical_vulnerabilities > 0 {
            RemediationPriority::Critical
        } else if report.high_vulnerabilities > 5 {
            RemediationPriority::High
        } else if report.medium_vulnerabilities > 10 {
            RemediationPriority::Medium
        } else {
            RemediationPriority::Low
        }
    }

    /// Assess HIPAA Privacy Rule compliance
    fn assess_privacy_rule_compliance(&self, phi_events: &[crate::audit::AuditEvent]) -> f64 {
        // Simplified assessment - in reality this would be much more comprehensive
        let unauthorized_phi_access = phi_events.iter()
            .filter(|e| !e.success)
            .count();

        if unauthorized_phi_access == 0 {
            100.0
        } else {
            80.0 - (unauthorized_phi_access as f64 * 5.0).min(80.0)
        }
    }

    /// Assess HIPAA Security Rule compliance
    fn assess_security_rule_compliance(&self, audit_integrity: &AuditIntegrityReport) -> f64 {
        audit_integrity.integrity_score
    }

    /// Assess breach notification compliance
    fn assess_breach_notification_compliance(&self) -> f64 {
        // Placeholder - would check if breach notification procedures are in place
        95.0
    }

    /// Generate HIPAA risk assessment
    fn generate_hipaa_risk_assessment(&self, score: f64) -> HipaaRiskLevel {
        match score {
            s if s >= 90.0 => HipaaRiskLevel::Low,
            s if s >= 80.0 => HipaaRiskLevel::Moderate,
            s if s >= 70.0 => HipaaRiskLevel::High,
            _ => HipaaRiskLevel::Critical,
        }
    }

    /// Generate HIPAA required actions
    fn generate_hipaa_required_actions(&self, score: f64) -> Vec<String> {
        let mut actions = Vec::new();

        if score < 90.0 {
            actions.push("Conduct comprehensive HIPAA compliance assessment".to_string());
            actions.push("Review and update HIPAA policies and procedures".to_string());
            actions.push("Provide HIPAA training to all personnel".to_string());
        }

        if score < 80.0 {
            actions.push("Implement additional security controls".to_string());
            actions.push("Enhance audit logging and monitoring".to_string());
        }

        actions
    }
}

/// Compliance weights for scoring
struct ComplianceWeights {
    audit_integrity: f64,
    compliance_status: f64,
    phi_protection: f64,
    vulnerability_management: f64,
    security_testing: f64,
}

/// Comprehensive compliance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveComplianceReport {
    pub report_id: uuid::Uuid,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub report_period: ReportPeriod,
    pub overall_compliance_score: f64,
    pub compliance_level: ComplianceLevel,
    pub audit_integrity_report: AuditIntegrityReport,
    pub compliance_status: ComplianceStatus,
    pub phi_protection_status: PhiProtectionStatus,
    pub vulnerability_assessment: VulnerabilityAssessment,
    pub security_test_results: SecurityTestReport,
    pub recommendations: Vec<String>,
    pub next_review_date: chrono::DateTime<chrono::Utc>,
}

/// Report period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportPeriod {
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
}

/// Compliance levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplianceLevel {
    FullyCompliant,
    MostlyCompliant,
    PartiallyCompliant,
    NeedsImprovement,
    NonCompliant,
}

/// PHI protection status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhiProtectionStatus {
    pub test_results: PhiTestResults,
    pub phi_leaks_prevented: usize,
    pub detection_accuracy: f64,
}

/// Vulnerability assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityAssessment {
    pub scan_report: VulnerabilityReport,
    pub risk_level: RiskLevel,
    pub critical_vulnerabilities: usize,
    pub remediation_priority: RemediationPriority,
}

/// Risk levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Remediation priority
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemediationPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Daily compliance summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyComplianceSummary {
    pub date: chrono::NaiveDate,
    pub total_audit_events: usize,
    pub security_events: usize,
    pub failed_events: usize,
    pub events_by_type: HashMap<String, usize>,
    pub risk_indicators: Vec<RiskIndicator>,
    pub compliance_status: crate::compliance::ComplianceLevel,
    pub active_alerts: usize,
    pub critical_alerts: usize,
}

/// Risk indicator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskIndicator {
    pub indicator_type: String,
    pub severity: RiskSeverity,
    pub count: usize,
    pub description: String,
}

/// Risk severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// HIPAA compliance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HipaaComplianceReport {
    pub report_date: chrono::DateTime<chrono::Utc>,
    pub hipaa_version: String,
    pub overall_compliance_score: f64,
    pub privacy_rule_score: f64,
    pub security_rule_score: f64,
    pub breach_notification_score: f64,
    pub phi_protection_effectiveness: f64,
    pub audit_trail_integrity: f64,
    pub risk_assessment: HipaaRiskLevel,
    pub required_actions: Vec<String>,
}

/// HIPAA risk levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HipaaRiskLevel {
    Low,
    Moderate,
    High,
    Critical,
}