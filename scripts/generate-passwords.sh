#!/bin/bash

# Ferrumyx Password Generation Utility
# Generates secure random passwords for all services

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Generate a secure random password
generate_password() {
    local length=${1:-32}
    openssl rand -base64 $length | tr -d "=+/" | cut -c1-$length
}

# Generate passwords for all services
generate_all_passwords() {
    log_info "Generating secure passwords for all services..."

    local postgres_pass=$(generate_password 32)
    local readonly_pass=$(generate_password 32)
    local redis_pass=$(generate_password 32)
    local grafana_pass=$(generate_password 16)

    echo "POSTGRES_PASSWORD=$postgres_pass"
    echo "READONLY_PASSWORD=$readonly_pass"
    echo "REDIS_PASSWORD=$redis_pass"
    echo "GRAFANA_ADMIN_PASSWORD=$grafana_pass"

    log_success "Passwords generated successfully"
    log_info "IMPORTANT: Save these passwords securely. They will not be shown again."
}

# Generate password for specific service
generate_service_password() {
    local service=$1
    local length=${2:-32}

    case $service in
        postgres|db)
            echo "POSTGRES_PASSWORD=$(generate_password $length)"
            ;;
        readonly)
            echo "READONLY_PASSWORD=$(generate_password $length)"
            ;;
        redis)
            echo "REDIS_PASSWORD=$(generate_password $length)"
            ;;
        grafana)
            echo "GRAFANA_ADMIN_PASSWORD=$(generate_password $length)"
            ;;
        *)
            log_error "Unknown service: $service"
            echo "Usage: $0 <postgres|readonly|redis|grafana> [length]"
            exit 1
            ;;
    esac
}

# Main function
main() {
    if [[ $# -eq 0 ]]; then
        generate_all_passwords
    elif [[ $# -eq 1 ]]; then
        generate_service_password "$1"
    elif [[ $# -eq 2 ]]; then
        generate_service_password "$1" "$2"
    else
        log_error "Invalid usage"
        echo "Usage: $0 [service] [length]"
        echo "Services: postgres, readonly, redis, grafana"
        echo "If no service specified, generates all passwords"
        exit 1
    fi
}

main "$@"