# Spill. deployment notes

Slice 18 adds production-ish service packaging and operational seams. This is not a managed production runbook; it is the minimum deployable shape for the MVP.

## Services

- `web`: Next.js app, port `3000`
- `api`: Rust Axum API/WebSocket service, port `4000`
- `postgres`: primary persistence

## Required configuration

API:

- `DATABASE_URL`
- `SPILLIO_API_ADDR` (default in Docker: `0.0.0.0:4000`)
- `SPILLIO_KLIPY_API_KEY` for real Klipy media search. Without it the API falls back to local fixture media.
- `RUST_LOG` (recommended: `info`)

Web:

- `SPILLIO_API_URL`
- `NEXT_PUBLIC_SPILLIO_API_URL`
- `SPILLIO_AUTH_MODE`
- `SPILLIO_AUTH_EMAIL_HEADER`
- `SPILLIO_AUTH_NAME_HEADER`

Production deployments should run with `SPILLIO_AUTH_MODE=proxy` behind a trusted
identity layer such as Google IAP or oauth2-proxy. Local development can use
`SPILLIO_AUTH_MODE=local`.

## Local production-ish run

```bash
docker compose --profile app up --build
```

Run migrations explicitly before serving a shared environment:

```bash
DATABASE_URL=postgres://spillio:spillio@localhost:5432/spillio pnpm db:migrate
```

The compose `app` profile also runs a one-shot `migrate` service before `api` starts.

## Health checks

API:

```bash
curl http://127.0.0.1:4000/health
```

Web:

```bash
curl http://127.0.0.1:3000
```

## Logs and ownership

- API logs use `tracing_subscriber` and respect `RUST_LOG`.
- Docker Compose exposes service logs through `docker compose logs api web`.
- Migration ownership: whoever deploys the API owns running `spillio-api migrate` before or during rollout.
- Data ownership: Postgres volume/backups are the operator's responsibility.
