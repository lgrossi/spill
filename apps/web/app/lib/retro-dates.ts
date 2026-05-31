import type { RetroBoard, RetroSummary } from "./contracts";

type RetroDateSource =
  | Pick<RetroSummary, "planned_for" | "happened_at">
  | Pick<RetroBoard["retro"], "planned_for" | "happened_at">;

export function displayRetroDate(source: RetroDateSource) {
  return formatDateOnly(source.happened_at ?? source.planned_for);
}

export function formatDateOnly(value: string) {
  const date = new Date(value.includes("T") ? value : `${value}T00:00:00`);
  if (Number.isNaN(date.getTime())) return "Date not set";
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric",
  }).format(date);
}

export function isPlannedForDue(plannedFor: string) {
  return plannedFor <= localDateString(new Date());
}

export function localDateString(date: Date) {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}
