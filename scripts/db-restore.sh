#!/bin/bash

# Ferrumyx Database Restore Script
# Restore Ferrumyx database and related data from backups

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

# Default configuration
BACKUP_DIR="./backups"
BACKUP_ID=""
DRY_RUN=false
FORCE_RESTORE=false

# Load environment variables
load_environment() {
    if [[ -f ".env" ]]; then
        export $(grep -v '^#' .env | xargs)
    fi

    # Set defaults if not set
    export POSTGRES_USER=${POSTGRES_USER:-postgres}
    export POSTGRES_DB=${POSTGRES_DB:-ferrumyx}
    export DATABASE_URL=${DATABASE_URL:-postgresql://$POSTGRES_USER:$POSTGRES_PASSWORD@postgres:5432/$POSTGRES_DB}
}

# Parse command line arguments
parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --backup-id)
                BACKUP_ID="$2"
                shift 2
                ;;
            --backup-dir)
                BACKUP_DIR="$2"
                shift 2
                ;;
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --force)
                FORCE_RESTORE=true
                shift
                ;;
            --help)
                echo "Usage: $0 [OPTIONS]"
                echo
                echo "Options:"
                echo "  --backup-id ID       Backup ID to restore (required)"
                echo "  --backup-dir DIR     Backup directory (default: ./backups)"
                echo "  --dry-run            Show what would be restored without doing it"
                echo "  --force              Force restore without confirmation prompts"
                echo "  --help               Show this help message"
                echo
                echo "Examples:"
                echo "  $0 --backup-id backup-20240101-120000-abc123"
                echo "  $0 --backup-id backup-20240101-120000-abc123 --dry-run"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done

    if [[ -z "$BACKUP_ID" ]]; then
        log_error "Backup ID is required. Use --backup-id to specify which backup to restore."
        exit 1
    fi
}

# Validate backup exists
validate_backup() {
    log_info "Validating backup: $BACKUP_ID"

    local manifest_file="$BACKUP_DIR/${BACKUP_ID}-manifest.txt"

    if [[ ! -f "$manifest_file" ]]; then
        log_error "Backup manifest not found: $manifest_file"
        log_info "Available backups:"
        ls -1 "$BACKUP_DIR"/backup-*-manifest.txt 2>/dev/null | sed 's/.*backup-\(.*\)-manifest\.txt/\1/' || echo "No backups found"
        exit 1
    fi

    log_success "Backup manifest found"

    # Show backup information
    echo
    echo "Backup Information:"
    echo "==================="
    cat "$manifest_file"
    echo
}

# Confirm restore operation
confirm_restore() {
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "DRY RUN MODE - No changes will be made"
        return
    fi

    if [[ "$FORCE_RESTORE" == "false" ]]; then
        echo
        echo "WARNING: This will overwrite existing data!"
        echo "========================================"
        echo "Backup ID: $BACKUP_ID"
        echo "Target Database: $POSTGRES_DB"
        echo "Target Host: postgres:5432"
        echo
        read -p "Are you sure you want to proceed with the restore? (yes/no): " -r
        if [[ ! "$REPLY" =~ ^[Yy][Ee][Ss]$ ]]; then
            log_info "Restore cancelled by user"
            exit 0
        fi
    fi
}

# Check database connectivity
check_database_connection() {
    log_info "Checking database connectivity..."

    if ! docker-compose exec -T postgres pg_isready -U "$POSTGRES_USER" -h localhost >/dev/null 2>&1; then
        log_error "Cannot connect to database"
        exit 1
    fi

    log_success "Database connection established"
}

# Create pre-restore backup
create_pre_restore_backup() {
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "DRY RUN: Would create pre-restore backup"
        return
    fi

    log_info "Creating pre-restore backup..."

    local pre_restore_backup_id="pre-restore-$(date +%Y%m%d-%H%M%S)"

    # Use existing backup script
    export BACKUP_ID="$pre_restore_backup_id"
    if [[ -f "scripts/db-backup.sh" ]]; then
        bash scripts/db-backup.sh --backup-dir "$BACKUP_DIR"
        log_success "Pre-restore backup created: $pre_restore_backup_id"
    else
        log_warn "Backup script not found, skipping pre-restore backup"
    fi
}

# Stop application services
stop_services() {
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "DRY RUN: Would stop application services"
        return
    fi

    log_info "Stopping application services..."

    # Stop services that depend on the database
    docker-compose stop ferrumyx-web ironclaw-agent 2>/dev/null || true

    log_success "Application services stopped"
}

# Restore database
restore_database() {
    log_info "Restoring database..."

    local database_backup="$BACKUP_DIR/${BACKUP_ID}-database.sql.gz"

    if [[ ! -f "$database_backup" ]]; then
        log_error "Database backup file not found: $database_backup"
        exit 1
    fi

    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "DRY RUN: Would restore database from $database_backup"
        return
    fi

    # Terminate active connections to the database
    log_info "Terminating active database connections..."
    docker-compose exec -T postgres psql -U "$POSTGRES_USER" -d postgres -c "
        SELECT pg_terminate_backend(pid)
        FROM pg_stat_activity
        WHERE datname = '$POSTGRES_DB' AND pid <> pg_backend_pid();
    " 2>/dev/null || true

    # Drop and recreate database
    log_info "Recreating database..."
    docker-compose exec -T postgres psql -U "$POSTGRES_USER" -d postgres -c "DROP DATABASE IF EXISTS $POSTGRES_DB;" 2>/dev/null || true
    docker-compose exec -T postgres psql -U "$POSTGRES_USER" -d postgres -c "CREATE DATABASE $POSTGRES_DB;"

    # Restore from backup
    log_info "Restoring database from backup..."
    gunzip -c "$database_backup" | docker-compose exec -T postgres psql -U "$POSTGRES_USER" -d "$POSTGRES_DB"

    log_success "Database restored successfully"
}

