#!/bin/bash

# Ferrumyx First-Run Setup Script
# Complete environment initialization for Ferrumyx deployment

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

# Check if running as root
check_root() {
    if [[ $EUID -eq 0 ]]; then
        log_error "This script should not be run as root"
        exit 1
    fi
}

# Check system requirements
check_system_requirements() {
    log_info "Checking system requirements..."

    # Check OS
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        log_info "Linux detected"
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        log_info "macOS detected"
    else
        log_error "Unsupported operating system: $OSTYPE"
        exit 1
    fi

    # Check required commands
    local required_commands=("docker" "docker-compose" "curl" "git")
    for cmd in "${required_commands[@]}"; do
        if ! command -v "$cmd" &> /dev/null; then
            log_error "Required command '$cmd' not found. Please install it first."
            exit 1
        fi
    done

    # Check Docker version
    if ! docker --version | grep -q "Docker version"; then
        log_error "Docker is not running or accessible"
        exit 1
    fi

    log_success "System requirements check passed"
}

# Setup environment variables
setup_environment() {
    log_info "Setting up environment variables..."

    local env_file=".env"

    if [[ ! -f "$env_file" ]]; then
        log_warn "No .env file found. Creating from template..."

        if [[ -f ".env.example" ]]; then
            cp .env.example .env
            log_info "Copied .env.example to .env"
        else
            log_error ".env.example not found. Please create environment configuration."
            exit 1
        fi
    fi

    # Generate secure random values for sensitive variables if empty
    if grep -q "^POSTGRES_PASSWORD=$" .env 2>/dev/null; then
        local postgres_pass=$(openssl rand -base64 32)
        sed -i.bak "s/^POSTGRES_PASSWORD=$/POSTGRES_PASSWORD=$postgres_pass/" .env
        log_info "Generated secure PostgreSQL password"
    fi

    if grep -q "^READONLY_PASSWORD=$" .env 2>/dev/null; then
        local readonly_pass=$(openssl rand -base64 32)
        sed -i.bak "s/^READONLY_PASSWORD=$/READONLY_PASSWORD=$readonly_pass/" .env
        log_info "Generated secure readonly password"
    fi

    if grep -q "^REDIS_PASSWORD=$" .env 2>/dev/null; then
        local redis_pass=$(openssl rand -base64 32)
        sed -i.bak "s/^REDIS_PASSWORD=$/REDIS_PASSWORD=$redis_pass/" .env
        log_info "Generated secure Redis password"
    fi

    if grep -q "^GRAFANA_ADMIN_PASSWORD=$" .env 2>/dev/null; then
        local grafana_pass=$(openssl rand -base64 16)
        sed -i.bak "s/^GRAFANA_ADMIN_PASSWORD=$/GRAFANA_ADMIN_PASSWORD=$grafana_pass/" .env
        log_info "Generated secure Grafana admin password"
    fi

    if grep -q "IRONCLAW_API_KEY=changeme" .env 2>/dev/null; then
        log_warn "IRONCLAW_API_KEY is set to default value. Please update with your actual API key."
    fi

    log_success "Environment variables configured"
}

# Initialize Docker networks and volumes
setup_docker() {
    log_info "Setting up Docker networks and volumes..."

    # Create network if it doesn't exist
    if ! docker network ls | grep -q "ferrumyx-network"; then
        docker network create ferrumyx-network
        log_info "Created ferrumyx-network"
    fi

    # Create volumes if they don't exist
    local volumes=("postgres_data" "postgres_backup" "redis_data" "grafana_data" "loki_data" "prometheus_data")
    for volume in "${volumes[@]}"; do
        if ! docker volume ls | grep -q "$volume"; then
            docker volume create "$volume"
            log_info "Created volume: $volume"
        fi
    done

    log_success "Docker setup completed"
}

