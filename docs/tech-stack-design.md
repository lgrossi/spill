# Spill. - Tech Stack Design Discussion

## Current state

Spill. (project nickname: spillio) is a greenfield cooperative retrospective web tool. No code exists yet. The first deployment target is an internal tool-hosting environment, and the product is expected to be vibe-coded. That makes AI training-data coverage of the chosen stack a primary selection criterion alongside deployment fit.

## Desired end state

A deployed, collaborative retro board running as two services:

- Rust API/WebSocket backend
- Next.js frontend
- Postgres database

## Design decisions

### D1. Rust for the backend

**Chosen.** The backend should be Rust, using Axum, Tokio, and SQLx.

The backend owns:

- HTTP API
- WebSocket board sync
- Postgres access
- AI job dispatch
- connector ingestion API
- board phase/domain invariants

Rust is a good fit for Spill. because the domain has constrained rules:

- phase transitions
- draft card visibility
- vote constraints
- one-off clustering
- action confirmation states
- strict connector payload validation

Axum and SQLx also have strong AI training-data coverage, which makes AI-assisted implementation more practical.

**Ownership implication:** The product team owns the Rust Dockerfile, tracing wiring, library upgrades, CVE patching, and runtime maintenance. This should be captured in deployment/onboarding documentation.

**Rejected:** Tauri is a desktop app framework and is architecturally incompatible with a server-authoritative multi-user web tool.

### D2. Next.js for the frontend

**Chosen.** The frontend should be a Next.js (React) SSR app deployed as a Node.js service.

Next.js is a good fit because:

- React has strong AI/code-generation coverage.
- The required UI patterns are common in React examples: drag-and-drop boards, GIF pickers, realtime state, phase transitions, drawers, trays, and overlays.
- Next.js as an SSR layer in front of a Rust API is well documented.
- It keeps frontend hosting separate from API/WebSocket ownership.

**Rejected:** SvelteKit has no meaningful technical advantage for this product and less AI prior art for the expected implementation style. A raw static SPA adds little value and increases friction around hosting, auth, and initial board load.

### D3. Split deployment: Rust API + Next.js as separate services

**Chosen.** The Rust backend owns API and WebSocket surfaces. Next.js owns frontend SSR and browser delivery. They are deployed as separate services.

Communication model:

- Next.js talks to Rust over internal HTTP.
- Browser clients connect directly to the Rust WebSocket endpoint.
- A monorepo is acceptable for sharing contracts, but not required.

**Rejected:** Rust serving the Next.js build conflates infrastructure ownership, removes the standard frontend-service scaffold advantage, and creates a messy build pipeline.

### D4. Postgres via SQLx

**Chosen.** SQLx provides async Postgres access with compile-time query validation. This is a strong fit for a domain with complex invariants such as phase transitions, draft card visibility, and vote counts.

Use raw SQL with type-checked queries. Do not introduce an ORM for MVP. Migrations should use `sqlx migrate`.

### D5. Realtime via Axum WebSocket

**Chosen.** Board sync runs over WebSocket managed by the Rust backend.

Realtime events include:

- card updates
- ready state
- vote counts
- phase transitions
- summary/send status

Axum's native WebSocket support is sufficient for MVP scale. No frontend WebSocket relay is needed for MVP.

## Patterns to follow

- Axum handlers thin; domain logic in typed service modules behind trait boundaries.
- Phase state machine as an explicit Rust enum with valid transition enforcement.
- AI jobs as explicit database-backed records: input, output, status, timestamp.
- Next.js server components for initial board load; client components only for interactive surfaces.
- Connector ingestion API with strict payload validation.

## Patterns to avoid

- Board sync logic in Next.js API routes; WebSocket lives in Rust only.
- Shared mutable state across Axum handlers without explicit locking; use channels or an actor pattern.
- Fire-and-forget AI jobs.
- AI output presented as final; AI output is draft/proposal pending human confirmation.
- Repeated background reclustering; clustering is a one-off board mutation per product spec.

## Compatibility and rollback

Greenfield: no migration concerns.

Deployment/onboarding documentation must capture Rust stack ownership explicitly.

## Open questions

- Auth: use platform-provided identity headers in the first internal deployment, with invite-based board access. Auth must be abstracted cleanly so the internal identity provider can be swapped out for open-source deployments.
- GIF provider: Giphy, Tenor, or self-hosted proxy.
- AI provider abstraction: single provider for MVP or provider-agnostic interface from day one.
- WebSocket scale: define what triggers a move from single-instance Axum fanout to Redis pub/sub or equivalent.
