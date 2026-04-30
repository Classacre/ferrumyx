//! Ferrumyx Security Compliance and Monitoring
//!
//! This crate provides comprehensive security compliance verification including:
//! - Automated security test suites
//! - Continuous compliance monitoring
//! - Vulnerability scanning integration
//! - Audit trail verification
//! - PHI detection and protection testing
//! - Automated compliance reporting

pub mod audit;
pub mod compliance;
pub mod encryption;
pub mod phi;
pub mod scanner;
pub mod tests;
pub mod monitoring;
pub mod reporting;
pub mod runtime_monitoring;
pub mod threat_detection;
pub mod correlation_engine;
pub mod incident_response;
pub mod dashboard;

use std::sync::Arc;
use tokio::sync::RwLock;

/// Global security state
#[derive(Clone)]
pub struct SecurityState {
    /// Audit manager
    pub audit: Arc<audit::AuditManager>,
    /// Compliance monitor
    pub compliance: Arc<compliance::ComplianceMonitor>,
    /// PHI detector
    pub phi_detector: Arc<phi::PhiDetector>,
    /// Vulnerability scanner
    pub scanner: Arc<scanner::VulnerabilityScanner>,
    /// Security test runner
    pub test_runner: Arc<tests::SecurityTestRunner>,
    /// Compliance reporter
    pub reporter: Arc<reporting::ComplianceReporter>,
    /// Runtime security monitor
    pub runtime_monitor: Arc<runtime_monitoring::RuntimeSecurityMonitor>,
    /// Advanced threat detector
    pub threat_detector: Arc<threat_detection::AdvancedThreatDetector>,
    /// Correlation engine
    pub correlation_engine: Arc<correlation_engine::CorrelationEngine>,
    /// Incident response engine
    pub incident_response: Arc<incident_response::IncidentResponseEngine>,
    /// Security monitor
    pub security_monitor: Arc<monitoring::SecurityMonitor>,
}

/// Initialize security compliance system
pub async fn init_security() -> anyhow::Result<SecurityState> {
    let audit = Arc::new(audit::AuditManager::new().await?);
    let compliance = Arc::new(compliance::ComplianceMonitor::new(audit.clone()).await?);
    let phi_detector = Arc::new(phi::PhiDetector::new()?);
    let scanner = Arc::new(scanner::VulnerabilityScanner::new().await?);
    let test_runner = Arc::new(tests::SecurityTestRunner::new().await?);
    let reporter = Arc::new(reporting::ComplianceReporter::new().await?);

    // Initialize advanced security components
    let correlation_engine = Arc::new(correlation_engine::CorrelationEngine::new(audit.clone()).await?);
    let incident_response = Arc::new(incident_response::IncidentResponseEngine::new(audit.clone()).await?);
    let runtime_monitor = Arc::new(runtime_monitoring::RuntimeSecurityMonitor::new(
        audit.clone(),
        correlation_engine.clone(),
        incident_response.clone(),
    ).await?);
    let threat_detector = Arc::new(threat_detection::AdvancedThreatDetector::new(audit.clone()).await?);

    // Initialize security monitor
    let security_monitor = Arc::new(monitoring::SecurityMonitor::new(
        audit.clone(),
        compliance.clone(),
        phi_detector.clone(),
        scanner.clone(),
        test_runner.clone(),
    ).await?);

    Ok(SecurityState {
        audit,
        compliance,
        phi_detector,
        scanner,
        test_runner,
        reporter,
        runtime_monitor,
        threat_detector,
        correlation_engine,
        incident_response,
        security_monitor,
    })
}

/// Initialize security dashboard
pub fn init_dashboard(security_state: Arc<SecurityState>) -> Arc<dashboard::SecurityDashboard> {
    Arc::new(dashboard::SecurityDashboard::new(security_state))
}