# Ferrumyx Automated Security Compliance System

This directory contains the comprehensive automated security compliance verification system for Ferrumyx, a secure oncology discovery platform.

## Overview

The Ferrumyx Security Compliance System provides:

- **Automated Security Tests**: Comprehensive test suite covering all security domains
- **Continuous Compliance Monitoring**: Real-time monitoring and alerting
- **Vulnerability Scanning**: Automated dependency and code vulnerability detection
- **Audit Trail Verification**: Complete audit logging with integrity verification
- **PHI Detection Testing**: Automated Protected Health Information detection and protection
- **Compliance Reporting**: Automated generation of compliance status reports

## Architecture

### Core Components

1. **ferrumyx-security crate** (`crates/ferrumyx-security/`)
   - Audit trail management with integrity verification
   - PHI detection and protection
   - Compliance monitoring and alerting
   - Vulnerability scanning integration
   - Automated security testing
   - Compliance reporting

2. **Python Test Suite** (`tests/security/`)
   - Legacy security tests (being migrated to Rust)
   - Integration testing
   - Compliance verification scripts

## Quick Start

### Prerequisites

```bash
# Install Rust security tools
cargo install cargo-audit
cargo install cargo-deny

# Install Python dependencies (if using Python tests)
pip install -r requirements-security.txt
```

### Running Security Tests

#### Option 1: Automated Compliance Verification (Recommended)

```bash
python tests/security/run_automated_compliance.py
```

This runs the complete automated security compliance verification suite and generates a comprehensive report.

#### Option 2: Individual Component Testing

```bash
# Run Rust security tests
cargo test -p ferrumyx-security

# Run Python security tests
python tests/security/run_security_tests.py
```

#### Option 3: Build and Run Security Crate

```bash
# Build the security crate
cargo build -p ferrumyx-security

# Run with sample data
cargo run -p ferrumyx-security -- --help
```

## Configuration

Security settings are configured in `ferrumyx-security.toml`:

```toml
[security]
enabled = true
environment = "production"

[encryption]
key_rotation_days = 90

[audit]
enabled = true
retention_days = 2555  # 7 years HIPAA compliance

[compliance]
continuous_monitoring = true
hipaa_compliance_required = true
```

## Security Features

### 1. Automated Security Tests

The system runs comprehensive tests covering:

- **Authentication & Authorization**: Password hashing, token generation, access controls
- **Encryption**: Data encryption/decryption, key management, integrity verification
- **Audit Logging**: Event logging, integrity checking, compliance verification
- **PHI Protection**: Detection accuracy, content filtering, privacy compliance
- **Access Control**: Role-based permissions, policy enforcement
- **Vulnerability Management**: Dependency scanning, security updates
- **Network Security**: HTTPS enforcement, secure communications
- **Configuration Security**: Secure defaults, environment validation

### 2. Continuous Compliance Monitoring

- Real-time monitoring of security events
- Automated alerting for compliance violations
- Continuous audit trail verification
- PHI access pattern analysis
- Encryption key rotation tracking

### 3. Vulnerability Scanning

- Integration with `cargo-audit` for Rust dependencies
- Integration with `cargo-deny` for license and security checks
- Custom code vulnerability detection
- Automated scanning schedules

### 4. Audit Trail Verification

- Cryptographic integrity verification
- Tamper detection and alerting
- Chronological ordering validation
- Missing event detection
- Automated integrity reporting

### 5. PHI Detection Testing

- Keyword-based PHI detection
- Context-aware analysis
- Risk scoring and classification
- Automated testing with known PHI patterns
- Accuracy measurement and reporting

### 6. Compliance Reporting

- **Daily Compliance Summaries**: Event counts, risk indicators, active alerts
- **Comprehensive Compliance Reports**: Full security assessment with recommendations
- **HIPAA Compliance Reports**: Specific healthcare compliance validation
- **Automated Report Generation**: Scheduled and on-demand reporting

## Compliance Standards

The system is designed to support:

- **HIPAA**: Healthcare data protection and privacy
- **NIST Cybersecurity Framework**: Security controls and risk management
- **OWASP**: Web application security best practices
- **ISO 27001**: Information security management

## Integration

### With Ferrumyx Application

```rust
use ferrumyx_security::{init_security, SecurityState};

// Initialize security system
let security_state = init_security().await?;

// Start continuous monitoring
security_state.compliance.start_monitoring().await?;

// Run security assessment
let assessment = security_state.monitoring.force_security_assessment().await?;
println!("Security Score: {:.1}%", assessment.overall_score);
```

### With CI/CD Pipeline

```yaml
# .github/workflows/security.yml
name: Security Compliance
on:
  push:
    branches: [main]
  schedule:
    - cron: '0 2 * * *'  # Daily at 2 AM

jobs:
  security:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rust/cargo@v1
        with:
          command: build
      - name: Run Security Tests
        run: python tests/security/run_automated_compliance.py
      - name: Upload Report
        uses: actions/upload-artifact@v3
        with:
          name: security-report
          path: security_compliance_report.json
```

## Monitoring and Alerting

### Real-time Monitoring

The system provides continuous monitoring with:

- Security event alerting
- Compliance violation detection
- Performance monitoring
- Resource usage tracking

### Dashboard Integration

Security metrics are exposed for integration with monitoring dashboards:

```rust
// Get current monitoring status
let status = security_monitor.get_monitoring_status().await?;
println!("Overall Health: {:.1}%", status.overall_health);
println!("Active Alerts: {}", status.active_alerts);
```

## Troubleshooting

### Common Issues

1. **Build Failures**
   ```bash
   # Check Rust version
   rustc --version
   # Update dependencies
   cargo update
   ```

2. **Test Failures**
   ```bash
   # Run with verbose output
   python tests/security/run_automated_compliance.py -v
   # Check test logs
   cat tests/security/results.json
   ```

3. **Tool Installation Issues**
   ```bash
   # Install cargo-audit
   cargo install cargo-audit
   # Install cargo-deny
   cargo install cargo-deny
   ```

### Debug Mode

Enable debug logging:

```bash
export RUST_LOG=ferrumyx_security=debug
cargo test -p ferrumyx-security
```

## Development

### Adding New Security Tests

```rust
// In tests.rs
pub async fn run_custom_security_test(&self) -> anyhow::Result<Vec<SecurityTestResult>> {
    // Implement custom security test
    // Return test results
}
```

### Extending Compliance Rules

```rust
// In compliance.rs
pub struct CustomComplianceRule {
    // Define custom compliance checks
}
```

## Security Considerations

- **Key Management**: Encryption keys should be managed securely (HSM, key vault)
- **Audit Storage**: Audit logs should be stored securely with backup
- **PHI Handling**: Never log or store actual PHI data in audit trails
- **Network Security**: Ensure all communications use TLS 1.3+
- **Access Control**: Implement principle of least privilege

## Support

For security-related issues or questions:

1. Check the security compliance report for specific recommendations
2. Review test failure details in the logs
3. Ensure all prerequisites are installed
4. Verify configuration settings in `ferrumyx-security.toml`

## License

This security system is part of Ferrumyx and follows the same Apache-2.0 OR MIT license.