# Restore Redis data
restore_redis() {
    log_info "Restoring Redis data..."

    local redis_backup="$BACKUP_DIR/${BACKUP_ID}-redis.rdb.gz"

    if [[ ! -f "$redis_backup" ]]; then
        log_warn "Redis backup file not found: $redis_backup"
        return
    fi

    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "DRY RUN: Would restore Redis from $redis_backup"
        return
    fi

    # Stop Redis
    docker-compose stop redis

    # Restore Redis dump
    gunzip -c "$redis_backup" > /tmp/redis-restore.rdb
    docker cp /tmp/redis-restore.rdb "$(docker-compose ps -q redis)":/data/dump.rdb
    rm -f /tmp/redis-restore.rdb

    # Start Redis
    docker-compose start redis

    # Wait for Redis to be ready
    local max_attempts=10
    local attempt=1
    while [[ $attempt -le $max_attempts ]]; do
        if docker-compose exec -T redis redis-cli ping | grep -q "PONG"; then
            log_success "Redis restored successfully"
            return
        fi
        sleep 2
        ((attempt++))
    done

    log_error "Redis failed to start after restore"
}

# Restore configuration files
restore_config() {
    log_info "Restoring configuration files..."

    local config_backup="$BACKUP_DIR/${BACKUP_ID}-config.tar.gz"

    if [[ ! -f "$config_backup" ]]; then
        log_warn "Configuration backup file not found: $config_backup"
        return
    fi

    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "DRY RUN: Would restore configuration from $config_backup"
        log_info "Files that would be restored:"
        tar -tzf "$config_backup"
        return
    fi

    # Create backup of current config files
    local config_backup_dir="$BACKUP_DIR/config-backup-$(date +%Y%m%d-%H%M%S)"
    mkdir -p "$config_backup_dir"

    local config_files=(".env" "docker-compose.yml" "docker-compose.prod.yml" "docker-compose.dev.yml")
    for config_file in "${config_files[@]}"; do
        if [[ -f "$config_file" ]]; then
            cp "$config_file" "$config_backup_dir/"
        fi
    done

    # Restore config files
    tar -xzf "$config_backup" -C .

    log_success "Configuration files restored (current configs backed up to $config_backup_dir)"
}

# Start services
start_services() {
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "DRY RUN: Would start application services"
        return
    fi

    log_info "Starting application services..."

    # Start services
    docker-compose start ferrumyx-web ironclaw-agent

    # Wait for services to be ready
    log_info "Waiting for services to be ready..."
    sleep 10

    log_success "Application services started"
}

# Run post-restore health checks
run_health_checks() {
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "DRY RUN: Would run health checks"
        return
    fi

    log_info "Running post-restore health checks..."

    # Check database connectivity
    if docker-compose exec -T postgres psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c "SELECT 1;" >/dev/null 2>&1; then
        log_success "Database health check passed"
    else
        log_error "Database health check failed"
        exit 1
    fi

    # Check Redis connectivity
    if docker-compose exec -T redis redis-cli ping | grep -q "PONG"; then
        log_success "Redis health check passed"
    else
        log_warn "Redis health check failed"
    fi

    # Check application health
    local max_attempts=20
    local attempt=1
    while [[ $attempt -le $max_attempts ]]; do
        if curl -f -s http://localhost:3000/health >/dev/null 2>&1; then
            log_success "Application health check passed"
            break
        fi
        log_info "Waiting for application... (attempt $attempt/$max_attempts)"
        sleep 5
        ((attempt++))
    done

    if [[ $attempt -gt $max_attempts ]]; then
        log_error "Application health check failed"
        exit 1
    fi
}

# Display restore summary
display_restore_summary() {
    log_success "Ferrumyx restore completed successfully!"

    echo
    echo "========================================"
    echo "Restore Summary"
    echo "========================================"
    echo "Backup ID: $BACKUP_ID"
    echo "Restored to: $POSTGRES_DB"
    echo "Completed: $(date)"
    echo
    if [[ "$DRY_RUN" == "true" ]]; then
        echo "DRY RUN MODE - No actual restore performed"
    else
        echo "✓ Database restored"
        echo "✓ Redis data restored"
        echo "✓ Configuration files restored"
        echo "✓ Services restarted"
        echo "✓ Health checks passed"
    fi
    echo
    echo "For backup: bash scripts/db-backup.sh"
    echo "========================================"
}

# Main restore function
main() {
    echo "========================================"
    echo "Ferrumyx Database Restore"
    echo "========================================"
    echo

    parse_arguments "$@"
    load_environment
    validate_backup
    confirm_restore
    check_database_connection
    create_pre_restore_backup
    stop_services
    restore_database
    restore_redis
    restore_config
    start_services
    run_health_checks
    display_restore_summary
}

# Run main function
main "$@"