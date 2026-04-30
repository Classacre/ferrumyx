#!/bin/bash
# backup-restore.sh - Ferrumyx PostgreSQL Backup and Restore Automation
# Usage: ./backup-restore.sh [backup|restore|list] [options]

set -euo pipefail

# Configuration - override with environment variables
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-ferrumyx}"
DB_USER="${DB_USER:-ferrumyx_backup}"
DB_PASSWORD="${DB_PASSWORD:-}"
BACKUP_DIR="${BACKUP_DIR:-/var/backups/ferrumyx}"
RETENTION_DAYS="${RETENTION_DAYS:-30}"
COMPRESSION_LEVEL="${COMPRESSION_LEVEL:-6}"

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

# Setup environment
setup_env() {
    # Create backup directory if it doesn't exist
    mkdir -p "$BACKUP_DIR"

    # Export password for pg_dump/pg_restore
    if [ -n "$DB_PASSWORD" ]; then
        export PGPASSWORD="$DB_PASSWORD"
    fi
}

# Generate backup filename
generate_backup_filename() {
    local timestamp=$(date '+%Y%m%d_%H%M%S')
    echo "${BACKUP_DIR}/${DB_NAME}_${timestamp}.sql.gz"
}

# Create database backup
backup_database() {
    local backup_file=$(generate_backup_filename)

    log_info "Starting database backup: $backup_file"

    # Create backup with pg_dump
    pg_dump \
        --host="$DB_HOST" \
        --port="$DB_PORT" \
        --username="$DB_USER" \
        --dbname="$DB_NAME" \
        --no-password \
        --compress="$COMPRESSION_LEVEL" \
        --format=custom \
        --verbose \
        --file="$backup_file"

    # Verify backup file was created and has content
    if [ -s "$backup_file" ]; then
        local size=$(du -h "$backup_file" | cut -f1)
        log_success "Backup completed successfully. Size: $size"
        echo "$backup_file"
    else
        log_error "Backup failed - file is empty or not created"
        exit 1
    fi
}

# List available backups
list_backups() {
    log_info "Available backups in $BACKUP_DIR:"

    if [ ! -d "$BACKUP_DIR" ]; then
        log_error "Backup directory does not exist: $BACKUP_DIR"
        exit 1
    fi

    local count=0
    while IFS= read -r -d '' file; do
        local size=$(du -h "$file" | cut -f1)
        local mtime=$(stat -c '%y' "$file" | cut -d'.' -f1)
        echo "  $file ($size) - $mtime"
        ((count++))
    done < <(find "$BACKUP_DIR" -name "*.sql.gz" -o -name "*.backup" -print0 | sort -z)

    if [ $count -eq 0 ]; then
        log_warn "No backup files found"
    else
        log_info "Found $count backup file(s)"
    fi
}

# Restore database from backup
restore_database() {
    local backup_file="$1"

    if [ -z "$backup_file" ]; then
        log_error "Backup file not specified"
        echo "Usage: $0 restore <backup_file>"
        exit 1
    fi

    if [ ! -f "$backup_file" ]; then
        log_error "Backup file does not exist: $backup_file"
        exit 1
    fi

    log_warn "This will OVERWRITE the current database!"
    read -p "Are you sure you want to continue? (yes/no): " confirm

    if [ "$confirm" != "yes" ]; then
        log_info "Restore cancelled"
        exit 0
    fi

    log_info "Starting database restore from: $backup_file"

    # Terminate active connections (except ours)
    log_info "Terminating active connections..."
    psql \
        --host="$DB_HOST" \
        --port="$DB_PORT" \
        --username="$DB_USER" \
        --dbname="postgres" \
        --no-password \
        --command="SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '$DB_NAME' AND pid <> pg_backend_pid();"

    # Drop and recreate database
    log_info "Recreating database..."
    psql \
        --host="$DB_HOST" \
        --port="$DB_PORT" \
        --username="$DB_USER" \
        --dbname="postgres" \
        --no-password \
        --command="DROP DATABASE IF EXISTS $DB_NAME;"

    psql \
        --host="$DB_HOST" \
        --port="$DB_PORT" \
        --username="$DB_USER" \
        --dbname="postgres" \
        --no-password \
        --command="CREATE DATABASE $DB_NAME;"

    # Restore from backup
    log_info "Restoring data..."
    pg_restore \
        --host="$DB_HOST" \
        --port="$DB_PORT" \
        --username="$DB_USER" \
        --dbname="$DB_NAME" \
        --no-password \
        --verbose \
        "$backup_file"

    log_success "Database restore completed successfully"
}

