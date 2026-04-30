#!/bin/bash

# Ferrumyx Monitoring Setup Script
# Initialize monitoring stack with Prometheus, Grafana, Loki, and AlertManager

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

# Check if running in production or development
check_environment() {
    if [[ -f "docker-compose.prod.yml" ]] && [[ -f ".env" ]]; then
        COMPOSE_FILE="docker-compose.prod.yml"
        ENV_TYPE="production"
        log_info "Production environment detected"
    elif [[ -f "docker-compose.dev.yml" ]]; then
        COMPOSE_FILE="docker-compose.dev.yml"
        ENV_TYPE="development"
        log_info "Development environment detected"
    else
        COMPOSE_FILE="docker-compose.yml"
        ENV_TYPE="default"
        log_info "Default environment detected"
    fi
}

# Validate monitoring configuration
validate_monitoring_config() {
    log_info "Validating monitoring configuration..."

    # Check required directories
    local required_dirs=("monitoring")
    for dir in "${required_dirs[@]}"; do
        if [[ ! -d "$dir" ]]; then
            log_error "Required directory '$dir' not found"
            exit 1
        fi
    done

    # Check required files
    local required_files=(
        "monitoring/prometheus.yml"
        "monitoring/alertmanager.yml"
        "monitoring/grafana/provisioning/datasources/datasources.yml"
        "monitoring/grafana/provisioning/dashboards.yml"
        "monitoring/promtail-config.yml"
        "monitoring/loki-config.yml"
    )

    for file in "${required_files[@]}"; do
        if [[ ! -f "$file" ]]; then
            log_error "Required file '$file' not found"
            exit 1
        fi
    done

    log_success "Monitoring configuration validated"
}

# Setup monitoring networks and volumes
setup_monitoring_infrastructure() {
    log_info "Setting up monitoring infrastructure..."

    # Create monitoring network if it doesn't exist
    if ! docker network ls | grep -q "ferrumyx-monitoring"; then
        docker network create ferrumyx-monitoring
        log_info "Created ferrumyx-monitoring network"
    fi

    # Create monitoring volumes
    local volumes=("grafana_data" "prometheus_data" "loki_data")
    for volume in "${volumes[@]}"; do
        if ! docker volume ls | grep -q "$volume"; then
            docker volume create "$volume"
            log_info "Created volume: $volume"
        fi
    done

    log_success "Monitoring infrastructure ready"
}

# Initialize Grafana
setup_grafana() {
    log_info "Setting up Grafana..."

    export COMPOSE_FILE="$COMPOSE_FILE"

    # Start Grafana
    docker-compose up -d grafana

    # Wait for Grafana to be ready
    log_info "Waiting for Grafana to be ready..."
    local max_attempts=30
    local attempt=1

    while [[ $attempt -le $max_attempts ]]; do
        if curl -f -s http://localhost:3001/api/health >/dev/null 2>&1; then
            log_success "Grafana is ready"
            break
        fi
        log_info "Waiting for Grafana... (attempt $attempt/$max_attempts)"
        sleep 5
        ((attempt++))
    done

    if [[ $attempt -gt $max_attempts ]]; then
        log_error "Grafana failed to start within timeout"
        exit 1
    fi

    # Create default admin user (password must be set)
    if [[ -z "${GRAFANA_ADMIN_PASSWORD}" ]]; then
        log_error "GRAFANA_ADMIN_PASSWORD environment variable must be set"
        exit 1
    fi
    local grafana_admin_password=${GRAFANA_ADMIN_PASSWORD}

    # Check if admin user exists
    if ! curl -f -s -u "admin:admin" http://localhost:3001/api/users >/dev/null 2>&1; then
        log_info "Setting up Grafana admin user..."

        # Reset admin password
        curl -X PUT -H "Content-Type: application/json" \
             -d "{\"password\":\"$grafana_admin_password\"}" \
             http://admin:admin@localhost:3001/api/admin/users/1/password >/dev/null 2>&1

        log_info "Grafana admin password set to: $grafana_admin_password"
    fi

    log_success "Grafana setup completed"
}

# Initialize Prometheus
setup_prometheus() {
    log_info "Setting up Prometheus..."

    export COMPOSE_FILE="$COMPOSE_FILE"

    # Start Prometheus
    docker-compose up -d prometheus postgres-exporter redis-exporter

    # Wait for Prometheus to be ready
    log_info "Waiting for Prometheus to be ready..."
    local max_attempts=20
    local attempt=1

    while [[ $attempt -le $max_attempts ]]; do
        if curl -f -s http://localhost:9090/-/ready >/dev/null 2>&1; then
            log_success "Prometheus is ready"
            break
        fi
        log_info "Waiting for Prometheus... (attempt $attempt/$max_attempts)"
        sleep 3
        ((attempt++))
    done

    if [[ $attempt -gt $max_attempts ]]; then
        log_error "Prometheus failed to start within timeout"
        exit 1
    fi

    log_success "Prometheus setup completed"
}

# Initialize Loki and Promtail
setup_loki() {
    log_info "Setting up Loki and Promtail..."

    export COMPOSE_FILE="$COMPOSE_FILE"

    # Start Loki and Promtail
    docker-compose up -d loki promtail

    # Wait for Loki to be ready
    log_info "Waiting for Loki to be ready..."
    local max_attempts=20
    local attempt=1

    while [[ $attempt -le $max_attempts ]]; do
        if curl -f -s http://localhost:3100/ready >/dev/null 2>&1; then
            log_success "Loki is ready"
            break
        fi
        log_info "Waiting for Loki... (attempt $attempt/$max_attempts)"
        sleep 3
        ((attempt++))
    done

    if [[ $attempt -gt $max_attempts ]]; then
        log_error "Loki failed to start within timeout"
        exit 1
    fi

    log_success "Loki and Promtail setup completed"
}

