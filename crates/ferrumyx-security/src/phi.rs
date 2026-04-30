//! PHI (Protected Health Information) detection and protection

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// PHI detection patterns and rules
#[derive(Debug, Clone)]
pub struct PhiDetector {
    patterns: HashMap<String, Regex>,
    keywords: Vec<String>,
    context_rules: Vec<ContextRule>,
    research_indicators: Vec<String>,
    patient_specific_indicators: Vec<String>,
}

impl PhiDetector {
    /// Create new PHI detector
    pub fn new() -> anyhow::Result<Self> {
        let mut patterns = HashMap::new();

        // Email pattern
        patterns.insert(
            "email".to_string(),
            Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b")?
        );

        // Phone number patterns
        patterns.insert(
            "phone_us".to_string(),
            Regex::new(r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b")?
        );

        // SSN pattern
        patterns.insert(
            "ssn".to_string(),
            Regex::new(r"\b\d{3}[-]?\d{2}[-]?\d{4}\b")?
        );

        // Date of birth patterns
        patterns.insert(
            "dob".to_string(),
            Regex::new(r"\b\d{1,2}[-/]\d{1,2}[-/]\d{4}\b")?
        );

        // Medical record numbers
        patterns.insert(
            "medical_record".to_string(),
            Regex::new(r"\bMRN[-]?\d{6,10}\b")?
        );

        // Provider IDs
        patterns.insert(
            "provider_id".to_string(),
            Regex::new(r"\bNPI[-]?\d{10}\b")?
        );

        let keywords = vec![
            "patient".to_string(),
            "medical record".to_string(),
            "diagnosis".to_string(),
            "treatment".to_string(),
            "medication".to_string(),
            "symptoms".to_string(),
            "clinical trial".to_string(),
            "healthcare".to_string(),
            "hospital".to_string(),
            "physician".to_string(),
            "doctor".to_string(),
            "nurse".to_string(),
            "social security".to_string(),
            "ssn".to_string(),
            "date of birth".to_string(),
            "dob".to_string(),
            "address".to_string(),
            "phone".to_string(),
            "email".to_string(),
            "insurance".to_string(),
            "billing".to_string(),
        ];

        let context_rules = vec![
            ContextRule {
                name: "medical_context".to_string(),
                keywords: vec!["patient".to_string(), "diagnosis".to_string(), "treatment".to_string(), "medical".to_string()],
                weight: 2.0,
            },
            ContextRule {
                name: "personal_info".to_string(),
                keywords: vec!["ssn".to_string(), "dob".to_string(), "address".to_string(), "phone".to_string(), "email".to_string()],
                weight: 3.0,
            },
            ContextRule {
                name: "clinical_data".to_string(),
                keywords: vec!["clinical trial".to_string(), "biomarker".to_string(), "mutation".to_string(), "gene".to_string()],
                weight: 1.5,
            },
        ];

        // Research vs patient-specific indicators
        let research_indicators = vec![
            "latest treatments".to_string(),
            "what are the".to_string(),
            "show me".to_string(),
            "research".to_string(),
            "studies".to_string(),
            "evidence".to_string(),
            "general".to_string(),
            "overview".to_string(),
            "information about".to_string(),
        ];

        let patient_specific_indicators = vec![
            "my patient".to_string(),
            "this patient".to_string(),
            "patient named".to_string(),
            "mr. ".to_string(),
            "mrs. ".to_string(),
            "dr. ".to_string(),
            "age ".to_string(),
            "born on".to_string(),
            "diagnosed with".to_string(),
            "medical record".to_string(),
            "chart".to_string(),
            "case study".to_string(),
        ];

        Ok(Self {
            patterns,
            keywords,
            context_rules,
            research_indicators,
            patient_specific_indicators,
        })
    }

    /// Detect PHI in text
    pub fn detect_phi(&self, text: &str) -> PhiDetectionResult {
        let mut detections = Vec::new();
        let mut confidence_score = 0.0;
        let text_lower = text.to_lowercase();

        // Determine if this is research/general inquiry vs patient-specific
        let is_research_query = self.is_research_query(&text_lower);
        let is_patient_specific = self.is_patient_specific(&text_lower);

        // Pattern-based detection (always high risk)
        for (pattern_name, regex) in &self.patterns {
            for mat in regex.find_iter(text) {
                detections.push(PhiDetection {
                    phi_type: pattern_name.clone(),
                    content: mat.as_str().to_string(),
                    start_position: mat.start(),
                    end_position: mat.end(),
                    confidence: 0.95, // Very high confidence for regex matches
                    context: self.extract_context(text, mat.start(), mat.end()),
                });
                confidence_score += 0.95;
            }
        }

        // Keyword-based detection with context analysis
        for keyword in &self.keywords {
            if text_lower.contains(keyword) {
                let context_weight = self.calculate_context_weight(&text_lower);
                let mut keyword_confidence = 0.6 * context_weight;

                // Adjust confidence based on research vs patient-specific context
                if is_research_query && !is_patient_specific {
                    // Research queries with medical keywords are lower risk
                    keyword_confidence *= 0.3;
                } else if is_patient_specific {
                    // Patient-specific queries are higher risk
                    keyword_confidence *= 1.5;
                }

                // Skip low-confidence detections for research queries
                if keyword_confidence > 0.2 {
                    detections.push(PhiDetection {
                        phi_type: "keyword".to_string(),
                        content: keyword.clone(),
                        start_position: text_lower.find(keyword).unwrap_or(0),
                        end_position: text_lower.find(keyword).unwrap_or(0) + keyword.len(),
                        confidence: keyword_confidence.min(1.0),
                        context: self.extract_context(text, 0, text.len()),
                    });
                    confidence_score += keyword_confidence;
                }
            }
        }

        // Calculate overall risk score
        let base_risk_score = if detections.is_empty() {
            0.0
        } else {
            (confidence_score / detections.len() as f64).min(1.0)
        };

        // Final risk adjustment based on context
        let risk_score = if is_research_query && base_risk_score < 0.5 {
            base_risk_score * 0.5 // Reduce risk for clear research queries
        } else if is_patient_specific {
            base_risk_score * 1.2 // Increase risk for patient-specific
        } else {
            base_risk_score
        }.min(1.0);

        PhiDetectionResult {
            has_phi: !detections.is_empty() && risk_score > 0.3,
            detections,
            risk_score,
            recommended_action: self.recommend_action(risk_score),
        }
    }

    /// Extract context around a detection
    fn extract_context(&self, text: &str, start: usize, end: usize) -> String {
        let context_start = start.saturating_sub(50);
        let context_end = (end + 50).min(text.len());

        text[context_start..context_end].to_string()
    }

    /// Calculate context weight based on surrounding keywords
    fn calculate_context_weight(&self, text: &str) -> f64 {
        let mut total_weight = 1.0;

        for rule in &self.context_rules {
            let keyword_count = rule.keywords.iter()
                .filter(|keyword| text.contains(*keyword))
                .count();

            if keyword_count > 0 {
                total_weight *= rule.weight * (keyword_count as f64 * 0.5 + 1.0);
            }
        }

        total_weight.min(3.0) // Cap at 3x multiplier
    }

    /// Check if text appears to be a research/general inquiry
    fn is_research_query(&self, text: &str) -> bool {
        self.research_indicators.iter().any(|indicator| text.contains(indicator))
    }

    /// Check if text appears to be patient-specific
    fn is_patient_specific(&self, text: &str) -> bool {
        self.patient_specific_indicators.iter().any(|indicator| text.contains(indicator))
    }

    /// Recommend action based on risk score
    fn recommend_action(&self, risk_score: f64) -> PhiAction {
        match risk_score {
            r if r >= 0.8 => PhiAction::Block,
            r if r >= 0.6 => PhiAction::Quarantine,
            r if r >= 0.3 => PhiAction::Flag,
            _ => PhiAction::Allow,
        }
    }

    /// Test PHI detection with known test cases
    pub fn test_phi_detection(&self) -> PhiTestResults {
        let test_cases = vec![
            PhiTestCase {
                input: "Patient John Doe has SSN 123-45-6789 and was diagnosed with cancer".to_string(),
                expected_phi_types: vec!["ssn".to_string(), "keyword".to_string()],
                expected_risk: 0.8,
            },
            PhiTestCase {
                input: "The KRAS G12D mutation is common in colorectal cancer".to_string(),
                expected_phi_types: vec![], // Research context, low risk
                expected_risk: 0.0,
            },
            PhiTestCase {
                input: "Contact the patient at john.doe@email.com for follow-up".to_string(),
                expected_phi_types: vec!["email".to_string()],
                expected_risk: 0.7,
            },
            PhiTestCase {
                input: "What are the latest treatments for KRAS G12D pancreatic cancer?".to_string(),
                expected_phi_types: vec![], // Research query, should be low risk
                expected_risk: 0.0,
            },
            PhiTestCase {
                input: "My patient was diagnosed with BRCA1/2 mutations in breast cancer".to_string(),
                expected_phi_types: vec!["keyword".to_string()],
                expected_risk: 0.6, // Patient-specific, higher risk
            },
        ];

        let mut results = Vec::new();
        let mut passed = 0;
        let mut total = test_cases.len();

        for test_case in test_cases {
            let detection = self.detect_phi(&test_case.input);
            let detected_types: Vec<String> = detection.detections.iter()
                .map(|d| d.phi_type.clone())
                .collect();

            let risk_match = (detection.risk_score - test_case.expected_risk).abs() < 0.2;
            let types_match = detected_types.len() == test_case.expected_phi_types.len() &&
                detected_types.iter().all(|t| test_case.expected_phi_types.contains(t));

            let passed_test = risk_match && types_match;

            if passed_test {
                passed += 1;
            }

            results.push(PhiTestResult {
                test_case: test_case.input,
                expected_types: test_case.expected_phi_types,
                detected_types,
                expected_risk: test_case.expected_risk,
                detected_risk: detection.risk_score,
                passed: passed_test,
            });
        }

        PhiTestResults {
            total_tests: total,
            passed_tests: passed,
            accuracy: passed as f64 / total as f64,
            results,
        }
    }
}

/// PHI detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhiDetectionResult {
    pub has_phi: bool,
    pub detections: Vec<PhiDetection>,
    pub risk_score: f64,
    pub recommended_action: PhiAction,
}

/// Individual PHI detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhiDetection {
    pub phi_type: String,
    pub content: String,
    pub start_position: usize,
    pub end_position: usize,
    pub confidence: f64,
    pub context: String,
}

/// Recommended actions for PHI handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PhiAction {
    Allow,
    Flag,
    Quarantine,
    Block,
}

/// Context rule for PHI detection
#[derive(Debug, Clone)]
struct ContextRule {
    name: String,
    keywords: Vec<String>,
    weight: f64,
}

/// Test case for PHI detection
struct PhiTestCase {
    input: String,
    expected_phi_types: Vec<String>,
    expected_risk: f64,
}

/// PHI test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhiTestResult {
    pub test_case: String,
    pub expected_types: Vec<String>,
    pub detected_types: Vec<String>,
    pub expected_risk: f64,
    pub detected_risk: f64,
    pub passed: bool,
}

/// PHI test results summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhiTestResults {
    pub total_tests: usize,
    pub passed_tests: usize,
    pub accuracy: f64,
    pub results: Vec<PhiTestResult>,
}