# Docker Configuration

This directory contains all Docker-related configuration files for Ferrumyx deployment.

## Files

### Compose Files
- docker-compose.yml - Base production configuration
- docker-compose.dev.yml - Development environment with hot-reload
- docker-compose.prod.yml - Production environment with security hardening
- docker-compose.override.yml - Local customization overrides
- docker-compose.gpu.yml - GPU acceleration configuration
- docker-compose.webui.yml - Web UI specific configuration

### Dockerfiles
- Dockerfile.web - Ferrumyx web server container
- Dockerfile.postgres - PostgreSQL with pgvector extension
- Dockerfile.redis - Redis caching server
- Dockerfile.webui - Web UI assets container

### Infrastructure
- 
ginx.conf - Reverse proxy and SSL termination configuration

## Usage

See [DEPLOYMENT.md](../docs/operations/deployment.md) for detailed deployment instructions.