# Run database migrations
setup_database() {
    log_info "Setting up database..."

    # Wait for PostgreSQL to be ready
    log_info "Waiting for PostgreSQL to be ready..."
    local max_attempts=30
    local attempt=1

    while [[ $attempt -le $max_attempts ]]; do
        if docker-compose exec -T postgres pg_isready -U postgres -h localhost >/dev/null 2>&1; then
            log_success "PostgreSQL is ready"
            break
        fi
        log_info "Waiting for PostgreSQL... (attempt $attempt/$max_attempts)"
        sleep 2
        ((attempt++))
    done

    if [[ $attempt -gt $max_attempts ]]; then
        log_error "PostgreSQL failed to start within timeout"
        exit 1
    fi

    # Set database user passwords
    if [[ -f "set-passwords.sh" ]]; then
        log_info "Setting database user passwords..."
        bash set-passwords.sh
    else
        log_warn "set-passwords.sh not found. Skipping password setup."
    fi

    # Run migrations
    if [[ -f "run-migrations.sh" ]]; then
        log_info "Running database migrations..."
        bash run-migrations.sh
    else
        log_warn "run-migrations.sh not found. Skipping database migrations."
    fi

    log_success "Database setup completed"
}

# Setup monitoring stack
setup_monitoring() {
    log_info "Setting up monitoring stack..."

    # Start monitoring services
    docker-compose up -d grafana prometheus loki promtail alertmanager postgres-exporter redis-exporter

    # Wait for services to be healthy
    log_info "Waiting for monitoring services to be healthy..."

    local services=("grafana" "prometheus" "loki")
    for service in "${services[@]}"; do
        local max_attempts=20
        local attempt=1

        while [[ $attempt -le $max_attempts ]]; do
            if docker-compose ps "$service" | grep -q "Up"; then
                log_success "$service is healthy"
                break
            fi
            log_info "Waiting for $service... (attempt $attempt/$max_attempts)"
            sleep 3
            ((attempt++))
        done

        if [[ $attempt -gt $max_attempts ]]; then
            log_error "$service failed to start within timeout"
            exit 1
        fi
    done

    log_success "Monitoring setup completed"
}

# Initialize application services
setup_application() {
    log_info "Setting up application services..."

    # Build and start all services
    docker-compose up -d --build

    # Wait for services to be healthy
    log_info "Waiting for application services to be healthy..."

    local services=("ferrumyx-web" "ironclaw-agent")
    for service in "${services[@]}"; do
        local max_attempts=30
        local attempt=1

        while [[ $attempt -le $max_attempts ]]; do
            if docker-compose ps "$service" | grep -q "Up"; then
                log_success "$service is healthy"
                break
            fi
            log_info "Waiting for $service... (attempt $attempt/$max_attempts)"
            sleep 5
            ((attempt++))
        done

        if [[ $attempt -gt $max_attempts ]]; then
            log_error "$service failed to start within timeout"
            exit 1
        fi
    done

    log_success "Application setup completed"
}

# Run health checks
run_health_checks() {
    log_info "Running health checks..."

    # Run health check script if it exists
    if [[ -f "scripts/health-check.sh" ]]; then
        bash scripts/health-check.sh
    else
        log_warn "Health check script not found. Running basic checks..."

        # Basic health checks
        local health_endpoints=("http://localhost:3000/health")
        for endpoint in "${health_endpoints[@]}"; do
            if curl -f -s "$endpoint" >/dev/null; then
                log_success "Health check passed for $endpoint"
            else
                log_error "Health check failed for $endpoint"
                exit 1
            fi
        done
    fi

    log_success "All health checks passed"
}

# Display setup summary
display_summary() {
    log_success "Ferrumyx setup completed successfully!"
    echo
    echo "========================================"
    echo "Setup Summary:"
    echo "========================================"
    echo "✓ System requirements verified"
    echo "✓ Environment variables configured"
    echo "✓ Docker networks and volumes created"
    echo "✓ Database initialized and migrated"
    echo "✓ Monitoring stack deployed"
    echo "✓ Application services running"
    echo "✓ Health checks passed"
    echo
    echo "Services available at:"
    echo "- Ferrumyx Web UI: http://localhost:3000"
    echo "- Grafana: http://localhost:3001"
    echo "- Prometheus: http://localhost:9090"
    echo
    echo "Next steps:"
    echo "1. Configure your IronClaw API key in .env"
    echo "2. Access the web UI to complete setup"
    echo "3. Review monitoring dashboards"
    echo
    echo "For development setup, run: bash scripts/dev-setup.sh"
    echo "========================================"
}

# Main setup function
main() {
    echo "========================================"
    echo "Ferrumyx First-Run Setup"
    echo "========================================"
    echo

    check_root
    check_system_requirements
    setup_environment
    setup_docker
    setup_database
    setup_monitoring
    setup_application
    run_health_checks
    display_summary
}

# Run main function
main "$@"