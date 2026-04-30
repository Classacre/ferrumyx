//! Audit trail verification and management

use crate::encryption::EncryptionManager;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

/// Audit event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditEventType {
    /// Authentication events
    Authentication,
    /// Authorization events
    Authorization,
    /// Data access events
    DataAccess,
    /// Data modification events
    DataModification,
    /// Security events
    Security,
    /// Compliance events
    Compliance,
    /// PHI access events
    PhiAccess,
    /// PHI detection events
    PhiDetection,
    /// PHI blocking events
    PhiBlocking,
}

impl std::fmt::Display for AuditEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditEventType::Authentication => write!(f, "Authentication"),
            AuditEventType::Authorization => write!(f, "Authorization"),
            AuditEventType::DataAccess => write!(f, "DataAccess"),
            AuditEventType::DataModification => write!(f, "DataModification"),
            AuditEventType::Security => write!(f, "Security"),
            AuditEventType::Compliance => write!(f, "Compliance"),
            AuditEventType::PhiAccess => write!(f, "PhiAccess"),
            AuditEventType::PhiDetection => write!(f, "PhiDetection"),
            AuditEventType::PhiBlocking => write!(f, "PhiBlocking"),
        }
    }
}

/// Audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub event_type: AuditEventType,
    pub user_id: Option<String>,
    pub resource: String,
    pub action: String,
    pub data_class: String,
    pub timestamp: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub details: HashMap<String, serde_json::Value>,
    pub hash: String, // Integrity hash
}

/// Audit manager for managing audit trails
pub struct AuditManager {
    pool: PgPool,
    encryption: EncryptionManager,
}

impl AuditManager {
    /// Create new audit manager
    pub async fn new() -> anyhow::Result<Self> {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/ferrumyx".to_string());

        let pool = PgPool::connect(&database_url).await?;
        let encryption = EncryptionManager::new()?;

        // Create audit tables if they don't exist
        Self::init_tables(&pool).await?;

        Ok(Self { pool, encryption })
    }

    /// Initialize audit tables
    async fn init_tables(pool: &PgPool) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_events (
                id UUID PRIMARY KEY,
                event_type TEXT NOT NULL,
                user_id TEXT,
                resource TEXT NOT NULL,
                action TEXT NOT NULL,
                data_class TEXT NOT NULL,
                timestamp TIMESTAMPTZ NOT NULL,
                ip_address INET,
                user_agent TEXT,
                success BOOLEAN NOT NULL,
                details JSONB,
                hash TEXT NOT NULL,
                created_at TIMESTAMPTZ DEFAULT NOW()
            );

