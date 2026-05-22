# SpillItOut API Contracts

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

## Identity contract

The first internal deployment uses platform-provided request headers. The auth layer is intentionally abstracted so another provider can replace it later.

Required header:

- `x-spillio-user-subject`

Optional header:

- `x-spillio-user-name`

If the display name header is absent, the API uses the subject as display name.

## Link access model

MVP access is link-based:

- if a participant has the retro link, they can view/edit the board
- no complex role/permission model in MVP
- this is represented as an access policy seam in the API service, not hard-coded into frontend routes

Host/member roles can still exist as retro participant metadata for future UX behavior.

## Current session endpoint

`GET /api/session`

Success:

```json
{
  "user": {
    "subject": "user-123",
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
