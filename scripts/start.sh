#!/bin/bash
# start.sh - Start PostgreSQL and PgBouncer

set -e

# Start PostgreSQL in the background
docker-entrypoint.sh postgres &

# Wait for PostgreSQL to be ready
until pg_isready -U ferrumyx -d ferrumyx; do
    echo "Waiting for PostgreSQL..."
    sleep 2
done

echo "PostgreSQL is ready"

# Start PgBouncer
exec pgbouncer /etc/pgbouncer/pgbouncer.ini