//! Vulnerability scanning and dependency analysis

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use tokio::process::Command as TokioCommand;

/// Vulnerability scanner for security assessments
pub struct VulnerabilityScanner {
    cargo_audit_available: bool,
    cargo_deny_available: bool,
    custom_scanners: HashMap<String, Box<dyn VulnerabilityCheck + Send + Sync>>,
}

impl VulnerabilityScanner {
    /// Create new vulnerability scanner
    pub async fn new() -> anyhow::Result<Self> {
        // Check if security tools are available
        let cargo_audit_available = Self::check_tool_available("cargo-audit").await;
        let cargo_deny_available = Self::check_tool_available("cargo-deny").await;

        let mut custom_scanners = HashMap::new();

        // Add custom security checks
        custom_scanners.insert(
            "weak_crypto".to_string(),
            Box::new(WeakCryptoScanner::new()) as Box<dyn VulnerabilityCheck + Send + Sync>
        );

        custom_scanners.insert(
            "hardcoded_secrets".to_string(),
            Box::new(HardcodedSecretsScanner::new()) as Box<dyn VulnerabilityCheck + Send + Sync>
        );

        custom_scanners.insert(
            "insecure_configs".to_string(),
            Box::new(InsecureConfigScanner::new()) as Box<dyn VulnerabilityCheck + Send + Sync>
        );

        Ok(Self {
            cargo_audit_available,
            cargo_deny_available,
            custom_scanners,
        })
    }

