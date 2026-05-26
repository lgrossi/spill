#!/bin/sh
set -e

# Construct DATABASE_URL from standard PG* env vars when not set explicitly.
# Works with Cloud SQL Unix socket (PGHOST = socket directory path) and
# standard TCP Postgres deployments alike.
if [ -z "$DATABASE_URL" ] && [ -n "$PGHOST" ]; then
  export DATABASE_URL="postgresql://${PGUSER}:${PGPASSWORD}@/${PGDATABASE}?host=${PGHOST}"
fi

# Run migrations before serving. sqlx migrations are idempotent — safe on every start.
/usr/local/bin/spillio-api migrate

exec "$@"
