#!/bin/sh
set -e

# Construct DATABASE_URL from Claudius-injected PG* vars when not set explicitly.
# Claudius provides: PGHOST (Cloud SQL Unix socket path), PGUSER, PGPASSWORD, PGDATABASE.
# SQLx accepts the socket via the host= query parameter.
if [ -z "$DATABASE_URL" ] && [ -n "$PGHOST" ]; then
  export DATABASE_URL="postgresql://${PGUSER}:${PGPASSWORD}@/${PGDATABASE}?host=${PGHOST}"
fi

# Run migrations before serving. sqlx migrations are idempotent — safe on every start.
/usr/local/bin/spillio-api migrate

exec "$@"