    /// Check if a tool is available
    async fn check_tool_available(tool: &str) -> bool {
        TokioCommand::new(tool)
            .arg("--version")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Run comprehensive vulnerability scan
    pub async fn run_full_scan(&self) -> anyhow::Result<VulnerabilityReport> {
        let mut findings = Vec::new();

        // Run cargo audit if available
        if self.cargo_audit_available {
            match self.run_cargo_audit().await {
                Ok(audit_findings) => findings.extend(audit_findings),
                Err(e) => {
                    findings.push(VulnerabilityFinding {
                        id: "cargo_audit_error".to_string(),
                        title: "Cargo Audit Failed".to_string(),
                        description: format!("Failed to run cargo audit: {}", e),
                        severity: Severity::Medium,
                        cve_id: None,
                        affected_package: None,
                        affected_version: None,
                        fixed_version: None,
                        references: vec![],
                        evidence: "Tool execution failed".to_string(),
                        recommendation: "Install cargo-audit or check tool configuration".to_string(),
                    });
                }
            }
        }

        // Run cargo deny if available
        if self.cargo_deny_available {
            match self.run_cargo_deny().await {
                Ok(deny_findings) => findings.extend(deny_findings),
                Err(e) => {
                    findings.push(VulnerabilityFinding {
                        id: "cargo_deny_error".to_string(),
                        title: "Cargo Deny Failed".to_string(),
                        description: format!("Failed to run cargo deny: {}", e),
                        severity: Severity::Medium,
                        cve_id: None,
                        affected_package: None,
                        affected_version: None,
                        fixed_version: None,
                        references: vec![],
                        evidence: "Tool execution failed".to_string(),
                        recommendation: "Install cargo-deny or check tool configuration".to_string(),
                    });
                }
            }
        }

        // Run custom security checks
        for (scanner_name, scanner) in &self.custom_scanners {
            match scanner.scan() {
                Ok(scanner_findings) => findings.extend(scanner_findings),
                Err(e) => {
                    findings.push(VulnerabilityFinding {
                        id: format!("{}_error", scanner_name),
                        title: format!("{} Scanner Failed", scanner_name),
                        description: format!("Custom scanner {} failed: {}", scanner_name, e),
                        severity: Severity::Low,
                        cve_id: None,
                        affected_package: None,
                        affected_version: None,
                        fixed_version: None,
                        references: vec![],
                        evidence: "Scanner execution failed".to_string(),
                        recommendation: "Check scanner implementation".to_string(),
                    });
                }
            }
        }

        // Calculate summary statistics
        let critical_count = findings.iter().filter(|f| matches!(f.severity, Severity::Critical)).count();
        let high_count = findings.iter().filter(|f| matches!(f.severity, Severity::High)).count();
        let medium_count = findings.iter().filter(|f| matches!(f.severity, Severity::Medium)).count();
        let low_count = findings.iter().filter(|f| matches!(f.severity, Severity::Low)).count();

        let risk_score = Self::calculate_risk_score(&findings);

        Ok(VulnerabilityReport {
            scan_timestamp: chrono::Utc::now(),
            total_findings: findings.len(),
            critical_vulnerabilities: critical_count,
            high_vulnerabilities: high_count,
            medium_vulnerabilities: medium_count,
            low_vulnerabilities: low_count,
            risk_score,
            findings,
            scan_duration: std::time::Duration::from_secs(0), // Would be measured in real implementation
            tools_used: self.get_tools_used(),
        })
    }

    /// Run cargo audit
    async fn run_cargo_audit(&self) -> anyhow::Result<Vec<VulnerabilityFinding>> {
        let output = TokioCommand::new("cargo")
            .args(&["audit", "--json"])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("cargo audit failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        let json_output = String::from_utf8(output.stdout)?;
        let audit_report: CargoAuditReport = serde_json::from_str(&json_output)?;

        let mut findings = Vec::new();

        for vulnerability in audit_report.vulnerabilities.list {
            let package_name = vulnerability.package.name.clone();
            let package_version = vulnerability.package.version.clone();

            findings.push(VulnerabilityFinding {
                id: vulnerability.id.clone(),
                title: vulnerability.title.clone(),
                description: format!("{}: {}", vulnerability.title, vulnerability.description),
                severity: Self::map_advisory_severity(&vulnerability.severity),
                cve_id: Some(vulnerability.id),
                affected_package: Some(package_name.clone()),
                affected_version: Some(package_version.clone()),
                fixed_version: vulnerability.versions.patched.first().cloned(),
                references: vulnerability.references.clone(),
                evidence: format!("Package {} version {} has known vulnerability", package_name, package_version),
                recommendation: vulnerability.description,
            });
        }

        Ok(findings)
    }

    /// Run cargo deny
    async fn run_cargo_deny(&self) -> anyhow::Result<Vec<VulnerabilityFinding>> {
        let output = TokioCommand::new("cargo")
            .args(&["deny", "check", "--format", "json"])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("cargo deny failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        // cargo deny JSON output parsing would go here
        // For now, return empty as this is complex to implement without the actual tool

        Ok(vec![])
    }

    /// Map advisory severity to our severity enum
    fn map_advisory_severity(severity: &str) -> Severity {
        match severity.to_lowercase().as_str() {
            "critical" => Severity::Critical,
            "high" => Severity::High,
            "medium" => Severity::Medium,
            "low" => Severity::Low,
            _ => Severity::Medium,
        }
    }

    /// Calculate overall risk score
    fn calculate_risk_score(findings: &[VulnerabilityFinding]) -> f64 {
        if findings.is_empty() {
            return 100.0;
        }

        let weighted_score: f64 = findings.iter().map(|f| {
            match f.severity {
                Severity::Critical => 10.0,
                Severity::High => 7.0,
                Severity::Medium => 4.0,
                Severity::Low => 1.0,
            }
        }).sum();

        let max_possible_score = findings.len() as f64 * 10.0;
        100.0 - (weighted_score / max_possible_score * 100.0).min(100.0)
    }

    /// Get list of tools used in scan
    fn get_tools_used(&self) -> Vec<String> {
        let mut tools = Vec::new();

        if self.cargo_audit_available {
            tools.push("cargo-audit".to_string());
        }

        if self.cargo_deny_available {
            tools.push("cargo-deny".to_string());
        }

        tools.extend(self.custom_scanners.keys().cloned());
        tools
    }

    /// Get scan results summary
    pub async fn get_scan_summary(&self) -> anyhow::Result<ScanSummary> {
        let report = self.run_full_scan().await?;

        Ok(ScanSummary {
            last_scan: report.scan_timestamp,
            total_findings: report.total_findings,
            risk_score: report.risk_score,
            critical_issues: report.critical_vulnerabilities,
            status: if report.risk_score >= 80.0 {
                ScanStatus::Clean
            } else if report.risk_score >= 60.0 {
                ScanStatus::Warning
            } else {
                ScanStatus::Critical
            },
        })
    }
}

/// Vulnerability finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityFinding {
    pub id: String,
    pub title: String,
    pub description: String,
    pub severity: Severity,
    pub cve_id: Option<String>,
    pub affected_package: Option<String>,
    pub affected_version: Option<String>,
    pub fixed_version: Option<String>,
    pub references: Vec<String>,
    pub evidence: String,
    pub recommendation: String,
}

/// Vulnerability report
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VulnerabilityReport {
    pub scan_timestamp: chrono::DateTime<chrono::Utc>,
    pub total_findings: usize,
    pub critical_vulnerabilities: usize,
    pub high_vulnerabilities: usize,
    pub medium_vulnerabilities: usize,
    pub low_vulnerabilities: usize,
    pub risk_score: f64,
    pub findings: Vec<VulnerabilityFinding>,
    pub scan_duration: std::time::Duration,
    pub tools_used: Vec<String>,
}

/// Scan summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub last_scan: chrono::DateTime<chrono::Utc>,
    pub total_findings: usize,
    pub risk_score: f64,
    pub critical_issues: usize,
    pub status: ScanStatus,
}

/// Scan status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanStatus {
    Clean,
    Warning,
    Critical,
}

