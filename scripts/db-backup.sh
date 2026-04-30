#!/bin/bash

# Ferrumyx Database Backup Script
# Create backups of Ferrumyx database and related data

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
BACKUP_RETENTION_DAYS=30
COMPRESSION_LEVEL=6

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
            --backup-dir)
                BACKUP_DIR="$2"
                shift 2
                ;;
            --retention-days)
                BACKUP_RETENTION_DAYS="$2"
                shift 2
                ;;
            --compression-level)
                COMPRESSION_LEVEL="$2"
                shift 2
                ;;
            --help)
                echo "Usage: $0 [OPTIONS]"
                echo
                echo "Options:"
                echo "  --backup-dir DIR        Backup directory (default: ./backups)"
                echo "  --retention-days DAYS   Keep backups for N days (default: 30)"
                echo "  --compression-level LVL Compression level 1-9 (default: 6)"
                echo "  --help                  Show this help message"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
}

# Setup backup directory
setup_backup_directory() {
    log_info "Setting up backup directory: $BACKUP_DIR"

    mkdir -p "$BACKUP_DIR"

    # Test write permissions
    if ! touch "$BACKUP_DIR/.test" 2>/dev/null; then
        log_error "Cannot write to backup directory: $BACKUP_DIR"
        exit 1
    fi
    rm -f "$BACKUP_DIR/.test"

    log_success "Backup directory ready"
}

