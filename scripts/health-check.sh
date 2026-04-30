#!/bin/bash

# Ferrumyx Health Check Script
# Comprehensive system health validation

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

# Configuration
QUIET_MODE=false
DETAILED_MODE=false
JSON_OUTPUT=false
EXIT_ON_FIRST_FAILURE=false

# Health check results
declare -A CHECK_RESULTS
TOTAL_CHECKS=0
PASSED_CHECKS=0
FAILED_CHECKS=0

# Parse command line arguments
parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --quiet|-q)
                QUIET_MODE=true
                shift
                ;;
            --detailed|-d)
                DETAILED_MODE=true
                shift
                ;;
            --json)
                JSON_OUTPUT=true
                shift
                ;;
            --fail-fast)
                EXIT_ON_FIRST_FAILURE=true
                shift
                ;;
            --help)
                echo "Usage: $0 [OPTIONS]"
                echo
                echo "Options:"
                echo "  --quiet, -q         Suppress detailed output"
                echo "  --detailed, -d      Show detailed check information"
                echo "  --json              Output results in JSON format"
                echo "  --fail-fast         Exit on first failure"
                echo "  --help              Show this help message"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
}

# Record check result
record_check() {
    local check_name=$1
    local status=$2
    local message=$3
    local details=$4

    CHECK_RESULTS["$check_name"]="$status|$message|$details"
    ((TOTAL_CHECKS++))

    if [[ "$status" == "PASS" ]]; then
        ((PASSED_CHECKS++))
    else
        ((FAILED_CHECKS++))
    fi

    if [[ "$QUIET_MODE" == "false" ]]; then
        if [[ "$status" == "PASS" ]]; then
            log_success "$check_name: $message"
        elif [[ "$status" == "WARN" ]]; then
            log_warn "$check_name: $message"
        else
            log_error "$check_name: $message"
        fi

        if [[ "$DETAILED_MODE" == "true" ]] && [[ -n "$details" ]]; then
            echo "  Details: $details"
        fi
    fi

    if [[ "$EXIT_ON_FIRST_FAILURE" == "true" ]] && [[ "$status" == "FAIL" ]]; then
        log_error "Exiting on first failure as requested"
        exit 1
    fi
}

# Docker infrastructure checks
check_docker_infrastructure() {
    log_info "Checking Docker infrastructure..."

    # Check Docker daemon
    if ! docker info >/dev/null 2>&1; then
        record_check "docker_daemon" "FAIL" "Docker daemon not accessible"
        return
    fi
    record_check "docker_daemon" "PASS" "Docker daemon accessible"

    # Check required networks
    local networks=("ferrumyx-network" "ferrumyx-monitoring")
    for network in "${networks[@]}"; do
        if docker network ls --format "{{.Name}}" | grep -q "^${network}$"; then
            record_check "network_${network}" "PASS" "Network $network exists"
        else
            record_check "network_${network}" "FAIL" "Network $network missing"
        fi
    done

    # Check required volumes
    local volumes=("postgres_data" "redis_data" "grafana_data" "prometheus_data")
    for volume in "${volumes[@]}"; do
        if docker volume ls --format "{{.Name}}" | grep -q "^${volume}$"; then
            record_check "volume_${volume}" "PASS" "Volume $volume exists"
        else
            record_check "volume_${volume}" "WARN" "Volume $volume missing"
        fi
    done
}

# Service health checks
check_service_health() {
    log_info "Checking service health..."

    # Core services to check
    local services=("postgres" "redis" "ferrumyx-web" "ironclaw-agent")

    for service in "${services[@]}"; do
        if docker-compose ps "$service" --format "{{.Status}}" | grep -q "Up"; then
            record_check "service_${service}_status" "PASS" "Service $service is running"
        else
            record_check "service_${service}_status" "FAIL" "Service $service is not running"
            continue
        fi

        # Service-specific health checks
        case $service in
            "postgres")
                if docker-compose exec -T postgres pg_isready -U postgres -h localhost >/dev/null 2>&1; then
                    record_check "service_postgres_connectivity" "PASS" "PostgreSQL accepting connections"
                else
                    record_check "service_postgres_connectivity" "FAIL" "PostgreSQL not accepting connections"
                fi
                ;;
            "redis")
                if docker-compose exec -T redis redis-cli ping | grep -q "PONG"; then
                    record_check "service_redis_connectivity" "PASS" "Redis responding to ping"
                else
                    record_check "service_redis_connectivity" "FAIL" "Redis not responding"
                fi
                ;;
            "ferrumyx-web")
                if curl -f -s --max-time 10 http://localhost:3000/health >/dev/null 2>&1; then
                    record_check "service_web_health" "PASS" "Web service health endpoint responding"
                else
                    record_check "service_web_health" "FAIL" "Web service health endpoint not responding"
                fi
                ;;
            "ironclaw-agent")
                # Check if agent container is healthy (basic check)
                if docker-compose ps ironclaw-agent --format "{{.Status}}" | grep -q "healthy\|running"; then
                    record_check "service_agent_status" "PASS" "Agent service healthy"
                else
                    record_check "service_agent_status" "WARN" "Agent service status uncertain"
                fi
                ;;
        esac
    done
}

