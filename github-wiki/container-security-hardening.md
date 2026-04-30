# Container Security Hardening Guide

This document outlines the security hardening measures implemented across all Ferrumyx Docker containers to ensure compliance with security best practices and minimize attack surfaces.

## Overview

Ferrumyx implements comprehensive container security hardening following the principle of least privilege, defense in depth, and secure defaults. All containers are configured to run as non-root users, with minimal required capabilities and runtime protections.

## Hardened Containers

### Core Application Containers

#### ferrumyx-web
- **Base Image**: Debian Bullseye Slim
- **User**: Non-root `ferrumyx` user
- **Security Features**:
  - Non-root execution
  - Minimal runtime dependencies (`ca-certificates`, `libssl1.1`)
  - `--no-install-recommends` for package installation
  - Health check implementation
  - No privilege escalation capabilities

#### ferrumyx-agent
- **Base Image**: Debian Bullseye Slim
- **User**: Non-root `ferrumyx` user
- **Security Features**:
  - Non-root execution
  - Minimal runtime dependencies
  - Network access restrictions
  - Capability dropping

#### ferrumyx-ingestion
- **Base Image**: Debian Bullseye Slim
- **User**: Non-root `ferrumyx` user
- **Security Features**:
  - Non-root execution
  - File and network access controls
  - Minimal package installation

#### ferrumyx-kg
- **Base Image**: Debian Bullseye Slim
- **User**: Non-root `ferrumyx` user
- **Security Features**:
  - Non-root execution
  - Data processing isolation

#### ferrumyx-ranker
- **Base Image**: Debian Bullseye Slim
- **User**: Non-root `ferrumyx` user
- **Security Features**:
  - Non-root execution
  - ML operation isolation

#### ferrumyx-molecules
- **Base Image**: Debian Bullseye Slim
- **User**: Non-root `ferrumyx` user
- **Security Features**:
  - Non-root execution
  - External tool execution controls

### Infrastructure Containers

#### postgres (ferrumyx-postgres)
- **Base Image**: PostgreSQL 15 with pgvector
- **User**: `postgres` system user
- **Security Features**:
  - Non-root execution
  - PgBouncer connection pooling
  - Minimal capabilities (CHOWN, SETGID, SETUID, DAC_OVERRIDE)
  - Custom initialization scripts
  - Backup and restore security

#### redis (ferrumyx-redis)
- **Base Image**: Redis 7 Alpine
- **User**: `redis` system user
- **Security Features**:
  - Non-root execution
  - Password authentication required
  - Dangerous command renaming (FLUSHDB, FLUSHALL, SHUTDOWN, CONFIG, DEBUG)
  - Memory limits and LRU eviction
  - Custom configuration for security

#### ferrumyx-webui
- **Base Image**: Nginx Alpine
- **User**: `ferrumyx` user
- **Security Features**:
  - Non-root execution
  - Security headers implemented:
    - X-Frame-Options: DENY
    - X-Content-Type-Options: nosniff
    - X-XSS-Protection: 1; mode=block
    - Referrer-Policy: strict-origin-when-cross-origin
    - Content-Security-Policy
  - HTTPS Strict Transport Security (when SSL enabled)

## Docker Compose Security Options

All services in `docker-compose.yml` include:

```yaml
security_opt:
  - no-new-privileges:true
cap_drop:
  - ALL
```

Specific services have additional capabilities as needed:
- PostgreSQL: CHOWN, SETGID, SETUID, DAC_OVERRIDE

## Runtime Protections

### AppArmor Profiles
- Docker default AppArmor profile applied where available
- Prevents unauthorized system calls and file access

### SELinux
- Compatible with SELinux enforcing mode
- Labels applied for proper isolation

### Network Security
- Services isolated in `ferrumyx-network`
- Monitoring network internal only
- No privileged ports exposed externally

## Security Scanning Integration

### Automated Scanning
- `scripts/security-scan.sh` script for vulnerability scanning
- Uses Trivy for container image scanning
- Scans for HIGH and CRITICAL severity vulnerabilities
- Integrated into CI/CD pipeline

### Manual Scanning
```bash
# Run security scan
./scripts/security-scan.sh

# Scan specific image
trivy image --severity HIGH,CRITICAL ferrumyx-web
```

## Dependency Minimization

- All containers use `--no-install-recommends` for package installation
- Minimal base images (Alpine or Debian Slim)
- Runtime dependencies only (no build tools)
- Clean package cache after installation

## Build Security

- Multi-stage builds remove build dependencies from final images
- No secrets committed to image layers
- Proper ownership and permissions set

## Monitoring and Auditing

### Runtime Security
- All containers include health checks
- Logging configured for security events
- Resource limits enforced

### Vulnerability Management
- Regular base image updates
- Automated dependency updates
- Security patch management

## Compliance

This hardening ensures compliance with:
- NIST SP 800-190 (Container Security)
- CIS Docker Benchmark
- OWASP Container Security
- SOC 2 security requirements

## Verification

To verify hardening effectiveness:

1. **User Verification**:
   ```bash
   docker exec <container> whoami  # Should not be root
   ```

2. **Capability Check**:
   ```bash
   docker exec <container> capsh --print  # Should show minimal caps
   ```

3. **Security Scan**:
   ```bash
   ./scripts/security-scan.sh
   ```

## Maintenance

- Regularly update base images
- Monitor for new security vulnerabilities
- Update security configurations as needed
- Review and rotate secrets

## Emergency Procedures

In case of security incident:
1. Stop affected containers
2. Analyze logs and events
3. Apply security patches
4. Rebuild and redeploy hardened images
5. Update security policies if needed