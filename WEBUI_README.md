# Ferrumyx Web Interface

Production-ready web interface for Ferrumyx with optimized Docker deployment.

## Components

- **Dockerfile.webui**: Multi-stage build with Node.js optimization and Nginx runtime
- **nginx.conf**: Optimized Nginx configuration with security headers and compression
- **nginx/default.conf**: Server configuration with CORS, caching, and rate limiting
- **build-webui.sh**: Build optimization script (expandable for minification/bundling)
- **.env.webui.example**: Environment configuration template
- **docker-compose.webui.yml**: Docker Compose setup for web UI and API

## Quick Start

1. **Build the web interface:**
   ```bash
   ./build-webui.sh
   ```

2. **Build and run with Docker:**
   ```bash
   docker build -f Dockerfile.webui -t ferrumyx-webui .
   docker run -p 8080:80 ferrumyx-webui
   ```

3. **Or use Docker Compose:**
   ```bash
   cp .env.webui.example .env
   # Edit .env with your configuration
   docker-compose -f docker-compose.webui.yml up -d
   ```

## Environment Configuration

Copy `.env.webui.example` to `.env` and configure:

- `API_ENDPOINT`: Backend API URL
- `CORS_ORIGIN`: Allowed CORS origins
- `NODE_ENV`: Environment (development/staging/production)

## Security Features

- Non-root user execution
- Security headers (CSP, HSTS, X-Frame-Options)
- Rate limiting for API and static assets
- Gzip compression
- HTTPS-ready configuration

## Performance Optimizations

- Aggressive caching for static assets (1 year)
- Gzip compression for text-based content
- Optimized Nginx configuration
- Health checks for container orchestration

## Development

For development with hot reloading, use the Rust server directly:

```bash
cargo run --package ferrumyx-web
```

## Production Deployment

1. Build optimized assets: `./build-webui.sh`
2. Configure environment variables
3. Deploy with Docker Compose or Kubernetes
4. Set up SSL/TLS termination (recommended)
5. Configure monitoring and logging

## Asset Optimization

The build script currently copies files as-is. To add optimization:

1. Install Node.js dependencies for minification
2. Add CSS/JS minification steps to `build-webui.sh`
3. Implement asset fingerprinting for cache busting
4. Add image optimization

## Health Checks

- Container health check: `http://localhost/health`
- Readiness probe for API dependencies
- Liveness probe for container health

## Monitoring

Logs are available at `/var/log/nginx/` when using Docker volumes.
Configure external monitoring for:
- Nginx access/error logs
- Container resource usage
- API response times