import { spillColors, type ColumnAccent } from "@/components/spill-ui";
import type { RetroBoard, RetroCard } from "@/lib/contracts";

export function sortedCards(cards: RetroCard[], phase: string) {
  const visible = [...cards];
  if (phase === "voting") {
    visible.sort((a, b) => b.vote_count - a.vote_count);
  }
  return visible;
}

export function columnSemantic(column: RetroBoard["columns"][number], index: number): { kind: ColumnAccent; label: string; color: string } {
  const title = column.title.toLowerCase();
  const savedColor = column.accent_color || undefined;
  if (isActionsColumn(column) || title.includes("action")) return { kind: "action", label: column.title.toLowerCase(), color: spillColors.action };
  if (title.includes("feeling")) return { kind: "mood", label: column.title.toLowerCase(), color: savedColor ?? "#0f5f72" };
  if (title.includes("mood") || title.includes("mad") || title.includes("sad") || title.includes("glad")) return { kind: "mood", label: column.title.toLowerCase(), color: savedColor ?? spillColors.mood };
  if (title.includes("well") || title.includes("liked") || title.includes("learned") || title.includes("wind") || title.includes("continue") || title.includes("glad")) return { kind: "well", label: column.title.toLowerCase(), color: savedColor ?? spillColors.well };
  if (title.includes("wrong") || title.includes("lacked") || title.includes("improve") || title.includes("anchor") || title.includes("rocks") || title.includes("stop") || title.includes("mad") || title.includes("sad")) return { kind: "wrong", label: column.title.toLowerCase(), color: savedColor ?? spillColors.wrong };
  const fallback = [
    { kind: "mood" as const, color: spillColors.mood },
    { kind: "well" as const, color: spillColors.well },
    { kind: "wrong" as const, color: spillColors.wrong },
    { kind: "action" as const, color: spillColors.action },
  ][index % 4];
  return { ...fallback, color: savedColor ?? fallback.color, label: column.title.toLowerCase() };
}

export function isActionsColumn(column: RetroBoard["columns"][number]) {
  return column.column_key === "actions" || column.title.toLowerCase().includes("action");
}

export function cardLabel(card: RetroCard) {
  return card.body_text || card.gif_alt_text || "media card";
}

export function presenceForPhase(phase: RetroBoard["retro"]["phase"]) {
  if (phase === "voting") return "voting" as const;
  if (phase === "completed" || phase === "action_discussion") return "ready" as const;
  return "writing" as const;
}

export function actionVoteCount(action: RetroBoard["actions"][number], cards: RetroCard[]) {
  if (action.source_card_id) {
    return cards.find((card) => card.id === action.source_card_id)?.vote_count ?? 0;
  }
  if (action.source_cluster_id) {
    return cards.find((card) => card.cluster_id === action.source_cluster_id && card.parent_card_id === null)?.vote_count ?? 0;
  }
  return 0;
}

export function voteLabel(votes: number) {
  if (votes === 0) return "no votes";
  return votes === 1 ? "1 vote" : `${votes} votes`;
}
