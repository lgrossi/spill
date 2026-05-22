# SpillItOut companions

Slice 14 adds first-party companion tooling for Pi and Claude Code. The companion does not write to a retro by default.

## Review flow

1. Generate the default personal retro prompt:

   ```bash
   pnpm --filter @spillio/companions exec spillio-companion prompt
   ```

2. Optionally include local/session context. Context is opt-in only:

   ```bash
   pnpm --filter @spillio/companions exec spillio-companion prompt \
     --include-local-context \
     --context-file /tmp/context.txt \
     --include-session-context
   ```

3. Draft a reviewed ingestion payload:

   ```bash
   pnpm --filter @spillio/companions exec spillio-companion draft \
     --source pi \
     --kind wentWell \
     --text "Pairing caught the regression early" \
     --idempotency-key pi-2026-05-22-1 > /tmp/spillio-payload.json
   ```

4. Review/edit/reject the JSON. Nothing is sent until `send --approve` is used.

5. Send an approved payload:

   ```bash
   pnpm --filter @spillio/companions exec spillio-companion send \
     --file /tmp/spillio-payload.json \
     --retro-id "$RETRO_ID" \
     --user-subject "$USER_SUBJECT" \
     --approve
   ```

## Approved card kinds

The companion accepts only:

- `mood`
- `wentWell`
- `wentWrong`

Sources are limited to:

- `pi`
- `claude_code`

Placement defaults to `user_deck`. Direct private draft placement requires a target column:

```bash
pnpm --filter @spillio/companions exec spillio-companion draft \
  --source claude_code \
  --kind wentWrong \
  --text "Deploy feedback was too slow" \
  --placement retro_draft \
  --target-column-id "$COLUMN_ID"
```
