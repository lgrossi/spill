"use client";

import Link from "next/link";
import type { CSSProperties } from "react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { RetroSummary } from "@/lib/api";
import { SYSTEM_RECURRING_TAGS } from "@/lib/contracts";
import { fieldControlClass, phaseColor, phaseLabel } from "./spill-ui";
import { DeleteBoardButton } from "./delete-board-button";
import { CloneRetroButton } from "./clone-retro-button";

const pageSize = 5;
const phaseOptions = [
  ["all", "all statuses"],
  ["writing", "writing"],
  ["discussion", "review"],
  ["voting", "voting"],
  ["action_discussion", "action"],
  ["completed", "done"],
] as const;

export function BoardHistory({
  boards,
  initialQuery,
  initialShown,
  initialStatus,
}: {
  boards: RetroSummary[];
  initialQuery: string;
  initialShown: number;
  initialStatus: string;
}) {
  const inputRef = useRef<HTMLInputElement>(null);
  const syncTimer = useRef<number | null>(null);
  const [query, setQuery] = useState(initialQuery);
  const [status, setStatus] = useState(normalizeStatus(initialStatus));
  const [shown, setShown] = useState(initialShown);

  const filteredBoards = useMemo(() => filterBoards(boards, query, status), [boards, query, status]);
  const visibleCount = Math.min(filteredBoards.length, Math.max(pageSize, shown));
  const rows = filteredBoards.slice(0, visibleCount);

  useEffect(() => {
    return () => {
      if (syncTimer.current) window.clearTimeout(syncTimer.current);
    };
  }, []);

  function syncUrl(nextQuery: string, nextStatus: string, nextShown: number, debounce = true) {
    if (syncTimer.current) window.clearTimeout(syncTimer.current);
    const apply = () => {
      window.history.replaceState(null, "", historyHref(nextQuery, nextStatus, nextShown));
    };
    if (debounce) {
      syncTimer.current = window.setTimeout(apply, 200);
    } else {
      apply();
    }
  }

  function applyQuery(value: string) {
    setQuery(value);
    setShown(pageSize);
    syncUrl(value, status, pageSize);
  }

  function clearQuery() {
    setQuery("");
    setShown(pageSize);
    syncUrl("", status, pageSize, false);
    inputRef.current?.focus();
  }

  function applyStatus(value: string) {
    const nextStatus = normalizeStatus(value);
    setStatus(nextStatus);
    setShown(pageSize);
    syncUrl(query, nextStatus, pageSize, false);
  }

  function showMore() {
    const nextShown = Math.min(filteredBoards.length, visibleCount + pageSize);
    setShown(nextShown);
    syncUrl(query, status, nextShown, false);
  }

  function showLess() {
    setShown(pageSize);
    syncUrl(query, status, pageSize, false);
  }

  return (
    <section className="mt-7">
      <div>
        <div className="flex flex-wrap items-end justify-between gap-3">
          <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">board history</p>
          <div className="mt-0.5 text-[11px] text-spill-muted">
            <p>{filteredBoards.length} matching boards</p>
            {query.trim() || status !== "all" ? (
              <p>{query.trim() ? `for "${query.trim()}"` : "all boards"}{status !== "all" ? ` - ${phaseLabel(status)}` : ""}</p>
            ) : null}
          </div>
        </div>
        <form action="/" className="mt-2 flex w-full items-center gap-2" method="get">
          <div className="relative min-w-0 flex-1">
            <svg className="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-spill-muted" viewBox="0 0 20 20" fill="none" aria-hidden="true">
              <circle cx="8.5" cy="8.5" r="5.25" stroke="currentColor" strokeWidth="2" />
              <path d="m12.5 12.5 4 4" stroke="currentColor" strokeLinecap="round" strokeWidth="2" />
            </svg>
            <input
              aria-label="Search boards"
              className={`${fieldControlClass} w-full pl-8 ${query ? "pr-9" : ""}`}
              name="q"
              onChange={(event) => applyQuery(event.currentTarget.value)}
              placeholder="Search boards"
              ref={inputRef}
              type="text"
              value={query}
            />
            {query ? (
              <button
                aria-label="Clear board search"
                className="absolute right-2 top-1/2 grid h-6 w-6 -translate-y-1/2 place-items-center rounded-full text-[15px] font-bold leading-none text-spill-muted transition hover:bg-[var(--paper-2)] hover:text-spill-fg focus-visible:outline-none focus-visible:shadow-[var(--focus)]"
                onClick={clearQuery}
                type="button"
              >
                ×
              </button>
            ) : null}
          </div>
          <select
            aria-label="Filter boards by status"
            className={`${fieldControlClass} shrink-0 text-[12px]`}
            onChange={(event) => applyStatus(event.currentTarget.value)}
            name="status"
            style={{ width: 166 }}
            value={status}
          >
            {phaseOptions.map(([value, label]) => (
              <option key={value} value={value}>{label}</option>
            ))}
          </select>
          <button className="sr-only" type="submit">Apply board filters</button>
        </form>
      </div>

      <div className="sp-panel-grain mt-3 overflow-hidden rounded-[12px] border border-spill-line bg-spill-panel shadow-[var(--shadow-1)]" data-board-table>
        {rows.length === 0 ? (
          <div className="px-3.5 py-4 text-[12px] font-semibold text-spill-muted">No boards match that search.</div>
        ) : (
          rows.map((board) => (
            <div
              className="group relative border-b border-spill-line last:border-b-0 hover:bg-[var(--panel-hi)]"
              key={`${board.id}-${board.title}`}
            >
              <Link
                className="grid grid-cols-[minmax(0,1fr)_auto] items-start gap-4 px-3.5 py-2.5 pr-24 text-[12px] focus-visible:outline-none focus-visible:shadow-[inset_0_0_0_3px_rgba(207,79,79,0.20)]"
                data-board-row
                href={`/retros/${board.id}`}
              >
                <span className="min-w-0">
                  <span className="flex min-w-0 items-center gap-2">
                    {board.cover_gif_url ? <img alt={board.cover_gif_alt_text ?? ""} className="h-7 w-7 shrink-0 rounded-[7px] object-cover" src={board.cover_gif_url} /> : null}
                    <span className="block min-w-0 truncate font-extrabold text-spill-fg">{board.title}</span>
                    {board.phase === "completed" && board.team_mood ? <span className="shrink-0 rounded-full bg-[var(--paper)] px-2 py-0.5 text-[9px] font-extrabold uppercase tracking-[0.08em] text-spill-muted">{board.team_mood.replaceAll("-", " ")}</span> : null}
                  </span>
                  <span className="text-[11px] text-spill-muted">
                    {boardDateLine(board)} . updated {formatBoardDate(board.last_activity_at)} . {board.participant_count} people . {board.column_count} cols . {board.unresolved_action_count} actions
                  </span>
                </span>
                <span
                  className="mt-0.5 rounded-full border px-2 py-0.5 text-[10px] font-extrabold uppercase tracking-[0.08em]"
                  style={{
                    borderColor: `${phaseColor(board.phase)}66`,
                    backgroundColor: `${phaseColor(board.phase)}18`,
                    color: phaseColor(board.phase),
                  } as CSSProperties}
                >
                  {phaseLabel(board.phase)}
                </span>
              </Link>
              <div className="absolute right-1.5 top-1/2 flex -translate-y-1/2 items-center gap-1">
                {board.phase === "completed" ? (
                  <CloneRetroButton retroId={board.id} title={board.title} scheduledAt={board.scheduled_at} createdAt={board.created_at} />
                ) : null}
                <DeleteBoardButton retroId={board.id} boardTitle={board.title} />
              </div>
            </div>
          ))
        )}
        {filteredBoards.length > pageSize ? (
          <div className="flex items-center justify-between border-t border-spill-line bg-[var(--panel-hi)]/45 px-3.5 py-2 text-[11px] text-spill-muted">
            <span>Showing {visibleCount} of {filteredBoards.length}</span>
            {visibleCount < filteredBoards.length ? (
              <button className="font-extrabold text-spill-fg transition hover:text-spill-wrong" onClick={showMore} type="button">show more</button>
            ) : (
              <button className="font-extrabold text-spill-fg transition hover:text-spill-wrong" onClick={showLess} type="button">show less</button>
            )}
          </div>
        ) : null}
      </div>
    </section>
  );
}

