import { clusterCards, moveDraftCard } from "./api";

export async function moveBoardCard(retroId: string, cardId: string, body: unknown) {
  const columnId = field(body, "column_id");
  const beforeCardId = field(body, "before_card_id") || undefined;

  if (!columnId) {
    return { ok: false as const, error: "column_id is required" };
  }

  return { ok: true as const, value: await moveDraftCard(retroId, cardId, columnId, beforeCardId) };
}

export async function clusterBoardCard(retroId: string, cardId: string, body: unknown) {
  const targetCardId = field(body, "target_card_id");

  if (!targetCardId) {
    return { ok: false as const, error: "target_card_id is required" };
  }

  return { ok: true as const, value: await clusterCards(retroId, cardId, targetCardId) };
}

function field(body: unknown, key: string) {
  if (!body || typeof body !== "object") {
    return "";
  }
  const value = (body as Record<string, unknown>)[key];
  return typeof value === "string" ? value : "";
}
