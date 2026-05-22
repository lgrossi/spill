# SpillItOut / spillio

SpillItOut is a board-first cooperative retrospective app. `spillio` is the developer/project nickname.

## Stack

- Next.js frontend in `apps/web`
- Rust Axum API/WebSocket service in `services/api`
- Postgres for persistence

## Local setup

```bash
pnpm install
cp .env.example .env
pnpm db:up
pnpm db:migrate
pnpm dev:api
pnpm dev:web
```

The API health endpoint is available at:

```bash
curl http://127.0.0.1:4000/health
```

## Checks

```bash
pnpm check
pnpm build
```

## Project docs

- `docs/product-overview.md`
- `docs/designer-mock-brief.md`
- `docs/mvp-product-doc.md`
- `docs/tech-stack-design.md`
- `docs/implementation-plan.md`
- `docs/companions.md`
- `mock/index.html`
