//! Continuous security monitoring

use crate::audit::AuditManager;
use crate::compliance::{ComplianceMonitor, ComplianceStatus};
use crate::phi::PhiDetector;
use crate::scanner::{VulnerabilityScanner, ScanSummary};
use crate::tests::{SecurityTestRunner, SecurityTestReport};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{self, Duration};

/// Security monitoring system
pub struct SecurityMonitor {
    audit_manager: Arc<AuditManager>,
    compliance_monitor: Arc<ComplianceMonitor>,
    phi_detector: Arc<PhiDetector>,
    vuln_scanner: Arc<VulnerabilityScanner>,
    test_runner: Arc<SecurityTestRunner>,
    monitoring_state: Arc<RwLock<MonitoringState>>,
}

impl SecurityMonitor {
    /// Create new security monitor
    pub async fn new(
        audit_manager: Arc<AuditManager>,
        compliance_monitor: Arc<ComplianceMonitor>,
        phi_detector: Arc<PhiDetector>,
        vuln_scanner: Arc<VulnerabilityScanner>,
        test_runner: Arc<SecurityTestRunner>,
    ) -> anyhow::Result<Self> {
        let monitoring_state = Arc::new(RwLock::new(MonitoringState::default()));

        Ok(Self {
            audit_manager,
            compliance_monitor,
            phi_detector,
            vuln_scanner,
            test_runner,
            monitoring_state,
        })
    }

    /// Start continuous security monitoring
    pub async fn start_monitoring(self: Arc<Self>) -> anyhow::Result<()> {
        let self_clone1 = self.clone();
        let self_clone2 = self.clone();
        let self_clone3 = self.clone();
        let self_clone4 = self.clone();

        let compliance_handle = tokio::spawn(async move {
            self_clone1.start_compliance_monitoring().await;
        });
        let vuln_handle = tokio::spawn(async move {
            self_clone2.start_vulnerability_monitoring().await;
        });
        let test_handle = tokio::spawn(async move {
            self_clone3.start_security_test_monitoring().await;
        });
        let phi_handle = tokio::spawn(async move {
            self_clone4.start_phi_monitoring().await;
        });

        // Wait for all monitoring tasks to start
        let _ = tokio::try_join!(compliance_handle, vuln_handle, test_handle, phi_handle);

        Ok(())
    }

    /// Start compliance monitoring
    async fn start_compliance_monitoring(&self) {
        let compliance_monitor = self.compliance_monitor.clone();
        let monitoring_state = self.monitoring_state.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(300)); // 5 minutes

