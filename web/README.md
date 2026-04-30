# Web Assets Directory

This directory contains web UI assets and configuration.

## Files

- package.json - Node.js dependencies and scripts
- package-lock.json - Dependency lock file for reproducible builds

## Usage

The web assets are built automatically during Docker container creation. For local development:

`ash
# Install dependencies
npm install

# Build assets
npm run build

# Development server
npm run dev
`

See WEBUI_README.md for detailed web UI documentation.