# Monitoring stack checks
check_monitoring_health() {
    log_info "Checking monitoring health..."

    # Monitoring services to check
    local monitoring_services=("grafana" "prometheus" "loki" "alertmanager")

    for service in "${monitoring_services[@]}"; do
        if docker-compose ps "$service" --format "{{.Status}}" | grep -q "Up"; then
            record_check "monitoring_${service}_status" "PASS" "Monitoring service $service is running"
        else
            record_check "monitoring_${service}_status" "WARN" "Monitoring service $service not running"
            continue
        fi

        # Service-specific checks
        case $service in
            "grafana")
                if curl -f -s --max-time 5 http://localhost:3001/api/health >/dev/null 2>&1; then
                    record_check "monitoring_grafana_health" "PASS" "Grafana health endpoint responding"
                else
                    record_check "monitoring_grafana_health" "FAIL" "Grafana health endpoint not responding"
                fi
                ;;
            "prometheus")
                if curl -f -s --max-time 5 http://localhost:9090/-/ready >/dev/null 2>&1; then
                    record_check "monitoring_prometheus_health" "PASS" "Prometheus ready"
                else
                    record_check "monitoring_prometheus_health" "FAIL" "Prometheus not ready"
                fi
                ;;
            "loki")
                if curl -f -s --max-time 5 http://localhost:3100/ready >/dev/null 2>&1; then
                    record_check "monitoring_loki_health" "PASS" "Loki ready"
                else
                    record_check "monitoring_loki_health" "FAIL" "Loki not ready"
                fi
                ;;
        esac
    done

    # Check if metrics are being collected
    if curl -f -s --max-time 5 http://localhost:9090/api/v1/targets >/dev/null 2>&1; then
        local targets_up=$(curl -s http://localhost:9090/api/v1/targets 2>/dev/null | grep -o '"up":1' | wc -l)
        record_check "monitoring_metrics_collection" "PASS" "Metrics collection active" "$targets_up targets reporting"
    else
        record_check "monitoring_metrics_collection" "WARN" "Metrics collection status unknown"
    fi
}