# Clean up old backups
cleanup_old_backups() {
    log_info "Cleaning up backups older than $RETENTION_DAYS days"

    local deleted_count=0
    while IFS= read -r -d '' file; do
        log_info "Deleting old backup: $file"
        rm -f "$file"
        ((deleted_count++))
    done < <(find "$BACKUP_DIR" -name "*.sql.gz" -o -name "*.backup" -mtime +"$RETENTION_DAYS" -print0)

    if [ $deleted_count -gt 0 ]; then
        log_success "Cleaned up $deleted_count old backup(s)"
    else
        log_info "No old backups to clean up"
    fi
}

# Point-in-time recovery (PITR) setup
setup_pitr() {
    log_info "Setting up Point-in-Time Recovery (PITR)"

    # Enable WAL archiving
    psql \
        --host="$DB_HOST" \
        --port="$DB_PORT" \
        --username="$DB_USER" \
        --dbname="$DB_NAME" \
        --no-password \
        --command="ALTER SYSTEM SET wal_level = replica;"

    psql \
        --host="$DB_HOST" \
        --port="$DB_PORT" \
        --username="$DB_USER" \
        --dbname="$DB_NAME" \
        --no-password \
        --command="ALTER SYSTEM SET archive_mode = on;"

    psql \
        --host="$DB_HOST" \
        --port="$DB_PORT" \
        --username="$DB_USER" \
        --dbname="$DB_NAME" \
        --no-password \
        --command="ALTER SYSTEM SET archive_command = 'cp %p $BACKUP_DIR/wal/%f';"

    # Create WAL archive directory
    mkdir -p "$BACKUP_DIR/wal"

    log_success "PITR setup completed. Reload PostgreSQL configuration."
    log_warn "Run: SELECT pg_reload_conf();"
}

# Export data to anonymized format
export_anonymized() {
    local output_file="$1"

    if [ -z "$output_file" ]; then
        output_file="${BACKUP_DIR}/anonymized_export_$(date '+%Y%m%d_%H%M%S').sql"
    fi

    log_info "Creating anonymized data export: $output_file"

    # Export schema only (no data)
    pg_dump \
        --host="$DB_HOST" \
        --port="$DB_PORT" \
        --username="$DB_USER" \
        --dbname="$DB_NAME" \
        --no-password \
        --schema-only \
        --compress="$COMPRESSION_LEVEL" \
        --format=custom \
        --file="${output_file}_schema"

    # Export anonymized data (sample papers, no PII)
    psql \
        --host="$DB_HOST" \
        --port="$DB_PORT" \
        --username="$DB_USER" \
        --dbname="$DB_NAME" \
        --no-password \
        --command="
        COPY (
            SELECT
                id,
                doi,
                title,
                'REDACTED' as abstract_text,
                source,
                published_at,
                'REDACTED' as authors,
                journal,
                parse_status,
                open_access
            FROM papers
            LIMIT 1000
        ) TO STDOUT WITH CSV HEADER;
        " > "${output_file}_sample_data.csv"

    log_success "Anonymized export completed: ${output_file}_schema, ${output_file}_sample_data.csv"
}

# Main command handling
main() {
    setup_env

    case "${1:-help}" in
        backup)
            backup_database
            ;;
        restore)
            restore_database "${2:-}"
            ;;
        list)
            list_backups
            ;;
        cleanup)
            cleanup_old_backups
            ;;
        pitr-setup)
            setup_pitr
            ;;
        export-anon)
            export_anonymized "${2:-}"
            ;;
        help|*)
            echo "Ferrumyx PostgreSQL Backup/Restore Tool"
            echo ""
            echo "Usage: $0 <command> [options]"
            echo ""
            echo "Commands:"
            echo "  backup              Create a new database backup"
            echo "  restore <file>      Restore database from backup file"
            echo "  list                List available backup files"
            echo "  cleanup             Remove backups older than RETENTION_DAYS"
            echo "  pitr-setup          Setup Point-in-Time Recovery"
            echo "  export-anon [file]  Export anonymized sample data"
            echo "  help                Show this help message"
            echo ""
            echo "Environment Variables:"
            echo "  DB_HOST             Database host (default: localhost)"
            echo "  DB_PORT             Database port (default: 5432)"
            echo "  DB_NAME             Database name (default: ferrumyx)"
            echo "  DB_USER             Database user (default: ferrumyx_backup)"
            echo "  DB_PASSWORD         Database password"
            echo "  BACKUP_DIR          Backup directory (default: /var/backups/ferrumyx)"
            echo "  RETENTION_DAYS      Days to keep backups (default: 30)"
            echo "  COMPRESSION_LEVEL   pg_dump compression level (default: 6)"
            ;;
    esac
}

main "$@"