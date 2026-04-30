//! Advanced threat detection with ML-enhanced pattern recognition

use crate::audit::AuditManager;
use crate::phi::PhiDetector;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration, Timelike};

/// Advanced threat detection engine
pub struct AdvancedThreatDetector {
    /// Basic pattern detector
    pattern_detector: Arc<PatternDetector>,
    /// Behavioral analyzer
    behavioral_analyzer: Arc<BehavioralAnalyzer>,
    /// ML-enhanced PHI detector
    ml_phi_detector: Arc<MLPhiDetector>,
    /// Threat intelligence feed
    threat_intelligence: Arc<ThreatIntelligence>,
    /// Detection history
    detection_history: Arc<RwLock<VecDeque<DetectionEvent>>>,
}

impl AdvancedThreatDetector {
    /// Create new advanced threat detector
    pub async fn new(audit_manager: Arc<AuditManager>) -> anyhow::Result<Self> {
        let pattern_detector = Arc::new(PatternDetector::new()?);
        let behavioral_analyzer = Arc::new(BehavioralAnalyzer::new(audit_manager.clone()).await?);
        let ml_phi_detector = Arc::new(MLPhiDetector::new().await?);
        let threat_intelligence = Arc::new(ThreatIntelligence::new().await?);

        Ok(Self {
            pattern_detector,
            behavioral_analyzer,
            ml_phi_detector,
            threat_intelligence,
            detection_history: Arc::new(RwLock::new(VecDeque::with_capacity(10000))),
        })
    }

    /// Analyze content for threats
    pub async fn analyze_content(&self, content: &str, context: &AnalysisContext) -> ThreatAnalysis {
        let mut threats = Vec::new();
        let mut risk_score = 0.0;

        // Pattern-based detection
        let pattern_threats = self.pattern_detector.analyze(content, context).await;
        threats.extend(pattern_threats.threats);
        risk_score += pattern_threats.risk_score;

        // Behavioral analysis
        let behavioral_threats = self.behavioral_analyzer.analyze_behavior(context).await;
        threats.extend(behavioral_threats.threats);
        risk_score += behavioral_threats.risk_score;

        // ML-enhanced PHI detection
        let phi_threats = self.ml_phi_detector.analyze_phi(content, context).await;
        threats.extend(phi_threats.threats);
        risk_score += phi_threats.risk_score;

        // Threat intelligence correlation
        let intelligence_threats = self.threat_intelligence.correlate_threats(&threats, context).await;
        threats.extend(intelligence_threats.threats);
        risk_score += intelligence_threats.risk_score;

        // Normalize risk score
        risk_score = risk_score.min(1.0);

        // Record detection event
        self.record_detection_event(content, context, &threats, risk_score).await;

        let confidence = self.calculate_confidence(&threats);
        let recommended_actions = self.generate_recommendations(risk_score, &threats);

        ThreatAnalysis {
            threats,
            risk_score,
            confidence,
            recommended_actions,
        }
    }

    /// Record detection event in history
    async fn record_detection_event(
        &self,
        content: &str,
        context: &AnalysisContext,
        threats: &[Threat],
        risk_score: f64,
    ) {
        let event = DetectionEvent {
            timestamp: Utc::now(),
            content_hash: self.hash_content(content),
            context: context.clone(),
            threats: threats.to_vec(),
            risk_score,
        };

        let mut history = self.detection_history.write().await;
        history.push_back(event);

        // Keep only recent history (last 24 hours)
        let cutoff = Utc::now() - Duration::hours(24);
        while history.front().map_or(false, |e| e.timestamp < cutoff) {
            history.pop_front();
        }
    }

    /// Calculate overall confidence in analysis
    fn calculate_confidence(&self, threats: &[Threat]) -> f64 {
        if threats.is_empty() {
            return 1.0; // High confidence in no threats
        }

        let avg_confidence = threats.iter()
            .map(|t| t.confidence)
            .sum::<f64>() / threats.len() as f64;

        // Adjust based on threat correlation
        if threats.len() > 1 {
            avg_confidence * 1.2 // Multiple threats increase confidence
        } else {
            avg_confidence
        }.min(1.0)
    }

