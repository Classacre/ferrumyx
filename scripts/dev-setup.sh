#!/bin/bash

# Ferrumyx Development Setup Script
# Local development environment setup for Ferrumyx

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

# Check if running in development mode
check_development_mode() {
    if [[ ! -f "docker-compose.dev.yml" ]]; then
        log_error "Development compose file not found. Make sure you're in the project root."
        exit 1
    fi
    log_info "Development mode detected"
}

# Setup development environment variables
setup_dev_environment() {
    log_info "Setting up development environment variables..."

    local env_file=".env.dev"

    if [[ ! -f "$env_file" ]]; then
        log_warn "No .env.dev file found. Creating from template..."

        if [[ -f ".env.example" ]]; then
            cp .env.example .env.dev
            log_info "Copied .env.example to .env.dev"
        else
            log_error ".env.example not found. Please create development environment configuration."
            exit 1
        fi
    fi

    # Override production settings with development-friendly values
    echo "# Development overrides" >> .env.dev
    echo "LOG_LEVEL=debug" >> .env.dev
    echo "BIOCLAW_TOOLS_ENABLED=true" >> .env.dev
    echo "FERRUMYX_DEV_MODE=true" >> .env.dev

    # Use development database settings
    if grep -q "POSTGRES_DB=" .env.dev; then
        sed -i.bak 's/POSTGRES_DB=.*/POSTGRES_DB=ferrumyx_dev/' .env.dev
    fi

    log_success "Development environment variables configured"
}

# Setup development tools
setup_dev_tools() {
    log_info "Setting up development tools..."

    # Install Rust if not present
    if ! command -v cargo &> /dev/null; then
        log_info "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
        log_success "Rust installed"
    else
        log_info "Rust already installed"
    fi

    # Install Node.js dependencies for web UI if present
    if [[ -f "package.json" ]]; then
        log_info "Installing Node.js dependencies..."
        if command -v npm &> /dev/null; then
            npm install
            log_success "Node.js dependencies installed"
        else
            log_warn "npm not found. Skipping Node.js dependencies."
        fi
    fi

    # Setup pre-commit hooks if git repo
    if [[ -d ".git" ]] && [[ -f "crates/ferrumyx-runtime-core/scripts/pre-commit-safety.sh" ]]; then
        log_info "Setting up pre-commit hooks..."
        cp crates/ferrumyx-runtime-core/scripts/pre-commit-safety.sh .git/hooks/pre-commit
        chmod +x .git/hooks/pre-commit
        log_success "Pre-commit hooks configured"
    fi

    log_success "Development tools setup completed"
}

# Setup development database
setup_dev_database() {
    log_info "Setting up development database..."

    # Use development compose file
    export COMPOSE_FILE=docker-compose.dev.yml

    # Start database services
    docker-compose up -d postgres redis

    # Wait for PostgreSQL
    log_info "Waiting for PostgreSQL..."
    local max_attempts=30
    local attempt=1

    while [[ $attempt -le $max_attempts ]]; do
        if docker-compose exec -T postgres pg_isready -U postgres -h localhost >/dev/null 2>&1; then
            log_success "PostgreSQL is ready"
            break
        fi
        log_info "Waiting... (attempt $attempt/$max_attempts)"
        sleep 2
        ((attempt++))
    done

    if [[ $attempt -gt $max_attempts ]]; then
        log_error "PostgreSQL failed to start"
        exit 1
    fi

    # Create development database if it doesn't exist
    docker-compose exec -T postgres psql -U postgres -c "CREATE DATABASE IF NOT EXISTS ferrumyx_dev;" 2>/dev/null || true

    # Run migrations for development
    if [[ -f "run-migrations.sh" ]]; then
        log_info "Running database migrations for development..."
        bash run-migrations.sh
    fi

    log_success "Development database setup completed"
}

# Setup development services
setup_dev_services() {
    log_info "Setting up development services..."

    export COMPOSE_FILE=docker-compose.dev.yml

    # Build and start development services
    docker-compose up -d --build ferrumyx-web ironclaw-agent

    # Wait for services
    log_info "Waiting for services to be ready..."
    local services=("ferrumyx-web" "ironclaw-agent")
    for service in "${services[@]}"; do
        local max_attempts=20
        local attempt=1

        while [[ $attempt -le $max_attempts ]]; do
            if docker-compose ps "$service" | grep -q "Up"; then
                log_success "$service is ready"
                break
            fi
            log_info "Waiting for $service... (attempt $attempt/$max_attempts)"
            sleep 3
            ((attempt++))
        done

        if [[ $attempt -gt $max_attempts ]]; then
            log_error "$service failed to start"
            exit 1
        fi
    done

    log_success "Development services setup completed"
}

# Setup development monitoring (optional)
setup_dev_monitoring() {
    log_info "Setting up development monitoring..."

    export COMPOSE_FILE=docker-compose.dev.yml

    # Ask user if they want monitoring in development
    read -p "Enable monitoring stack in development? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        docker-compose up -d grafana prometheus loki promtail
        log_success "Development monitoring enabled"
    else
        log_info "Development monitoring skipped"
    fi
}

# Setup hot reload for development
setup_hot_reload() {
    log_info "Setting up hot reload for development..."

    # Check if cargo watch is available
    if ! command -v cargo-watch &> /dev/null; then
        log_info "Installing cargo-watch for hot reload..."
        cargo install cargo-watch
    fi

    # Create development watch script
    cat > scripts/dev-watch.sh << 'EOF'
#!/bin/bash
# Development watch script for hot reload

echo "Starting development watch mode..."
echo "Press Ctrl+C to stop"

# Watch Rust code changes
cargo watch -x 'run --bin ferrumyx-agent' &
RUST_PID=$!

# Watch for other changes as needed
# Add more watch commands here

# Wait for processes
wait $RUST_PID
EOF

    chmod +x scripts/dev-watch.sh
    log_success "Hot reload setup completed"
}

# Display development setup summary
display_dev_summary() {
    log_success "Ferrumyx development setup completed!"
    echo
    echo "========================================"
    echo "Development Environment Ready"
    echo "========================================"
    echo "✓ Development environment variables configured"
    echo "✓ Development tools installed"
    echo "✓ Development database initialized"
    echo "✓ Development services running"
    echo "✓ Hot reload configured"
    echo
    echo "Services available at:"
    echo "- Ferrumyx Web UI: http://localhost:3000"
    echo "- Development Database: localhost:5432/ferrumyx_dev"
    echo
    echo "Development commands:"
    echo "- Start all services: docker-compose -f docker-compose.dev.yml up -d"
    echo "- Stop all services: docker-compose -f docker-compose.dev.yml down"
    echo "- View logs: docker-compose -f docker-compose.dev.yml logs -f"
    echo "- Hot reload: bash scripts/dev-watch.sh"
    echo "- Run tests: cargo test"
    echo "- Build release: cargo build --release"
    echo
    echo "For production deployment, run: bash scripts/deploy.sh"
    echo "========================================"
}

# Main development setup function
main() {
    echo "========================================"
    echo "Ferrumyx Development Setup"
    echo "========================================"
    echo

    check_development_mode
    setup_dev_environment
    setup_dev_tools
    setup_dev_database
    setup_dev_services
    setup_dev_monitoring
    setup_hot_reload
    display_dev_summary
}

# Run main function
main "$@"