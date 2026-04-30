# Ferrumyx Operations Guide

## Monitoring

Ferrumyx includes comprehensive monitoring capabilities to ensure system health, performance, and security. This guide covers monitoring setup, metrics collection, alerting, and dashboard usage.

### Built-in Monitoring Stack

Ferrumyx ships with a complete observability stack:

- **Prometheus**: Metrics collection and storage
- **Grafana**: Visualization and dashboards
- **Loki**: Log aggregation
- **AlertManager**: Alert routing and notification
- **cAdvisor**: Container metrics
- **Node Exporter**: System metrics

### Setting Up Monitoring

#### Automated Setup

```bash
# Setup monitoring stack
bash scripts/monitoring-setup.sh

# Access interfaces
# Grafana: http://localhost:3001 (admin/admin)
# Prometheus: http://localhost:9090
# AlertManager: http://localhost:9093
```

#### Manual Setup

```yaml
# docker-compose.monitoring.yml
version: '3.8'
services:
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./monitoring/prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus_data:/prometheus

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3001:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - grafana_data:/var/lib/grafana

  loki:
    image: grafana/loki:latest
    ports:
      - "3100:3100"
    volumes:
      - loki_data:/loki

  promtail:
    image: grafana/promtail:latest
    volumes:
      - /var/log:/var/log
      - ./monitoring/promtail.yml:/etc/promtail/config.yml
```

### Key Metrics to Monitor

#### Application Metrics

| Metric | Description | Threshold | Alert |
|--------|-------------|-----------|-------|
| `ferrumyx_requests_total` | Total API requests | - | - |
| `ferrumyx_request_duration_seconds` | Request latency | >5s | Warning |
| `ferrumyx_ingestion_duration_seconds` | Literature ingestion time | >300s | Warning |
| `ferrumyx_agent_loops_total` | Agent loop executions | - | - |
| `ferrumyx_memory_usage_bytes` | Application memory usage | >2GB | Critical |
| `ferrumyx_active_connections` | Database connections | >50 | Warning |

#### System Metrics

| Metric | Description | Threshold | Alert |
|--------|-------------|-----------|-------|
| `node_cpu_usage` | CPU utilization | >80% | Warning |
| `node_memory_usage` | Memory utilization | >90% | Critical |
| `node_disk_usage` | Disk utilization | >85% | Warning |
| `node_network_receive_bytes` | Network traffic | - | - |

#### Database Metrics

| Metric | Description | Threshold | Alert |
|--------|-------------|-----------|-------|
| `pg_stat_activity_count` | Active connections | >20 | Warning |
| `pg_database_size_bytes` | Database size | >100GB | Info |
| `pg_stat_user_tables_n_tup_ins` | Insert rate | - | - |
| `pg_stat_user_tables_n_tup_upd` | Update rate | - | - |

### Alerting Configuration

#### Prometheus Alert Rules

```yaml
# monitoring/prometheus/alert_rules.yml
groups:
  - name: ferrumyx
    rules:
      - alert: HighRequestLatency
        expr: histogram_quantile(0.95, rate(ferrumyx_request_duration_seconds_bucket[5m])) > 5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High request latency detected"
          description: "95th percentile request latency is {{ $value }}s"

      - alert: HighMemoryUsage
        expr: ferrumyx_memory_usage_bytes / 1024 / 1024 / 1024 > 2
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High memory usage"
          description: "Memory usage is {{ $value }}GB"

      - alert: DatabaseDown
        expr: pg_up == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Database is down"
          description: "PostgreSQL is not responding"

      - alert: HighErrorRate
        expr: rate(ferrumyx_requests_total{status=~"5.."}[5m]) / rate(ferrumyx_requests_total[5m]) > 0.1
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High error rate detected"
          description: "Error rate is {{ $value | humanizePercentage }}"
```

#### AlertManager Configuration

