#!/bin/bash
# Build optimization script for Ferrumyx Web Interface
# This script handles asset optimization, minification, and bundling

set -e

echo "🚀 Starting Ferrumyx Web Interface build optimization..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
SOURCE_DIR="crates/ferrumyx-web"
BUILD_DIR="dist"
STATIC_DIR="$SOURCE_DIR/static"
TEMPLATES_DIR="$SOURCE_DIR/templates"

# Clean build directory
echo -e "${YELLOW}Cleaning build directory...${NC}"
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

# Copy static assets
echo -e "${GREEN}Copying static assets...${NC}"
if [ -d "$STATIC_DIR" ]; then
    cp -r "$STATIC_DIR"/* "$BUILD_DIR"/
else
    echo -e "${RED}Warning: Static directory not found${NC}"
fi

# Copy templates
echo -e "${GREEN}Copying templates...${NC}"
if [ -d "$TEMPLATES_DIR" ]; then
    cp -r "$TEMPLATES_DIR"/* "$BUILD_DIR"/
else
    echo -e "${RED}Warning: Templates directory not found${NC}"
fi

# TODO: Add asset optimization steps here when needed
# Examples:
# - CSS minification with cssnano/postcss
# - JavaScript minification with terser
# - Image optimization with imagemin
# - HTML minification
# - Asset versioning/fingerprinting

# For now, just ensure proper file permissions
echo -e "${GREEN}Setting file permissions...${NC}"
find "$BUILD_DIR" -type f -exec chmod 644 {} \;
find "$BUILD_DIR" -type d -exec chmod 755 {} \;

# Generate cache-busting version for assets
# This is a simple approach - in production, consider more robust versioning
if [ -f "$BUILD_DIR/css/main.css" ]; then
    CSS_VERSION=$(md5sum "$BUILD_DIR/css/main.css" | cut -d' ' -f1 | cut -c1-8)
    echo -e "${GREEN}CSS version: $CSS_VERSION${NC}"
fi

if [ -f "$BUILD_DIR/js/main.js" ]; then
    JS_VERSION=$(md5sum "$BUILD_DIR/js/main.js" | cut -d' ' -f1 | cut -c1-8)
    echo -e "${GREEN}JS version: $JS_VERSION${NC}"
fi

# Create .env template for runtime configuration
cat > "$BUILD_DIR/.env.example" << EOF
# Ferrumyx Web Interface Environment Configuration

# API Endpoint (used by nginx for CSP and proxy)
API_ENDPOINT=http://localhost:3000

# CORS Origin (for development, use * for production specify domains)
CORS_ORIGIN=*

# Environment (dev, staging, prod)
NODE_ENV=production

# Optional: CDN URL for assets (if using CDN)
# CDN_URL=https://cdn.example.com

# Optional: Analytics/Sentry DSN
# SENTRY_DSN=
EOF

echo -e "${GREEN}Build optimization completed successfully!${NC}"
echo -e "${GREEN}Build output: $BUILD_DIR${NC}"

# Display build summary
echo -e "\n${YELLOW}Build Summary:${NC}"
echo "Static files: $(find "$BUILD_DIR" -type f | wc -l)"
echo "Total size: $(du -sh "$BUILD_DIR" | cut -f1)"

exit 0