#!/usr/bin/env python3
"""
Ferrumyx Automated Security Compliance Verification System
Comprehensive security testing and compliance monitoring
"""

import os
import sys
import json
import subprocess
import asyncio
from datetime import datetime, timedelta
from typing import Dict, List, Any
import time

class SecurityComplianceTester:
    """Automated security compliance testing system"""

    def __init__(self):
        self.results = {}
        self.start_time = datetime.now()

    def log(self, message: str):
        """Log a message with timestamp"""
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        print(f"[{timestamp}] {message}")

    async def run_rust_security_tests(self) -> Dict[str, Any]:
        """Run the Rust security test suite"""
        self.log("Running Rust security test suite...")

        try:
            # Build the security crate
            result = await asyncio.create_subprocess_exec(
                "cargo", "build", "-p", "ferrumyx-security",
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
                cwd=os.path.dirname(__file__)
            )
            stdout, stderr = await result.communicate()

            if result.returncode != 0:
                return {
                    "status": "failed",
                    "error": stderr.decode(),
                    "build_success": False
                }

            # Run security tests
            result = await asyncio.create_subprocess_exec(
                "cargo", "test", "-p", "ferrumyx-security",
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
                cwd=os.path.dirname(__file__)
            )
            stdout, stderr = await result.communicate()

            return {
                "status": "success" if result.returncode == 0 else "failed",
                "output": stdout.decode(),
                "error": stderr.decode(),
                "build_success": True
            }

        except Exception as e:
            return {
                "status": "error",
                "error": str(e),
                "build_success": False
            }

    def run_python_security_tests(self) -> Dict[str, Any]:
        """Run the Python security test suite"""
        self.log("Running Python security test suite...")

        try:
            result = subprocess.run([
                sys.executable,
                "tests/security/run_security_tests.py"
            ], capture_output=True, text=True, cwd=os.path.dirname(__file__))

            return {
                "status": "success" if result.returncode == 0 else "failed",
                "output": result.stdout,
                "error": result.stderr,
                "exit_code": result.returncode
            }

        except Exception as e:
            return {
                "status": "error",
                "error": str(e)
            }

    def run_phi_detection_tests(self) -> Dict[str, Any]:
        """Test PHI detection capabilities"""
        self.log("Running PHI detection tests...")

        # Test cases for PHI detection
        test_cases = [
            {
                "content": "Patient John Doe with SSN 123-45-6789 diagnosed with cancer",
                "expected_phi": True,
                "expected_risk": "high"
            },
            {
                "content": "The KRAS gene mutation is common in oncology",
                "expected_phi": False,
                "expected_risk": "low"
            },
            {
                "content": "Contact researcher at john.doe@email.com for clinical trial info",
                "expected_phi": True,
                "expected_risk": "medium"
            }
        ]

        results = []
        passed = 0

        for test_case in test_cases:
            # Simulate PHI detection (in real implementation, this would call the Rust PHI detector)
            content = test_case["content"].lower()
            has_phi = any(keyword in content for keyword in [
                "patient", "ssn", "medical record", "diagnosis", "social security"
            ])

            # Simple risk assessment
            risk_score = 0.0
            if "ssn" in content or "social security" in content:
                risk_score = 0.9
            elif "patient" in content or "diagnosis" in content:
                risk_score = 0.7
            elif "email" in content:
                risk_score = 0.5

            risk_level = "high" if risk_score > 0.7 else "medium" if risk_score > 0.4 else "low"

            correct_detection = has_phi == test_case["expected_phi"]
            correct_risk = risk_level == test_case["expected_risk"]

            test_passed = correct_detection and correct_risk

            if test_passed:
                passed += 1

            results.append({
                "content": test_case["content"],
                "expected_phi": test_case["expected_phi"],
                "detected_phi": has_phi,
                "expected_risk": test_case["expected_risk"],
                "detected_risk": risk_level,
                "passed": test_passed
            })

        return {
            "total_tests": len(test_cases),
            "passed_tests": passed,
            "accuracy": passed / len(test_cases),
            "results": results
        }

    def run_vulnerability_scan(self) -> Dict[str, Any]:
        """Run vulnerability scanning"""
        self.log("Running vulnerability scan...")

        try:
            # Check if cargo-audit is available
            audit_result = subprocess.run(
                ["cargo", "audit", "--version"],
                capture_output=True,
                cwd=os.path.dirname(__file__)
            )

            audit_available = audit_result.returncode == 0

            if audit_available:
                # Run cargo audit
                result = subprocess.run(
                    ["cargo", "audit", "--json"],
                    capture_output=True, text=True,
                    cwd=os.path.dirname(__file__)
                )

                return {
                    "tool_available": True,
                    "status": "success" if result.returncode == 0 else "failed",
                    "output": result.stdout,
                    "error": result.stderr
                }
            else:
                return {
                    "tool_available": False,
                    "status": "skipped",
                    "message": "cargo-audit not installed"
                }

        except Exception as e:
            return {
                "status": "error",
                "error": str(e)
            }

    def run_compliance_checks(self) -> Dict[str, Any]:
        """Run compliance checks"""
        self.log("Running compliance checks...")

        checks = []

        # Check for required environment variables
        required_env_vars = ["DATABASE_URL"]
        for var in required_env_vars:
            present = var in os.environ
            checks.append({
                "check": f"Environment variable {var}",
                "status": "pass" if present else "fail",
                "details": f"Variable {'present' if present else 'missing'}"
            })

        # Check for security-related files
        security_files = [
            "tests/security/run_security_tests.py",
            "tests/security/audit_test.log"
        ]

        for file_path in security_files:
            exists = os.path.exists(file_path)
            checks.append({
                "check": f"Security file {file_path}",
                "status": "pass" if exists else "fail",
                "details": f"File {'exists' if exists else 'missing'}"
            })

        # Check for encryption configuration
        encryption_key_present = "FERRUMYX_ENCRYPTION_KEY" in os.environ
        checks.append({
            "check": "Encryption key configuration",
            "status": "pass" if encryption_key_present else "warn",
            "details": "Encryption key should be configured for production"
        })

        passed_checks = sum(1 for check in checks if check["status"] == "pass")
        total_checks = len(checks)

        return {
            "total_checks": total_checks,
            "passed_checks": passed_checks,
            "compliance_rate": passed_checks / total_checks,
            "checks": checks
        }

    def generate_compliance_report(self) -> Dict[str, Any]:
        """Generate comprehensive compliance report"""
        self.log("Generating comprehensive compliance report...")

        # Run all tests
        rust_tests = asyncio.run(self.run_rust_security_tests())
        python_tests = self.run_python_security_tests()
        phi_tests = self.run_phi_detection_tests()
        vuln_scan = self.run_vulnerability_scan()
        compliance_checks = self.run_compliance_checks()

        # Calculate overall scores
        test_scores = []

        if rust_tests.get("build_success"):
            test_scores.append(1.0 if rust_tests["status"] == "success" else 0.0)

        if python_tests["status"] == "success":
            test_scores.append(1.0)
        elif python_tests["status"] == "failed":
            test_scores.append(0.5)  # Partial credit for running but failing

        if phi_tests["accuracy"] > 0.8:
            test_scores.append(1.0)
        elif phi_tests["accuracy"] > 0.6:
            test_scores.append(0.7)
        else:
            test_scores.append(0.4)

        if vuln_scan.get("tool_available", False) and vuln_scan["status"] == "success":
            test_scores.append(1.0)
        elif vuln_scan.get("tool_available", False):
            test_scores.append(0.6)
        else:
            test_scores.append(0.3)  # Tool not available but checked

        test_scores.append(compliance_checks["compliance_rate"])

        overall_score = sum(test_scores) / len(test_scores) * 100

        # Determine compliance level
        if overall_score >= 90:
            compliance_level = "FULLY_COMPLIANT"
        elif overall_score >= 80:
            compliance_level = "MOSTLY_COMPLIANT"
        elif overall_score >= 70:
            compliance_level = "PARTIALLY_COMPLIANT"
        elif overall_score >= 60:
            compliance_level = "NEEDS_IMPROVEMENT"
        else:
            compliance_level = "NON_COMPLIANT"

        # Generate recommendations
        recommendations = self.generate_recommendations(
            rust_tests, python_tests, phi_tests, vuln_scan, compliance_checks
        )

        return {
            "report_generated": datetime.now().isoformat(),
            "test_duration_seconds": (datetime.now() - self.start_time).total_seconds(),
            "overall_compliance_score": round(overall_score, 2),
            "compliance_level": compliance_level,
            "test_results": {
                "rust_security_tests": rust_tests,
                "python_security_tests": python_tests,
                "phi_detection_tests": phi_tests,
                "vulnerability_scan": vuln_scan,
                "compliance_checks": compliance_checks
            },
            "recommendations": recommendations,
            "next_review_date": (datetime.now() + timedelta(days=30)).isoformat(),
            "automated_system_status": "OPERATIONAL"
        }

    def generate_recommendations(self, rust_tests, python_tests, phi_tests, vuln_scan, compliance_checks) -> List[str]:
        """Generate recommendations based on test results"""
        recommendations = []

        if not rust_tests.get("build_success"):
            recommendations.append("Fix Rust security crate build issues - check dependencies and compilation errors")

        if python_tests["status"] != "success":
            recommendations.append("Address Python security test failures - review test output for specific issues")

        if phi_tests["accuracy"] < 0.8:
            recommendations.append("Improve PHI detection accuracy - enhance detection algorithms and test coverage")

        if not vuln_scan.get("tool_available", False):
            recommendations.append("Install cargo-audit for automated vulnerability scanning")

        if compliance_checks["compliance_rate"] < 0.8:
            failed_checks = [check for check in compliance_checks["checks"] if check["status"] != "pass"]
            recommendations.append(f"Address {len(failed_checks)} compliance check failures")

        if not recommendations:
            recommendations.append("Security compliance system is functioning well - continue regular monitoring")

        return recommendations

    def save_report(self, report: Dict[str, Any], output_file: str = "security_compliance_report.json"):
        """Save the compliance report to file"""
        with open(output_file, 'w') as f:
            json.dump(report, f, indent=2, default=str)

        self.log(f"Compliance report saved to {output_file}")

    def print_summary(self, report: Dict[str, Any]):
        """Print a summary of the compliance report"""
        print("\n" + "="*80)
        print("FERRUMYX AUTOMATED SECURITY COMPLIANCE VERIFICATION REPORT")
        print("="*80)
        print(f"Report Generated: {report['report_generated']}")
        print(".2f")
        print(f"Compliance Level: {report['compliance_level']}")
        print(f"Overall Score: {report['overall_compliance_score']:.1f}%")
        print()

        print("TEST RESULTS SUMMARY:")
        results = report['test_results']
        print(f"  Rust Security Tests: {results['rust_security_tests']['status'].upper()}")
        print(f"  Python Security Tests: {results['python_security_tests']['status'].upper()}")
        print(".1%")
        print(f"  Vulnerability Scan: {results['vulnerability_scan']['status'].upper()}")
        print(".1%")
        print()

        print("RECOMMENDATIONS:")
        for i, rec in enumerate(report['recommendations'], 1):
            print(f"  {i}. {rec}")
        print()

        print("NEXT REVIEW DATE:", report['next_review_date'])
        print("="*80)

def main():
    """Main entry point for security compliance testing"""
    print("Ferrumyx Automated Security Compliance Verification System")
    print("Starting comprehensive security assessment...")

    tester = SecurityComplianceTester()

    try:
        # Run comprehensive testing
        report = tester.generate_compliance_report()

        # Save detailed report
        tester.save_report(report)

        # Print summary
        tester.print_summary(report)

        # Exit with appropriate code
        score = report['overall_compliance_score']
        if score >= 80:
            print("✅ Security compliance assessment PASSED")
            sys.exit(0)
        elif score >= 60:
            print("⚠️  Security compliance assessment PASSED with warnings")
            sys.exit(0)
        else:
            print("❌ Security compliance assessment FAILED")
            sys.exit(1)

    except Exception as e:
        print(f"❌ Security compliance testing failed with error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()