    /// Generate recommended actions based on analysis
    fn generate_recommendations(&self, risk_score: f64, threats: &[Threat]) -> Vec<String> {
        let mut recommendations = Vec::new();

        match risk_score {
            r if r >= 0.8 => {
                recommendations.push("Immediate blocking recommended".to_string());
                recommendations.push("Escalate to security team".to_string());
            }
            r if r >= 0.6 => {
                recommendations.push("Content quarantine recommended".to_string());
                recommendations.push("Manual review required".to_string());
            }
            r if r >= 0.3 => {
                recommendations.push("Content flagged for review".to_string());
                recommendations.push("Monitor user activity".to_string());
            }
            _ => {
                recommendations.push("Content appears safe".to_string());
            }
        }

        // Add specific recommendations based on threat types
        for threat in threats {
            match threat.threat_type.as_str() {
                "phi_leakage" => {
                    recommendations.push("PHI data detected - ensure HIPAA compliance".to_string());
                }
                "malicious_payload" => {
                    recommendations.push("Potential malware detected - scan recommended".to_string());
                }
                "suspicious_behavior" => {
                    recommendations.push("Unusual user behavior detected - investigate account".to_string());
                }
                _ => {}
            }
        }

        recommendations
    }

    /// Hash content for tracking (non-cryptographic)
    fn hash_content(&self, content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    /// Get detection statistics
    pub async fn get_detection_stats(&self) -> DetectionStats {
        let history = self.detection_history.read().await;

        let total_analyses = history.len();
        let high_risk_detections = history.iter()
            .filter(|e| e.risk_score >= 0.8)
            .count();
        let medium_risk_detections = history.iter()
            .filter(|e| e.risk_score >= 0.6 && e.risk_score < 0.8)
            .count();
        let low_risk_detections = history.iter()
            .filter(|e| e.risk_score >= 0.3 && e.risk_score < 0.6)
            .count();

        let threat_types = history.iter()
            .flat_map(|e| &e.threats)
            .fold(HashMap::new(), |mut acc, threat| {
                *acc.entry(threat.threat_type.clone()).or_insert(0) += 1;
                acc
            });

        DetectionStats {
            total_analyses,
            high_risk_detections,
            medium_risk_detections,
            low_risk_detections,
            threat_types,
            average_risk_score: if total_analyses > 0 {
                history.iter().map(|e| e.risk_score).sum::<f64>() / total_analyses as f64
            } else {
                0.0
            },
            last_update: Utc::now(),
        }
    }
}

/// Pattern-based threat detector
pub struct PatternDetector {
    patterns: HashMap<String, Vec<String>>,
    compiled_patterns: HashMap<String, regex::Regex>,
}

impl PatternDetector {
    /// Create new pattern detector
    pub fn new() -> anyhow::Result<Self> {
        let mut patterns = HashMap::new();

        // Malware patterns
        patterns.insert("malware_signatures".to_string(), vec![
            r"(?i)eval\s*\(".to_string(),
            r"(?i)base64_decode\s*\(".to_string(),
            r"(?i)system\s*\(".to_string(),
            r"(?i)exec\s*\(".to_string(),
            r"(?i)shell_exec\s*\(".to_string(),
        ]);

        // Data exfiltration patterns
        patterns.insert("data_exfiltration".to_string(), vec![
            r"(?i)(wget|curl)\s+.*ftp://".to_string(),
            r"(?i)scp\s+.*@".to_string(),
            r"(?i)rsync\s+.*::".to_string(),
        ]);

        // Privilege escalation patterns
        patterns.insert("privilege_escalation".to_string(), vec![
            r"(?i)sudo\s+.*-s".to_string(),
            r"(?i)su\s+.*-".to_string(),
            r"(?i)chmod\s+4755".to_string(),
        ]);

        let mut compiled_patterns = HashMap::new();
        for (category, pattern_list) in &patterns {
            for pattern in pattern_list {
                let regex = regex::Regex::new(pattern)?;
                compiled_patterns.insert(format!("{}:{}", category, pattern), regex);
            }
        }

        Ok(Self {
            patterns,
            compiled_patterns,
        })
    }

