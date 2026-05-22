export const CARD_KINDS = new Set(["mood", "wentWell", "wentWrong"]);
export const SOURCES = new Set(["pi", "claude_code"]);
export const PLACEMENTS = new Set(["user_deck", "retro_draft"]);

export const DEFAULT_PERSONAL_RETRO_PROMPT = `You are preparing private retrospective notes for SpillItOut.

Write short, concrete cards in these buckets:
- mood: how the work felt
- wentWell: practices, moments, or outcomes worth repeating
- wentWrong: friction, ambiguity, incidents, or risks to improve

Do not send anything automatically. Produce candidate cards for review, edit, or reject.`;

export function buildPrompt({ includeLocalContext = false, includeSessionContext = false, contextText = "" } = {}) {
  const sections = [DEFAULT_PERSONAL_RETRO_PROMPT];
  if (includeLocalContext || includeSessionContext) {
    sections.push("Opt-in context:");
    if (includeLocalContext) {
      sections.push(`Local context:\n${contextText || "(no local context provided)"}`);
    }
    if (includeSessionContext) {
      sections.push("Session context may be summarized by the calling companion before candidate cards are produced.");
    }
  }
  return `${sections.join("\n\n")}\n`;
}

export function buildIngestionPayload(input) {
  const source = requireChoice("source", input.source, SOURCES);
  const cardKind = requireChoice("kind", input.kind, CARD_KINDS);
  const placement = requireChoice("placement", input.placement ?? "user_deck", PLACEMENTS);
  const text = normalizeOptional(input.text);
  const gifUrl = normalizeOptional(input.gifUrl);
  const idempotencyKey = normalizeOptional(input.idempotencyKey);
  const targetColumnId = normalizeOptional(input.targetColumnId);

  if (!text && !gifUrl) {
    throw new Error("text or gifUrl is required");
  }
  if (placement === "retro_draft" && !targetColumnId) {
    throw new Error("targetColumnId is required for retro_draft placement");
  }

  return {
    source,
    placement,
    target_column_id: placement === "retro_draft" ? targetColumnId : null,
    suggested_text: text,
    gif_url: gifUrl,
    idempotency_key: idempotencyKey,
    source_metadata: {
      companion: source,
      card_kind: cardKind,
      review_state: "approved",
      local_context_included: Boolean(input.includeLocalContext),
      session_context_included: Boolean(input.includeSessionContext),
    },
    raw_payload: {
      kind: cardKind,
      text,
      gif_url: gifUrl,
    },
  };
}

export function buildReviewEnvelope(payload) {
  return {
    review_required: true,
    instructions: "Review/edit/reject this payload. Send only with --approve.",
    payload,
  };
}

function requireChoice(field, value, allowed) {
  const normalized = normalizeOptional(value);
  if (!normalized || !allowed.has(normalized)) {
    throw new Error(`${field} must be one of ${Array.from(allowed).join(", ")}`);
  }
  return normalized;
}

function normalizeOptional(value) {
  if (value === undefined || value === null) {
    return null;
  }
  const trimmed = String(value).trim();
  return trimmed.length > 0 ? trimmed : null;
}
