# Development Environment Setup Runbook

## Overview
This runbook provides step-by-step instructions for setting up a local development environment for Ferrumyx using the ferrumyx-setup CLI tool.

## Prerequisites
- Docker and Docker Compose installed
- Git client
- At least 8GB RAM available
- 20GB free disk space
- Ferrumyx repository cloned

## Quick Setup (CLI-Based - Recommended)

### Option 1: Complete Setup (Configuration + Infrastructure)
```bash
# Clone the repository
git clone https://github.com/Classacre/ferrumyx.git
cd ferrumyx

# Run complete development setup
ferrumyx-setup setup --environment development
```

### Option 2: Configuration Only (Then Manual Infrastructure)
```bash
# Clone the repository
git clone https://github.com/Classacre/ferrumyx.git
cd ferrumyx

# Configure environment interactively
ferrumyx-setup wizard --environment development

# Then start services manually
docker-compose -f docker-compose.dev.yml up -d
```

### Option 3: Non-Interactive Setup
```bash
# Clone the repository
git clone https://github.com/Classacre/ferrumyx.git
cd ferrumyx

# Run automated setup with defaults
ferrumyx-setup setup --environment development --non-interactive
```

## Development Workflow

### Starting Development
```bash
# Start all development services (if not using CLI setup)
docker-compose -f docker-compose.dev.yml up -d

# Or use CLI for complete setup
ferrumyx-setup setup --environment development
```

### Running Tests
```bash
# Run all tests
cargo test --workspace --features postgres

# Run specific test
cargo test test_function_name

# Run with coverage (requires cargo-tarpaulin)
cargo tarpaulin --workspace --features postgres --out Xml
```

### Code Quality Checks
```bash
# Format code
cargo fmt --all

# Run clippy
cargo clippy --workspace -- -D warnings

# Run lint (JavaScript/TypeScript)
npm run lint
```

### Debugging

#### View Logs
```bash
# All services
docker-compose -f docker-compose.dev.yml logs -f

# Specific service
docker-compose -f docker-compose.dev.yml logs -f ferrumyx-web

# Database logs
docker-compose -f docker-compose.dev.yml logs -f postgres
```

#### Access Database
```bash
# Connect to PostgreSQL
docker-compose -f docker-compose.dev.yml exec postgres psql -U postgres -d ferrumyx_dev

# Connect to Redis
docker-compose -f docker-compose.dev.yml exec redis redis-cli
```

#### Debug Application
```bash
# Check health endpoints
curl http://localhost:3000/health

# Run configuration validation
ferrumyx-setup validate

# Check service health (manual)
docker-compose -f docker-compose.dev.yml ps
```

## Environment Configuration

### Development vs Production
| Setting | Development | Production |
|---------|-------------|------------|
| Database | ferrumyx_dev | ferrumyx |
| Log Level | debug | info/warn |
| Auto-restart | on-failure | unless-stopped |
| Volumes | ephemeral | persistent |

### Environment Variables
```bash
# Core Settings
DATABASE_URL=postgres://postgres:password@localhost:5432/ferrumyx_dev
REDIS_URL=redis://localhost:6379
LOG_LEVEL=debug

# IronClaw Settings
IRONCLAW_API_KEY=your-api-key-here
IRONCLAW_ENDPOINT=https://api.ironclaw.ai

# BioClaw Settings
BIOCLAW_TOOLS_ENABLED=true
FERRUMYX_DEV_MODE=true
```

## Troubleshooting

### Common Issues

#### Database Connection Failed
```bash
# Check if PostgreSQL is running
docker-compose -f docker-compose.dev.yml ps postgres

# Restart database
docker-compose -f docker-compose.dev.yml restart postgres

# Check database logs
docker-compose -f docker-compose.dev.yml logs postgres
```

#### Port Already in Use
```bash
# Find process using port
lsof -i :3000

# Kill process
kill -9 <PID>

# Or change port in docker-compose.dev.yml
```

#### Build Failures
```bash
# Clear Docker cache
docker system prune -a

# Rebuild without cache
docker-compose -f docker-compose.dev.yml build --no-cache
```

#### Permission Issues
```bash
# Fix script permissions
chmod +x scripts/*.sh

# Check Docker permissions
docker run hello-world
```

### Getting Help
1. Check the logs: `docker-compose -f docker-compose.dev.yml logs`
2. Validate configuration: `ferrumyx-setup validate`
3. Review documentation: See wiki sections above
4. Create an issue: https://github.com/Classacre/ferrumyx/issues

## Cleanup

### Stop Development Environment
```bash
# Stop all services
docker-compose -f docker-compose.dev.yml down

# Remove volumes (WARNING: destroys data)
docker-compose -f docker-compose.dev.yml down -v

# Clean up unused resources
docker system prune
```

### Reset Development Environment
```bash
# Stop services
docker-compose -f docker-compose.dev.yml down

# Remove database volume
docker volume rm ferrumyx_postgres_data_dev

# Restart fresh (CLI will handle setup)
docker-compose -f docker-compose.dev.yml up -d
ferrumyx-setup setup --environment development --config-only
```

## Performance Tips

### Development Optimization
```bash
# Use development database (lighter)
export DATABASE_URL=postgres://postgres:password@localhost:5432/ferrumyx_dev

# Enable debug logging selectively
export RUST_LOG=ferrumyx=debug,tokio=info

# Use less resource-intensive settings
export BIOCLAW_TOOLS_ENABLED=false
```

### Resource Monitoring
```bash
# Monitor resource usage
docker stats

# Check disk usage
df -h

# Monitor database performance
docker-compose -f docker-compose.dev.yml exec postgres psql -U postgres -d ferrumyx_dev -c "SELECT * FROM pg_stat_activity;"
```

## Security Notes

### Development Security
- Use strong passwords for database
- Don't commit secrets to repository
- Use `.env.dev` for development-specific settings
- Regularly update dependencies

### Access Control
- Development environment accessible only locally
- No external access unless configured
- Database not exposed externally by default