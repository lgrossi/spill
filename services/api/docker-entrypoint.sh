#!/bin/sh
set -e

# Cloud Run sets PORT; SPILLIO_API_ADDR must be 0.0.0.0 so the container
# is reachable. Falls back to 8080 for local runs without PORT set.
export SPILLIO_API_ADDR="0.0.0.0:${PORT:-8080}"

# Run migrations before serving. Skipped when SPILLIO_RUN_MIGRATIONS=false —
# set by CI on branch deploys so feature branches don't advance the shared
# dev schema ahead of main. Always runs on main (default: true).
if [ "${SPILLIO_RUN_MIGRATIONS:-true}" != "false" ]; then
  /usr/local/bin/spillio-api migrate
fi

exec "$@"
