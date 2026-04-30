# Ferrumyx Contributing Guide

## Welcome

Thank you for your interest in contributing to Ferrumyx! This document provides guidelines and information for contributors.

## Table of Contents

- [Development Setup](#development-setup)
- [Development Workflow](#development-workflow)
- [Code Standards](#code-standards)
- [Testing](#testing)
- [Submitting Changes](#submitting-changes)
- [Review Process](#review-process)
- [Community Guidelines](#community-guidelines)

## Development Setup

### Prerequisites

- Rust 1.70+ with Cargo
- Node.js 18+ and npm
- Docker and Docker Compose
- Git

### Quick Start

1. **Clone the repository**
   ```bash
   git clone https://github.com/Classacre/ferrumyx.git
   cd ferrumyx
   ```

2. **Setup development environment**
   ```bash
   # Automated setup
   bash scripts/dev-setup.sh

   # Or manual setup
   cp .env.example .env.dev
   npm install
   export COMPOSE_FILE=docker-compose.dev.yml
   docker-compose up -d
   ```

3. **Verify setup**
   ```bash
   bash scripts/health-check.sh
   ```

For detailed setup instructions, see [Development Environment Setup](runbooks/development-environment-setup.md).

## Development Workflow

### Branching Strategy

We use a simplified Git Flow:

- `main` - Production-ready code
- `develop` - Integration branch for features
- `feature/*` - Feature branches
- `bugfix/*` - Bug fix branches
- `release/*` - Release preparation

### Creating a Feature Branch

```bash
# Start from develop
git checkout develop
git pull origin develop

# Create feature branch
git checkout -b feature/your-feature-name

# Push to remote
git push -u origin feature/your-feature-name
```

### Daily Development Cycle

1. **Pull latest changes**
   ```bash
   git checkout develop
   git pull origin develop
   git checkout feature/your-branch
   git rebase develop
   ```

2. **Make changes with tests**
   ```bash
   # Write code
   # Add tests
   cargo test

   # Run linting
   cargo clippy
   cargo fmt
   ```

3. **Commit changes**
   ```bash
   git add .
   git commit -m "feat: add new feature

   - Add feature description
   - Update tests
   - Update documentation"
   ```

4. **Push and create PR**
   ```bash
   git push origin feature/your-branch
   # Create pull request on GitHub
   ```

## Code Standards

### Rust Code Standards

#### Formatting
```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check
```

#### Linting
```bash
# Run clippy
cargo clippy -- -D warnings

# Fix common issues
cargo clippy --fix
```

#### Naming Conventions
- **Modules**: `snake_case`
- **Types**: `PascalCase`
- **Functions/Methods**: `snake_case`
- **Variables**: `snake_case`
- **Constants**: `SCREAMING_SNAKE_CASE`

#### Code Structure
```
src/
├── main.rs                 # Application entry point
├── lib.rs                  # Library definition
├── config.rs              # Configuration handling
├── db/                    # Database layer
│   ├── mod.rs
│   ├── postgres.rs
│   └── libsql.rs
├── agent/                 # Agent logic
├── tools/                 # Tool implementations
├── channels/              # Communication channels
└── utils/                 # Utility functions
```

### JavaScript/TypeScript Standards

#### Linting
```bash
# Run ESLint
npm run lint

# Fix issues
npm run lint:fix
```

#### Code Style
- Use ES6+ features
- Prefer `const` and `let` over `var`
- Use arrow functions for anonymous functions
- Use template literals over string concatenation
- Follow React best practices (if applicable)

### Documentation Standards

#### Code Documentation
```rust
/// Brief description of the function
///
/// # Arguments
/// * `param1` - Description of parameter
/// * `param2` - Description of parameter
///
/// # Returns
/// Description of return value
///
/// # Examples
/// ```
/// let result = my_function(param1, param2);
/// ```
fn my_function(param1: Type, param2: Type) -> ReturnType {
    // implementation
}
```

#### Commit Messages

Follow conventional commits:

```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Code style changes
- `refactor`: Code refactoring
- `test`: Testing
- `chore`: Maintenance

Examples:
```
feat(auth): add OAuth2 support for Google

- Implement OAuth2 flow
- Add Google provider configuration
- Update user model for OAuth data

Closes #123
```

```
fix(db): resolve connection timeout issue

Connection pooling was not properly configured for high load scenarios.
Increased pool size from 10 to 50 connections.

Fixes #456
```

## Testing

### Unit Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run tests for specific package
cargo test -p ferrumyx-runtime-core
```

### Integration Tests

```bash
# Run integration tests
cargo test --test integration

# Run with database
export DATABASE_URL=postgres://localhost:5432/ferrumyx_test
cargo test --test integration --features postgres
```

### End-to-End Tests

```bash
# Run E2E tests
npm run test:e2e

# Or with Docker
docker-compose -f docker-compose.dev.yml up -d
npm run test:e2e
```

### Test Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --features postgres,libsql --out Html

# Open coverage report
open tarpaulin-report.html
```

### Performance Testing

```bash
# Run benchmarks
cargo bench

# Profile with flamegraph
cargo flamegraph --bin ferrumyx-agent --features postgres
```

## Submitting Changes

### Pull Request Process

1. **Ensure tests pass**
   ```bash
   cargo test --features postgres,libsql
   npm test
   ```

2. **Update documentation**
   - Update README if needed
   - Update API documentation
   - Update changelog

3. **Create pull request**
   - Use descriptive title
   - Fill out PR template
   - Reference related issues
   - Add screenshots for UI changes

4. **PR Title Format**
   ```
   type(scope): brief description
   ```

### PR Checklist

- [ ] Tests pass locally
- [ ] Code follows style guidelines
- [ ] Documentation updated
- [ ] No security vulnerabilities
- [ ] Performance not degraded
- [ ] Breaking changes documented

## Review Process

### Code Review Guidelines

**Reviewers should check:**
- Code correctness and security
- Test coverage and quality
- Performance implications
- Documentation completeness
- Adherence to coding standards

**Authors should:**
- Respond to review comments promptly
- Make requested changes
- Re-request review when ready
- Keep PRs focused and small

### Automated Checks

All PRs must pass:
- ✅ Code formatting (`cargo fmt --check`)
- ✅ Linting (`cargo clippy`, `npm run lint`)
- ✅ Tests (`cargo test`)
- ✅ Security scan (Trivy, cargo-audit)
- ✅ License compliance check

## Issue Tracking

### Bug Reports

When reporting bugs, please include:

1. **Clear title** describing the issue
2. **Steps to reproduce**
3. **Expected behavior**
4. **Actual behavior**
5. **Environment details**
   - OS version
   - Rust/Node versions
   - Ferrumyx version/commit
6. **Logs and error messages**
7. **Screenshots** (if applicable)

### Feature Requests

For feature requests, please include:

1. **Clear title** for the feature
2. **Problem description** - what's the current limitation?
3. **Proposed solution** - how should it work?
4. **Use cases** - who would use this and why?
5. **Alternatives considered** - what other approaches were thought of?

### Issue Labels

- `bug`: Something isn't working
- `enhancement`: New feature or improvement
- `documentation`: Documentation improvements
- `good first issue`: Suitable for newcomers
- `help wanted`: Community contribution needed
- `question`: Further information needed

## Community Guidelines

### Code of Conduct

We follow a code of conduct to ensure a welcoming environment for all contributors. Please:

- Be respectful and inclusive
- Focus on constructive feedback
- Help newcomers learn and contribute
- Report unacceptable behavior to maintainers

### Getting Help

- **Documentation**: Check the [docs/](docs/) directory
- **Issues**: Search existing issues before creating new ones
- **Discussions**: Use GitHub Discussions for questions
- **Discord/Slack**: Join our community chat for real-time help

### Recognition

Contributors are recognized through:
- GitHub contributor statistics
- Changelog mentions
- Community shoutouts
- Co-authorship on papers/presentations

## Security Considerations

### Reporting Security Issues

If you discover a security vulnerability, please:

1. **DO NOT** create a public issue
2. Email security@ferrumyx.org with details
3. Allow time for fix before public disclosure
4. Receive credit for responsible disclosure

### Security Best Practices

- Never commit secrets or credentials
- Use environment variables for configuration
- Follow the principle of least privilege
- Keep dependencies updated
- Run security scans regularly

## License

By contributing to Ferrumyx, you agree that your contributions will be licensed under the same license as the project (MIT OR Apache-2.0).

---

Thank you for contributing to Ferrumyx! 🚀