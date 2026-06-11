# Spill companion CLI (`spill`)

`spill` is the single-binary companion (Rust, in `cli/`) that an agent uses to
read a user's retro board and push reviewed cards onto it. It replaces the old
JS/Python companions. The board API is the source of truth — no window or column
guessing.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/lgrossi/spill/main/scripts/install-spill.sh | sh
```

It drops `spill` in `~/.local/bin` (override with `SPILL_BIN_DIR`). The binary
self-updates: a throttled background check installs newer releases and applies
them on the next run. Force it with `spill update`.

## Authentication

Identity rides in a first-party token minted by the web app behind its auth
proxy. `spill` fetches it for you:

```bash
spill login            # opens the browser, captures the token via loopback
spill login --manual   # headless/SSH: paste a token from <web>/api/token
spill logout           # clear the cached token
```

The token is cached under `~/.config/spill` and refreshed automatically on a
401. Override endpoints with `SPILLIO_API_URL` / `SPILLIO_WEB_URL`, or pass a
ready token with `--token` / `SPILLIO_API_TOKEN`.

For local development against an API running with `SPILLIO_AUTH_MODE=local` and
no token secret, use `--on-behalf-of you@example.com` or
`SPILLIO_ON_BEHALF_OF=you@example.com`. This sends the dev-only
`x-spillio-on-behalf-of` header instead of a bearer token.

## 1. State (read-only, run first)

```bash
spill state
```

Returns the target board (scheduled/writing), the previous retro in the same
series, the derived `{since, until}` window, and the board's real columns
(`id`, `key`, `title`, `position`). Map each drafted card to a column `id`.

## 2. Publish (human-gated)

Build a JSON list of cards, each mapped to a column `id`:

```json
[{ "column_id": "<id from state>", "kind": "wentWell", "text": "<one line>", "gif_url": null }]
```

`kind` is `mood | wentWell | wentWrong`. Then:

```bash
spill publish --retro-id <target.id> --file cards.json --confirm
```

Without `--confirm` it refuses and only reports the count + destination — the
human gate. `publish` is phase-aware: in `writing`/`voting` cards land directly
on the columns as your private drafts. Scheduled boards are read-only for the
CLI until deck acceptance is wired in the web UI.
