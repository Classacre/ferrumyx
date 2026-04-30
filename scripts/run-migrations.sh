#!/bin/bash
# run-migrations.sh - Ferrumyx Database Migration Runner
# Applies pending migrations in order

set -euo pipefail

# Configuration
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-ferrumyx}"
DB_USER="${DB_USER:-ferrumyx}"
DB_PASSWORD="${DB_PASSWORD:-}"
MIGRATIONS_DIR="${MIGRATIONS_DIR:-./migrations}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $(date '+%Y-%m-%d %H:%M:%S') - $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $(date '+%Y-%m-%d %H:%M:%S') - $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $(date '+%Y-%m-%d %H:%M:%S') - $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $(date '+%Y-%m-%d %H:%M:%S') - $1"
}

# Check if psql is available
check_dependencies() {
    if ! command -v psql >/dev/null 2>&1; then
        log_error "psql command not found. Please install PostgreSQL client."
        exit 1
    fi
}

# Get list of applied migrations
get_applied_migrations() {
    psql \
        --host="$DB_HOST" \
        --port="$DB_PORT" \
        --username="$DB_USER" \
        --dbname="$DB_NAME" \
        --no-password \
        --tuples-only \
        --command="
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version VARCHAR(255) PRIMARY KEY,
            applied_at TIMESTAMPTZ DEFAULT NOW()
        );
        SELECT version FROM schema_migrations ORDER BY version;
        " 2>/dev/null || echo ""
}

# Apply a single migration
apply_migration() {
    local migration_file="$1"
    local version=$(basename "$migration_file" | sed 's/\..*//')

    log_info "Applying migration: $version"

    # Check if already applied
    local applied=$(psql \
        --host="$DB_HOST" \
        --port="$DB_PORT" \
        --username="$DB_USER" \
        --dbname="$DB_NAME" \
        --no-password \
        --tuples-only \
        --command="SELECT version FROM schema_migrations WHERE version = '$version';" 2>/dev/null || echo "")

    if [ -n "$applied" ]; then
        log_warn "Migration $version already applied, skipping"
        return 0
    fi

    # Apply migration
    if psql \
        --host="$DB_HOST" \
        --port="$DB_PORT" \
        --username="$DB_USER" \
        --dbname="$DB_NAME" \
        --no-password \
        --file="$migration_file"; then

        # Record migration as applied
        psql \
            --host="$DB_HOST" \
            --port="$DB_PORT" \
            --username="$DB_USER" \
            --dbname="$DB_NAME" \
            --no-password \
            --command="INSERT INTO schema_migrations (version) VALUES ('$version');"

        log_success "Migration $version applied successfully"
    else
        log_error "Failed to apply migration $version"
        return 1
    fi
}

# Rollback migration
rollback_migration() {
    local version="$1"

    log_info "Rolling back migration: $version"

    # This is a simplified rollback - in production you'd want specific rollback scripts
    log_warn "Simplified rollback - only removing migration record"
    psql \
        --host="$DB_HOST" \
        --port="$DB_PORT" \
        --username="$DB_USER" \
        --dbname="$DB_NAME" \
        --no-password \
        --command="DELETE FROM schema_migrations WHERE version = '$version';"

    log_success "Migration $version rolled back"
}

# Main migration runner
run_migrations() {
    local applied_migrations=$(get_applied_migrations)

    log_info "Checking for pending migrations in $MIGRATIONS_DIR"

    local applied_count=0
    local pending_count=0

    # Find and apply migrations in order
    while IFS= read -r -d '' migration_file; do
        local version=$(basename "$migration_file" | sed 's/\..*//')

        if echo "$applied_migrations" | grep -q "^$version$"; then
            ((applied_count++))
        else
            if apply_migration "$migration_file"; then
                ((pending_count++))
            else
                log_error "Migration failed: $version"
                exit 1
            fi
        fi
    done < <(find "$MIGRATIONS_DIR" -name "*.sql" -print0 | sort -z)

    log_success "Migration complete: $applied_count already applied, $pending_count newly applied"
}

# Show migration status
show_status() {
    local applied_migrations=$(get_applied_migrations)

    echo "Migration Status:"
    echo "=================="

    local count=0
    while IFS= read -r -d '' migration_file; do
        local version=$(basename "$migration_file" | sed 's/\..*//')
        local status="PENDING"

        if echo "$applied_migrations" | grep -q "^$version$"; then
            status="APPLIED"
        fi

        printf "%-20s [%s]\n" "$version" "$status"
        ((count++))
    done < <(find "$MIGRATIONS_DIR" -name "*.sql" -print0 | sort -z)

    echo ""
    echo "Total migrations: $count"
}

# Export password for psql
export PGPASSWORD="$DB_PASSWORD"

# Main command handling
main() {
    check_dependencies

    case "${1:-status}" in
        run)
            run_migrations
            ;;
        status)
            show_status
            ;;
        rollback)
            if [ -z "${2:-}" ]; then
                log_error "Migration version required for rollback"
                echo "Usage: $0 rollback <version>"
                exit 1
            fi
            rollback_migration "$2"
            ;;
        *)
            echo "Ferrumyx Migration Runner"
            echo ""
            echo "Usage: $0 <command> [options]"
            echo ""
            echo "Commands:"
            echo "  run                 Apply pending migrations"
            echo "  status              Show migration status"
            echo "  rollback <version>  Rollback a specific migration"
            echo ""
            echo "Environment Variables:"
            echo "  DB_HOST             Database host (default: localhost)"
            echo "  DB_PORT             Database port (default: 5432)"
            echo "  DB_NAME             Database name (default: ferrumyx)"
            echo "  DB_USER             Database user (default: ferrumyx)"
            echo "  DB_PASSWORD         Database password"
            echo "  MIGRATIONS_DIR      Migrations directory (default: ./migrations)"
            ;;
    esac
}

main "$@"