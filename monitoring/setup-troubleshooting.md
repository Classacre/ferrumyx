# Ferrumyx Monitoring Setup and Troubleshooting Guide

## Prerequisites
- Docker and Docker Compose installed
- At least 4GB RAM available for monitoring stack
- Ports 9090, 3001, 9093, 3100 available

## Setup Instructions

1. **Start the monitoring stack:**
   ```bash
   docker-compose up -d prometheus grafana alertmanager loki promtail postgres-exporter node-exporter
   ```

2. **Access monitoring interfaces:**
   - Prometheus: http://localhost:9090
   - Grafana: http://localhost:3001 (admin / $GRAFANA_ADMIN_PASSWORD)
   - Alertmanager: http://localhost:9093
   - Loki: http://localhost:3100

3. **Configure Grafana:**
   - Login with admin / $GRAFANA_ADMIN_PASSWORD
   - Dashboards will be auto-provisioned
   - Add Loki as a datasource if needed: URL http://loki:3100

## Environment Variables

Add to your `.env` file:
```bash
# Monitoring
GRAFANA_ADMIN_PASSWORD=secure_password_here

# Alertmanager email (optional)
SMTP_USER=your-email@gmail.com
SMTP_PASS=your-app-password
```

## Troubleshooting

### Services Not Starting
```bash
# Check service status
docker-compose ps

# View logs
docker-compose logs <service_name>
```

### Metrics Not Appearing
1. Verify service is exposing `/metrics` endpoint
2. Check Prometheus targets: http://localhost:9090/targets
3. Ensure network connectivity between services

### Alerts Not Working
1. Check Alertmanager configuration
2. Verify SMTP settings in alertmanager.yml
3. Test email delivery manually

### High Resource Usage
- Reduce scrape intervals in prometheus.yml
- Limit data retention in Prometheus config
- Monitor container resource usage

### Log Aggregation Issues
1. Check Promtail config syntax
2. Verify log file permissions
3. Ensure Loki is accessible from Promtail

## Common Issues

### PostgreSQL Metrics Not Collecting
- Verify postgres-exporter DATA_SOURCE_NAME
- Check database connectivity
- Ensure readonly user has necessary permissions

### Node Exporter No Data
- Ensure host volumes are mounted correctly
- Check if running in privileged mode if needed

### Grafana Dashboards Empty
- Wait for auto-provisioning (may take a few minutes)
- Check provisioning config files
- Restart Grafana container

## Scaling Considerations

For production deployment:
- Use external PostgreSQL for Grafana and Loki storage
- Configure Prometheus federation for multi-region
- Set up proper backup and retention policies
- Implement authentication and TLS

## Security Best Practices

- Change default Grafana admin password
- Use HTTPS for all monitoring interfaces
- Restrict network access to monitoring ports
- Regularly update Docker images
- Monitor for PHI data in logs and metrics