# Production Deployment Runbook

## Overview
This runbook provides comprehensive procedures for deploying Ferrumyx to production environments.

## Prerequisites
- Production server with Docker and Docker Compose
- Domain name and SSL certificates
- Database backups available
- Monitoring and alerting configured
- SSH access to production server

## Pre-deployment Checklist

### Infrastructure Requirements
- [ ] Ubuntu 20.04+ or RHEL 8+ server
- [ ] Minimum 8GB RAM, 4 CPU cores
- [ ] 100GB SSD storage
- [ ] Docker 20.10+ and Docker Compose 2.0+
- [ ] SSL certificate (Let's Encrypt or commercial)

### Access and Security
- [ ] SSH key-based access configured
- [ ] Firewall rules for ports 22, 80, 443, 3000
- [ ] Database passwords set and secured
- [ ] API keys and secrets configured
- [ ] Backup storage accessible

### Application Configuration
- [ ] Environment variables configured
- [ ] Domain name and SSL certificates ready
- [ ] Monitoring endpoints configured
- [ ] Log aggregation setup

## Deployment Procedures

### Automated Deployment (Recommended)

#### Using Ferrumyx CLI (Recommended)
```bash
# On deployment server
cd /opt/ferrumyx

# Run complete production setup
ferrumyx-setup setup --environment production

# Or configuration only (then deploy manually)
ferrumyx-setup setup --environment production --config-only
```

#### Using GitHub Actions
1. Push to main branch or create release tag
2. GitHub Actions will automatically:
   - Build Docker images
   - Run tests and security scans
   - Deploy to staging
   - Deploy to production (after approval)

### Manual Deployment

#### Step 1: Prepare Deployment Server
```bash
# Create application directory
sudo mkdir -p /opt/ferrumyx
sudo chown $USER:$USER /opt/ferrumyx
cd /opt/ferrumyx

# Clone repository
git clone https://github.com/Classacre/ferrumyx.git .
git checkout main
```

#### Step 2: Configure Environment
```bash
# Copy production environment template
cp .env.example .env

# Edit production configuration
nano .env

# Required production variables:
DATABASE_URL=postgresql://ferrumyx:secure_password@postgres:5432/ferrumyx
REDIS_URL=redis://redis:secure_password@redis:6379
IRONCLAW_API_KEY=your-production-api-key
LOG_LEVEL=info
FERRUMYX_WEB_ADDR=0.0.0.0:3000
SSL_CERT_PATH=/path/to/cert.pem
SSL_KEY_PATH=/path/to/key.pem
```

#### Step 3: Setup SSL Certificates
```bash
# Using Let's Encrypt (recommended)
sudo apt install certbot
sudo certbot certonly --standalone -d yourdomain.com

# Or copy commercial certificates
sudo mkdir -p /etc/ssl/ferrumyx
sudo cp cert.pem /etc/ssl/ferrumyx/
sudo cp key.pem /etc/ssl/ferrumyx/
sudo chmod 600 /etc/ssl/ferrumyx/key.pem
```

#### Step 4: Configure Reverse Proxy (Nginx)
```bash
# Install Nginx
sudo apt install nginx

# Create site configuration
sudo nano /etc/nginx/sites-available/ferrumyx

# Add configuration:
server {
    listen 80;
    server_name yourdomain.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name yourdomain.com;

    ssl_certificate /etc/letsencrypt/live/yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/yourdomain.com/privkey.pem;

    # SSL configuration
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512:ECDHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;

    # Security headers
    add_header X-Frame-Options DENY;
    add_header X-Content-Type-Options nosniff;
    add_header X-XSS-Protection "1; mode=block";
    add_header Strict-Transport-Security "max-age=63072000; includeSubDomains; preload";

    # Proxy to Ferrumyx
    location / {
        proxy_pass http://localhost:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_cache_bypass $http_upgrade;
        proxy_read_timeout 86400;
    }

    # Health check endpoint
    location /health {
        proxy_pass http://localhost:3000/health;
        access_log off;
    }
}

# Enable site
sudo ln -s /etc/nginx/sites-available/ferrumyx /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

#### Step 5: Database Setup
```bash
# Start database services
docker-compose -f docker-compose.prod.yml up -d postgres redis

# Wait for database
sleep 30

# Create production database
docker-compose -f docker-compose.prod.yml exec postgres psql -U postgres -c "CREATE DATABASE ferrumyx;"

# Run migrations
bash scripts/db-migrate.sh
```

#### Step 6: Deploy Application
```bash
# Build and start all services
docker-compose -f docker-compose.prod.yml up -d --build

# Wait for deployment
sleep 60

# Check deployment status
docker-compose -f docker-compose.prod.yml ps
```

#### Step 7: Configure Monitoring
```bash
# Setup monitoring stack
bash scripts/monitoring-setup.sh

# Configure Grafana
# Access: http://yourdomain.com:3001
# Default credentials: admin/admin
```

#### Step 8: Post-deployment Verification
```bash
# Run health checks
bash scripts/health-check.sh

# Test application endpoints
curl -k https://yourdomain.com/health
curl -k https://yourdomain.com/api/v1/status

# Verify SSL
openssl s_client -connect yourdomain.com:443 -servername yourdomain.com
```

## Scaling Procedures

### Horizontal Scaling

#### Adding Application Instances
```bash
# Scale web service
docker-compose -f docker-compose.prod.yml up -d --scale ferrumyx-web=3

# Update Nginx for load balancing
upstream ferrumyx_backend {
    server localhost:3000;
    server localhost:3001;
    server localhost:3002;
}

# Reload Nginx
sudo nginx -s reload
```

#### Database Scaling
```bash
# Setup read replicas
# Edit docker-compose.prod.yml to add replica services

# Configure connection pooling
# Use PgBouncer for connection pooling
docker-compose -f docker-compose.prod.yml up -d pgbouncer
```

### Vertical Scaling

#### Increasing Resources
```bash
# Update Docker Compose resources
# Edit docker-compose.prod.yml:
services:
  ferrumyx-web:
    deploy:
      resources:
        limits:
          cpus: '2.0'
          memory: 4G
        reservations:
          cpus: '1.0'
          memory: 2G

# Restart with new resources
docker-compose -f docker-compose.prod.yml up -d
```

## Backup and Recovery

### Automated Backups
```bash
# Configure cron for automated backups
crontab -e

# Add backup schedule (daily at 2 AM)
0 2 * * * cd /opt/ferrumyx && bash scripts/db-backup.sh

# Add backup rotation (weekly)
0 3 * * 0 cd /opt/ferrumyx && find backups -name "backup-*" -mtime +7 -delete
```

### Manual Backup
```bash
# Full system backup
bash scripts/db-backup.sh

# Configuration backup
tar -czf backup-config-$(date +%Y%m%d).tar.gz .env docker-compose.prod.yml nginx.conf

# Docker volume backup
docker run --rm -v ferrumyx_postgres_data:/data -v $(pwd):/backup alpine tar czf /backup/postgres-data.tar.gz -C /data .
```

### Recovery Procedures

#### Database Recovery
```bash
# Stop application
docker-compose -f docker-compose.prod.yml stop ferrumyx-web ironclaw-agent

# Restore database
bash scripts/db-restore.sh --backup-id backup-20240101-020000-abc123

# Restart application
docker-compose -f docker-compose.prod.yml start ferrumyx-web ironclaw-agent
```

#### Full System Recovery
```bash
# Stop all services
docker-compose -f docker-compose.prod.yml down

# Restore volumes from backup
docker run --rm -v ferrumyx_postgres_data:/data -v $(pwd):/backup alpine sh -c "cd /data && tar xzf /backup/postgres-data.tar.gz"

# Restore configuration
tar -xzf backup-config-20240101.tar.gz

# Start services
docker-compose -f docker-compose.prod.yml up -d
```

## Monitoring and Alerting

### Key Metrics to Monitor
- Application response times
- Database connection pool usage
- Error rates and logs
- Resource utilization (CPU, memory, disk)
- SSL certificate expiration

### Alert Configuration
```yaml
# prometheus/alert_rules.yml
groups:
  - name: ferrumyx
    rules:
      - alert: HighErrorRate
        expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.1
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High error rate detected"

      - alert: DatabaseDown
        expr: pg_up == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Database is down"
```

### Log Aggregation
```yaml
# promtail-config.yml
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
```

## Security Incident Response

### Breach Response Plan
1. **Immediate Actions**
   - Isolate affected systems
   - Stop compromised services
   - Notify security team
   - Preserve evidence

2. **Investigation**
   - Review access logs
   - Check system integrity
   - Analyze attack vectors
   - Identify compromised data

3. **Recovery**
   - Restore from clean backups
   - Update security measures
   - Monitor for recurrence
   - Document incident

### Security Measures
```bash
# Enable audit logging
docker-compose -f docker-compose.prod.yml exec postgres psql -U postgres -c "ALTER SYSTEM SET log_statement = 'all';"

# Configure fail2ban
sudo apt install fail2ban
sudo cp /etc/fail2ban/jail.conf /etc/fail2ban/jail.local
sudo systemctl enable fail2ban
sudo systemctl start fail2ban
```

## Performance Tuning

### Application Optimization
```bash
# Database connection pooling
# Configure in .env:
DATABASE_POOL_SIZE=10
DATABASE_MAX_CONNECTIONS=20

# Redis optimization
REDIS_MAX_CONNECTIONS=50
REDIS_TIMEOUT=300

# Application settings
FERRUMYX_WORKER_THREADS=4
FERRUMYX_MAX_REQUEST_SIZE=10MB
```

### Database Optimization
```sql
-- Performance tuning queries
ALTER SYSTEM SET shared_buffers = '256MB';
ALTER SYSTEM SET effective_cache_size = '1GB';
ALTER SYSTEM SET work_mem = '4MB';
ALTER SYSTEM SET maintenance_work_mem = '64MB';
ALTER SYSTEM SET checkpoint_completion_target = 0.9;
ALTER SYSTEM SET wal_buffers = '16MB';
ALTER SYSTEM SET default_statistics_target = 100;

-- Create indexes for performance
CREATE INDEX CONCURRENTLY idx_conversations_user_id ON conversations(user_id);
CREATE INDEX CONCURRENTLY idx_messages_conversation_id ON messages(conversation_id);
```

### System Tuning
```bash
# Increase file descriptors
echo "fs.file-max = 65536" | sudo tee -a /etc/sysctl.conf
echo "* soft nofile 65536" | sudo tee -a /etc/security/limits.conf
echo "* hard nofile 65536" | sudo tee -a /etc/security/limits.conf

# Optimize network
echo "net.core.somaxconn = 65536" | sudo tee -a /etc/sysctl.conf
echo "net.ipv4.tcp_max_syn_backlog = 65536" | sudo tee -a /etc/sysctl.conf

sudo sysctl -p
```

## Troubleshooting

### Common Production Issues

#### Service Not Starting
```bash
# Check logs
docker-compose -f docker-compose.prod.yml logs ferrumyx-web

# Check resource usage
docker stats

# Check configuration
docker-compose -f docker-compose.prod.yml config
```

#### Database Connection Issues
```bash
# Test database connectivity
docker-compose -f docker-compose.prod.yml exec postgres pg_isready -U ferrumyx

# Check connection pool
docker-compose -f docker-compose.prod.yml exec postgres psql -U ferrumyx -d ferrumyx -c "SELECT * FROM pg_stat_activity;"

# Restart database
docker-compose -f docker-compose.prod.yml restart postgres
```

#### High Resource Usage
```bash
# Monitor processes
top -c

# Check Docker resource usage
docker stats --no-stream

# Analyze logs for issues
docker-compose -f docker-compose.prod.yml logs --tail=1000 | grep ERROR
```

#### SSL Certificate Issues
```bash
# Check certificate validity
openssl x509 -in /etc/ssl/ferrumyx/cert.pem -text -noout | grep -A 2 "Validity"

# Renew Let's Encrypt certificate
sudo certbot renew

# Reload Nginx
sudo nginx -s reload
```

## Maintenance Procedures

### Regular Maintenance Tasks
```bash
# Weekly tasks
# Update packages
sudo apt update && sudo apt upgrade -y

# Rotate logs
docker-compose -f docker-compose.prod.yml exec loki /usr/bin/logrotate /etc/logrotate.d/loki

# Clean up old images
docker image prune -f

# Database maintenance
docker-compose -f docker-compose.prod.yml exec postgres psql -U postgres -d ferrumyx -c "VACUUM ANALYZE;"

# Monthly tasks
# Security updates
sudo unattended-upgrades

# Backup verification
bash scripts/db-restore.sh --backup-id $(ls backups/ | tail -1) --dry-run

# Certificate renewal check
sudo certbot certificates
```

### Emergency Maintenance
```bash
# Emergency shutdown
docker-compose -f docker-compose.prod.yml down

# Emergency restart
docker-compose -f docker-compose.prod.yml up -d

# Force restart specific service
docker-compose -f docker-compose.prod.yml restart ferrumyx-web
```

## Rollback Procedures

### Quick Rollback
```bash
# Rollback to previous deployment
bash scripts/deploy.sh --rollback $(ls backups/ | grep backup- | tail -1)
```

### Full Rollback
```bash
# Stop current deployment
docker-compose -f docker-compose.prod.yml down

# Restore from backup
bash scripts/db-restore.sh --backup-id <backup-id>

# Rollback code
git checkout <previous-commit>
docker-compose -f docker-compose.prod.yml build
docker-compose -f docker-compose.prod.yml up -d
```

## Documentation Updates

### Post-deployment Documentation
- Update network diagrams
- Document new endpoints
- Update monitoring dashboards
- Review security policies
- Update incident response plans

### Change Management
- Document all changes
- Update configuration management
- Review access controls
- Update backup procedures