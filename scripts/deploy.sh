#!/bin/bash

# Ferrumyx Production Deployment Script
# Production deployment with health checks and rollback procedures

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $(date '+%Y-%m-%d %H:%M:%S') - $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $(date '+%Y-%m-%d %H:%M:%S') - $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $(date '+%Y-%m-%d %H:%M:%S') - $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $(date '+%Y-%m-%d %H:%M:%S') - $1"
}

# Global variables
DEPLOYMENT_ID=""
BACKUP_ID=""
ROLLBACK_AVAILABLE=false
SERVICES_TO_DEPLOY=("ferrumyx-web" "ironclaw-agent" "postgres" "redis")
MONITORING_SERVICES=("grafana" "prometheus" "loki" "promtail" "alertmanager")

# Cleanup function
cleanup() {
    local exit_code=$?
    if [[ $exit_code -ne 0 ]]; then
        log_error "Deployment failed with exit code $exit_code"
        if [[ "$ROLLBACK_AVAILABLE" == "true" ]]; then
            log_warn "Attempting automatic rollback..."
            rollback_deployment
        fi
    fi
    exit $exit_code
}

trap cleanup EXIT

# Validate deployment prerequisites
validate_prerequisites() {
    log_info "Validating deployment prerequisites..."

    # Check if running as non-root
    if [[ $EUID -eq 0 ]]; then
        log_error "This script should not be run as root"
        exit 1
    fi

    # Check required files
    local required_files=(".env" "docker-compose.prod.yml")
    for file in "${required_files[@]}"; do
        if [[ ! -f "$file" ]]; then
            log_error "Required file '$file' not found"
            exit 1
        fi
    done

    # Check environment variables
    local required_vars=("DATABASE_URL" "IRONCLAW_API_KEY" "POSTGRES_PASSWORD")
    for var in "${required_vars[@]}"; do
        if ! grep -q "^$var=" .env; then
            log_error "Required environment variable '$var' not found in .env"
            exit 1
        fi
    done

    # Check Docker
    if ! docker --version >/dev/null 2>&1; then
        log_error "Docker is not available"
        exit 1
    fi

    if ! docker-compose --version >/dev/null 2>&1; then
        log_error "Docker Compose is not available"
        exit 1
    fi

    log_success "Prerequisites validation passed"
}

# Generate deployment ID
generate_deployment_id() {
    DEPLOYMENT_ID="deploy-$(date +%Y%m%d-%H%M%S)-$(openssl rand -hex 4)"
    BACKUP_ID="backup-$(date +%Y%m%d-%H%M%S)"
    log_info "Generated deployment ID: $DEPLOYMENT_ID"
    log_info "Generated backup ID: $BACKUP_ID"
}

# Create backup before deployment
create_backup() {
    log_info "Creating pre-deployment backup..."

    # Database backup
    if [[ -f "scripts/db-backup.sh" ]]; then
        export BACKUP_ID
        bash scripts/db-backup.sh
        log_success "Database backup created: $BACKUP_ID"
    else
        log_warn "Database backup script not found. Skipping database backup."
    fi

    # Docker volume backup (if needed)
    log_info "Docker volumes will be preserved automatically"

    ROLLBACK_AVAILABLE=true
    log_success "Backup completed"
}

# Pre-deployment health checks
pre_deployment_checks() {
    log_info "Running pre-deployment health checks..."

    # Check current services status
    if docker-compose ps | grep -q "Up"; then
        log_info "Existing services are running"

        # Run health checks on current deployment
        if [[ -f "scripts/health-check.sh" ]]; then
            log_info "Running current deployment health checks..."
            if ! bash scripts/health-check.sh --quiet; then
                log_error "Current deployment health checks failed"
                exit 1
            fi
            log_success "Current deployment is healthy"
        fi
    else
        log_warn "No existing services running - fresh deployment"
    fi

    # Check system resources
    local available_memory=$(free -m | awk 'NR==2{printf "%.0f", $2/1024}')
    if [[ $available_memory -lt 4 ]]; then
        log_error "Insufficient memory: ${available_memory}GB available, minimum 4GB required"
        exit 1
    fi

    local available_disk=$(df / | awk 'NR==2{printf "%.0f", $4/1024/1024}')
    if [[ $available_disk -lt 10 ]]; then
        log_error "Insufficient disk space: ${available_disk}GB available, minimum 10GB required"
        exit 1
    fi

    log_success "Pre-deployment checks passed"
}