```yaml
# monitoring/alertmanager.yml
global:
  smtp_smarthost: 'smtp.gmail.com:587'
  smtp_from: 'alerts@ferrumyx.org'
  smtp_auth_username: 'alerts@ferrumyx.org'
  smtp_auth_password: 'your-password'

route:
  group_by: ['alertname']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 1h
  receiver: 'email'

receivers:
  - name: 'email'
    email_configs:
      - to: 'ops@yourcompany.com'
        subject: '{{ .GroupLabels.alertname }}'
        body: '{{ .CommonAnnotations.description }}'
```

### Grafana Dashboards

#### Pre-built Dashboards

Ferrumyx includes several pre-configured dashboards:

1. **System Overview**: CPU, memory, disk, network metrics
2. **Application Performance**: Request rates, latency, error rates
3. **Database Performance**: Connection pools, query performance, storage
4. **Ingestion Pipeline**: Processing rates, queue depths, success rates
5. **Security Monitoring**: Failed logins, suspicious activities, audit events

#### Custom Dashboard Creation

```json
// Example panel for request latency
{
  "title": "Request Latency",
  "type": "graph",
  "targets": [
    {
      "expr": "histogram_quantile(0.95, rate(ferrumyx_request_duration_seconds_bucket[5m]))",
      "legendFormat": "95th percentile"
    }
  ],
  "yAxes": [
    {
      "unit": "seconds",
      "min": 0
    }
  ]
}
```

### Log Aggregation

#### Loki Configuration

```yaml
# monitoring/loki-config.yml
auth_enabled: false

server:
  http_listen_port: 3100

ingester:
  lifecycler:
    address: 127.0.0.1
    ring:
      kvstore:
        store: inmemory
      replication_factor: 1
    final_sleep: 0s
  chunk_idle_period: 1h
  chunk_target_size: 1048576
  max_chunk_age: 1h

schema_config:
  configs:
    - from: 2020-10-24
      store: boltdb-shipper
      object_store: filesystem
      schema: v11
      index:
        prefix: index_
        period: 24h

storage_config:
  boltdb_shipper:
    active_index_directory: /loki/boltdb-shipper-active
    cache_location: /loki/boltdb-shipper-cache
    cache_ttl: 24h
    shared_store: filesystem
  filesystem:
    directory: /loki/chunks

limits_config:
  reject_old_samples: true
  reject_old_samples_max_age: 168h
```

#### Promtail Configuration

```yaml
# monitoring/promtail.yml
server:
  http_listen_port: 9080
  grpc_listen_port: 0

positions:
  filename: /tmp/positions.yaml

clients:
  - url: http://loki:3100/loki/api/v1/push

scrape_configs:
  - job_name: ferrumyx
    static_configs:
      - targets:
          - localhost
        labels:
          job: ferrumyx
          __path__: /var/log/ferrumyx/*.log

  - job_name: system
    static_configs:
      - targets:
          - localhost
        labels:
          job: system
          __path__: /var/log/syslog
```

### Health Checks

#### Application Health Endpoints

```bash
# Basic health check
curl http://localhost:3000/health

# Detailed health check
curl http://localhost:3000/health?detailed=true

# Database health
curl http://localhost:3000/health/database

# Ingestion pipeline health
curl http://localhost:3000/health/ingestion
```

#### Automated Health Monitoring

```bash
# Health check script
#!/bin/bash
HEALTH_URL="http://localhost:3000/health"

if curl -f -s "$HEALTH_URL" > /dev/null; then
    echo "✅ Ferrumyx is healthy"
    exit 0
else
    echo "❌ Ferrumyx is unhealthy"
    exit 1
fi
```

## Maintenance

### Regular Maintenance Tasks

#### Daily Tasks

```bash
# Check system health
bash scripts/health-check.sh

# Monitor resource usage
docker stats --no-stream

# Check for security updates
bash scripts/security-check.sh

# Rotate application logs
bash scripts/log-rotate.sh
```

#### Weekly Tasks

```bash
# Update Docker images
docker-compose pull

# Clean up unused resources
docker system prune -f

# Database maintenance
docker-compose exec postgres vacuumdb --all --analyze

# Check backup integrity
bash scripts/backup-verify.sh
```

#### Monthly Tasks

