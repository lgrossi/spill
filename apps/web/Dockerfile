# syntax=docker/dockerfile:1.7
###############################################################################
# spillio web - Next.js standalone build, distroless runtime.
#
# The runtime image carries only the traced standalone bundle: no npm, no
# pnpm, no full node_modules. That toolchain is what previously pulled the
# Sysdig-flagged transitive CVEs into the published image.
###############################################################################
# node 24 + distroless trixie (debian13): the bookworm (debian12) distroless
# base lags on libssl3 / libc6 patches that the Sysdig gate rejects, so track
# the newer Debian line which ships them fresh.
ARG NODE_VERSION=24

FROM registry.hub.docker.com/library/node:${NODE_VERSION}-trixie-slim AS build
WORKDIR /app
ENV NEXT_TELEMETRY_DISABLED=1
RUN corepack enable
COPY . .
RUN pnpm install --frozen-lockfile
RUN pnpm --filter @spillio/web build

FROM gcr.io/distroless/nodejs${NODE_VERSION}-debian13 AS runtime
WORKDIR /app
ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1
ENV HOSTNAME=0.0.0.0
# Cloud Run injects $PORT; the standalone server honours it. Compose/default
# local runs use Next's default 3000.
#
# With outputFileTracingRoot at the repo root, the standalone bundle keeps the
# workspace layout: server entry at apps/web/server.js, hoisted deps at the
# bundle root.
COPY --from=build /app/apps/web/.next/standalone ./
COPY --from=build /app/apps/web/.next/static ./apps/web/.next/static
COPY --from=build /app/apps/web/public ./apps/web/public
EXPOSE 3000
# distroless/nodejs uses `node` as the entrypoint.
CMD ["apps/web/server.js"]