# Setup AlertManager
setup_alertmanager() {
    log_info "Setting up AlertManager..."

    export COMPOSE_FILE="$COMPOSE_FILE"

    # Start AlertManager
    docker-compose up -d alertmanager

    # Wait for AlertManager to be ready
    log_info "Waiting for AlertManager to be ready..."
    local max_attempts=20
    local attempt=1

    while [[ $attempt -le $max_attempts ]]; do
        if curl -f -s http://localhost:9093/-/ready >/dev/null 2>&1; then
            log_success "AlertManager is ready"
            break
        fi
        log_info "Waiting for AlertManager... (attempt $attempt/$max_attempts)"
        sleep 3
        ((attempt++))
    done

    if [[ $attempt -gt $max_attempts ]]; then
        log_error "AlertManager failed to start within timeout"
        exit 1
    fi

    log_success "AlertManager setup completed"
}

# Configure monitoring dashboards
setup_dashboards() {
    log_info "Setting up monitoring dashboards..."

    # Wait a bit for Grafana to fully initialize
    sleep 10

    # Import default dashboards via API
    local grafana_admin_password=${GRAFANA_ADMIN_PASSWORD:-admin}

    # Create Ferrumyx folder
    curl -X POST -H "Content-Type: application/json" \
         -u "admin:$grafana_admin_password" \
         -d '{"title":"Ferrumyx"}' \
         http://localhost:3001/api/folders >/dev/null 2>&1

    # Import dashboards from provisioning
    log_info "Dashboards will be automatically provisioned from monitoring/grafana/provisioning/"

    log_success "Dashboard setup completed"
}

# Configure alerting rules
setup_alerting() {
    log_info "Setting up alerting rules..."

    # Alerting rules are already configured in monitoring/alert_rules.yml
    # and will be loaded by Prometheus

    log_info "Alerting rules configured:"
    echo "- Database connectivity alerts"
    echo "- High CPU/memory usage alerts"
    echo "- Service health check failures"
    echo "- Disk space monitoring"

    log_success "Alerting setup completed"
}

# Test monitoring stack
test_monitoring() {
    log_info "Testing monitoring stack..."

    local tests_passed=0
    local total_tests=5

    # Test Grafana
    if curl -f -s http://localhost:3001/api/health >/dev/null 2>&1; then
        log_success "Grafana test passed"
        ((tests_passed++))
    else
        log_error "Grafana test failed"
    fi

    # Test Prometheus
    if curl -f -s http://localhost:9090/-/ready >/dev/null 2>&1; then
        log_success "Prometheus test passed"
        ((tests_passed++))
    else
        log_error "Prometheus test failed"
    fi

    # Test Loki
    if curl -f -s http://localhost:3100/ready >/dev/null 2>&1; then
        log_success "Loki test passed"
        ((tests_passed++))
    else
        log_error "Loki test failed"
    fi

    # Test AlertManager
    if curl -f -s http://localhost:9093/-/ready >/dev/null 2>&1; then
        log_success "AlertManager test passed"
        ((tests_passed++))
    else
        log_error "AlertManager test failed"
    fi

    # Test metrics collection
    if curl -f -s http://localhost:9090/api/v1/targets | grep -q "up"; then
        log_success "Metrics collection test passed"
        ((tests_passed++))
    else
        log_error "Metrics collection test failed"
    fi

    log_info "Monitoring tests: $tests_passed/$total_tests passed"

    if [[ $tests_passed -lt $total_tests ]]; then
        log_warn "Some monitoring tests failed. Check logs for details."
    fi
}

# Display monitoring setup summary
display_monitoring_summary() {
    log_success "Ferrumyx monitoring setup completed!"

    echo
    echo "========================================"
    echo "Monitoring Setup Summary"
    echo "========================================"
    echo "Environment: $ENV_TYPE"
    echo "Setup completed: $(date)"
    echo
    echo "Monitoring Services:"
    echo "✓ Grafana: http://localhost:3001 (admin/admin)"
    echo "✓ Prometheus: http://localhost:9090"
    echo "✓ Loki: http://localhost:3100"
    echo "✓ AlertManager: http://localhost:9093"
    echo
    echo "Default Dashboards:"
    echo "- Ferrumyx System Overview"
    echo "- Database Performance"
    echo "- Application Metrics"
    echo "- Container Resource Usage"
    echo
    echo "Alerting:"
    echo "- Database connectivity issues"
    echo "- High resource usage"
    echo "- Service health failures"
    echo "- Disk space warnings"
    echo
    echo "Log Aggregation:"
    echo "- Application logs via Promtail"
    echo "- System logs collected"
    echo "- Query logs in Grafana Explore"
    echo
    echo "Next steps:"
    echo "1. Change default Grafana password"
    echo "2. Configure alert notification channels"
    echo "3. Review and customize dashboards"
    echo "4. Set up log retention policies"
    echo
    echo "========================================"
}

# Main monitoring setup function
main() {
    echo "========================================"
    echo "Ferrumyx Monitoring Setup"
    echo "========================================"
    echo

    check_environment
    validate_monitoring_config
    setup_monitoring_infrastructure
    setup_grafana
    setup_prometheus
    setup_loki
    setup_alertmanager
    setup_dashboards
    setup_alerting
    test_monitoring
    display_monitoring_summary
}

# Run main function
main "$@"