    /// Analyze content for pattern-based threats
    pub async fn analyze(&self, content: &str, context: &AnalysisContext) -> ThreatAnalysis {
        let mut threats = Vec::new();
        let mut risk_score = 0.0;

        for (pattern_key, regex) in &self.compiled_patterns {
            if regex.is_match(content) {
                let threat_type = pattern_key.split(':').next().unwrap_or("unknown");
                let confidence = match threat_type {
                    "malware_signatures" => 0.9,
                    "data_exfiltration" => 0.8,
                    "privilege_escalation" => 0.85,
                    _ => 0.7,
                };

                threats.push(Threat {
                    threat_type: threat_type.to_string(),
                    severity: match threat_type {
                        "malware_signatures" | "privilege_escalation" => Severity::High,
                        "data_exfiltration" => Severity::Medium,
                        _ => Severity::Medium,
                    },
                    confidence,
                    evidence: vec![format!("Pattern match: {}", pattern_key)],
                    context: context.clone(),
                });

                risk_score += confidence;
            }
        }

        ThreatAnalysis {
            threats,
            risk_score: risk_score.min(1.0),
            confidence: 0.95, // High confidence in pattern matching
            recommended_actions: Vec::new(),
        }
    }
}

/// Behavioral analyzer for user behavior patterns
pub struct BehavioralAnalyzer {
    audit_manager: Arc<AuditManager>,
    user_profiles: RwLock<HashMap<String, UserProfile>>,
}

impl BehavioralAnalyzer {
    /// Create new behavioral analyzer
    pub async fn new(audit_manager: Arc<AuditManager>) -> anyhow::Result<Self> {
        Ok(Self {
            audit_manager,
            user_profiles: RwLock::new(HashMap::new()),
        })
    }

    /// Analyze user behavior for anomalies
    pub async fn analyze_behavior(&self, context: &AnalysisContext) -> ThreatAnalysis {
        let mut threats = Vec::new();
        let mut risk_score = 0.0;

        if let Some(user_id) = &context.user_id {
            let mut profiles = self.user_profiles.write().await;
            let profile = profiles.entry(user_id.clone())
                .or_insert_with(|| UserProfile::new(user_id.clone()));

            // Update profile with current activity
            profile.record_activity(context);

            // Check for behavioral anomalies
            if let Some(anomaly) = profile.detect_anomaly(context) {
                threats.push(Threat {
                    threat_type: "suspicious_behavior".to_string(),
                    severity: anomaly.severity,
                    confidence: anomaly.confidence,
                    evidence: anomaly.evidence,
                    context: context.clone(),
                });
                risk_score += anomaly.confidence;
            }
        }

        ThreatAnalysis {
            threats,
            risk_score: risk_score.min(1.0),
            confidence: 0.8,
            recommended_actions: Vec::new(),
        }
    }
}

/// ML-enhanced PHI detector
pub struct MLPhiDetector {
    // Placeholder for ML model integration
    // In a real implementation, this would load a trained ML model
}

impl MLPhiDetector {
    /// Create new ML PHI detector
    pub async fn new() -> anyhow::Result<Self> {
        // Initialize ML model here
        Ok(Self {})
    }

