# Documentation Reorganization Manifest

## File Relocations

### From Root to docs/user/
- `USER_GUIDE.md` → `docs/user/getting-started.md`

### From Root to docs/developer/
- `API_REFERENCE.md` → `docs/developer/api-reference.md`
- `TESTING.md` → `docs/developer/testing.md`
- `CONTRIBUTING.md` → `docs/developer/contributing.md`

### From Root to docs/operations/
- `DEPLOYMENT.md` → `docs/operations/deployment.md`
- `TROUBLESHOOTING.md` → `docs/operations/troubleshooting.md`
- `OPERATIONS_GUIDE.md` → `docs/operations/monitoring.md`

### From Root to docs/technical/
- `ARCHITECTURE.md` → `docs/technical/architecture.md`
- `DATABASE_README.md` → `docs/technical/database.md`

### From docs/ to docs/technical/
- `docs/COMPLIANCE.md` → `docs/technical/compliance.md`

### From github-wiki/ to docs/technical/
- `github-wiki/Security-&-Compliance.md` → `docs/technical/security.md`
- `github-wiki/Performance-&-Scaling.md` → `docs/technical/performance.md`
- `github-wiki/Architecture-&-Design.md` → `docs/technical/architecture-design.md`

### From github-wiki/ to docs/developer/
- `github-wiki/Developer-Guide.md` → `docs/developer/developer-guide.md`

### From github-wiki/ to docs/operations/
- `github-wiki/Operations-Guide.md` → `docs/operations/operations-guide.md`

### From Root to docs/runbooks/
- `PRODUCTION_OPERATIONS_RUNBOOK.md` → `docs/runbooks/incident-response.md`

### From Root to docs/reports/
- `PRODUCTION_READINESS_REPORT.md` → `docs/reports/production-readiness.md`
- `PHI_protection_enhancement_report.md` → `docs/reports/security-audit.md`
- `PERFORMANCE_TESTING_README.md` → `docs/reports/performance-analysis.md`

## New Files Created
- `docs/README.md` (main documentation index)
- `docs/user/research-workflows.md`
- `docs/user/multi-channel-usage.md`
- `docs/user/data-analysis.md`
- `docs/developer/setup.md`
- `docs/user/README.md`
- `docs/developer/README.md`
- `docs/operations/README.md`
- `docs/technical/README.md`
- `docs/runbooks/README.md`
- `docs/reports/README.md`

## Cross-Reference Updates
- Updated links in `docs/user/getting-started.md` to point to new locations
- Updated documentation links in root `README.md` to reflect new structure
- Created section indexes for navigation

## Remaining Files
Some files remain in root or github-wiki/ that may need further consolidation:
- Various report files (PRODUCTION_HARDENING_REPORT.md, etc.)
- github-wiki/ files not moved
- Performance analyzer scripts and other tools

## Directory Structure Created
```
docs/
├── README.md (documentation index)
├── user/
│   ├── README.md
│   ├── getting-started.md
│   ├── research-workflows.md
│   ├── multi-channel-usage.md
│   └── data-analysis.md
├── developer/
│   ├── README.md
│   ├── setup.md
│   ├── api-reference.md
│   ├── contributing.md
│   ├── testing.md
│   └── developer-guide.md
├── operations/
│   ├── README.md
│   ├── deployment.md
│   ├── monitoring.md
│   ├── troubleshooting.md
│   └── operations-guide.md
├── technical/
│   ├── README.md
│   ├── architecture.md
│   ├── security.md
│   ├── performance.md
│   ├── compliance.md
│   ├── database.md
│   └── architecture-design.md
├── runbooks/
│   ├── README.md
│   ├── production-deployment.md
│   ├── incident-response.md
│   └── (existing development-environment-setup.md)
└── reports/
    ├── README.md
    ├── production-readiness.md
    ├── security-audit.md
    └── performance-analysis.md
```