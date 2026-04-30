# Ferrumyx Deployment Guide

## Overview

This guide covers deploying Ferrumyx in various environments, from local development to production clusters.

## Table of Contents

- [Quick Start](#quick-start)
- [Prerequisites](#prerequisites)
- [Environment Configuration](#environment-configuration)
- [Local Deployment](#local-deployment)
- [Production Deployment](#production-deployment)
- [Cloud Deployment](#cloud-deployment)
- [Scaling](#scaling)
- [Monitoring](#monitoring)
- [Troubleshooting](#troubleshooting)

## Quick Start

### Automated Setup

```bash
# Clone repository
git clone https://github.com/Classacre/ferrumyx.git
cd ferrumyx

# First-time setup
bash scripts/setup.sh

# For development
bash scripts/dev-setup.sh

# For production
bash scripts/deploy.sh
```

### Manual Setup

```bash
# Copy environment configuration
cp .env.example .env

# Start services
docker-compose up -d

# Run migrations
bash scripts/db-migrate.sh

# Verify deployment
bash scripts/health-check.sh
```

## Prerequisites

### System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| CPU | 2 cores | 4+ cores |
| RAM | 4GB | 8GB+ |
| Disk | 20GB | 100GB+ SSD |
| Network | 10Mbps | 100Mbps+ |

### Software Requirements

- **Docker**: 20.10+
- **Docker Compose**: 2.0+
- **Git**: 2.30+
- **OpenSSL**: For certificate generation

### Optional Components

- **Nginx/HAProxy**: Reverse proxy and load balancing
- **PostgreSQL**: External database (instead of containerized)
- **Redis**: External cache (instead of containerized)
- **Monitoring**: Prometheus, Grafana, Loki stack

## Environment Configuration

### Environment Variables

#### Core Configuration

```bash
# Database (passwords set via environment variables)
DATABASE_URL=postgresql://ferrumyx:${POSTGRES_PASSWORD}@host:5432/ferrumyx
REDIS_URL=redis://redis:${REDIS_PASSWORD}@redis:6379

# Application
FERRUMYX_WEB_ADDR=0.0.0.0:3000
LOG_LEVEL=info
SECRET_KEY=your-secret-key-here

# IronClaw Integration
IRONCLAW_API_KEY=your-api-key
IRONCLAW_ENDPOINT=https://api.ironclaw.ai

# BioClaw Tools
BIOCLAW_TOOLS_ENABLED=true
BIOCLAW_WASM_PATH=/opt/ferrumyx/wasm
```

#### Development Configuration

```bash
# .env.dev (generate passwords using scripts/generate-passwords.sh)
COMPOSE_FILE=docker-compose.dev.yml
LOG_LEVEL=debug
FERRUMYX_DEV_MODE=true
DATABASE_URL=postgres://postgres:${POSTGRES_PASSWORD}@localhost:5432/ferrumyx_dev
POSTGRES_PASSWORD=
REDIS_PASSWORD=
READONLY_PASSWORD=
GRAFANA_ADMIN_PASSWORD=
```

#### Production Configuration

```bash
# .env.prod (use Docker secrets for passwords)
COMPOSE_FILE=docker-compose.prod.yml
LOG_LEVEL=warn
FERRUMYX_WEB_ADDR=0.0.0.0:3000
# Database and Redis URLs use environment variables populated from Docker secrets
DATABASE_URL=postgresql://ferrumyx:${DB_PASSWORD}@postgres:5432/ferrumyx
REDIS_URL=redis://redis:${REDIS_PASSWORD}@redis:6379
SSL_CERT_PATH=/etc/ssl/ferrumyx/cert.pem
SSL_KEY_PATH=/etc/ssl/ferrumyx/key.pem
```

### Docker Compose Files

#### Development (`docker-compose.dev.yml`)

```yaml
version: '3.8'
services:
  ferrumyx-web:
    build:
      context: .
      dockerfile: Dockerfile.web
    ports:
      - "3000:3000"
    environment:
      - LOG_LEVEL=debug
    volumes:
      - .:/app
      - /app/node_modules
    command: npm run dev

  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: ferrumyx_dev
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: password
    volumes:
      - postgres_dev_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"

volumes:
  postgres_dev_data:
```

#### Production (`docker-compose.prod.yml`)

```yaml
version: '3.8'
services:
  ferrumyx-web:
    build:
      context: .
      dockerfile: Dockerfile.web
    ports:
      - "3000:3000"
    environment:
      - LOG_LEVEL=warn
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  ironclaw-agent:
    build:
      context: .
      dockerfile: Dockerfile.agent
    environment:
      - IRONCLAW_API_KEY=${IRONCLAW_API_KEY}
    restart: unless-stopped
    depends_on:
      postgres:
        condition: service_healthy

  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: ferrumyx
      POSTGRES_USER: ferrumyx
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - postgres_backup:/var/backups
    restart: unless-stopped
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ferrumyx"]
      interval: 30s
      timeout: 10s
      retries: 3

  redis:
    image: redis:7-alpine
    command: redis-server --requirepass ${REDIS_PASSWORD}
    volumes:
      - redis_data:/data
    restart: unless-stopped

volumes:
  postgres_data:
  postgres_backup:
  redis_data:
```

## Local Deployment

### Docker Compose (Recommended)

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f

# Stop services
docker-compose down

# Clean restart
docker-compose down -v && docker-compose up -d
```

### Manual Installation

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Clone and build
git clone https://github.com/Classacre/ferrumyx.git
cd ferrumyx
cargo build --release

# Install Node.js dependencies
npm install

# Setup database
createdb ferrumyx
psql ferrumyx < migrations/schema.sql

# Run application
./target/release/ferrumyx-web
```

## Production Deployment

### Single Server Deployment

```bash
# On production server
sudo mkdir -p /opt/ferrumyx
sudo chown $USER:$USER /opt/ferrumyx
cd /opt/ferrumyx

# Clone repository
git clone https://github.com/Classacre/ferrumyx.git .

# Configure environment
cp .env.example .env
nano .env  # Edit production settings

# Deploy
bash scripts/deploy.sh
```

### Load Balanced Deployment

```yaml
# docker-compose.prod.lb.yml
version: '3.8'
services:
  ferrumyx-web:
    build:
      context: .
      dockerfile: Dockerfile.web
    deploy:
      replicas: 3
      restart_policy:
        condition: on-failure
    networks:
      - ferrumyx-network

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
      - ssl_certs:/etc/ssl/certs
    depends_on:
      - ferrumyx-web
    networks:
      - ferrumyx-network

  # ... other services
```

### Blue-Green Deployment

```bash
# Deploy to blue environment
export COMPOSE_FILE=docker-compose.blue.yml
docker-compose up -d

# Test blue environment
curl -f http://blue-environment/health

# Switch traffic to blue
# Update load balancer configuration

# Keep green as rollback option
export COMPOSE_FILE=docker-compose.green.yml
docker-compose down
```

## Cloud Deployment

### AWS ECS/Fargate

```yaml
# ecs-task-definition.json
{
  "family": "ferrumyx",
  "taskRoleArn": "arn:aws:iam::123456789012:role/ecsTaskRole",
  "executionRoleArn": "arn:aws:iam::123456789012:role/ecsTaskExecutionRole",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "1024",
  "memory": "2048",
  "containerDefinitions": [
    {
      "name": "ferrumyx-web",
      "image": "123456789012.dkr.ecr.us-east-1.amazonaws.com/ferrumyx:latest",
      "portMappings": [
        {
          "containerPort": 3000,
          "hostPort": 3000,
          "protocol": "tcp"
        }
      ],
      "environment": [
        {"name": "DATABASE_URL", "value": "..."},
        {"name": "REDIS_URL", "value": "..."}
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/ferrumyx",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "ecs"
        }
      },
      "healthCheck": {
        "command": ["CMD-SHELL", "curl -f http://localhost:3000/health"],
        "interval": 30,
        "timeout": 5,
        "retries": 3
      }
    }
  ]
}
```

### Google Cloud Run

```yaml
# cloud-run.yaml
apiVersion: serving.knative.dev/v1
kind: Service
metadata:
  name: ferrumyx
spec:
  template:
    spec:
      containers:
      - image: gcr.io/project-id/ferrumyx:latest
        ports:
        - containerPort: 3000
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              key: database-url
              name: ferrumyx-secrets
        - name: REDIS_URL
          valueFrom:
            secretKeyRef:
              key: redis-url
              name: ferrumyx-secrets
        resources:
          limits:
            cpu: 1000m
            memory: 2Gi
        startupProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 30
          periodSeconds: 10
```

### Kubernetes Deployment

```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ferrumyx
spec:
  replicas: 3
  selector:
    matchLabels:
      app: ferrumyx
  template:
    metadata:
      labels:
        app: ferrumyx
    spec:
      containers:
      - name: ferrumyx-web
        image: ferrumyx:latest
        ports:
        - containerPort: 3000
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: ferrumyx-secrets
              key: database-url
        - name: REDIS_URL
          valueFrom:
            secretKeyRef:
              name: ferrumyx-secrets
              key: redis-url
        resources:
          requests:
            cpu: 500m
            memory: 1Gi
          limits:
            cpu: 1000m
            memory: 2Gi
        livenessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 5
          periodSeconds: 5
```

## Scaling

### Horizontal Scaling

#### Application Layer

```bash
# Scale web service
docker-compose up -d --scale ferrumyx-web=5

# Or in Kubernetes
kubectl scale deployment ferrumyx --replicas=5
```

#### Database Scaling

```yaml
# Read replica configuration
services:
  postgres-primary:
    image: postgres:15
    environment:
      POSTGRES_DB: ferrumyx
      POSTGRES_USER: ferrumyx
    volumes:
      - postgres_primary_data:/var/lib/postgresql/data

  postgres-replica:
    image: postgres:15
    environment:
      POSTGRES_DB: ferrumyx
      POSTGRES_USER: ferrumyx
    command: >
      bash -c "
      until pg_basebackup --pgdata=/tmp/data -R --slot=replication_slot --host=postgres-primary --port=5432; do
        sleep 1
      done &&
      echo 'hot_standby = on' >> /tmp/data/postgresql.conf &&
      exec postgres -D /tmp/data
      "
    volumes:
      - postgres_replica_data:/tmp/data
    depends_on:
      - postgres-primary
```

### Vertical Scaling

#### Resource Limits

```yaml
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
```

#### Database Optimization

```sql
-- Increase connection pool
ALTER SYSTEM SET max_connections = 200;

-- Optimize memory settings
ALTER SYSTEM SET shared_buffers = '512MB';
ALTER SYSTEM SET effective_cache_size = '2GB';
ALTER SYSTEM SET work_mem = '8MB';
```

## Monitoring

### Built-in Monitoring

Ferrumyx includes a complete monitoring stack:

```bash
# Setup monitoring
bash scripts/monitoring-setup.sh

# Access dashboards
# Grafana: http://localhost:3001 (admin/admin)
# Prometheus: http://localhost:9090
# Loki: http://localhost:3100
```

### External Monitoring

#### Prometheus Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'ferrumyx'
    static_configs:
      - targets: ['ferrumyx:3000']
    metrics_path: '/metrics'

  - job_name: 'postgres'
    static_configs:
      - targets: ['postgres-exporter:9187']
```

#### Health Checks

```bash
# Application health
curl http://localhost:3000/health

# Database health
curl http://localhost:9090/api/v1/targets

# Comprehensive check
bash scripts/health-check.sh --detailed --json
```

## Troubleshooting

### Common Issues

#### Service Won't Start

```bash
# Check logs
docker-compose logs ferrumyx-web

# Check resource usage
docker stats

# Check configuration
docker-compose config

# Restart service
docker-compose restart ferrumyx-web
```

#### Database Connection Issues

```bash
# Test connection
docker-compose exec postgres pg_isready -U ferrumyx

# Check database logs
docker-compose logs postgres

# Reset database
docker-compose down -v
docker-compose up -d postgres
bash scripts/db-migrate.sh
```

#### Performance Issues

```bash
# Monitor resources
docker stats

# Check application metrics
curl http://localhost:3000/metrics

# Profile application
cargo flamegraph --bin ferrumyx-web
```

#### Networking Issues

```bash
# Check network connectivity
docker network ls
docker network inspect ferrumyx-network

# Test service communication
docker-compose exec ferrumyx-web curl http://postgres:5432
```

### Logs and Debugging

#### Application Logs

```bash
# View all logs
docker-compose logs -f

# View specific service logs
docker-compose logs -f ferrumyx-web

# Search logs
docker-compose logs | grep ERROR

# Export logs
docker-compose logs > ferrumyx-logs-$(date +%Y%m%d).txt
```

#### System Logs

```bash
# Docker daemon logs
sudo journalctl -u docker.service

# System resource logs
dmesg | tail -50

# Network logs
sudo tcpdump -i any port 3000 -w capture.pcap
```

### Recovery Procedures

#### Quick Restart

```bash
# Restart all services
docker-compose restart

# Restart specific service
docker-compose restart ferrumyx-web

# Force recreate
docker-compose up -d --force-recreate
```

#### Database Recovery

```bash
# Restore from backup
bash scripts/db-restore.sh --backup-id $(ls backups/ | tail -1)

# Recreate database
docker-compose exec postgres dropdb ferrumyx
docker-compose exec postgres createdb ferrumyx
bash scripts/db-migrate.sh
```

#### Full System Reset

```bash
# Stop everything
docker-compose down -v

# Clean up
docker system prune -a

# Fresh start
docker-compose up -d
bash scripts/db-migrate.sh
bash scripts/health-check.sh
```

## Security Considerations

### Production Security

#### SSL/TLS Configuration

```nginx
# nginx.conf
server {
    listen 443 ssl http2;
    server_name yourdomain.com;

    ssl_certificate /etc/ssl/certs/cert.pem;
    ssl_certificate_key /etc/ssl/private/key.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512;

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

#### Secrets Management

Use the provided scripts to generate and manage secrets:

```bash
# Generate all secrets for production
./scripts/generate-secrets.sh

# Generate individual secrets
./scripts/generate-secrets.sh db
./scripts/generate-secrets.sh redis
./scripts/generate-secrets.sh api-keys

# For AWS Secrets Manager integration
export DB_PASSWORD=$(aws secretsmanager get-secret-value --secret-id ferrumyx/db --query SecretString --output text)
export REDIS_PASSWORD=$(aws secretsmanager get-secret-value --secret-id ferrumyx/redis --query SecretString --output text)
```

#### Network Security

```bash
# Configure firewall
sudo ufw allow 22/tcp
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw --force enable

# Use internal networks
docker network create --internal ferrumyx-internal
```

### Backup Security

```bash
# Encrypt backups
bash scripts/db-backup.sh
openssl enc -aes-256-cbc -salt -in backup-file.sql.gz -out backup-file.sql.gz.enc

# Secure backup storage
aws s3 cp backup-file.sql.gz.enc s3://ferrumyx-backups/

# Backup verification
openssl enc -d -aes-256-cbc -in backup-file.sql.gz.enc | gunzip | head -10
```

## Performance Optimization

### Application Tuning

```bash
# Worker threads
export FERRUMYX_WORKER_THREADS=$(nproc)

# Connection pooling
export DATABASE_POOL_SIZE=20
export REDIS_POOL_SIZE=10

# Caching
export FERRUMYX_CACHE_TTL=3600
```

### Database Tuning

```sql
-- Performance settings
ALTER SYSTEM SET shared_buffers = '256MB';
ALTER SYSTEM SET effective_cache_size = '1GB';
ALTER SYSTEM SET work_mem = '4MB';
ALTER SYSTEM SET maintenance_work_mem = '64MB';

-- Create indexes
CREATE INDEX CONCURRENTLY idx_conversations_created_at ON conversations(created_at);
CREATE INDEX CONCURRENTLY idx_messages_conversation_id ON messages(conversation_id);
```

### System Tuning

```bash
# Increase limits
echo "fs.file-max = 65536" | sudo tee -a /etc/sysctl.conf
echo "* soft nofile 65536" | sudo tee -a /etc/security/limits.conf

# Network tuning
echo "net.core.somaxconn = 65536" | sudo tee -a /etc/sysctl.conf

sudo sysctl -p
```

## Maintenance

### Regular Tasks

#### Daily
```bash
# Check health
bash scripts/health-check.sh

# Rotate logs
docker-compose exec loki logrotate /etc/logrotate.d/loki

# Update images
docker-compose pull
```

#### Weekly
```bash
# Security updates
sudo apt update && sudo apt upgrade

# Database maintenance
docker-compose exec postgres vacuumdb --all --analyze

# Backup verification
bash scripts/db-restore.sh --backup-id $(ls backups/ | tail -1) --dry-run
```

#### Monthly
```bash
# Full backup test
bash scripts/db-backup.sh
bash scripts/db-restore.sh --backup-id $(ls backups/ | tail -1) --dry-run

# Certificate renewal
sudo certbot renew

# Performance review
# Check metrics in Grafana
# Review slow queries
# Analyze resource usage
```

### Emergency Procedures

```bash
# Emergency stop
docker-compose down

# Emergency restart
docker-compose up -d --force-recreate

# Emergency rollback
bash scripts/deploy.sh --rollback $(ls backups/ | tail -1)
```

---

For detailed runbooks, see the [runbooks/](runbooks/) directory.