            CREATE INDEX IF NOT EXISTS idx_audit_events_timestamp ON audit_events(timestamp);
            CREATE INDEX IF NOT EXISTS idx_audit_events_user ON audit_events(user_id);
            CREATE INDEX IF NOT EXISTS idx_audit_events_type ON audit_events(event_type);
            CREATE INDEX IF NOT EXISTS idx_audit_events_resource ON audit_events(resource);
            "#
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Log an audit event
    pub async fn log_event(&self, mut event: AuditEvent) -> anyhow::Result<()> {
        // Generate integrity hash
        let event_data = format!(
            "{}{}{}{}{}{}{}{}{}",
            event.id,
            serde_json::to_string(&event.event_type)?,
            event.user_id.as_deref().unwrap_or(""),
            event.resource,
            event.action,
            event.data_class,
            event.timestamp.to_rfc3339(),
            event.success,
            serde_json::to_string(&event.details)?
        );

        event.hash = self.encryption.hash_data(event_data.as_bytes());

        // Encrypt sensitive details if any
        let encrypted_details = if event.details.contains_key("phi_data") {
            let mut encrypted = event.details.clone();
            if let Some(phi_value) = encrypted.get_mut("phi_data") {
                if let serde_json::Value::String(phi_str) = phi_value {
                    *phi_value = serde_json::Value::String(
                        self.encryption.encrypt(phi_str.as_bytes())?
                    );
                }
            }
            encrypted
        } else {
            event.details
        };

        sqlx::query(
            r#"
            INSERT INTO audit_events (
                id, event_type, user_id, resource, action, data_class,
                timestamp, ip_address, user_agent, success, details, hash
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#
        )
        .bind(event.id)
        .bind(serde_json::to_string(&event.event_type)?)
        .bind(&event.user_id)
        .bind(&event.resource)
        .bind(&event.action)
        .bind(&event.data_class)
        .bind(event.timestamp)
        .bind(&event.ip_address)
        .bind(&event.user_agent)
        .bind(event.success)
        .bind(serde_json::Value::Object(encrypted_details.into_iter().collect()))
        .bind(&event.hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Verify audit trail integrity
    pub async fn verify_integrity(&self, start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> anyhow::Result<AuditIntegrityReport> {
        let events = sqlx::query_as::<_, AuditEventRow>(
            "SELECT * FROM audit_events WHERE timestamp BETWEEN $1 AND $2 ORDER BY timestamp"
        )
        .bind(start_time)
        .bind(end_time)
        .fetch_all(&self.pool)
        .await?;

        let mut integrity_violations = Vec::new();
        let mut previous_hash: Option<&AuditEventRow> = None;

        for event_row in &events {
            // Verify individual event hash
            let event_data = format!(
                "{}{}{}{}{}{}{}{}{}",
                event_row.id,
                event_row.event_type,
                event_row.user_id.as_deref().unwrap_or(""),
                event_row.resource,
                event_row.action,
                event_row.data_class,
                event_row.timestamp.to_rfc3339(),
                event_row.success,
                event_row.details
            );

            let calculated_hash = self.encryption.hash_data(event_data.as_bytes());

            if calculated_hash != event_row.hash {
                integrity_violations.push(AuditViolation {
                    event_id: event_row.id,
                    violation_type: ViolationType::HashMismatch,
                    description: "Event hash does not match calculated hash".to_string(),
                });
            }

            // Check for chronological ordering (basic chain validation)
            if let Some(prev_hash) = previous_hash {
                if event_row.timestamp < prev_hash.timestamp {
                    integrity_violations.push(AuditViolation {
                        event_id: event_row.id,
                        violation_type: ViolationType::ChronologicalViolation,
                        description: "Event timestamp is earlier than previous event".to_string(),
                    });
                }
            }

            previous_hash = Some(event_row);
        }

        // Check for gaps in audit trail (missing events)
        let total_events = events.len();
        let expected_events = self.estimate_expected_events(start_time, end_time).await?;

        if total_events < expected_events * 0.9 as usize {
            integrity_violations.push(AuditViolation {
                event_id: Uuid::nil(), // No specific event
                violation_type: ViolationType::MissingEvents,
                description: format!(
                    "Potentially missing events: expected ~{}, found {}",
                    expected_events, total_events
                ),
            });
        }

        let integrity_score = if integrity_violations.is_empty() {
            100.0
        } else {
            100.0 - (integrity_violations.len() as f64 / events.len() as f64) * 100.0
        };

        Ok(AuditIntegrityReport {
            period_start: start_time,
            period_end: end_time,
            total_events: events.len(),
            violations: integrity_violations,
            integrity_score,
        })
    }

    /// Estimate expected number of events for integrity checking
    async fn estimate_expected_events(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> anyhow::Result<usize> {
        // Get average events per hour from historical data
        let hours = (end - start).num_hours() as f64;

        let avg_events_per_hour: (f64,) = sqlx::query_as(
            "SELECT COALESCE(AVG(event_count), 1.0) FROM (
                SELECT COUNT(*) as event_count
                FROM audit_events
                WHERE timestamp >= NOW() - INTERVAL '30 days'
                GROUP BY DATE_TRUNC('hour', timestamp)
            ) as hourly_counts"
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or((1.0,));

        Ok((avg_events_per_hour.0 * hours) as usize)
    }

    /// Log PHI detection event
    pub async fn log_phi_detection(
        &self,
        user_id: Option<String>,
        channel: String,
        content_snippet: String,
        risk_score: f64,
        action_taken: String,
        detection_details: serde_json::Value,
    ) -> anyhow::Result<()> {
        let mut details = HashMap::new();
        details.insert("channel".to_string(), serde_json::Value::String(channel));
        details.insert("content_snippet".to_string(), serde_json::Value::String(content_snippet));
        details.insert("risk_score".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(risk_score).unwrap()));
        details.insert("action_taken".to_string(), serde_json::Value::String(action_taken));
        details.insert("detection_details".to_string(), detection_details);

        let event = AuditEvent {
            id: Uuid::new_v4(),
            event_type: AuditEventType::PhiDetection,
            user_id,
            resource: "phi_detector".to_string(),
            action: "detect".to_string(),
            data_class: "phi_detection".to_string(),
            timestamp: Utc::now(),
            ip_address: None,
            user_agent: None,
            success: true,
            details: details.into(),
            hash: String::new(),
        };

        self.log_event(event).await
    }

    /// Log PHI blocking event
    pub async fn log_phi_blocking(
        &self,
        user_id: Option<String>,
        channel: String,
        reason: String,
        blocked_content_length: usize,
    ) -> anyhow::Result<()> {
        let mut details = HashMap::new();
        details.insert("channel".to_string(), serde_json::Value::String(channel));
        details.insert("reason".to_string(), serde_json::Value::String(reason));
        details.insert("blocked_content_length".to_string(), serde_json::Value::Number(blocked_content_length.into()));

        let event = AuditEvent {
            id: Uuid::new_v4(),
            event_type: AuditEventType::PhiBlocking,
            user_id,
            resource: "phi_filter".to_string(),
            action: "block".to_string(),
            data_class: "phi_blocking".to_string(),
            timestamp: Utc::now(),
            ip_address: None,
            user_agent: None,
            success: true,
            details: details.into(),
            hash: String::new(),
        };

        self.log_event(event).await
    }

    /// Get audit events for compliance reporting
    pub async fn get_events_for_compliance(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        event_types: Option<Vec<AuditEventType>>,
    ) -> anyhow::Result<Vec<AuditEvent>> {
        let mut query = sqlx::QueryBuilder::new(
            "SELECT id, event_type, user_id, resource, action, data_class, timestamp, ip_address, user_agent, success, details, hash FROM audit_events WHERE timestamp BETWEEN "
        );

        query.push_bind(start_time).push(" AND ").push_bind(end_time);

        if let Some(types) = event_types {
            query.push(" AND event_type = ANY(").push_bind(
                types.into_iter().map(|t| serde_json::to_string(&t).unwrap()).collect::<Vec<_>>()
            ).push(")");
        }

        query.push(" ORDER BY timestamp");

        let rows: Vec<AuditEventRow> = query.build_query_as().fetch_all(&self.pool).await?;

        let mut events = Vec::new();
        for row in rows {
            events.push(AuditEvent {
                id: row.id,
                event_type: serde_json::from_str(&row.event_type)?,
                user_id: row.user_id,
                resource: row.resource,
                action: row.action,
                data_class: row.data_class,
                timestamp: row.timestamp,
                ip_address: row.ip_address,
                user_agent: row.user_agent,
                success: row.success,
                details: serde_json::from_str(&row.details)?,
                hash: row.hash,
            });
        }

        Ok(events)
    }
}

/// Database row representation
#[derive(sqlx::FromRow)]
struct AuditEventRow {
    id: Uuid,
    event_type: String,
    user_id: Option<String>,
    resource: String,
    action: String,
    data_class: String,
    timestamp: DateTime<Utc>,
    ip_address: Option<String>,
    user_agent: Option<String>,
    success: bool,
    details: String,
    hash: String,
}

/// Audit integrity report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditIntegrityReport {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_events: usize,
    pub violations: Vec<AuditViolation>,
    pub integrity_score: f64,
}

/// Audit violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditViolation {
    pub event_id: Uuid,
    pub violation_type: ViolationType,
    pub description: String,
}

/// Types of audit violations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationType {
    HashMismatch,
    ChronologicalViolation,
    MissingEvents,
    TamperingDetected,
}