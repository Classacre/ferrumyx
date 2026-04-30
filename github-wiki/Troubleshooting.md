# Troubleshooting

This comprehensive troubleshooting guide provides diagnostic procedures, common issues, and resolution steps for Ferrumyx v2.0.0 deployments. It covers application errors, database issues, performance problems, and security incidents with structured diagnostic approaches.

## Table of Contents

- [Quick Diagnosis](#quick-diagnosis)
- [Error Codes & Logs](#error-codes--logs)
- [Service Issues](#service-issues)
- [Database Issues](#database-issues)
- [Application Issues](#application-issues)
- [Performance Issues](#performance-issues)
- [Security Issues](#security-issues)
- [Network Issues](#network-issues)
- [Build & Deployment Issues](#build--deployment-issues)
- [Getting Help](#getting-help)

## Quick Diagnosis

### Health Check Script

Run comprehensive system health checks:

```bash
# Detailed health assessment
bash scripts/health-check.sh --detailed

# Quick status check
curl -f http://localhost:3000/health
```

### System Status Check

```bash
# Check all services
docker-compose ps

# Monitor resource usage
docker stats --no-stream

# Review recent errors
docker-compose logs --tail=50 | grep -i error

# Test basic connectivity
curl -f http://localhost:3000/health
```

### Log Analysis

```bash
# Recent application logs
docker-compose logs --tail=100 ferrumyx-web

# Database connection logs
docker-compose logs postgres | tail -50

# System event logs
sudo journalctl -u docker.service --since "1 hour ago"
```

## Error Codes & Logs

### Common Error Codes

Ferrumyx uses structured error codes for consistent troubleshooting:

#### Application Errors (FERR-APP-*)

| Error Code | Description | Symptoms | Resolution |
|------------|-------------|----------|------------|
| `FERR-APP-001` | Configuration validation failed | Service won't start, config errors in logs | Check `.env` file syntax and required variables |
| `FERR-APP-002` | Database connection failed | "Connection refused" errors | Verify PostgreSQL is running and credentials are correct |
| `FERR-APP-003` | LLM provider unavailable | Chat requests fail with 503 | Check API keys and network connectivity to LLM provider |
| `FERR-APP-004` | IronClaw agent timeout | Requests hang for >5 minutes | Check agent resource usage and increase timeouts if needed |
| `FERR-APP-005` | WASM sandbox error | Tool execution fails with sandbox errors | Verify WASM modules are properly compiled and loaded |
| `FERR-APP-006` | Memory limit exceeded | OOM kills, service restarts | Increase Docker memory limits or optimize memory usage |

#### Database Errors (FERR-DB-*)

| Error Code | Description | Symptoms | Resolution |
|------------|-------------|----------|------------|
| `FERR-DB-001` | Connection pool exhausted | "Pool exhausted" errors | Increase pool size or optimize query performance |
| `FERR-DB-002` | Deadlock detected | Transaction failures | Review transaction isolation and locking patterns |
| `FERR-DB-003` | pgvector extension missing | Vector operations fail | Install pgvector extension: `CREATE EXTENSION vector;` |
| `FERR-DB-004` | Disk space exhausted | "No space left" errors | Free up disk space or increase storage allocation |
| `FERR-DB-005` | Index corruption | Query performance degrades | Rebuild indexes: `REINDEX TABLE CONCURRENTLY table_name;` |

#### Ingestion Errors (FERR-ING-*)

| Error Code | Description | Symptoms | Resolution |
|------------|-------------|----------|------------|
| `FERR-ING-001` | Source API rate limited | "Rate limit exceeded" in ingestion logs | Reduce ingestion concurrency or increase delays between requests |
| `FERR-ING-002` | PDF parsing failed | "Parse error" for full-text processing | Check PDF quality or enable fallback parsing modes |
| `FERR-ING-003` | Embedding service timeout | Embedding generation stalls | Switch to local embedding model or increase timeouts |
| `FERR-ING-004` | Duplicate detection failed | Same papers ingested multiple times | Check fuzzy matching configuration and database indexes |
| `FERR-ING-005` | Sci-Hub access blocked | Full-text acquisition fails | Configure alternative full-text sources or reduce Sci-Hub usage |

#### Security Errors (FERR-SEC-*)

| Error Code | Description | Symptoms | Resolution |
|------------|-------------|----------|------------|
| `FERR-SEC-001` | Invalid API key | 401 Unauthorized responses | Verify API key format and permissions |
| `FERR-SEC-002` | PHI data exposure attempt | Requests blocked with security warnings | Review data classification and access controls |
| `FERR-SEC-003` | Audit log write failed | Security events not logged | Check audit log storage and permissions |
| `FERR-SEC-004` | Encryption key rotation needed | Encryption operations fail | Rotate encryption keys following security procedures |
| `FERR-SEC-005` | WASM sandbox breach attempt | Suspicious tool execution blocked | Review WASM module security and update sandbox rules |

### Log Analysis Guide

#### Log Levels
- **ERROR**: System errors requiring immediate attention
- **WARN**: Potential issues or degraded performance
- **INFO**: Normal operations and state changes
- **DEBUG**: Detailed debugging information
- **TRACE**: Very detailed execution traces

#### Structured Log Format
```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "level": "ERROR",
  "service": "ferrumyx-web",
  "error_code": "FERR-APP-002",
  "message": "Database connection failed",
  "context": {
    "user_id": "uuid",
    "request_id": "req-123",
    "endpoint": "/api/search"
  },
  "stack_trace": "...",
  "metadata": {
    "version": "2.0.0",
    "environment": "production"
  }
}
```

#### Common Log Patterns

**Database Connection Issues:**
```
ERROR Database connection failed: FERR-DB-001
  context: { host: "postgres", port: 5432, database: "ferrumyx" }
  cause: "Connection refused (os error 61)"
```

**Ingestion Timeouts:**
```
WARN Ingestion job timeout: FERR-ING-003
  context: { job_id: "job-123", duration_seconds: 1800, max_duration: 1200 }
  message: "Job exceeded maximum duration, terminating"
```

**Security Events:**
```
WARN PHI data exposure attempt blocked: FERR-SEC-002
  context: { user_id: "user-456", ip: "192.168.1.100", endpoint: "/api/search" }
  message: "Query contained potential PHI data, request blocked"
```

#### Diagnostic Commands

```bash
# Search for specific error codes
docker-compose logs ferrumyx-web | grep "FERR-APP-001"

# Find errors in time range
docker-compose logs --since "1 hour ago" | grep ERROR

# Count errors by type
docker-compose logs | grep '"level":"ERROR"' | jq -r '.error_code' | sort | uniq -c

# Monitor real-time errors
docker-compose logs -f | grep --line-buffered ERROR
```

## Service Issues

### Service Won't Start

**Symptoms:**
- `docker-compose ps` shows service as not running
- Exit codes in logs

**Diagnosis:**
```bash
# Check service logs
docker-compose logs <service-name>

# Check service configuration
docker-compose config

# Test service manually
docker-compose run --rm <service-name> <command>
```

**Common Solutions:**

1. **Port conflicts:**
   ```bash
   # Find process using port
   lsof -i :3000

   # Change port in docker-compose.yml
   sed -i 's/3000:3000/3001:3000/' docker-compose.yml
   ```

2. **Resource limits:**
   ```bash
   # Check available resources
   free -h
   df -h

   # Increase Docker memory limit
   # Edit /etc/docker/daemon.json
   {
     "default-ulimits": {
       "nofile": {
         "Name": "nofile",
         "Hard": 64000,
         "Soft": 64000
       }
     }
   }
   ```

3. **Dependency issues:**
   ```bash
   # Check service dependencies
   docker-compose up -d postgres redis
   docker-compose up -d ferrumyx-web
   ```

### Service Crashing

**Symptoms:**
- Service restarts repeatedly
- Out of memory errors

**Diagnosis:**
```bash
# Check crash logs
docker-compose logs --tail=200 ferrumyx-web | grep -A 10 -B 10 "panic\|error"

# Check resource usage
docker stats ferrumyx-web

# Check system limits
ulimit -a
```

**Solutions:**

1. **Memory issues:**
   ```yaml
   # docker-compose.yml
   services:
     ferrumyx-web:
       deploy:
         resources:
           limits:
             memory: 2G
           reservations:
             memory: 1G
   ```

2. **Configuration errors:**
   ```bash
   # Validate environment
   docker-compose exec ferrumyx-web env | grep -E "(DATABASE|REDIS|API_KEY)"

   # Check configuration file
   docker-compose exec ferrumyx-web cat config.toml
   ```

### Service Unhealthy

**Symptoms:**
- Health checks failing
- Service marked as unhealthy

**Diagnosis:**
```bash
# Check health endpoint
curl -v http://localhost:3000/health

# Check health check configuration
docker-compose ps
docker inspect <container-id> | jq .State.Health
```

**Solutions:**
```bash
# Fix health check
# docker-compose.yml
services:
  ferrumyx-web:
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
```

## Database Issues

### Connection Failures

**Symptoms:**
- "Connection refused" errors
- Database connection timeouts

**Diagnosis:**
```bash
# Test database connectivity
docker-compose exec postgres pg_isready -U ferrumyx

# Check database logs
docker-compose logs postgres | tail -50

# Test from application
docker-compose exec ferrumyx-web curl -f http://postgres:5432
```

**Solutions:**

1. **Database not started:**
   ```bash
   docker-compose up -d postgres
   docker-compose logs postgres
   ```

2. **Authentication issues:**
   ```bash
   # Check credentials
   grep DATABASE_URL .env

   # Test connection manually
   docker-compose exec postgres psql -U ferrumyx -d ferrumyx -c "SELECT 1;"
   ```

3. **Network issues:**
   ```bash
   # Check Docker network
   docker network inspect ferrumyx-network

   # Restart network
   docker-compose down
   docker-compose up -d postgres
   ```

### Database Corruption

**Symptoms:**
- "Database disk image is malformed" errors
- Inconsistent data

**Diagnosis:**
```bash
# Check database integrity
docker-compose exec postgres psql -U postgres -d ferrumyx -c "PRAGMA integrity_check;"

# Check disk space
df -h /var/lib/docker

# Check for corruption logs
docker-compose logs postgres | grep -i corrupt
```

**Solutions:**

1. **Restore from backup:**
   ```bash
   # List available backups
   ls -la backups/

   # Restore latest backup
   bash scripts/db-restore.sh --backup-id $(ls backups/ | grep backup- | tail -1)
   ```

2. **Repair database:**
   ```bash
   # For PostgreSQL
   docker-compose exec postgres psql -U postgres -d ferrumyx -c "REINDEX DATABASE ferrumyx;"
   docker-compose exec postgres psql -U postgres -d ferrumyx -c "VACUUM FULL;"
   ```

### Migration Failures

**Symptoms:**
- Migration scripts fail
- Schema inconsistencies

**Diagnosis:**
```bash
# Check migration status
docker-compose exec postgres psql -U postgres -d ferrumyx -c "SELECT * FROM schema_migrations ORDER BY version DESC LIMIT 5;"

# Check migration logs
bash scripts/db-migrate.sh 2>&1 | tee migration.log
```

**Solutions:**

1. **Failed migration:**
   ```bash
   # Check migration file
   cat migrations/$(ls migrations/ | tail -1)

   # Manual migration
   docker-compose exec postgres psql -U postgres -d ferrumyx -f migrations/<failed-migration>.sql
   ```

2. **Rollback migration:**
   ```bash
   # Identify failed migration
   # Manually reverse changes or restore from backup
   ```

## Application Issues

### Application Errors

**Symptoms:**
- 500 Internal Server Error
- Application crashes
- Feature not working

**Diagnosis:**
```bash
# Check application logs
docker-compose logs --tail=100 ferrumyx-web

# Check error details
docker-compose logs ferrumyx-web | grep -A 5 -B 5 "ERROR\|panic"

# Test API endpoints
curl -v http://localhost:3000/api/v1/health
```

**Solutions:**

1. **Configuration issues:**
   ```bash
   # Validate environment
   docker-compose exec ferrumyx-web env | grep -E "(DATABASE|REDIS|API_KEY)"

   # Check configuration file
   docker-compose exec ferrumyx-web cat config.toml
   ```

2. **Dependency issues:**
   ```bash
   # Check IronClaw connectivity
   curl -f https://api.ironclaw.ai/health

   # Test BioClaw tools
   docker-compose exec ferrumyx-web ls /opt/ferrumyx/wasm/
   ```

### Performance Degradation

**Symptoms:**
- Slow response times
- High CPU/memory usage
- Timeout errors

**Diagnosis:**
```bash
# Check application metrics
curl http://localhost:3000/metrics

# Profile application
docker stats ferrumyx-web

# Check database performance
docker-compose exec postgres psql -U postgres -d ferrumyx -c "SELECT * FROM pg_stat_activity WHERE state = 'active';"
```

**Solutions:**

1. **Optimize database queries:**
   ```sql
   -- Add indexes
   CREATE INDEX CONCURRENTLY idx_conversations_user_id ON conversations(user_id);

   -- Analyze query performance
   EXPLAIN ANALYZE SELECT * FROM conversations WHERE user_id = '123';
   ```

2. **Increase resources:**
   ```yaml
   services:
     ferrumyx-web:
       deploy:
         resources:
           limits:
             cpus: '2.0'
             memory: 4G
   ```

3. **Enable caching:**
   ```bash
   # Configure Redis cache
   export REDIS_CACHE_TTL=3600
   export FERRUMYX_CACHE_ENABLED=true
   ```

## Performance Issues

### High Resource Usage

**Symptoms:**
- High CPU usage
- Memory leaks
- Disk I/O bottlenecks

**Diagnosis:**
```bash
# Monitor resources
docker stats

# Check system load
uptime
top -b -n1 | head -20

# Profile application
cargo flamegraph --bin ferrumyx-web --features postgres
```

**Solutions:**

1. **Memory optimization:**
   ```bash
   # Check for memory leaks
   valgrind --tool=massif ./target/release/ferrumyx-web

   # Optimize Rust code
   cargo build --release --features optimize
   ```

2. **CPU optimization:**
   ```bash
   # Profile CPU usage
   perf record -F 99 -p $(pidof ferrumyx-web) -g -- sleep 60
   perf report
   ```

3. **Database optimization:**
   ```sql
   -- Optimize queries
   CREATE INDEX CONCURRENTLY idx_messages_timestamp ON messages(created_at);

   -- Update statistics
   ANALYZE VERBOSE;
   ```

### Slow Queries

**Symptoms:**
- Database queries taking too long
- Application timeouts

**Diagnosis:**
```bash
# Find slow queries
docker-compose exec postgres psql -U postgres -d ferrumyx -c "
SELECT pid, now() - pg_stat_activity.query_start AS duration, query
FROM pg_stat_activity
WHERE state = 'active' AND now() - pg_stat_activity.query_start > interval '30 seconds'
ORDER BY duration DESC;"

# Check query plans
EXPLAIN ANALYZE SELECT * FROM large_table WHERE condition = 'value';
```

**Solutions:**

1. **Add indexes:**
   ```sql
   CREATE INDEX CONCURRENTLY idx_table_column ON table_name(column_name);
   ```

2. **Optimize queries:**
   ```sql
   -- Use pagination
   SELECT * FROM table_name LIMIT 100 OFFSET 0;

   -- Use proper joins
   SELECT t1.*, t2.name FROM table1 t1 JOIN table2 t2 ON t1.id = t2.table1_id;
   ```

3. **Database tuning:**
   ```sql
   ALTER SYSTEM SET work_mem = '64MB';
   ALTER SYSTEM SET shared_buffers = '512MB';
   ```

## Security Issues

### Authentication Failures

**Symptoms:**
- Login failures
- API authentication errors

**Diagnosis:**
```bash
# Check IronClaw API key
grep IRONCLAW_API_KEY .env

# Test API connectivity
curl -H "Authorization: Bearer $IRONCLAW_API_KEY" https://api.ironclaw.ai/v1/models

# Check application auth logs
docker-compose logs ferrumyx-web | grep -i auth
```

**Solutions:**

1. **Update API key:**
   ```bash
   # Edit .env file
   nano .env
   # Update IRONCLAW_API_KEY

   # Restart services
   docker-compose restart ferrumyx-web
   ```

2. **Check token expiration:**
   ```bash
   # Verify token validity
   curl -H "Authorization: Bearer $IRONCLAW_API_KEY" https://api.ironclaw.ai/v1/me
   ```

### Permission Issues

**Symptoms:**
- Access denied errors
- File permission errors

**Diagnosis:**
```bash
# Check file permissions
ls -la /opt/ferrumyx/

# Check Docker user
docker-compose exec ferrumyx-web whoami
docker-compose exec ferrumyx-web id

# Check volume permissions
docker volume inspect ferrumyx_postgres_data
```

**Solutions:**

1. **Fix file permissions:**
   ```bash
   sudo chown -R 1000:1000 /opt/ferrumyx/
   sudo chmod -R 755 /opt/ferrumyx/
   ```

2. **Fix Docker permissions:**
   ```bash
   # Add user to docker group
   sudo usermod -aG docker $USER
   # Logout and login again
   ```

### SSL/TLS Issues

**Symptoms:**
- Certificate errors
- HTTPS connection failures

**Diagnosis:**
```bash
# Check certificate
openssl x509 -in /etc/ssl/ferrumyx/cert.pem -text -noout

# Test SSL connection
openssl s_client -connect localhost:443 -servername yourdomain.com

# Check certificate expiration
openssl x509 -in /etc/ssl/ferrumyx/cert.pem -enddate -noout
```

**Solutions:**

1. **Renew certificates:**
   ```bash
   # Using Let's Encrypt
   sudo certbot renew

   # Manual renewal
   # Obtain new certificate and update configuration
   ```

2. **Fix certificate chain:**
   ```bash
   # Combine certificate and chain
   cat cert.pem intermediate.pem > fullchain.pem
   ```

## Network Issues

### Connectivity Problems

**Symptoms:**
- Services can't communicate
- External API failures
- DNS resolution issues

**Diagnosis:**
```bash
# Test network connectivity
ping 8.8.8.8

# Test DNS resolution
nslookup api.ironclaw.ai

# Check Docker networks
docker network ls
docker network inspect ferrumyx-network

# Test service communication
docker-compose exec ferrumyx-web curl -f http://postgres:5432
```

**Solutions:**

1. **Network configuration:**
   ```bash
   # Restart Docker network
   docker-compose down
   docker network rm ferrumyx-network
   docker-compose up -d
   ```

2. **DNS issues:**
   ```bash
   # Add to /etc/resolv.conf
   nameserver 8.8.8.8
   nameserver 1.1.1.1
   ```

3. **Firewall issues:**
   ```bash
   # Check firewall rules
   sudo ufw status

   # Allow necessary ports
   sudo ufw allow 3000/tcp
   sudo ufw allow 5432/tcp
   ```

### Load Balancer Issues

**Symptoms:**
- Uneven load distribution
- Health check failures

**Diagnosis:**
```bash
# Check load balancer configuration
cat /etc/nginx/nginx.conf

# Test upstream servers
curl -f http://localhost:3000/health
curl -f http://localhost:3001/health  # If multiple instances

# Check load balancer logs
sudo tail -f /var/log/nginx/error.log
```

**Solutions:**

1. **Update upstream configuration:**
   ```nginx
   upstream ferrumyx_backend {
       least_conn;
       server localhost:3000 max_fails=3 fail_timeout=30s;
       server localhost:3001 max_fails=3 fail_timeout=30s;
   }
   ```

2. **Fix health checks:**
   ```nginx
   location /health {
       proxy_pass http://ferrumyx_backend;
       proxy_connect_timeout 5s;
       proxy_read_timeout 10s;
   }
   ```

## Build & Deployment Issues

### Build Failures

**Symptoms:**
- Docker build fails
- Compilation errors

**Diagnosis:**
```bash
# Check build logs
docker-compose build --no-cache 2>&1 | tee build.log

# Check dependencies
cargo check --features postgres,libsql

# Check disk space
df -h
```

**Solutions:**

1. **Dependency issues:**
   ```bash
   # Clear cache
   cargo clean
   rm -rf node_modules

   # Rebuild
   cargo build --features postgres,libsql
   npm install
   ```

2. **Docker issues:**
   ```bash
   # Clean Docker
   docker system prune -a

   # Build step by step
   docker build --target builder -t ferrumyx-builder .
   docker build -t ferrumyx-web .
   ```

### Deployment Failures

**Symptoms:**
- Deployment scripts fail
- Rollback occurs

**Diagnosis:**
```bash
# Check deployment logs
bash scripts/deploy.sh 2>&1 | tee deploy.log

# Check service health
bash scripts/health-check.sh --detailed

# Check resource availability
free -h
df -h
```

**Solutions:**

1. **Pre-deployment checks:**
   ```bash
   # Validate configuration
   docker-compose config

   # Test build
   docker-compose build --parallel

   # Check dependencies
   docker-compose pull
   ```

2. **Rollback issues:**
   ```bash
   # Manual rollback
   docker-compose down
   docker-compose -f docker-compose.backup.yml up -d

   # Check rollback logs
   docker-compose logs | grep -i error
   ```

## Getting Help

### Documentation Resources

- **[README.md](README.md)** - Main documentation
- **[DEPLOYMENT.md](DEPLOYMENT.md)** - Deployment guide
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - System architecture
- **[OPERATIONS_GUIDE.md](OPERATIONS_GUIDE.md)** - Operations procedures
- **[User Guides](User-Guides)** - Usage instructions

### Log Collection

For support requests, collect these logs:

```bash
# System information
uname -a
docker --version
docker-compose --version

# Service status
docker-compose ps
docker stats --no-stream

# Application logs
docker-compose logs --tail=500 ferrumyx-web > ferrumyx-web.log
docker-compose logs --tail=500 postgres > postgres.log

# Configuration
docker-compose config > compose-config.yml
cat .env | grep -v PASSWORD > env-safe.txt

# Create support bundle
tar -czf ferrumyx-support-$(date +%Y%m%d-%H%M%S).tar.gz \
  ferrumyx-web.log postgres.log compose-config.yml env-safe.txt
```

### Community Support

1. **GitHub Issues**: Create detailed issue with logs
2. **GitHub Discussions**: Ask questions and share solutions
3. **Discord**: Real-time community chat (#support, #troubleshooting)

### Professional Support

For enterprise deployments:

- Review maintenance agreements
- Contact support team
- Schedule consultation
- Access premium documentation

### Emergency Contacts

For critical production issues:

- On-call engineer: Available 24/7
- Security incidents: security@ferrumyx.org
- System status: https://status.ferrumyx.org

Remember: When in doubt, restore from backup and escalate to the appropriate team. Most issues can be resolved by checking logs, verifying configuration, and following the diagnostic procedures outlined above.