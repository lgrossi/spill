import { clampAccent, spillColors, type ColumnAccent } from "@/components/spill-ui";
import type { RetroBoard, RetroCard } from "@/lib/contracts";

export function columnSemantic(column: RetroBoard["columns"][number]): { kind: ColumnAccent; label: string; color: string } {
  const title = column.title.toLowerCase();
  const label = column.title.toLowerCase();
  if (isActionsColumn(column)) return { kind: "action", label, color: spillColors.action };
  if (title.includes("feeling")) return { kind: "mood", label, color: columnColor(column, "#0f5f72") };
  if (title.includes("mood") || title.includes("mad") || title.includes("sad") || title.includes("glad")) return { kind: "mood", label, color: columnColor(column, spillColors.mood) };
  if (title.includes("well") || title.includes("liked") || title.includes("learned") || title.includes("wind") || title.includes("continue") || title.includes("glad")) return { kind: "well", label, color: columnColor(column, spillColors.well) };
  if (title.includes("wrong") || title.includes("lacked") || title.includes("improve") || title.includes("anchor") || title.includes("rocks") || title.includes("stop") || title.includes("mad") || title.includes("sad")) return { kind: "improve", label, color: columnColor(column, spillColors.wrong) };
  return { kind: "neutral", label, color: columnColor(column, spillColors.muted) };
}

export function isActionsColumn(column: RetroBoard["columns"][number]) {
  return column.column_key === "actions" || hasActionWord(column.title);
}

function columnColor(column: RetroBoard["columns"][number], fallback: string) {
  const savedColor = column.accent_color || undefined;
  if (!savedColor || savedColor === spillColors.action) return fallback;
  // Card text is white; keep a custom accent dark enough to stay readable.
  return clampAccent(savedColor);
}

function hasActionWord(title: string) {
  return title.toLowerCase().split(/[^a-z0-9]+/).some((word) => word === "action" || word === "actions");
}

export function cardLabel(card: RetroCard) {
  return card.body_text || card.gif_alt_text || "media card";
}

export function presenceForPhase(phase: RetroBoard["retro"]["phase"]) {
  if (phase === "scheduled") return "away" as const;
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
