#!/bin/bash

# Ferrumyx Database Migration Script
# Run database migrations for Ferrumyx

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

# Check database connectivity
check_database_connection() {
    log_info "Checking database connectivity..."

    local max_attempts=30
    local attempt=1

    while [[ $attempt -le $max_attempts ]]; do
        if docker-compose exec -T postgres pg_isready -U "$POSTGRES_USER" -h localhost >/dev/null 2>&1; then
            log_success "Database connection established"
            return 0
        fi
        log_info "Waiting for database... (attempt $attempt/$max_attempts)"
        sleep 2
        ((attempt++))
    done

    log_error "Failed to connect to database after $max_attempts attempts"
    exit 1
}

# Run SQL migrations
run_sql_migrations() {
    log_info "Running SQL migrations..."

    # Create migrations table if it doesn't exist
    docker-compose exec -T postgres psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c "
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version VARCHAR(255) PRIMARY KEY,
            applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );
    " 2>/dev/null || log_warn "Could not create migrations table"

    # Run migrations from sql files
    local migrations_dir="migrations"
    if [[ -d "$migrations_dir" ]]; then
        for migration_file in "$migrations_dir"/*.sql; do
            if [[ -f "$migration_file" ]]; then
                local migration_name=$(basename "$migration_file" .sql)
                local version=$(echo "$migration_name" | cut -d'_' -f1)

                # Check if migration already applied
                local applied=$(docker-compose exec -T postgres psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -t -c "
                    SELECT COUNT(*) FROM schema_migrations WHERE version = '$version';
                " 2>/dev/null || echo "0")

                if [[ "$applied" -eq 0 ]]; then
                    log_info "Applying migration: $migration_name"
                    docker-compose exec -T postgres psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -f "$migration_file"

                    # Record migration
                    docker-compose exec -T postgres psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c "
                        INSERT INTO schema_migrations (version) VALUES ('$version');
                    "
                    log_success "Migration $migration_name applied successfully"
                else
                    log_info "Migration $migration_name already applied"
                fi
            fi
        done
    else
        log_warn "Migrations directory '$migrations_dir' not found"
    fi
}

# Run Rust-based migrations (if using refinery or similar)
run_rust_migrations() {
    log_info "Running Rust-based migrations..."

    # Check if refinery migrations exist
    if [[ -d "crates/ferrumyx-runtime-core/migrations" ]]; then
        log_info "Found refinery migrations in ferrumyx-runtime-core"

        # Run migrations using refinery CLI if available
        if command -v refinery &> /dev/null; then
            export DATABASE_URL
            refinery migrate -c crates/ferrumyx-runtime-core/refinery.toml -p crates/ferrumyx-runtime-core/migrations
            log_success "Rust migrations completed"
        else
            log_warn "refinery CLI not found. Skipping Rust migrations."
            log_info "To run Rust migrations manually: cargo install refinery_cli && refinery migrate ..."
        fi
    else
        log_info "No Rust migrations found"
    fi
}

# Run custom migration scripts
run_custom_migrations() {
    log_info "Running custom migration scripts..."

    local custom_dir="migrations/custom"
    if [[ -d "$custom_dir" ]]; then
        for script in "$custom_dir"/*.sh; do
            if [[ -f "$script" ]]; then
                log_info "Running custom migration: $(basename "$script")"
                bash "$script"
                log_success "Custom migration $(basename "$script") completed"
            fi
        done
    else
        log_info "No custom migration scripts found"
    fi
}

# Validate migration success
validate_migrations() {
    log_info "Validating migration success..."

    # Basic validation - check if we can connect and run a simple query
    if docker-compose exec -T postgres psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c "SELECT 1;" >/dev/null 2>&1; then
        log_success "Database connectivity validated"

        # Check schema_migrations table
        local migration_count=$(docker-compose exec -T postgres psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -t -c "
            SELECT COUNT(*) FROM schema_migrations;
        " 2>/dev/null || echo "0")

        log_info "Total migrations applied: $migration_count"
    else
        log_error "Database validation failed"
        exit 1
    fi
}

# Display migration summary
display_migration_summary() {
    log_success "Database migrations completed successfully!"

    echo
    echo "========================================"
    echo "Migration Summary"
    echo "========================================"
    echo "Database: $POSTGRES_DB"
    echo "Host: postgres:5432"
    echo "User: $POSTGRES_USER"
    echo

    # Show recent migrations
    echo "Recent migrations:"
    docker-compose exec -T postgres psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c "
        SELECT version, applied_at FROM schema_migrations
        ORDER BY applied_at DESC LIMIT 5;
    " 2>/dev/null || echo "No migration history available"

    echo
    echo "For database backup: bash scripts/db-backup.sh"
    echo "========================================"
}

# Main migration function
main() {
    echo "========================================"
    echo "Ferrumyx Database Migration"
    echo "========================================"
    echo

    load_environment
    check_database_connection
    run_sql_migrations
    run_rust_migrations
    run_custom_migrations
    validate_migrations
    display_migration_summary
}

# Run main function
main "$@"