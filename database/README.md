# Database Configuration

This directory contains all database-related configuration, initialization scripts, and data.

## Files

### Initialization
- init-db.sql - Database schema creation and initial setup
- seed-dev-data.sql - Development environment sample data
- init-pgbouncer.sh - Connection pooler initialization

### Configuration
- pgbouncer.ini - Connection pooling configuration
- postgresql.conf.template - PostgreSQL server configuration template
- redis.conf - Redis caching server configuration
- database.env.template - Database environment variables template
- userlist.txt - Database user authentication (for development)

## Database Architecture

Ferrumyx uses PostgreSQL with the pgvector extension for vector similarity search, and Redis for caching and session management.

## Setup

Database initialization is handled automatically during Docker deployment. For manual setup:

`ash
# Initialize database schema
psql -f database/init-db.sql

# Load development data
psql -f database/seed-dev-data.sql
`

See DATABASE_README.md for detailed database documentation.
