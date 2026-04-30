#!/bin/bash

# Security scanning script for Ferrumyx Docker images
# Requires Trivy: https://github.com/aquasecurity/trivy

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Images to scan
IMAGES=(
    "ferrumyx-web"
    "ferrumyx-agent"
    "ferrumyx-ingestion"
    "ferrumyx-kg"
    "ferrumyx-ranker"
    "ferrumyx-molecules"
    "ferrumyx-postgres"
    "ferrumyx-redis"
    "ferrumyx-webui"
)

# Check if Trivy is installed
if ! command -v trivy &> /dev/null; then
    echo -e "${RED}Error: Trivy is not installed.${NC}"
    echo "Install Trivy from: https://github.com/aquasecurity/trivy"
    echo "Or run: curl -sfL https://raw.githubusercontent.com/aquasecurity/trivy/main/contrib/install.sh | sh -"
    exit 1
fi

echo "Starting security scan of Ferrumyx Docker images..."
echo "=================================================="

FAILED_SCANS=()
PASSED_SCANS=()

for image in "${IMAGES[@]}"; do
    echo -e "\n${YELLOW}Scanning image: $image${NC}"

    # Build image if needed (assuming docker-compose or build context)
    if ! docker image inspect "$image" &> /dev/null; then
        echo "Image $image not found locally. Attempting to build..."
        case $image in
            ferrumyx-web)
                docker build -f crates/ferrumyx-web/Dockerfile -t "$image" .
                ;;
            ferrumyx-agent)
                docker build -f crates/ferrumyx-agent/Dockerfile -t "$image" .
                ;;
            ferrumyx-ingestion)
                docker build -f crates/ferrumyx-ingestion/Dockerfile -t "$image" .
                ;;
            ferrumyx-kg)
                docker build -f crates/ferrumyx-kg/Dockerfile -t "$image" .
                ;;
            ferrumyx-ranker)
                docker build -f crates/ferrumyx-ranker/Dockerfile -t "$image" .
                ;;
            ferrumyx-molecules)
                docker build -f crates/ferrumyx-molecules/Dockerfile -t "$image" .
                ;;
            ferrumyx-postgres)
                docker build -f Dockerfile.postgres -t "$image" .
                ;;
            ferrumyx-redis)
                docker build -f Dockerfile.redis -t "$image" .
                ;;
            ferrumyx-webui)
                docker build -f Dockerfile.webui -t "$image" .
                ;;
            *)
                echo -e "${RED}Unknown image: $image${NC}"
                continue
                ;;
        esac
    fi

    # Run Trivy scan
    if trivy image --exit-code 1 --severity HIGH,CRITICAL "$image" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ $image passed security scan${NC}"
        PASSED_SCANS+=("$image")
    else
        echo -e "${RED}✗ $image failed security scan${NC}"
        FAILED_SCANS+=("$image")

        # Show details for failed scan
        echo "Vulnerabilities found:"
        trivy image --severity HIGH,CRITICAL "$image" | grep -E "(HIGH|CRITICAL)"
    fi
done

echo ""
echo "=================================================="
echo "Security Scan Summary"
echo "=================================================="
echo "Passed: ${#PASSED_SCANS[@]} images"
echo "Failed: ${#FAILED_SCANS[@]} images"

if [ ${#FAILED_SCANS[@]} -gt 0 ]; then
    echo -e "\n${RED}Failed images:${NC}"
    for img in "${FAILED_SCANS[@]}"; do
        echo "  - $img"
    done
    echo -e "\n${YELLOW}Recommendation: Fix vulnerabilities before deployment${NC}"
    exit 1
else
    echo -e "\n${GREEN}All images passed security scanning!${NC}"
fi