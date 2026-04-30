#!/bin/bash
# init-pgbouncer.sh - Initialize PgBouncer users

echo "Initializing PgBouncer user authentication..."

# Create userlist.txt with proper MD5 hashes
# Passwords must be provided via environment variables for security

if [[ -z "${POSTGRES_PASSWORD}" ]]; then
    echo "ERROR: POSTGRES_PASSWORD environment variable must be set"
    exit 1
fi

if [[ -z "${READONLY_PASSWORD}" ]]; then
    echo "ERROR: READONLY_PASSWORD environment variable must be set"
    exit 1
fi

cat > /etc/pgbouncer/userlist.txt << EOF
"ferrumyx" "md5$(echo -n "${POSTGRES_PASSWORD}" | md5sum | cut -d' ' -f1)"
"ferrumyx_readonly" "md5$(echo -n "${READONLY_PASSWORD}" | md5sum | cut -d' ' -f1)"
EOF

chmod 600 /etc/pgbouncer/userlist.txt

echo "PgBouncer authentication initialized"