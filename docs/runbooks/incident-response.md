# Ferrumyx Production Operations Runbook

**Version:** 1.0  
**Effective Date:** 2026-04-30  
**Review Date:** 2026-07-30  

## Table of Contents

1. [System Overview](#system-overview)
2. [Daily Operations](#daily-operations)
3. [Weekly Operations](#weekly-operations)
4. [Monthly Operations](#monthly-operations)
5. [Emergency Procedures](#emergency-procedures)
6. [Deployment Procedures](#deployment-procedures)
7. [Backup and Recovery](#backup-and-recovery)
8. [Monitoring and Alerting](#monitoring-and-alerting)
9. [Security Operations](#security-operations)
10. [Performance Optimization](#performance-optimization)

## System Overview

### Architecture
Ferrumyx is a bioinformatics platform with the following components:
- **Web Interface**: Main user interface (Port 3000)
- **IronClaw Agent**: AI agent orchestration service
- **BioClaw WASM**: Computational biology tools
- **PostgreSQL**: Primary database
- **Redis**: Caching and session storage
- **NGINX**: Reverse proxy and load balancer
- **Monitoring Stack**: Prometheus, Grafana, Loki, AlertManager

### Key Contacts
- **Primary On-Call**: Systems Administrator
- **Security Lead**: Security Officer
- **Development Lead**: Lead Developer
- **Emergency Contact**: 24/7 Operations Center

### Access Information
- **Production Servers**: [Server IPs/Hostnames]
- **Monitoring Dashboard**: http://localhost:3001 (Grafana)
- **Metrics Endpoint**: http://localhost:9090 (Prometheus)
- **Logs Dashboard**: http://localhost:3100 (Loki)

## Daily Operations

### Morning Health Check (8:00 AM)

```bash
# Run comprehensive health check
bash scripts/health-check.sh --detailed

# Check monitoring dashboards
# 1. Grafana: Verify all services are green
# 2. Prometheus: Check targets are up
# 3. AlertManager: Review any active alerts

# Verify backup completion
ls -la backups/ | tail -5
```

**Success Criteria:**
- All services reporting healthy
- No critical alerts active
- Backup from previous night completed successfully

### Log Review (9:00 AM)

```bash
# Review application logs for errors
docker-compose logs --since 24h ferrumyx-web | grep -i error

# Review security events
docker-compose logs --since 24h | grep -i "auth\|security\|phi"

# Check for unusual patterns
docker-compose logs --since 24h | grep -c "error\|warn\|fail"
```

**Action Items:**
- Investigate any ERROR or CRITICAL log entries
- Review security events for anomalies
- Document any unusual patterns

### Resource Monitoring (10:00 AM)

```bash
# Check system resources
docker stats --no-stream

# Verify database connections
docker-compose exec postgres psql -U postgres -d ferrumyx -c "SELECT count(*) FROM pg_stat_activity;"

# Check Redis memory usage
docker-compose exec redis redis-cli info memory
```

**Thresholds:**
- CPU Usage: < 80%
- Memory Usage: < 85%
- Database Connections: < 50
- Redis Memory: < 256MB

### End-of-Day Summary (5:00 PM)

```bash
# Generate daily operations report
bash scripts/generate-daily-report.sh

# Verify all services still operational
curl -f http://localhost:3000/health

# Check for any new alerts
curl -s http://localhost:9093/api/v2/alerts | jq '.[] | select(.status.state == "firing")'
```

## Weekly Operations

### Monday: System Maintenance

```bash
# Update Docker images
docker-compose pull

# Rotate application logs
docker-compose exec loki logrotate /etc/logrotate.d/ferrumyx

# Clean up unused Docker resources
docker system prune -f

# Verify system integrity
bash scripts/system-integrity-check.sh
```

### Tuesday: Database Maintenance

```bash
# Run database vacuum and analyze
docker-compose exec postgres vacuumdb --all --analyze

# Check for long-running queries
docker-compose exec postgres psql -U postgres -d ferrumyx -c "
SELECT pid, now() - pg_stat_activity.query_start AS duration, query
FROM pg_stat_activity
WHERE state = 'active' AND now() - pg_stat_activity.query_start > interval '5 minutes'
ORDER BY duration DESC;"

# Update database statistics
docker-compose exec postgres psql -U postgres -d ferrumyx -c "ANALYZE;"
```

### Wednesday: Security Review

```bash
# Run security scan
bash scripts/security-scan.sh

# Review user access logs
docker-compose logs --since 7d | grep -i "login\|auth"

# Check for security updates
bash scripts/check-security-updates.sh

# Review audit logs
docker-compose logs --since 7d | grep -i "audit\|security"
```

### Thursday: Performance Analysis

```bash
# Analyze slow queries
docker-compose exec postgres psql -U postgres -d ferrumyx -c "
SELECT query, calls, total_time/calls as avg_time, rows
FROM pg_stat_statements
ORDER BY avg_time DESC
LIMIT 10;"

# Check application performance metrics
curl http://localhost:9090/api/v1/query?query=histogram_quantile(0.95,%20rate(http_request_duration_seconds_bucket[7d]))

# Review resource usage trends
# Access Grafana dashboard for weekly trends
```

### Friday: Backup Verification

```bash
# Test backup restoration (dry run)
bash scripts/db-restore.sh --backup-id $(ls backups/ | tail -1) --dry-run

# Verify backup integrity
bash scripts/backup-integrity-check.sh

# Check backup storage usage
du -sh backups/

# Generate weekly backup report
bash scripts/backup-report.sh --weekly
```

## Monthly Operations

### First Monday: Full System Audit

```bash
# Complete system audit
bash scripts/system-audit.sh

# Security compliance check
bash scripts/compliance-check.sh

# Performance benchmark
bash scripts/performance-benchmark.sh

# Generate audit report
bash scripts/generate-audit-report.sh
```

### Second Monday: Capacity Planning

```bash
# Analyze growth trends
bash scripts/capacity-analysis.sh

# Review resource utilization
bash scripts/resource-usage-report.sh --monthly

# Plan for upcoming capacity needs
# Review database growth projections
# Assess compute resource requirements
```

### Third Monday: Documentation Update

```bash
# Update system documentation
bash scripts/update-documentation.sh

# Review and update runbooks
# Verify contact information is current
# Update emergency procedures if needed
```

### Last Friday: Disaster Recovery Test

```bash
# Schedule disaster recovery drill
# Test backup restoration procedures
bash scripts/disaster-recovery-test.sh

# Verify failover procedures
# Document test results and improvements
```

## Emergency Procedures

### Critical System Down (Severity: Critical)

**Immediate Actions:**
```bash
# Assess the situation
bash scripts/health-check.sh --fail-fast

# Notify stakeholders
# Escalate to emergency response team

# Attempt automated recovery
docker-compose restart

# If automated recovery fails, initiate manual recovery
bash scripts/emergency-restart.sh
```

**Recovery Steps:**
1. Identify root cause
2. Execute appropriate recovery procedure
3. Verify system stability
4. Document incident
5. Conduct post-mortem analysis

### Database Failure (Severity: Critical)

**Immediate Actions:**
```bash
# Check database status
docker-compose exec postgres pg_isready -U postgres

# If database is down, attempt restart
docker-compose restart postgres

# If restart fails, initiate recovery
bash scripts/db-restore.sh --backup-id latest
```

**Recovery Steps:**
1. Stop application services
2. Restore from latest backup
3. Verify data integrity
4. Restart application services
5. Run comprehensive health checks

### Security Incident (Severity: Critical)

**Immediate Actions:**
```bash
# Isolate affected systems
bash scripts/security-incident-response.sh

# Preserve evidence
bash scripts/collect-forensic-data.sh

# Notify security team and legal
# Assess impact and scope
```

**Response Steps:**
1. Contain the incident
2. Investigate root cause
3. Remediate vulnerabilities
4. Restore normal operations
5. Conduct post-incident review

### Performance Degradation (Severity: High)

**Immediate Actions:**
```bash
# Identify bottleneck
bash scripts/performance-diagnostic.sh

# Scale resources if needed
docker-compose up -d --scale ferrumyx-web=3

# Optimize problematic queries
bash scripts/query-optimization.sh
```

**Recovery Steps:**
1. Implement temporary fixes
2. Investigate root cause
3. Apply permanent fixes
4. Monitor for recurrence

## Deployment Procedures

### Standard Deployment

```bash
# Pre-deployment validation
bash scripts/pre-deployment-check.sh

# Create deployment backup
bash scripts/create-deployment-backup.sh

# Execute deployment
bash scripts/deploy.sh

# Post-deployment validation
bash scripts/post-deployment-check.sh

# Update documentation
bash scripts/update-deployment-docs.sh
```

### Rollback Procedure

```bash
# Identify rollback point
# Usually the backup created before deployment

# Execute rollback
bash scripts/deploy.sh --rollback <backup-id>

# Verify rollback success
bash scripts/health-check.sh

# Document rollback reason
bash scripts/document-rollback.sh
```

### Blue-Green Deployment

```bash
# Deploy to blue environment
export COMPOSE_FILE=docker-compose.blue.yml
docker-compose up -d

# Test blue environment
bash scripts/test-blue-environment.sh

# Switch traffic to blue
bash scripts/switch-to-blue.sh

# Keep green as rollback option
# Monitor for 24 hours before decommissioning green
```

## Backup and Recovery

### Backup Schedule
- **Daily**: Database and configuration backups at 2:00 AM
- **Weekly**: Full system backup every Sunday at 3:00 AM
- **Monthly**: Long-term archive backup on first day of month

### Backup Verification
```bash
# Daily verification
bash scripts/daily-backup-check.sh

# Weekly comprehensive test
bash scripts/weekly-backup-test.sh

# Monthly disaster recovery test
bash scripts/monthly-dr-test.sh
```

### Recovery Procedures

#### Database Recovery
```bash
# Stop application services
docker-compose stop ferrumyx-web ironclaw-agent

# Restore database
bash scripts/db-restore.sh --backup-id <backup-id>

# Verify data integrity
bash scripts/verify-data-integrity.sh

# Restart services
docker-compose start ferrumyx-web ironclaw-agent
```

#### Full System Recovery
```bash
# Restore from full backup
bash scripts/full-system-restore.sh --backup-id <backup-id>

# Verify all components
bash scripts/comprehensive-health-check.sh

# Update monitoring
bash scripts/update-monitoring-post-restore.sh
```

## Monitoring and Alerting

### Alert Categories

#### Critical Alerts (Immediate Response Required)
- Database down
- Application unresponsive
- Security breach detected
- Data corruption detected

#### Warning Alerts (Response Within 1 Hour)
- High resource usage
- Slow response times
- Failed backup
- Security policy violation

#### Info Alerts (Monitor and Document)
- Service restart
- Configuration change
- User account changes
- Performance degradation

### Alert Response Procedures

#### Database Down Alert
1. Check database logs
2. Attempt database restart
3. If restart fails, initiate failover
4. Notify development team
5. Document incident

#### High CPU Usage Alert
1. Identify consuming process
2. Check for runaway queries
3. Scale resources if needed
4. Optimize problematic code
5. Monitor for resolution

#### Security Alert
1. Assess threat level
2. Isolate affected systems
3. Investigate incident
4. Remediate vulnerability
5. Report to security team

## Security Operations

### Access Control
- **Role-Based Access**: Admin, User, Read-Only roles
- **Multi-Factor Authentication**: Required for admin access
- **Session Management**: Automatic logout after inactivity
- **Audit Logging**: All access attempts logged

### Security Monitoring
```bash
# Daily security review
bash scripts/daily-security-review.sh

# Check for suspicious activity
bash scripts/anomaly-detection.sh

# Review access logs
bash scripts/access-log-review.sh
```

### Incident Response
1. **Detection**: Automated alerts and monitoring
2. **Assessment**: Evaluate impact and scope
3. **Containment**: Isolate affected systems
4. **Investigation**: Determine root cause
5. **Recovery**: Restore normal operations
6. **Lessons Learned**: Document and improve

## Performance Optimization

### Regular Maintenance
```bash
# Database optimization
bash scripts/db-optimization.sh

# Cache cleanup
bash scripts/cache-maintenance.sh

# Index maintenance
bash scripts/index-optimization.sh
```

### Performance Monitoring
- **Response Time**: Target <5 seconds for 95th percentile
- **Throughput**: Monitor requests per second
- **Resource Usage**: CPU <80%, Memory <85%
- **Database Performance**: Query optimization and indexing

### Scaling Procedures
```bash
# Horizontal scaling
docker-compose up -d --scale ferrumyx-web=<count>

# Vertical scaling
# Update resource limits in docker-compose.prod.yml
docker-compose up -d

# Database scaling
# Implement read replicas for read-heavy workloads
```

---

## Contact Information

**Primary Operations Team**
- Email: ops@ferrumyx.org
- Phone: [Emergency Contact Number]
- Slack: #ferrumyx-ops

**Security Team**
- Email: security@ferrumyx.org
- Phone: [Security Emergency Number]
- Slack: #ferrumyx-security

**Development Team**
- Email: dev@ferrumyx.org
- Slack: #ferrumyx-dev

**Vendor Contacts**
- Docker Support: support@docker.com
- PostgreSQL Support: support@postgresql.org
- Redis Support: support@redis.com

---

**Document Owner:** Operations Team  
**Review Frequency:** Quarterly  
**Last Updated:** 2026-04-30</content>
<parameter name="filePath">D:\AI\Ferrumyx\PRODUCTION_OPERATIONS_RUNBOOK.md