```bash
# Full system backup
bash scripts/db-backup.sh

# Security audit
bash scripts/security-audit.sh

# Performance review
bash scripts/performance-review.sh

# Update monitoring dashboards
bash scripts/update-dashboards.sh
```

### Database Maintenance

#### Vacuum and Analyze

```sql
-- Regular maintenance
VACUUM ANALYZE;

-- Aggressive vacuum for large tables
VACUUM FULL papers;
VACUUM FULL paper_chunks;

-- Reindex if needed
REINDEX TABLE CONCURRENTLY paper_chunks_embedding_idx;
```

#### Index Maintenance

```sql
-- Check index usage
SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan,
    idx_tup_read,
    idx_tup_fetch
FROM pg_stat_user_indexes
ORDER BY idx_scan DESC;

-- Rebuild unused indexes
DROP INDEX CONCURRENTLY unused_index;
CREATE INDEX CONCURRENTLY new_index ON table(column);
```

#### Query Performance Optimization

```sql
-- Identify slow queries
SELECT
    query,
    calls,
    total_time,
    mean_time,
    rows
FROM pg_stat_statements
ORDER BY mean_time DESC
LIMIT 10;

-- Analyze specific query
EXPLAIN ANALYZE SELECT * FROM papers WHERE doi = '10.1234/example';
```

### System Maintenance

#### Package Updates

```bash
# Update system packages
sudo apt update && sudo apt upgrade -y

# Update Rust toolchain
rustup update

# Update Node.js dependencies
npm audit fix

# Update Docker images
docker-compose pull
```

#### Log Rotation

```bash
# Configure logrotate
cat > /etc/logrotate.d/ferrumyx << EOF
/var/log/ferrumyx/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    create 644 ferrumyx ferrumyx
    postrotate
        docker-compose exec ferrumyx-web kill -HUP 1
    endscript
}
EOF
```

#### Disk Space Management

```bash
# Check disk usage
df -h

# Clean up old logs
find /var/log/ferrumyx -name "*.log" -mtime +30 -delete

# Clean up old backups (keep last 7)
ls -t backups/ | tail -n +8 | xargs -I {} rm backups/{}

# Docker cleanup
docker system prune -a --volumes
```

## Backup Procedures

### Automated Backups

#### Database Backups

```bash
# Daily database backup
#!/bin/bash
BACKUP_DIR="/opt/ferrumyx/backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="$BACKUP_DIR/ferrumyx_$TIMESTAMP.sql.gz"

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Dump database
docker-compose exec -T postgres pg_dumpall -U ferrumyx | gzip > "$BACKUP_FILE"

# Verify backup
if [ $? -eq 0 ]; then
    echo "✅ Backup completed: $BACKUP_FILE"
else
    echo "❌ Backup failed"
    exit 1
fi
```

#### Configuration Backups

```bash
# Configuration backup script
#!/bin/bash
BACKUP_DIR="/opt/ferrumyx/backups/config"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

mkdir -p "$BACKUP_DIR"

# Backup configurations
tar -czf "$BACKUP_DIR/config_$TIMESTAMP.tar.gz" \
    .env \
    docker-compose.yml \
    ferrumyx.toml \
    nginx.conf

# Backup secrets (encrypted)
openssl enc -aes-256-cbc -salt \
    -in .env.secrets \
    -out "$BACKUP_DIR/secrets_$TIMESTAMP.enc" \
    -k "$ENCRYPTION_KEY"
```

#### File System Backups

```bash
# Complete system backup
#!/bin/bash
BACKUP_DIR="/opt/ferrumyx/backups/full"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Backup database
bash scripts/db-backup.sh

# Backup configurations
bash scripts/config-backup.sh

# Backup uploaded files
tar -czf "$BACKUP_DIR/uploads_$TIMESTAMP.tar.gz" uploads/

# Backup workspace
tar -czf "$BACKUP_DIR/workspace_$TIMESTAMP.tar.gz" workspace/
```

### Backup Verification

#### Automated Verification