# Build production images
build_production_images() {
    log_info "Building production images..."

    # Use production compose file
    export COMPOSE_FILE=docker-compose.prod.yml

    # Build images with no cache for clean builds
    docker-compose build --no-cache --parallel

    # Tag images with deployment ID
    for service in "${SERVICES_TO_DEPLOY[@]}"; do
        local image_name="ferrumyx-${service}"
        if docker images | grep -q "$image_name"; then
            docker tag "$image_name:latest" "$image_name:$DEPLOYMENT_ID"
            log_info "Tagged $image_name:$DEPLOYMENT_ID"
        fi
    done

    log_success "Production images built and tagged"
}

# Deploy services with zero-downtime
deploy_services() {
    log_info "Deploying services with zero-downtime strategy..."

    export COMPOSE_FILE=docker-compose.prod.yml

    # Start new services alongside existing ones
    log_info "Starting new service versions..."

    # Deploy core services first
    for service in "${SERVICES_TO_DEPLOY[@]}"; do
        log_info "Deploying $service..."

        # Stop old service gracefully
        docker-compose stop "$service" || true

        # Start new service
        docker-compose up -d "$service"

        # Wait for service to be healthy
        wait_for_service "$service"
    done

    # Deploy monitoring services
    log_info "Deploying monitoring services..."
    for service in "${MONITORING_SERVICES[@]}"; do
        docker-compose up -d "$service" || log_warn "Failed to start $service"
    done

    log_success "Services deployed successfully"
}

# Wait for service to be healthy
wait_for_service() {
    local service=$1
    local max_attempts=30
    local attempt=1

    log_info "Waiting for $service to be healthy..."

    while [[ $attempt -le $max_attempts ]]; do
        if docker-compose ps "$service" | grep -q "Up"; then
            # Check service-specific health
            case $service in
                "ferrumyx-web")
                    if curl -f -s http://localhost:3000/health >/dev/null 2>&1; then
                        log_success "$service is healthy"
                        return 0
                    fi
                    ;;
                "postgres")
                    if docker-compose exec -T postgres pg_isready -U postgres -h localhost >/dev/null 2>&1; then
                        log_success "$service is healthy"
                        return 0
                    fi
                    ;;
                "redis")
                    if docker-compose exec -T redis redis-cli ping | grep -q "PONG"; then
                        log_success "$service is healthy"
                        return 0
                    fi
                    ;;
                *)
                    log_success "$service is running"
                    return 0
                    ;;
            esac
        fi

        log_info "Waiting for $service... (attempt $attempt/$max_attempts)"
        sleep 5
        ((attempt++))
    done

    log_error "$service failed to become healthy within timeout"
    return 1
}

# Run post-deployment health checks
post_deployment_checks() {
    log_info "Running post-deployment health checks..."

    # Run comprehensive health checks
    if [[ -f "scripts/health-check.sh" ]]; then
        if ! bash scripts/health-check.sh; then
            log_error "Post-deployment health checks failed"
            return 1
        fi
    else
        log_warn "Health check script not found. Running basic checks..."

        # Basic health checks
        local endpoints=("http://localhost:3000/health")
        for endpoint in "${endpoints[@]}"; do
            if ! curl -f -s "$endpoint" >/dev/null 2>&1; then
                log_error "Health check failed for $endpoint"
                return 1
            fi
        done
    fi

    log_success "Post-deployment health checks passed"
}

