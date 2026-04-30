//! Security monitoring dashboard and real-time metrics

use crate::audit::AuditManager;
use crate::compliance::{ComplianceMonitor, ComplianceStatus};
use crate::correlation_engine::{CorrelationEngine, CorrelationStats};
use crate::incident_response::{IncidentResponseEngine, IncidentResponseStats};
use crate::monitoring::SecurityMonitor;
use crate::runtime_monitoring::{RuntimeSecurityMonitor, MonitoringStats};
use crate::threat_detection::AdvancedThreatDetector;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};

/// Security dashboard providing real-time monitoring and metrics
pub struct SecurityDashboard {
    /// Security state
    security_state: Arc<super::SecurityState>,
    /// Dashboard data cache
    dashboard_data: Arc<RwLock<DashboardData>>,
    /// Update interval
    update_interval: Duration,
}

impl SecurityDashboard {
    /// Create new security dashboard
    pub fn new(security_state: Arc<super::SecurityState>) -> Self {
        Self {
            security_state,
            dashboard_data: Arc::new(RwLock::new(DashboardData::default())),
            update_interval: Duration::seconds(30), // Update every 30 seconds
        }
    }

    /// Start dashboard background updates
    pub async fn start_dashboard_updates(self: Arc<Self>) -> anyhow::Result<()> {
        let self_clone = self.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(self_clone.update_interval.to_std().unwrap());

            loop {
                interval.tick().await;

                if let Err(e) = self_clone.update_dashboard_data().await {
                    tracing::error!("Failed to update dashboard data: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Update dashboard data
    async fn update_dashboard_data(&self) -> anyhow::Result<()> {
        let monitoring_stats = self.security_state.runtime_monitor.get_monitoring_stats().await;
        let correlation_stats = self.security_state.correlation_engine.get_correlation_stats().await;
        let incident_stats = self.security_state.incident_response.get_response_stats().await;
        let compliance_status = self.security_state.compliance.get_compliance_status().await;
        let detection_stats = self.security_state.threat_detector.get_detection_stats().await;

        let alerts = self.security_state.correlation_engine.get_active_alerts().await;
        let incidents = self.security_state.incident_response.get_active_incidents().await;

        let dashboard_data = DashboardData {
            timestamp: Utc::now(),
            monitoring_stats,
            correlation_stats,
            incident_stats,
            compliance_status: Some(compliance_status),
            detection_stats,
            active_alerts: alerts,
            active_incidents: incidents,
            system_health: self.calculate_system_health(&monitoring_stats, &correlation_stats, &incident_stats),
        };

        *self.dashboard_data.write().await = dashboard_data;

        Ok(())
    }

    /// Calculate overall system health score
    fn calculate_system_health(
        &self,
        monitoring: &MonitoringStats,
        correlation: &CorrelationStats,
        incidents: &IncidentResponseStats,
    ) -> SystemHealth {
        let monitoring_score = if monitoring.total_requests > 0 {
            let block_rate = monitoring.blocked_requests as f64 / monitoring.total_requests as f64;
            (1.0 - block_rate.min(0.5) * 2.0) * 100.0 // Penalize high block rates
        } else {
            100.0
        };

        let alert_score = if correlation.active_alerts > 0 {
            let critical_weight = correlation.critical_alerts as f64 * 2.0;
            let high_weight = correlation.high_alerts as f64 * 1.0;
            let total_weight = critical_weight + high_weight;

            (1.0 - (total_weight / 10.0).min(1.0)) * 100.0
        } else {
            100.0
        };

        let incident_score = if incidents.total_incidents > 0 {
            let resolved_rate = incidents.resolved_incidents as f64 / incidents.total_incidents as f64;
            resolved_rate * 100.0
        } else {
            100.0
        };

        let overall_score = (monitoring_score + alert_score + incident_score) / 3.0;

        let status = if overall_score >= 90.0 {
            HealthStatus::Healthy
        } else if overall_score >= 70.0 {
            HealthStatus::Warning
        } else {
            HealthStatus::Critical
        };

        SystemHealth {
            overall_score,
            monitoring_score,
            alert_score,
            incident_score,
            status,
        }
    }

    /// Get current dashboard data
    pub async fn get_dashboard_data(&self) -> DashboardData {
        self.dashboard_data.read().await.clone()
    }

    /// Create Axum router for dashboard API
    pub fn create_router(self: Arc<Self>) -> Router {
        Router::new()
            .route("/api/dashboard", get(Self::get_dashboard))
            .route("/api/dashboard/health", get(Self::get_system_health))
            .route("/api/dashboard/alerts", get(Self::get_active_alerts))
            .route("/api/dashboard/incidents", get(Self::get_active_incidents))
            .route("/api/dashboard/metrics", get(Self::get_metrics))
            .route("/api/dashboard/alerts/:id/acknowledge", post(Self::acknowledge_alert))
            .route("/api/dashboard/incidents/:id/update", post(Self::update_incident))
            .with_state(self)
    }

    /// Get dashboard data endpoint
    async fn get_dashboard(
        State(dashboard): State<Arc<SecurityDashboard>>,
    ) -> Result<Json<DashboardData>, StatusCode> {
        let data = dashboard.get_dashboard_data().await;
        Ok(Json(data))
    }

    /// Get system health endpoint
    async fn get_system_health(
        State(dashboard): State<Arc<SecurityDashboard>>,
    ) -> Result<Json<SystemHealth>, StatusCode> {
        let data = dashboard.get_dashboard_data().await;
        Ok(Json(data.system_health))
    }

    /// Get active alerts endpoint
    async fn get_active_alerts(
        State(dashboard): State<Arc<SecurityDashboard>>,
    ) -> Result<Json<Vec<crate::correlation_engine::Alert>>, StatusCode> {
        let data = dashboard.get_dashboard_data().await;
        Ok(Json(data.active_alerts))
    }

    /// Get active incidents endpoint
    async fn get_active_incidents(
        State(dashboard): State<Arc<SecurityDashboard>>,
    ) -> Result<Json<Vec<crate::incident_response::Incident>>, StatusCode> {
        let data = dashboard.get_dashboard_data().await;
        Ok(Json(data.active_incidents))
    }

    /// Get metrics endpoint
    async fn get_metrics(
        State(dashboard): State<Arc<SecurityDashboard>>,
    ) -> Result<Json<DashboardMetrics>, StatusCode> {
        let data = dashboard.get_dashboard_data().await;

        let metrics = DashboardMetrics {
            total_requests: data.monitoring_stats.total_requests,
            blocked_requests: data.monitoring_stats.blocked_requests,
            active_alerts: data.correlation_stats.active_alerts,
            active_incidents: data.incident_stats.active_incidents,
            compliance_score: data.compliance_status.as_ref().map(|c| c.overall_score).unwrap_or(0.0),
            threat_detection_accuracy: data.detection_stats.average_risk_score,
        };

        Ok(Json(metrics))
    }

    /// Acknowledge alert endpoint
    async fn acknowledge_alert(
        State(dashboard): State<Arc<SecurityDashboard>>,
        axum::extract::Path(alert_id): axum::extract::Path<String>,
    ) -> Result<StatusCode, StatusCode> {
        dashboard.security_state.correlation_engine
            .acknowledge_alert(&alert_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(StatusCode::OK)
    }

    /// Update incident endpoint
    async fn update_incident(
        State(dashboard): State<Arc<SecurityDashboard>>,
        axum::extract::Path(incident_id): axum::extract::Path<String>,
        axum::extract::Json(update): axum::extract::Json<IncidentUpdate>,
    ) -> Result<StatusCode, StatusCode> {
        use crate::incident_response::IncidentStatus;

        let status = match update.status.as_str() {
            "active" => IncidentStatus::Active,
            "investigating" => IncidentStatus::Investigating,
            "mitigated" => IncidentStatus::Mitigated,
            "resolved" => IncidentStatus::Resolved,
            "closed" => IncidentStatus::Closed,
            _ => return Err(StatusCode::BAD_REQUEST),
        };

        dashboard.security_state.incident_response
            .update_incident_status(&incident_id, status)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(StatusCode::OK)
    }

    /// Generate HTML dashboard (basic implementation)
    pub async fn generate_html_dashboard(&self) -> String {
        let data = self.get_dashboard_data().await;

        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Ferrumyx Security Dashboard</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .metric {{ background: #f0f0f0; padding: 10px; margin: 10px 0; border-radius: 5px; }}
        .alert {{ background: #ffebee; border-left: 5px solid #f44336; padding: 10px; margin: 10px 0; }}
        .incident {{ background: #fff3e0; border-left: 5px solid #ff9800; padding: 10px; margin: 10px 0; }}
        .healthy {{ color: #4caf50; }}
        .warning {{ color: #ff9800; }}
        .critical {{ color: #f44336; }}
    </style>
</head>
<body>
    <h1>Ferrumyx Security Dashboard</h1>
    <p>Last updated: {}</p>

    <div class="metric">
        <h2>System Health</h2>
        <p class="{}">Overall Score: {:.1}%</p>
        <p>Monitoring: {:.1}% | Alerts: {:.1}% | Incidents: {:.1}%</p>
    </div>

    <div class="metric">
        <h2>Request Monitoring</h2>
        <p>Total Requests: {} | Blocked: {} | Allowed: {}</p>
    </div>

    <div class="metric">
        <h2>Active Alerts: {}</h2>
        {}
    </div>

    <div class="metric">
        <h2>Active Incidents: {}</h2>
        {}
    </div>

    <div class="metric">
        <h2>Compliance Status</h2>
        <p>Score: {:.1}% | Critical Alerts: {} | High Alerts: {}</p>
    </div>
</body>
</html>"#,
            data.timestamp.format("%Y-%m-%d %H:%M:%S"),
            match data.system_health.status {
                HealthStatus::Healthy => "healthy",
                HealthStatus::Warning => "warning",
                HealthStatus::Critical => "critical",
            },
            data.system_health.overall_score,
            data.system_health.monitoring_score,
            data.system_health.alert_score,
            data.system_health.incident_score,
            data.monitoring_stats.total_requests,
            data.monitoring_stats.blocked_requests,
            data.monitoring_stats.allowed_requests,
            data.active_alerts.len(),
            data.active_alerts.iter().map(|alert| {
                format!(r#"<div class="alert"><strong>{}</strong>: {} (Severity: {:?})</div>"#,
                    alert.title, alert.message, alert.severity)
            }).collect::<Vec<_>>().join(""),
            data.active_incidents.len(),
            data.active_incidents.iter().map(|incident| {
                format!(r#"<div class="incident"><strong>{}</strong>: {} (Status: {:?})</div>"#,
                    incident.incident_type, incident.description, incident.status)
            }).collect::<Vec<_>>().join(""),
            data.compliance_status.as_ref().map(|c| c.overall_score).unwrap_or(0.0),
            data.compliance_status.as_ref().map(|c| c.critical_alerts).unwrap_or(0),
            data.compliance_status.as_ref().map(|c| c.high_alerts).unwrap_or(0),
        )
    }
}

/// Dashboard data structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DashboardData {
    pub timestamp: DateTime<Utc>,
    pub monitoring_stats: MonitoringStats,
    pub correlation_stats: CorrelationStats,
    pub incident_stats: IncidentResponseStats,
    pub compliance_status: Option<ComplianceStatus>,
    pub detection_stats: crate::threat_detection::DetectionStats,
    pub active_alerts: Vec<crate::correlation_engine::Alert>,
    pub active_incidents: Vec<crate::incident_response::Incident>,
    pub system_health: SystemHealth,
}

/// System health status
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemHealth {
    pub overall_score: f64,
    pub monitoring_score: f64,
    pub alert_score: f64,
    pub incident_score: f64,
    pub status: HealthStatus,
}

/// Health status levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

/// Dashboard metrics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMetrics {
    pub total_requests: usize,
    pub blocked_requests: usize,
    pub active_alerts: usize,
    pub active_incidents: usize,
    pub compliance_score: f64,
    pub threat_detection_accuracy: f64,
}

/// Incident update request
#[derive(Debug, Deserialize)]
pub struct IncidentUpdate {
    pub status: String,
}