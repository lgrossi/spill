# Spill. API Contracts

## Purpose

The Rust API owns board state, identity extraction, WebSocket sync, connector ingestion, and AI jobs. The Next.js app renders the UI and calls this API through typed JSON contracts.

## Contract strategy

MVP starts with explicit JSON response/request structs in Rust and matching TypeScript types in the frontend where needed.

Rules:

- Rust structs are the source of truth for API payloads.
- JSON fields use stable snake_case or explicit documented keys.
- Responses return structured errors.
- Breaking response changes require updating this file and the matching frontend type.
- Generated OpenAPI or generated TypeScript can be added later, but is not required for Slice 3.

## Identity and access contract

The API is the sole authentication gate. It trusts a single credential: a
first-party token signed with the shared `SPILLIO_TOKEN_SECRET`. The web tier
(which sits behind its own auth proxy and therefore knows who the user is) is
the only minter — it vouches for the authenticated user by signing a short-lived
HS256 token the API verifies. There is no OIDC, JWKS, or service account: one
mechanism, vendor-neutral, identity bound into the token rather than a spoofable
header.

### Token mode (deployed)

Each request carries `Authorization: Bearer <first-party token>`. The API
verifies the HS256 signature against `SPILLIO_TOKEN_SECRET` and checks expiry,
then reads the identity from the claims:

- `email` (required) — the acting user; board ownership and ACL key.
- `name` (optional) — display hint.
- `retro` (optional) — board scope; present only on WebSocket tokens.

The participant subject is always derived server-side as
`email:sha256(lowercased-email)` — never an input. Token mode fails closed:
`SPILLIO_TOKEN_SECRET` is required or the API refuses to start. On-behalf-of and
other identity headers are ignored in token mode.

### Local mode (dev)

When `SPILLIO_AUTH_MODE` is unset/`local` (no token secret), the API trusts the
`x-spillio-on-behalf-of` header for the user plus an optional
`x-spillio-user-name`. The API refuses to start in local mode on Cloud Run.

### WebSocket (`/retros/{id}/events`)

Browsers cannot set headers on a WS handshake, so the connection presents a
short-lived, board-scoped token (the same first-party token, with a `retro`
claim) as a `Sec-WebSocket-Protocol` entry alongside the `spillio.ws.v1` marker.
The API verifies the token, checks the `retro` claim matches the requested
board, and confirms board membership before upgrading.

### Companion CLI

A signed-in user fetches a longer-lived token from the web UI (`GET /api/token`)
and passes it to the companion (`--token` / `SPILLIO_API_TOKEN`). The companion
presents it as `Authorization: Bearer`; identity rides in the token, so no
service account or impersonation is needed.

## Board access model

MVP access is invite-based:

- board creators receive a `host` grant
- invitees receive `member` grants
- uninvited users cannot open the board
- hosts can add and remove member grants
- every mutation is authorized against the board participant/grant model

## Current session endpoint

`GET /api/session`

Success:

```json
{
  "user": {
    "subject": "user-123",
    "email": "ava@example.com",
    "display_name": "Ava"
  },
  "access_model": {
    "kind": "link",
    "can_edit_with_link": true
  }
}
```

Missing identity:

```json
{
  "error": {
    "code": "unauthorized",
    "message": "missing required header x-spillio-user-subject"
  }
}
```

## Error shape

All API errors should use:

```json
{
  "error": {
    "code": "machine_readable_code",
    "message": "Human readable message"
  }
}
```
