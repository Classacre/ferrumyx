# Configuration Directory

This directory contains configuration templates and examples for Ferrumyx deployment.

## Files

- .env.example - Main environment variables template
- .env.webui.example - Web UI specific configuration
- ferrumyx-security.toml - Security configuration template

## Usage

Copy templates to create your configuration:

`ash
# Main configuration
cp config/.env.example .env

# Web UI configuration
cp config/.env.webui.example .env.webui

# Security settings
cp config/ferrumyx-security.toml .
`

Edit the copied files with your specific values before deployment.
