# Ferrumyx Resource Limits and Governance

## Overview

This document outlines the resource limits and governance implemented across all Ferrumyx services to ensure stable operation, prevent resource exhaustion, and enable graceful degradation under load.

## Resource Allocation Strategy

### Production Environment
- **Conservative limits**: Set to handle peak loads with buffer for unexpected spikes
- **Monitoring**: Comprehensive metrics collection with alerting
- **Auto-scaling**: Manual horizontal scaling with resource awareness

### Development Environment
- **Flexible limits**: Higher limits for debugging and testing
- **No strict enforcement**: Allow resource spikes during development

## Service Resource Limits

| Service | Memory Limit | CPU Limit | Memory Reservation | CPU Reservation | Purpose |
|---------|--------------|-----------|-------------------|-----------------|---------|
| ferrumyx-web | 1G | 1.0 | 512M | 0.5 | High throughput web API |
| ironclaw-agent | 2G | 2.0 | 1G | 1.0 | Background AI agent processing |
| ferrumyx-ingestion | 4G | 1.5 | 2G | 0.75 | I/O intensive data ingestion |
| ferrumyx-kg | 4G | 3.0 | 2G | 1.5 | CPU intensive entity extraction + GPU |
| ferrumyx-ranker | 4G | 2.0 | 2G | 1.0 | ML inference + GPU |
| ferrumyx-molecules | 2G | 1.5 | 1G | 0.75 | External tool execution |
| postgres | 4G | 2.0 | 2G | 1.0 | Database operations |
| redis | 512M | 0.5 | 256M | 0.25 | Cache operations |
| nginx | 256M | 0.5 | 128M | 0.25 | Reverse proxy |
| bioclaw-wasm | 4G | 2.0 | 2G | 1.0 | WASM container execution |
| gateway-service | 1G | 1.0 | 512M | 0.5 | Multi-channel gateway |
| prometheus | 2G | 1.0 | 1G | 0.5 | Metrics collection |
| grafana | 1G | 1.0 | 512M | 0.5 | Dashboard visualization |
| alertmanager | 512M | 0.5 | 256M | 0.25 | Alert management |
| node-exporter | 256M | 0.25 | 128M | 0.1 | System metrics |
| postgres-exporter | 256M | 0.25 | 128M | 0.1 | Database metrics |
| loki | 1G | 1.0 | 512M | 0.5 | Log aggregation |
| promtail | 256M | 0.5 | 128M | 0.25 | Log shipping |
| cadvisor | 512M | 0.5 | 256M | 0.25 | Container metrics |

## Monitoring and Alerting

### Metrics Collection
- **Prometheus**: Collects metrics from all services
- **cAdvisor**: Container resource usage (CPU, memory, disk, network)
- **Node Exporter**: Host system metrics
- **PostgreSQL Exporter**: Database performance metrics
- **Application Metrics**: Custom business metrics via /metrics endpoints

### Alert Rules
- **Container Resource Alerts**: High CPU/memory usage (>90%)
- **Service Health**: Down services with appropriate severity
- **Database Alerts**: Connection limits, slow queries
- **System Alerts**: Disk space, memory usage
- **OOM Events**: Containers killed due to memory exhaustion

## OOM Handling

All containers are configured with memory limits and OOM killer protection:
- Memory limits prevent unbounded growth
- Reservations ensure minimum resources
- Alerts trigger on OOM events
- Graceful degradation under memory pressure

## GPU Support

Services requiring GPU acceleration (ferrumyx-kg, ferrumyx-ranker) include device reservations:
- NVIDIA GPU passthrough when available
- Automatic fallback to CPU if GPU unavailable
- Resource limits apply regardless of compute device

## Disk Management

### Volume Limits
- Persistent volumes have no explicit quotas (Docker limitation)
- Monitoring tracks volume usage
- Backup volumes have retention policies

### Temporary Storage
- Use tmpfs mounts for temporary data where applicable
- Monitor disk usage via node-exporter

## Network Limits

- No bandwidth limits implemented (Docker Compose limitation)
- Network monitoring via cAdvisor
- Internal networks isolated for security

## Auto-Scaling

### Current Implementation
- Docker Compose does not support automatic scaling
- Manual scaling via `docker-compose up --scale`
- Resource-based scaling decisions informed by monitoring

### Future Considerations
- Migrate to Kubernetes for HPA (Horizontal Pod Autoscaling)
- Use Docker Swarm with replica management
- External tools like KEDA for event-driven scaling

## Performance Testing

### Load Testing
- Test resource limits under simulated load
- Verify graceful degradation
- Validate alert thresholds

### Resource Exhaustion Tests
- Memory pressure testing
- CPU saturation scenarios
- Disk space limits validation

## Configuration Overrides

### Production Overrides
Use `docker-compose.prod.yml` for stricter limits:
- Lower memory/CPU limits
- Swarm deployment with restart policies
- Secret management integration

### Development Overrides
Use `docker-compose.dev.yml` for relaxed limits:
- Higher resource allowances
- Exposed ports for debugging
- Volume mounts for hot-reload

## Troubleshooting

### Common Issues
- **OOM Killed**: Check memory limits vs usage
- **High CPU**: Investigate workload or add replicas
- **Slow Performance**: Monitor resource saturation

### Monitoring Dashboards
- Grafana dashboards for real-time metrics
- Alert manager for incident response
- Loki for log correlation

## Compliance

Resource limits help ensure:
- No single service can exhaust system resources
- Predictable performance under load
- Compliance with resource governance policies
- Cost control in cloud environments</content>
<parameter name="filePath">docs/resource-limits.md