# Database health checks
check_database_health() {
    log_info "Checking database health..."

    # Check database connectivity
    if ! docker-compose exec -T postgres pg_isready -U postgres -h localhost >/dev/null 2>&1; then
        record_check "database_connectivity" "FAIL" "Cannot connect to database"
        return
    fi

    # Check database size and connections
    local db_stats=$(docker-compose exec -T postgres psql -U postgres -d ferrumyx -c "
        SELECT
            pg_size_pretty(pg_database_size(current_database())) as db_size,
            (SELECT count(*) FROM pg_stat_activity) as connections,
            (SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public') as tables;
    " 2>/dev/null)

    if [[ $? -eq 0 ]]; then
        local db_size=$(echo "$db_stats" | sed -n '3p' | awk '{print $1}')
        local connections=$(echo "$db_stats" | sed -n '4p' | awk '{print $1}')
        local tables=$(echo "$db_stats" | sed -n '5p' | awk '{print $1}')

        record_check "database_stats" "PASS" "Database operational" "Size: $db_size, Connections: $connections, Tables: $tables"

        # Warn if too many connections
        if [[ $connections -gt 50 ]]; then
            record_check "database_connections" "WARN" "High connection count" "$connections active connections"
        else
            record_check "database_connections" "PASS" "Connection count normal" "$connections active connections"
        fi
    else
        record_check "database_stats" "FAIL" "Cannot retrieve database statistics"
    fi

    # Check for long-running queries
    local long_queries=$(docker-compose exec -T postgres psql -U postgres -d ferrumyx -c "
        SELECT count(*) FROM pg_stat_activity
        WHERE state = 'active' AND now() - query_start > interval '5 minutes';
    " 2>/dev/null | sed -n '3p' | awk '{print $1}')

    if [[ $long_queries -gt 0 ]]; then
        record_check "database_long_queries" "WARN" "Long-running queries detected" "$long_queries queries running >5 minutes"
    else
        record_check "database_long_queries" "PASS" "No long-running queries"
    fi
}

# Application health checks
check_application_health() {
    log_info "Checking application health..."

    # Check main application endpoint
    if curl -f -s --max-time 10 http://localhost:3000/health >/dev/null 2>&1; then
        record_check "application_main_endpoint" "PASS" "Main application endpoint responding"
    else
        record_check "application_main_endpoint" "FAIL" "Main application endpoint not responding"
    fi

    # Check API endpoints if available
    local api_endpoints=("/api/v1/health" "/api/v1/status")
    for endpoint in "${api_endpoints[@]}"; do
        if curl -f -s --max-time 5 "http://localhost:3000$endpoint" >/dev/null 2>&1; then
            record_check "application_api_${endpoint//\//_}" "PASS" "API endpoint $endpoint responding"
        else
            record_check "application_api_${endpoint//\//_}" "WARN" "API endpoint $endpoint not available"
        fi
    done

    # Check for error logs in containers
    local containers=("ferrumyx-web" "ironclaw-agent")
    for container in "${containers[@]}"; do
        local error_count=$(docker-compose logs --tail=100 "$container" 2>&1 | grep -i error | wc -l)
        if [[ $error_count -gt 0 ]]; then
            record_check "application_logs_${container}" "WARN" "Errors found in $container logs" "$error_count error messages in last 100 lines"
        else
            record_check "application_logs_${container}" "PASS" "No recent errors in $container logs"
        fi
    done
}

# System resource checks
check_system_resources() {
    log_info "Checking system resources..."

    # Check disk space
    local disk_usage=$(df / | tail -1 | awk '{print $5}' | sed 's/%//')
    if [[ $disk_usage -gt 90 ]]; then
        record_check "system_disk_space" "FAIL" "Critical disk space usage" "${disk_usage}% used"
    elif [[ $disk_usage -gt 80 ]]; then
        record_check "system_disk_space" "WARN" "High disk space usage" "${disk_usage}% used"
    else
        record_check "system_disk_space" "PASS" "Disk space usage normal" "${disk_usage}% used"
    fi

    # Check memory usage
    local memory_usage=$(free | grep Mem | awk '{printf "%.0f", $3/$2 * 100.0}')
    if [[ $memory_usage -gt 95 ]]; then
        record_check "system_memory_usage" "FAIL" "Critical memory usage" "${memory_usage}% used"
    elif [[ $memory_usage -gt 85 ]]; then
        record_check "system_memory_usage" "WARN" "High memory usage" "${memory_usage}% used"
    else
        record_check "system_memory_usage" "PASS" "Memory usage normal" "${memory_usage}% used"
    fi

    # Check Docker resource usage
    if command -v docker &> /dev/null; then
        local container_count=$(docker ps | wc -l)
        ((container_count--)) # Subtract header line
        record_check "docker_containers" "PASS" "Docker containers running" "$container_count containers active"
    fi
}

# Generate JSON output
generate_json_output() {
    echo "{"
    echo '  "timestamp": "'$(date -Iseconds)'",'
    echo '  "total_checks": '$TOTAL_CHECKS','
    echo '  "passed_checks": '$PASSED_CHECKS','
    echo '  "failed_checks": '$FAILED_CHECKS','
    echo '  "results": {'

    local first=true
    for check_name in "${!CHECK_RESULTS[@]}"; do
        if [[ "$first" == "true" ]]; then
            first=false
        else
            echo ","
        fi

        IFS='|' read -r status message details <<< "${CHECK_RESULTS[$check_name]}"
        echo -n '    "'$check_name'": {'
        echo -n '"status": "'$status'", '
        echo -n '"message": "'$message'"'
        if [[ -n "$details" ]]; then
            echo -n ', "details": "'$details'"'
        fi
        echo -n '}'
    done
    echo
    echo "  }"
    echo "}"
}

# Display summary
display_summary() {
    local health_percentage=$((PASSED_CHECKS * 100 / TOTAL_CHECKS))

    echo
    echo "========================================"
    echo "Ferrumyx Health Check Summary"
    echo "========================================"
    echo "Total checks: $TOTAL_CHECKS"
    echo "Passed: $PASSED_CHECKS"
    echo "Failed: $FAILED_CHECKS"
    echo "Health score: ${health_percentage}%"
    echo

    if [[ $FAILED_CHECKS -eq 0 ]]; then
        log_success "All health checks passed!"
    elif [[ $health_percentage -ge 80 ]]; then
        log_warn "Most health checks passed ($health_percentage%)"
    else
        log_error "Multiple health check failures detected ($health_percentage%)"
    fi

    echo "========================================"
}

# Main health check function
main() {
    if [[ "$QUIET_MODE" == "false" ]]; then
        echo "========================================"
        echo "Ferrumyx Health Check"
        echo "========================================"
        echo
    fi

    parse_arguments "$@"

    check_docker_infrastructure
    check_service_health
    check_monitoring_health
    check_database_health
    check_application_health
    check_system_resources

    if [[ "$JSON_OUTPUT" == "true" ]]; then
        generate_json_output
    else
        display_summary
    fi

    # Exit with appropriate code
    if [[ $FAILED_CHECKS -gt 0 ]]; then
        exit 1
    else
        exit 0
    fi
}

# Run main function
main "$@"