# Generate backup ID
generate_backup_id() {
    if [[ -z "$BACKUP_ID" ]]; then
        BACKUP_ID="backup-$(date +%Y%m%d-%H%M%S)-$(openssl rand -hex 4)"
    fi
    log_info "Backup ID: $BACKUP_ID"
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

# Create database backup
create_database_backup() {
    log_info "Creating database backup..."

    local backup_file="$BACKUP_DIR/${BACKUP_ID}-database.sql.gz"
    local temp_file="$BACKUP_DIR/${BACKUP_ID}-database.sql"

    # Create compressed database dump
    log_info "Dumping database to $backup_file..."

    docker-compose exec -T postgres pg_dumpall -U "$POSTGRES_USER" | gzip -"$COMPRESSION_LEVEL" > "$backup_file"

    if [[ ! -f "$backup_file" ]] || [[ ! -s "$backup_file" ]]; then
        log_error "Database backup failed"
        exit 1
    fi

    local backup_size=$(du -h "$backup_file" | cut -f1)
    log_success "Database backup created: $backup_file (${backup_size})"

    # Cleanup temp file if it exists
    rm -f "$temp_file"
}

# Backup Redis data
create_redis_backup() {
    log_info "Creating Redis backup..."

    local backup_file="$BACKUP_DIR/${BACKUP_ID}-redis.rdb.gz"

    # Trigger Redis SAVE command
    if docker-compose exec -T redis redis-cli SAVE >/dev/null 2>&1; then
        # Copy Redis dump file
        docker cp "$(docker-compose ps -q redis)":/data/dump.rdb /tmp/redis-dump.rdb 2>/dev/null || true

        if [[ -f "/tmp/redis-dump.rdb" ]]; then
            gzip -"$COMPRESSION_LEVEL" -c /tmp/redis-dump.rdb > "$backup_file"
            rm -f /tmp/redis-dump.rdb

            local backup_size=$(du -h "$backup_file" | cut -f1)
            log_success "Redis backup created: $backup_file (${backup_size})"
        else
            log_warn "Redis dump file not found, skipping Redis backup"
        fi
    else
        log_warn "Redis SAVE command failed, skipping Redis backup"
    fi
}

# Backup configuration files
create_config_backup() {
    log_info "Creating configuration backup..."

    local config_files=(".env" "docker-compose.yml" "docker-compose.prod.yml" "docker-compose.dev.yml")
    local config_backup="$BACKUP_DIR/${BACKUP_ID}-config.tar.gz"

    # Create temporary directory for config files
    local temp_dir=$(mktemp -d)
    local has_files=false

    for config_file in "${config_files[@]}"; do
        if [[ -f "$config_file" ]]; then
            cp "$config_file" "$temp_dir/"
            has_files=true
        fi
    done

    if [[ "$has_files" == "true" ]]; then
        tar -czf "$config_backup" -C "$temp_dir" .
        rm -rf "$temp_dir"

        local backup_size=$(du -h "$config_backup" | cut -f1)
        log_success "Configuration backup created: $config_backup (${backup_size})"
    else
        log_warn "No configuration files found to backup"
        rm -rf "$temp_dir"
    fi
}

# Backup Docker volumes (metadata only)
create_volumes_backup() {
    log_info "Creating Docker volumes metadata backup..."

    local volumes_file="$BACKUP_DIR/${BACKUP_ID}-volumes.txt"

    # List all Ferrumyx-related volumes
    {
        echo "# Ferrumyx Docker Volumes Backup - $BACKUP_ID"
        echo "# Created: $(date)"
        echo
        docker volume ls --filter "name=ferrumyx" --format "table {{.Name}}\t{{.Driver}}"
        echo
        echo "# Volume details:"
        for volume in $(docker volume ls -q --filter "name=ferrumyx"); do
            echo "## Volume: $volume"
            docker volume inspect "$volume" | jq . 2>/dev/null || docker volume inspect "$volume"
            echo
        done
    } > "$volumes_file"

    local backup_size=$(du -h "$volumes_file" | cut -f1)
    log_success "Volumes metadata backup created: $volumes_file (${backup_size})"
}

# Create backup manifest
create_backup_manifest() {
    log_info "Creating backup manifest..."

    local manifest_file="$BACKUP_DIR/${BACKUP_ID}-manifest.txt"

    {
        echo "Ferrumyx Backup Manifest"
        echo "======================="
        echo "Backup ID: $BACKUP_ID"
        echo "Created: $(date)"
        echo "Database: $POSTGRES_DB"
        echo "Host: $(hostname)"
        echo
        echo "Files in this backup:"
        ls -la "$BACKUP_DIR/${BACKUP_ID}"* 2>/dev/null | while read -r line; do
            echo "  $line"
        done
        echo
        echo "Backup verification:"
        echo "- Database dump: $([[ -f "$BACKUP_DIR/${BACKUP_ID}-database.sql.gz" ]] && echo "✓ Present" || echo "✗ Missing")"
        echo "- Redis dump: $([[ -f "$BACKUP_DIR/${BACKUP_ID}-redis.rdb.gz" ]] && echo "✓ Present" || echo "✗ Missing")"
        echo "- Config files: $([[ -f "$BACKUP_DIR/${BACKUP_ID}-config.tar.gz" ]] && echo "✓ Present" || echo "✗ Missing")"
        echo "- Volumes metadata: $([[ -f "$BACKUP_DIR/${BACKUP_ID}-volumes.txt" ]] && echo "✓ Present" || echo "✗ Missing")"
    } > "$manifest_file"

    log_success "Backup manifest created: $manifest_file"
}

# Cleanup old backups
cleanup_old_backups() {
    log_info "Cleaning up old backups (retention: ${BACKUP_RETENTION_DAYS} days)..."

    local cutoff_date=$(date -d "$BACKUP_RETENTION_DAYS days ago" +%Y%m%d 2>/dev/null || date -v-"${BACKUP_RETENTION_DAYS}d" +%Y%m%d 2>/dev/null)

    if [[ -z "$cutoff_date" ]]; then
        log_warn "Could not determine cutoff date, skipping cleanup"
        return
    fi

    local removed_count=0
    for backup_file in "$BACKUP_DIR"/backup-*; do
        if [[ -f "$backup_file" ]]; then
            local file_date=$(basename "$backup_file" | cut -d'-' -f2)
            if [[ "$file_date" < "$cutoff_date" ]]; then
                rm -f "$backup_file"
                ((removed_count++))
            fi
        fi
    done

    if [[ $removed_count -gt 0 ]]; then
        log_info "Cleaned up $removed_count old backup files"
    else
        log_info "No old backups to clean up"
    fi
}

# Verify backup integrity
verify_backup() {
    log_info "Verifying backup integrity..."

    local database_backup="$BACKUP_DIR/${BACKUP_ID}-database.sql.gz"

    if [[ -f "$database_backup" ]]; then
        # Test gzip integrity
        if gzip -t "$database_backup" 2>/dev/null; then
            log_success "Database backup integrity verified"
        else
            log_error "Database backup integrity check failed"
            exit 1
        fi
    fi

    local config_backup="$BACKUP_DIR/${BACKUP_ID}-config.tar.gz"
    if [[ -f "$config_backup" ]]; then
        # Test tar.gz integrity
        if tar -tzf "$config_backup" >/dev/null 2>&1; then
            log_success "Configuration backup integrity verified"
        else
            log_warn "Configuration backup integrity check failed"
        fi
    fi

    log_success "Backup verification completed"
}

# Display backup summary
display_backup_summary() {
    log_success "Ferrumyx backup completed successfully!"

    echo
    echo "========================================"
    echo "Backup Summary"
    echo "========================================"
    echo "Backup ID: $BACKUP_ID"
    echo "Location: $BACKUP_DIR"
    echo "Created: $(date)"
    echo
    echo "Backup files:"
    ls -lh "$BACKUP_DIR/${BACKUP_ID}"* 2>/dev/null || echo "No backup files found"
    echo
    echo "Total backup size: $(du -sh "$BACKUP_DIR/${BACKUP_ID}"* 2>/dev/null | awk '{sum += $1} END {print sum "B"}' || echo "Unknown")"
    echo
    echo "Retention policy: $BACKUP_RETENTION_DAYS days"
    echo
    echo "To restore: bash scripts/db-restore.sh --backup-id $BACKUP_ID"
    echo "========================================"
}

# Main backup function
main() {
    echo "========================================"
    echo "Ferrumyx Database Backup"
    echo "========================================"
    echo

    parse_arguments "$@"
    load_environment
    setup_backup_directory
    generate_backup_id
    check_database_connection
    create_database_backup
    create_redis_backup
    create_config_backup
    create_volumes_backup
    create_backup_manifest
    verify_backup
    cleanup_old_backups
    display_backup_summary
}

# Run main function
main "$@"