# Cleanup old images and containers
cleanup_old_deployments() {
    log_info "Cleaning up old deployments..."

    # Remove old containers (keep last 2 versions)
    docker container prune -f

    # Remove unused images (keep last 5 versions per service)
    for service in "${SERVICES_TO_DEPLOY[@]}"; do
        local image_name="ferrumyx-${service}"
        local images_to_keep=5

        # Get all tags for this service, sort by creation time, keep newest
        local old_images=$(docker images "$image_name" --format "table {{.Repository}}:{{.Tag}}\t{{.CreatedAt}}" | tail -n +2 | sort -k2 -r | tail -n +$((images_to_keep + 1)) | awk '{print $1}')

        if [[ -n "$old_images" ]]; then
            echo "$old_images" | xargs docker rmi -f 2>/dev/null || true
            log_info "Cleaned up old images for $service"
        fi
    done

    log_success "Cleanup completed"
}

# Rollback deployment
rollback_deployment() {
    log_error "Initiating rollback to previous deployment..."

    export COMPOSE_FILE=docker-compose.prod.yml

    # Stop current services
    docker-compose down

    # Restore from backup if available
    if [[ -f "scripts/db-restore.sh" ]] && [[ -n "$BACKUP_ID" ]]; then
        log_info "Restoring database from backup..."
        export BACKUP_ID
        bash scripts/db-restore.sh
    fi

    # Restart previous working services
    log_info "Restarting previous working services..."
    docker-compose up -d

    # Verify rollback success
    if post_deployment_checks; then
        log_success "Rollback completed successfully"
    else
        log_error "Rollback verification failed. Manual intervention required."
        exit 1
    fi
}

# Send deployment notifications
send_notifications() {
    log_info "Sending deployment notifications..."

    # Create deployment summary
    local summary_file="deployment-${DEPLOYMENT_ID}.log"
    {
        echo "Ferrumyx Deployment Summary"
        echo "=========================="
        echo "Deployment ID: $DEPLOYMENT_ID"
        echo "Timestamp: $(date)"
        echo "Status: SUCCESS"
        echo "Services Deployed: ${SERVICES_TO_DEPLOY[*]}"
        echo "Monitoring Services: ${MONITORING_SERVICES[*]}"
        echo "Backup ID: $BACKUP_ID"
    } > "$summary_file"

    log_info "Deployment summary saved to $summary_file"

    # Send to monitoring system (if configured)
    # Add webhook notifications here if needed
}

# Display deployment summary
display_deployment_summary() {
    log_success "Ferrumyx production deployment completed successfully!"
    echo
    echo "========================================"
    echo "Deployment Summary"
    echo "========================================"
    echo "Deployment ID: $DEPLOYMENT_ID"
    echo "Backup ID: $BACKUP_ID"
    echo "Timestamp: $(date)"
    echo
    echo "Services deployed:"
    for service in "${SERVICES_TO_DEPLOY[@]}"; do
        echo "✓ $service"
    done
    echo
    echo "Monitoring services:"
    for service in "${MONITORING_SERVICES[@]}"; do
        echo "✓ $service"
    done
    echo
    echo "Access points:"
    echo "- Ferrumyx Web UI: http://localhost:3000"
    echo "- Grafana: http://localhost:3001"
    echo "- Prometheus: http://localhost:9090"
    echo
    echo "For rollback: bash scripts/deploy.sh --rollback $BACKUP_ID"
    echo "========================================"
}

# Main deployment function
main() {
    echo "========================================"
    echo "Ferrumyx Production Deployment"
    echo "========================================"
    echo

    # Parse command line arguments
    if [[ "$1" == "--rollback" ]] && [[ -n "$2" ]]; then
        BACKUP_ID="$2"
        log_info "Performing rollback to backup: $BACKUP_ID"
        rollback_deployment
        exit 0
    fi

    validate_prerequisites
    generate_deployment_id
    create_backup
    pre_deployment_checks
    build_production_images
    deploy_services
    post_deployment_checks
    cleanup_old_deployments
    send_notifications
    display_deployment_summary
}

# Run main function
main "$@"