#!/bin/bash

# Ferrumyx Secrets Generation Script
# Generates Docker secrets for production deployment

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
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

# Create secrets directory
create_secrets_dir() {
    local secrets_dir="./secrets"
    if [[ ! -d "$secrets_dir" ]]; then
        mkdir -p "$secrets_dir"
        chmod 700 "$secrets_dir"
        log_info "Created secrets directory: $secrets_dir"
    fi
    echo "$secrets_dir"
}

# Generate database password secret
generate_db_password() {
    local secrets_dir=$(create_secrets_dir)
    local password_file="$secrets_dir/db_password.txt"

    if [[ -f "$password_file" ]]; then
        log_warn "Database password secret already exists: $password_file"
        read -p "Overwrite? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            return
        fi
    fi

    local password=$(generate_password 32)
    echo -n "$password" > "$password_file"
    chmod 600 "$password_file"
    log_success "Generated database password secret: $password_file"
}

# Generate Redis password secret
generate_redis_password() {
    local secrets_dir=$(create_secrets_dir)
    local password_file="$secrets_dir/redis_password.txt"

    if [[ -f "$password_file" ]]; then
        log_warn "Redis password secret already exists: $password_file"
        read -p "Overwrite? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            return
        fi
    fi

    local password=$(generate_password 32)
    echo -n "$password" > "$password_file"
    chmod 600 "$password_file"
    log_success "Generated Redis password secret: $password_file"
}

# Generate API keys secret
generate_api_keys() {
    local secrets_dir=$(create_secrets_dir)
    local keys_file="$secrets_dir/api_keys.json"

    if [[ -f "$keys_file" ]]; then
        log_warn "API keys secret already exists: $keys_file"
        read -p "Overwrite? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            return
        fi
    fi

    cat > "$keys_file" << EOF
{
  "openai_api_key": "",
  "anthropic_api_key": "",
  "ironclaw_api_key": "",
  "slack_bot_token": "",
  "discord_bot_token": "",
  "webhook_secret": "",
  "pubmed_api_key": "",
  "europepmc_api_key": ""
}
EOF
    chmod 600 "$keys_file"
    log_success "Generated API keys template: $keys_file"
    log_warn "Please fill in the API keys in $keys_file"
}

# Generate webhook secret
generate_webhook_secret() {
    local secrets_dir=$(create_secrets_dir)
    local secret_file="$secrets_dir/webhook_secret.txt"

    if [[ -f "$secret_file" ]]; then
        log_warn "Webhook secret already exists: $secret_file"
        read -p "Overwrite? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            return
        fi
    fi

    local secret=$(generate_password 64)
    echo -n "$secret" > "$secret_file"
    chmod 600 "$secret_file"
    log_success "Generated webhook secret: $secret_file"
}

# Generate SSL certificates (placeholder)
generate_ssl_certs() {
    local secrets_dir=$(create_secrets_dir)
    local cert_file="$secrets_dir/ssl_cert.pem"
    local key_file="$secrets_dir/ssl_key.pem"

    if [[ -f "$cert_file" || -f "$key_file" ]]; then
        log_warn "SSL certificates already exist"
        return
    fi

    log_warn "SSL certificate generation not implemented. Please provide your own certificates:"
    log_warn "  Certificate: $cert_file"
    log_warn "  Private key: $key_file"
}

# Generate Grafana admin password
generate_grafana_password() {
    local secrets_dir=$(create_secrets_dir)
    local password_file="$secrets_dir/grafana_admin_password.txt"

    if [[ -f "$password_file" ]]; then
        log_warn "Grafana admin password secret already exists: $password_file"
        read -p "Overwrite? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            return
        fi
    fi

    local password=$(generate_password 16)
    echo -n "$password" > "$password_file"
    chmod 600 "$password_file"
    log_success "Generated Grafana admin password secret: $password_file"
}

# Generate all secrets
generate_all_secrets() {
    log_info "Generating all Docker secrets for production..."

    generate_db_password
    generate_redis_password
    generate_api_keys
    generate_webhook_secret
    generate_ssl_certs
    generate_grafana_password

    log_success "All secrets generated successfully"
    echo
    log_info "Next steps:"
    log_info "1. Fill in API keys in ./secrets/api_keys.json"
    log_info "2. Obtain SSL certificates and place them in ./secrets/"
    log_info "3. Use docker-compose.prod.yml for production deployment"
}

# Main function
main() {
    case "${1:-all}" in
        all)
            generate_all_secrets
            ;;
        db|database)
            generate_db_password
            ;;
        redis)
            generate_redis_password
            ;;
        api-keys)
            generate_api_keys
            ;;
        webhook)
            generate_webhook_secret
            ;;
        ssl)
            generate_ssl_certs
            ;;
        grafana)
            generate_grafana_password
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Usage: $0 [all|db|redis|api-keys|webhook|ssl|grafana]"
            exit 1
            ;;
    esac
}

main "$@"