    /// Analyze content for PHI using ML-enhanced detection
    pub async fn analyze_phi(&self, content: &str, context: &AnalysisContext) -> ThreatAnalysis {
        // Use basic PHI detector as fallback
        // In production, this would use ML model for better accuracy
        let basic_detector = PhiDetector::new().unwrap();
        let result = basic_detector.detect_phi(content);

        let mut threats = Vec::new();
        let risk_score = if result.has_phi { result.risk_score } else { 0.0 };

        if result.has_phi {
            threats.push(Threat {
                threat_type: "phi_leakage".to_string(),
                severity: Severity::High,
                confidence: result.risk_score,
                evidence: vec![format!("PHI detected with risk score: {:.2}", result.risk_score)],
                context: context.clone(),
            });
        }

        ThreatAnalysis {
            threats,
            risk_score,
            confidence: 0.85, // ML-enhanced confidence
            recommended_actions: Vec::new(),
        }
    }
}

/// Threat intelligence feed
pub struct ThreatIntelligence {
    known_threats: RwLock<HashMap<String, ThreatIndicator>>,
}

impl ThreatIntelligence {
    /// Create new threat intelligence system
    pub async fn new() -> anyhow::Result<Self> {
        let mut known_threats = HashMap::new();

        // Initialize with some known threat indicators
        // In production, this would be updated from threat feeds
        known_threats.insert("malicious_ip_1".to_string(), ThreatIndicator {
            indicator_type: "ip_address".to_string(),
            value: "192.168.1.100".to_string(),
            severity: Severity::High,
            last_seen: Utc::now(),
            confidence: 0.95,
        });

        Ok(Self {
            known_threats: RwLock::new(known_threats),
        })
    }

    /// Correlate threats with intelligence data
    pub async fn correlate_threats(&self, threats: &[Threat], context: &AnalysisContext) -> ThreatAnalysis {
        let intelligence = self.known_threats.read().await;
        let mut correlated_threats = Vec::new();
        let mut risk_score = 0.0;

        for threat in threats {
            // Check if threat matches known indicators
            if let Some(indicator) = intelligence.get(&threat.threat_type) {
                correlated_threats.push(Threat {
                    threat_type: format!("known_{}", threat.threat_type),
                    severity: indicator.severity.clone(),
                    confidence: (threat.confidence + indicator.confidence) / 2.0,
                    evidence: vec![
                        threat.evidence.join("; "),
                        format!("Correlated with known threat: {}", indicator.value),
                    ],
                    context: context.clone(),
                });
                risk_score += indicator.confidence;
            }
        }

        ThreatAnalysis {
            threats: correlated_threats,
            risk_score: risk_score.min(1.0),
            confidence: 0.9,
            recommended_actions: Vec::new(),
        }
    }
}

/// User profile for behavioral analysis
#[derive(Debug, Clone)]
pub struct UserProfile {
    user_id: String,
    activities: VecDeque<UserActivity>,
    normal_patterns: HashMap<String, ActivityPattern>,
}

impl UserProfile {
    /// Create new user profile
    pub fn new(user_id: String) -> Self {
        Self {
            user_id,
            activities: VecDeque::with_capacity(1000),
            normal_patterns: HashMap::new(),
        }
    }

    /// Record user activity
    pub fn record_activity(&mut self, context: &AnalysisContext) {
        let activity = UserActivity {
            timestamp: Utc::now(),
            action: context.action.clone(),
            resource: context.resource.clone(),
            risk_level: 0.0, // Would be calculated based on context
        };

        self.activities.push_back(activity);

        // Maintain activity window (last 24 hours)
        let cutoff = Utc::now() - Duration::hours(24);
        while self.activities.front().map_or(false, |a| a.timestamp < cutoff) {
            self.activities.pop_front();
        }

        // Update normal patterns
        self.update_patterns();
    }

    /// Detect behavioral anomalies
    pub fn detect_anomaly(&self, context: &AnalysisContext) -> Option<BehavioralAnomaly> {
        let recent_activities: Vec<_> = self.activities.iter()
            .filter(|a| Utc::now() - a.timestamp < Duration::hours(1))
            .collect();

        // Check for unusual activity patterns
        let high_risk_actions = recent_activities.iter()
            .filter(|a| a.risk_level > 0.7)
            .count();

        if high_risk_actions > 5 {
            return Some(BehavioralAnomaly {
                anomaly_type: "high_risk_activity_burst".to_string(),
                severity: Severity::Medium,
                confidence: 0.8,
                evidence: vec![format!("{} high-risk actions in last hour", high_risk_actions)],
            });
        }

        // Check for unusual timing patterns
        if self.detect_timing_anomaly(&recent_activities) {
            return Some(BehavioralAnomaly {
                anomaly_type: "unusual_timing_pattern".to_string(),
                severity: Severity::Low,
                confidence: 0.6,
                evidence: vec!["Unusual activity timing detected".to_string()],
            });
        }

        None
    }