```bash
# Verify database backup
#!/bin/bash
BACKUP_FILE="$1"

# Check file exists and has content
if [ ! -s "$BACKUP_FILE" ]; then
    echo "❌ Backup file is empty or missing"
    exit 1
fi

# Test backup integrity
gunzip -c "$BACKUP_FILE" | head -10 > /dev/null
if [ $? -eq 0 ]; then
    echo "✅ Backup file is valid"
else
    echo "❌ Backup file is corrupted"
    exit 1
fi
```

#### Manual Verification

```bash
# Restore to test database
createdb ferrumyx_test
gunzip -c backup.sql.gz | psql -d ferrumyx_test

# Verify data integrity
psql -d ferrumyx_test -c "SELECT COUNT(*) FROM papers;"
psql -d ferrumyx_test -c "SELECT COUNT(*) FROM kg_facts;"

# Clean up
dropdb ferrumyx_test
```

### Recovery Procedures

#### Database Recovery

```bash
# Stop application
docker-compose stop ferrumyx-web ironclaw-agent

# Drop and recreate database
docker-compose exec postgres dropdb ferrumyx
docker-compose exec postgres createdb ferrumyx

# Restore from backup
gunzip -c backup.sql.gz | docker-compose exec -T postgres psql -d ferrumyx

# Restart application
docker-compose start ferrumyx-web ironclaw-agent

# Verify recovery
bash scripts/health-check.sh
```

#### Full System Recovery

```bash
# Stop all services
docker-compose down

# Restore configurations
tar -xzf config_backup.tar.gz

# Restore database
bash scripts/db-restore.sh --backup-file database_backup.sql.gz

# Restore file uploads
tar -xzf uploads_backup.tar.gz

# Start services
docker-compose up -d

# Run health checks
bash scripts/health-check.sh --comprehensive
```

### Backup Retention Policy

| Backup Type | Retention Period | Storage Location | Encryption |
|-------------|------------------|------------------|------------|
| Daily DB | 7 days | Local + Cloud | AES-256 |
| Weekly DB | 4 weeks | Cloud | AES-256 |
| Monthly DB | 1 year | Cloud | AES-256 |
| Config | 1 year | Git + Cloud | AES-256 |
| Full System | 6 months | Cloud | AES-256 |

### Disaster Recovery

#### Recovery Time Objectives (RTO)

- **Critical**: Database recovery - 4 hours
- **Important**: Full system recovery - 8 hours
- **Normal**: Service restart - 30 minutes

#### Recovery Point Objectives (RPO)

- **Critical Data**: Maximum 1 hour data loss
- **Important Data**: Maximum 4 hours data loss
- **Log Data**: Maximum 24 hours data loss

#### Disaster Recovery Plan

1. **Detection**: Monitoring alerts trigger incident response
2. **Assessment**: Evaluate damage and recovery options
3. **Communication**: Notify stakeholders and users
4. **Recovery**: Execute appropriate recovery procedure
5. **Testing**: Verify system functionality
6. **Lessons Learned**: Document and improve procedures

### Backup Security

#### Encryption

```bash
# Encrypt backups
openssl enc -aes-256-cbc -salt \
    -in backup.sql.gz \
    -out backup.sql.gz.enc \
    -k "$BACKUP_ENCRYPTION_KEY"

# Decrypt for restore
openssl enc -d -aes-256-cbc \
    -in backup.sql.gz.enc \
    -out backup.sql.gz \
    -k "$BACKUP_ENCRYPTION_KEY"
```

#### Access Controls

- Backups stored in encrypted cloud storage
- Access limited to authorized personnel
- Audit logging for backup access
- Regular key rotation

### Backup Testing

#### Quarterly Testing

```bash
# Test database restore
bash scripts/test-backup-restore.sh

# Test configuration restore
bash scripts/test-config-restore.sh

# Test full system restore
bash scripts/test-full-restore.sh
```

#### Annual Testing

- Complete disaster recovery simulation
- Third-party audit of backup procedures
- Update recovery procedures based on lessons learned

This operations guide ensures Ferrumyx systems remain reliable, secure, and recoverable. Regular monitoring, maintenance, and backup procedures are essential for production deployments.</content>
<parameter name="filePath">D:\AI\Ferrumyx\OPERATIONS_GUIDE.md