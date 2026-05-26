#!/bin/sh
set -e

# Cloud Run sets PORT; SPILLIO_API_ADDR must be 0.0.0.0 so the container
# is reachable. Falls back to 8080 for local runs without PORT set.
export SPILLIO_API_ADDR="0.0.0.0:${PORT:-8080}"

# Run migrations before serving. sqlx migrations are idempotent — safe on every start.
# PG* env vars (PGHOST, PGUSER, PGPASSWORD, PGDATABASE) are read directly by sqlx.
/usr/local/bin/spillio-api migrate

exec "$@"