    /// Update normal activity patterns
    fn update_patterns(&mut self) {
        // Simple pattern learning - count actions by hour
        let mut action_counts = HashMap::new();

        for activity in &self.activities {
            let hour = activity.timestamp.hour();
            let key = format!("{}:{}", activity.action, hour);
            *action_counts.entry(key).or_insert(0) += 1;
        }

        // Convert to patterns
        for (key, count) in action_counts {
            let pattern = ActivityPattern {
                action: key.clone(),
                average_count: count as f64 / 24.0, // Average per hour over 24 hours
                standard_deviation: 1.0, // Simplified
            };
            self.normal_patterns.insert(key, pattern);
        }
    }

    /// Detect timing anomalies
    fn detect_timing_anomaly(&self, activities: &[&UserActivity]) -> bool {
        if activities.len() < 3 {
            return false;
        }

        // Check for activities at unusual hours
        let current_hour = Utc::now().hour();
        let unusual_hours = vec![2, 3, 4, 5]; // 2-5 AM

        if unusual_hours.contains(&current_hour) && activities.len() > 10 {
            return true;
        }

        false
    }
}

/// Analysis context
#[derive(Debug, Clone)]
pub struct AnalysisContext {
    pub user_id: Option<String>,
    pub action: String,
    pub resource: String,
    pub source_ip: Option<String>,
    pub user_agent: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Threat analysis result
#[derive(Debug, Clone)]
pub struct ThreatAnalysis {
    pub threats: Vec<Threat>,
    pub risk_score: f64,
    pub confidence: f64,
    pub recommended_actions: Vec<String>,
}

/// Individual threat
#[derive(Debug, Clone)]
pub struct Threat {
    pub threat_type: String,
    pub severity: Severity,
    pub confidence: f64,
    pub evidence: Vec<String>,
    pub context: AnalysisContext,
}

/// Detection event for history
#[derive(Debug, Clone)]
pub struct DetectionEvent {
    pub timestamp: DateTime<Utc>,
    pub content_hash: u64,
    pub context: AnalysisContext,
    pub threats: Vec<Threat>,
    pub risk_score: f64,
}

/// Detection statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DetectionStats {
    pub total_analyses: usize,
    pub high_risk_detections: usize,
    pub medium_risk_detections: usize,
    pub low_risk_detections: usize,
    pub threat_types: HashMap<String, usize>,
    pub average_risk_score: f64,
    pub last_update: DateTime<Utc>,
}

/// Threat indicator from intelligence feeds
#[derive(Debug, Clone)]
pub struct ThreatIndicator {
    pub indicator_type: String,
    pub value: String,
    pub severity: Severity,
    pub last_seen: DateTime<Utc>,
    pub confidence: f64,
}

/// User activity record
#[derive(Debug, Clone)]
pub struct UserActivity {
    pub timestamp: DateTime<Utc>,
    pub action: String,
    pub resource: String,
    pub risk_level: f64,
}

/// Activity pattern for behavioral analysis
#[derive(Debug, Clone)]
pub struct ActivityPattern {
    pub action: String,
    pub average_count: f64,
    pub standard_deviation: f64,
}

/// Behavioral anomaly
#[derive(Debug, Clone)]
pub struct BehavioralAnomaly {
    pub anomaly_type: String,
    pub severity: Severity,
    pub confidence: f64,
    pub evidence: Vec<String>,
}

// Re-export types
pub use crate::runtime_monitoring::Severity;