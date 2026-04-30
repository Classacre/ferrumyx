# Ferrumyx Database Setup

This directory contains the comprehensive database configuration for Ferrumyx, including PostgreSQL with pgvector, migrations, backups, and monitoring.

## Quick Start

1. **Environment Setup**
   ```bash
   cp database.env.template database.env
   # Edit database.env with your configuration
   ```

2. **Start Database Services**
   ```bash
   docker-compose up postgres
   ```

3. **Run Migrations**
   ```bash
   ./run-migrations.sh run
   ```

4. **Seed Development Data**
   ```bash
   ./backup-restore.sh export-anon
   ```

## Directory Structure

```
database/
├── Dockerfile.postgres          # Custom PostgreSQL + PgBouncer container
├── init-db.sql                  # Initial database schema
├── seed-dev-data.sql            # Development seed data
├── run-migrations.sh            # Migration runner script
├── backup-restore.sh            # Backup/restore automation
├── pgbouncer.ini               # PgBouncer configuration
├── userlist.txt                # PgBouncer authentication
├── start.sh                    # Container startup script
├── init-pgbouncer.sh           # PgBouncer initialization
├── database.env.template       # Environment configuration
├── postgresql.conf.template    # PostgreSQL configuration
├── migrations/                 # Version-controlled schema migrations
│   ├── 001_initial_schema.sql
│   ├── 002_phase3_entity_tables.sql
│   ├── 003_external_data_providers.sql
│   └── 004_user_setup_and_permissions.sql
└── monitoring/                 # Monitoring and alerting setup
    ├── prometheus.yml
    ├── alertmanager.yml
    ├── alert_rules.yml
    └── grafana/
        └── provisioning/
            └── datasources/
                └── datasources.yml
```

## Database Architecture

### PostgreSQL + pgvector
- **Version**: PostgreSQL 15+ with pgvector extension
- **Vector Support**: 768-dimension embeddings (BiomedBERT-base)
- **Connection Pooling**: PgBouncer for high-performance connections
- **Replication**: Configurable for production HA

### Schema Overview
- **Papers**: Research paper metadata and full text
- **Chunks**: Document chunks with vector embeddings
- **Entities**: Named entities (genes, diseases, chemicals)
- **Knowledge Graph**: Facts and relationships between entities
- **Target Scores**: Drug target prioritization results
- **External Data**: TCGA, cBioPortal, COSMIC, GTEx, ChEMBL, Reactome

### Key Features
- **Vector Search**: HNSW indexing for similarity search
- **Full-text Search**: PostgreSQL FTS for paper content
- **ACID Compliance**: Full transactional support
- **PHI Compliance**: Audit logging and data anonymization
- **Backup/Restore**: Point-in-time recovery capabilities

## Connection Details

### Direct PostgreSQL Access
- **Host**: localhost:5432
- **Database**: ferrumyx
- **User**: ferrumyx
- **Password**: (from environment)

### Connection Pooling (PgBouncer)
- **Host**: localhost:6432
- **Database**: ferrumyx
- **User**: ferrumyx
- **Password**: (from environment)

### Read-Only Access
- **User**: ferrumyx_readonly
- **Permissions**: SELECT on all tables

## Backup and Recovery

### Automated Backups
```bash
# Daily backup
./backup-restore.sh backup

# List backups
./backup-restore.sh list

# Cleanup old backups (30+ days)
./backup-restore.sh cleanup
```

### Point-in-Time Recovery
```bash
# Enable PITR
./backup-restore.sh pitr-setup

# Restore from backup
./backup-restore.sh restore /path/to/backup.sql.gz
```

### Data Export
```bash
# Anonymized export for testing
./backup-restore.sh export-anon
```

## Monitoring and Alerting

### Metrics Collected
- PostgreSQL performance (queries, connections, locks)
- PgBouncer pool statistics
- System resources (CPU, memory, disk)
- Application health checks

### Alerts
- Database down
- High connection count
- Slow queries (>1000ms)
- Low disk space (<10%)
- High memory usage (>90%)

### Dashboards
- Grafana dashboards for visualization
- Prometheus metrics collection
- AlertManager for notifications

## Security

### Database Users
- **ferrumyx**: Full application access
- **ferrumyx_readonly**: Read-only for monitoring/analytics
- **ferrumyx_backup**: Backup operations only

### SSL/TLS
- Configurable SSL encryption
- Certificate-based authentication
- Audit logging for sensitive operations

### PHI Compliance
- Data anonymization for testing environments
- Audit trails for data access
- HIPAA-compliant data handling

## Performance Optimization

### Indexing Strategy
- B-tree indexes on frequently queried columns
- IVFFlat indexes on vector embeddings
- Partial indexes for filtered queries
- Composite indexes for common WHERE clauses

### Connection Pooling
- PgBouncer with configurable pool sizes
- Transaction-level pooling
- Connection reuse and recycling

### Memory Tuning
- Shared buffers: 256MB (configurable)
- Work memory: 4MB per connection
- Maintenance work memory: 64MB
- Effective cache size: 1GB

## Development Workflow

### Schema Changes
1. Create new migration in `migrations/`
2. Test migration on development database
3. Commit migration file
4. Deploy to staging/production
5. Run migration: `./run-migrations.sh run`

### Testing
```bash
# Run tests with test database
export DATABASE_URL="postgresql://ferrumyx:test@localhost:5432/ferrumyx_test"
cargo test

# Seed test data
psql -f seed-dev-data.sql
```

### Production Deployment
1. Update environment variables
2. Configure SSL certificates
3. Set up monitoring alerts
4. Enable automated backups
5. Configure replication (optional)

## Troubleshooting

### Common Issues

**Connection Refused**
- Check if PostgreSQL is running: `docker-compose ps`
- Verify connection string and credentials
- Check firewall settings

**Out of Memory**
- Increase Docker memory limits
- Tune PostgreSQL memory settings
- Monitor connection pool usage

**Slow Queries**
- Check query execution plans
- Verify indexes are present
- Monitor system resources

**Backup Failures**
- Check disk space in backup directory
- Verify database connectivity
- Check PostgreSQL logs

### Logs
```bash
# PostgreSQL logs
docker-compose logs postgres

# PgBouncer logs
docker-compose exec postgres tail -f /var/log/pgbouncer/pgbouncer.log

# Application logs
docker-compose logs ferrumyx-web
```

## Contributing

When making database changes:

1. **Always use migrations** - Never modify schema directly
2. **Test migrations** - Run on development database first
3. **Document changes** - Update this README as needed
4. **Backup first** - Especially in production environments
5. **Monitor impact** - Check performance after deployment

## Support

For issues or questions:
- Check logs in `docker-compose logs`
- Review PostgreSQL documentation
- Consult the Ferrumyx architecture docs
- Create an issue in the project repository