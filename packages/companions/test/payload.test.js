import assert from "node:assert/strict";
import test from "node:test";
import { buildIngestionPayload, buildPrompt, buildReviewEnvelope } from "../src/payload.js";

test("buildPrompt excludes local/session context unless explicitly enabled", () => {
  assert.equal(buildPrompt().includes("Opt-in context"), false);

  const prompt = buildPrompt({
    includeLocalContext: true,
    includeSessionContext: true,
    contextText: "branch diff summary",
  });
  assert.equal(prompt.includes("branch diff summary"), true);
  assert.equal(prompt.includes("Session context"), true);
});

test("buildIngestionPayload emits approved mood/wentWell/wentWrong connector payloads", () => {
  const payload = buildIngestionPayload({
    source: "pi",
    kind: "wentWell",
    text: "Review pairing caught a regression early",
    idempotencyKey: "pi-1",
    includeLocalContext: true,
  });

  assert.equal(payload.source, "pi");
  assert.equal(payload.placement, "user_deck");
  assert.equal(payload.suggested_text, "Review pairing caught a regression early");
  assert.equal(payload.source_metadata.card_kind, "wentWell");
  assert.equal(payload.source_metadata.review_state, "approved");
  assert.equal(payload.source_metadata.local_context_included, true);
});

test("buildIngestionPayload requires target column for direct draft mode", () => {
  assert.throws(
    () => buildIngestionPayload({ source: "claude_code", kind: "wentWrong", text: "Slow deploy", placement: "retro_draft" }),
    /targetColumnId/,
  );

  const payload = buildIngestionPayload({
    source: "claude_code",
    kind: "wentWrong",
    text: "Slow deploy",
    placement: "retro_draft",
    targetColumnId: "column-1",
  });
  assert.equal(payload.target_column_id, "column-1");
});

test("buildReviewEnvelope makes review/edit/reject explicit before sending", () => {
  const payload = buildIngestionPayload({ source: "pi", kind: "mood", text: "Cautiously optimistic" });
  const envelope = buildReviewEnvelope(payload);

  assert.equal(envelope.review_required, true);
  assert.match(envelope.instructions, /Review\/edit\/reject/);
});