function filterBoards(boards: RetroSummary[], query: string, status: string) {
  const q = query.trim().toLowerCase();
  return boards.filter((board) => {
    const tags = board.recurring_tags.filter((tag) => !SYSTEM_RECURRING_TAGS.has(tag.toLowerCase()));
    const matchesQuery = !q || board.title.toLowerCase().includes(q) || tags.some((tag) => tag.toLowerCase().includes(q));
    const matchesStatus = status === "all" || board.phase === status;
    return matchesQuery && matchesStatus;
  });
}

function historyHref(query: string, status: string, shown: number) {
  const params = new URLSearchParams();
  const trimmed = query.trim();
  if (trimmed) params.set("q", trimmed);
  if (status !== "all") params.set("status", status);
  if (shown > pageSize) params.set("show", String(shown));
  const search = params.toString();
  return search ? `/?${search}` : "/";
}

function formatBoardDate(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "updated recently";
  const now = new Date();
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    year: date.getFullYear() === now.getFullYear() ? undefined : "numeric",
  }).format(date);
}

function boardDateLine(board: RetroSummary) {
  if (board.phase === "completed" && board.completed_at) return `completed ${formatBoardDate(board.completed_at)}`;
  if (board.scheduled_at) return `scheduled ${formatBoardDate(board.scheduled_at)}`;
  return `created ${formatBoardDate(board.created_at)}`;
}

function normalizeStatus(status: string) {
  return phaseOptions.some(([value]) => value === status) ? status : "all";
}