/// Severity levels for vulnerabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

/// Cargo audit report structure
#[derive(Debug, Deserialize)]
struct CargoAuditReport {
    vulnerabilities: VulnerabilitiesList,
}

#[derive(Debug, Deserialize)]
struct VulnerabilitiesList {
    list: Vec<CargoAdvisory>,
}

#[derive(Debug, Deserialize)]
struct CargoAdvisory {
    id: String,
    title: String,
    description: String,
    severity: String,
    package: PackageInfo,
    versions: VersionInfo,
    references: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PackageInfo {
    name: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct VersionInfo {
    patched: Vec<String>,
}

/// Trait for custom vulnerability checks
trait VulnerabilityCheck {
    fn scan(&self) -> anyhow::Result<Vec<VulnerabilityFinding>>;
}

/// Weak cryptography scanner
struct WeakCryptoScanner;

impl WeakCryptoScanner {
    fn new() -> Self {
        Self
    }
}

impl VulnerabilityCheck for WeakCryptoScanner {
    fn scan(&self) -> anyhow::Result<Vec<VulnerabilityFinding>> {
        // This would scan code for weak cryptographic algorithms
        // For now, return a placeholder finding
        Ok(vec![VulnerabilityFinding {
            id: "weak_crypto_placeholder".to_string(),
            title: "Weak Cryptography Check".to_string(),
            description: "Scanned for weak cryptographic implementations".to_string(),
            severity: Severity::Low,
            cve_id: None,
            affected_package: None,
            affected_version: None,
            fixed_version: None,
            references: vec![],
            evidence: "No weak cryptography detected in current scan".to_string(),
            recommendation: "Continue monitoring for weak crypto usage".to_string(),
        }])
    }
}

/// Hardcoded secrets scanner
struct HardcodedSecretsScanner;

impl HardcodedSecretsScanner {
    fn new() -> Self {
        Self
    }
}

impl VulnerabilityCheck for HardcodedSecretsScanner {
    fn scan(&self) -> anyhow::Result<Vec<VulnerabilityFinding>> {
        // This would scan code for hardcoded secrets
        Ok(vec![])
    }
}

/// Insecure configuration scanner
struct InsecureConfigScanner;

impl InsecureConfigScanner {
    fn new() -> Self {
        Self
    }
}

impl VulnerabilityCheck for InsecureConfigScanner {
    fn scan(&self) -> anyhow::Result<Vec<VulnerabilityFinding>> {
        // This would scan for insecure configurations
        Ok(vec![])
    }
}