            loop {
                interval.tick().await;

                let status = compliance_monitor.get_compliance_status().await;
                let mut state = monitoring_state.write().await;
                state.last_compliance_check = Some(chrono::Utc::now());
                state.compliance_status = Some(status);
            }
        });
    }

    /// Start vulnerability monitoring
    async fn start_vulnerability_monitoring(&self) {
        let vuln_scanner = self.vuln_scanner.clone();
        let monitoring_state = self.monitoring_state.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(3600)); // 1 hour

            loop {
                interval.tick().await;

                match vuln_scanner.get_scan_summary().await {
                    Ok(summary) => {
                        let mut state = monitoring_state.write().await;
                        state.last_vulnerability_scan = Some(chrono::Utc::now());
                        state.vulnerability_summary = Some(summary);
                    }
                    Err(e) => {
                        tracing::error!("Vulnerability monitoring failed: {}", e);
                    }
                }
            }
        });
    }

    /// Start security test monitoring
    async fn start_security_test_monitoring(&self) {
        let test_runner = self.test_runner.clone();
        let monitoring_state = self.monitoring_state.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(7200)); // 2 hours

            loop {
                interval.tick().await;

                match test_runner.run_comprehensive_tests().await {
                    Ok(report) => {
                        let mut state = monitoring_state.write().await;
                        state.last_security_test = Some(chrono::Utc::now());
                        state.security_test_report = Some(report);
                    }
                    Err(e) => {
                        tracing::error!("Security test monitoring failed: {}", e);
                    }
                }
            }
        });
    }

    /// Start PHI monitoring
    async fn start_phi_monitoring(&self) {
        let phi_detector = self.phi_detector.clone();
        let monitoring_state = self.monitoring_state.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(1800)); // 30 minutes

            loop {
                interval.tick().await;

                // Run PHI detection tests
                let test_results = phi_detector.test_phi_detection();
                let phi_status = PhiMonitoringStatus {
                    last_check: chrono::Utc::now(),
                    detection_accuracy: test_results.accuracy,
                    total_tests: test_results.total_tests,
                    passed_tests: test_results.passed_tests,
                };

                let mut state = monitoring_state.write().await;
                state.phi_monitoring_status = Some(phi_status);
            }
        });
    }

    /// Get current monitoring status
    pub async fn get_monitoring_status(&self) -> MonitoringStatus {
        let state = self.monitoring_state.read().await;

        let overall_health = self.calculate_overall_health(&state);

        MonitoringStatus {
            overall_health,
            last_update: chrono::Utc::now(),
            compliance_status: state.compliance_status.clone(),
            vulnerability_summary: state.vulnerability_summary.clone(),
            security_test_report: state.security_test_report.clone(),
            phi_monitoring_status: state.phi_monitoring_status.clone(),
            active_alerts: self.compliance_monitor.get_active_alerts().await.len(),
        }
    }

    /// Calculate overall health score
    fn calculate_overall_health(&self, state: &MonitoringState) -> f64 {
        let mut scores = Vec::new();

        // Compliance score
        if let Some(compliance) = &state.compliance_status {
            let compliance_score = match compliance.status {
                crate::compliance::ComplianceLevel::Compliant => 100.0,
                crate::compliance::ComplianceLevel::Warning => 75.0,
                crate::compliance::ComplianceLevel::NonCompliant => 50.0,
            };
            scores.push(compliance_score);
        }

        // Vulnerability score
        if let Some(vuln) = &state.vulnerability_summary {
            scores.push(vuln.risk_score);
        }

        // Security test score
        if let Some(test_report) = &state.security_test_report {
            scores.push(test_report.success_rate);
        }

        // PHI detection score
        if let Some(phi) = &state.phi_monitoring_status {
            scores.push(phi.detection_accuracy * 100.0);
        }

        if scores.is_empty() {
            0.0
        } else {
            scores.iter().sum::<f64>() / scores.len() as f64
        }
    }

    /// Force immediate security assessment
    pub async fn force_security_assessment(&self) -> anyhow::Result<SecurityAssessment> {
        // Run all checks immediately
        let compliance_status = self.compliance_monitor.get_compliance_status().await;
        let vulnerability_summary = self.vuln_scanner.get_scan_summary().await?;
        let security_test_report = self.test_runner.run_comprehensive_tests().await?;
        let phi_test_results = self.phi_detector.test_phi_detection();

        let phi_status = PhiMonitoringStatus {
            last_check: chrono::Utc::now(),
            detection_accuracy: phi_test_results.accuracy,
            total_tests: phi_test_results.total_tests,
            passed_tests: phi_test_results.passed_tests,
        };

        let overall_score = self.calculate_assessment_score(
            &compliance_status,
            &vulnerability_summary,
            &security_test_report,
            &phi_status,
        );

        Ok(SecurityAssessment {
            timestamp: chrono::Utc::now(),
            overall_score,
            compliance_score: self.compliance_level_to_score(&compliance_status.status),
            vulnerability_score: vulnerability_summary.risk_score,
            test_success_rate: security_test_report.success_rate,
            phi_detection_accuracy: phi_status.detection_accuracy * 100.0,
            critical_issues: self.count_critical_issues(&security_test_report),
            recommendations: self.generate_assessment_recommendations(
                &compliance_status,
                &vulnerability_summary,
                &security_test_report,
            ),
        })
    }

    /// Calculate assessment score
    fn calculate_assessment_score(
        &self,
        compliance: &ComplianceStatus,
        vuln: &ScanSummary,
        tests: &SecurityTestReport,
        phi: &PhiMonitoringStatus,
    ) -> f64 {
        let compliance_score = self.compliance_level_to_score(&compliance.status);
        let scores = vec![
            compliance_score,
            vuln.risk_score,
            tests.success_rate,
            phi.detection_accuracy * 100.0,
        ];

        scores.iter().sum::<f64>() / scores.len() as f64
    }

    /// Convert compliance level to score
    fn compliance_level_to_score(&self, level: &crate::compliance::ComplianceLevel) -> f64 {
        match level {
            crate::compliance::ComplianceLevel::Compliant => 100.0,
            crate::compliance::ComplianceLevel::Warning => 75.0,
            crate::compliance::ComplianceLevel::NonCompliant => 50.0,
        }
    }

    /// Count critical issues in test report
    fn count_critical_issues(&self, report: &SecurityTestReport) -> usize {
        report.test_results.iter()
            .filter(|r| !r.passed && matches!(r.severity, crate::tests::TestSeverity::Critical))
            .count()
    }

    /// Generate assessment recommendations
    fn generate_assessment_recommendations(
        &self,
        compliance: &ComplianceStatus,
        vuln: &ScanSummary,
        tests: &SecurityTestReport,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if !matches!(compliance.status, crate::compliance::ComplianceLevel::Compliant) {
            recommendations.push("Address compliance violations to achieve full compliance".to_string());
        }

        if vuln.risk_score < 80.0 {
            recommendations.push("Review and remediate security vulnerabilities".to_string());
        }

        if tests.success_rate < 90.0 {
            recommendations.push("Fix failed security tests and improve overall security posture".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("Security posture is strong - continue regular monitoring".to_string());
        }

        recommendations
    }
}

/// Monitoring state
#[derive(Debug, Default)]
struct MonitoringState {
    last_compliance_check: Option<chrono::DateTime<chrono::Utc>>,
    last_vulnerability_scan: Option<chrono::DateTime<chrono::Utc>>,
    last_security_test: Option<chrono::DateTime<chrono::Utc>>,
    compliance_status: Option<ComplianceStatus>,
    vulnerability_summary: Option<ScanSummary>,
    security_test_report: Option<SecurityTestReport>,
    phi_monitoring_status: Option<PhiMonitoringStatus>,
}

/// PHI monitoring status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhiMonitoringStatus {
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub detection_accuracy: f64,
    pub total_tests: usize,
    pub passed_tests: usize,
}

/// Monitoring status summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringStatus {
    pub overall_health: f64,
    pub last_update: chrono::DateTime<chrono::Utc>,
    pub compliance_status: Option<ComplianceStatus>,
    pub vulnerability_summary: Option<ScanSummary>,
    pub security_test_report: Option<SecurityTestReport>,
    pub phi_monitoring_status: Option<PhiMonitoringStatus>,
    pub active_alerts: usize,
}

/// Security assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAssessment {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub overall_score: f64,
    pub compliance_score: f64,
    pub vulnerability_score: f64,
    pub test_success_rate: f64,
    pub phi_detection_accuracy: f64,
    pub critical_issues: usize,
    pub recommendations: Vec<String>,
}