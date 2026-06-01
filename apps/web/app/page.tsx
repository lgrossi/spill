import Link from "next/link";
import type { CSSProperties } from "react";
import { BoardHistory } from "@/components/board-history";
import { IdentityGate, IdentityUnavailable } from "@/components/identity-gate";
import { AppChrome, Avatar, Btn, PhaseBadge, Pill, SectionTitle, Tile, avatarColorForSeed, avatarInitials, phaseColor, phaseLabel, spillColors } from "@/components/spill-ui";
import { DeleteBoardButton } from "@/components/delete-board-button";
import { listRetros, type RetroOverview, type RetroSummary } from "@/lib/api";
import { SYSTEM_RECURRING_TAGS } from "@/lib/contracts";
import { displayRetroDate, isPlannedForDue } from "@/lib/retro-dates";
import { clearIdentityAction, completeActionItemAction } from "@/lib/actions";
import { currentIdentity, localIdentityEnabled, type SpillIdentity } from "@/lib/identity";

export default async function OverviewPage({
  searchParams,
}: {
  searchParams: Promise<{ q?: string; status?: string; show?: string }>;
}) {
  const filters = await searchParams;
  const identity = await currentIdentity();
  if (!identity) {
    return localIdentityEnabled() ? <IdentityGate /> : <IdentityUnavailable />;
  }

  const overviewResult = await loadOverview();
  if (!overviewResult.ok) {
    return <ApiUnavailable identity={identity} message={overviewResult.message} />;
  }

  const overview = overviewResult.overview;
  const allBoards = [...overview.active, ...overview.completed].sort((a, b) => Date.parse(b.last_activity_at) - Date.parse(a.last_activity_at));
  const pinnedBoards = overview.active.slice(0, 4);
  const openActions = allBoards.flatMap((board) => (board.open_actions ?? []).map((action) => ({ ...action, board }))).slice(0, 5);
  const recentBoards = [...allBoards]
    .sort((a, b) => Date.parse(b.last_opened_at ?? b.last_activity_at) - Date.parse(a.last_opened_at ?? a.last_activity_at))
    .slice(0, 4);
  const recurringTag = topRecurringTag(allBoards);

  return (
    <AppChrome
      actions={
        <>
          {identity.source === "local" ? (
            <form action={clearIdentityAction}>
              <input name="return_to" type="hidden" value="/" />
              <Btn kind="secondary" type="submit">{identity.displayName}</Btn>
            </form>
          ) : null}
        </>
      }
      presence={<UserAvatar identity={identity} status="ready" />}
    >
      <div className="grid flex-1 grid-cols-1 gap-8 overflow-y-auto p-6 md:p-8 lg:grid-cols-[minmax(0,1fr)_380px] lg:gap-10 lg:px-10">
        <section className="min-w-0">
          <div className="flex items-end justify-between gap-4">
            <SectionTitle kicker="boards in motion">Still pinned</SectionTitle>
            <Link aria-label="Create new board" className="text-[30px] font-extrabold leading-none transition hover:scale-105 focus-visible:outline-none focus-visible:shadow-[var(--focus)]" href="/retros/new" style={{ color: spillColors.wrong }} title="Create new board">+</Link>
          </div>
          <div className="mt-5 grid gap-4 md:grid-cols-2">
            {pinnedBoards.length === 0 ? (
              <Tile className="md:col-span-2">
                <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">quick start</p>
                <h2 className="mt-4 text-[28px] font-extrabold tracking-[-0.03em] text-spill-fg">Nothing on the wall yet.</h2>
                <p className="mt-2 max-w-md text-[13.5px] leading-6 text-[var(--fg-2)]">Create a Daylight Cork board and start writing. Drafts stay private until the team reveals them.</p>
                <div className="mt-5">
                  <Btn href="/retros/new" kind="primary">standard retro</Btn>
                </div>
              </Tile>
            ) : (
              pinnedBoards.map((board) => <BoardCard board={board} key={board.id} />)
            )}
          </div>

          <BoardHistory
            boards={allBoards}
            initialQuery={filters.q ?? ""}
            initialShown={Math.max(5, Number(filters.show ?? 5) || 5)}
            initialStatus={filters.status ?? "all"}
          />
        </section>

        <aside className="min-w-0 border-t border-spill-line pt-5 lg:border-l lg:border-t-0 lg:pl-6 lg:pt-0">
          <div>
            <div className="-rotate-1 font-hand text-[26px] leading-none text-spill-fg">still on the wall</div>
            <div className="mt-1 text-[11.5px] italic text-spill-muted">themes that keep coming back</div>
          </div>
          {recurringTag ? (
            <Tile className="mt-4 border-spill-wrong/50 bg-spill-wrong/10">
              <div className="flex items-center gap-2">
                <span className="h-2 w-2 rounded-full bg-spill-wrong shadow-[0_0_0_3px_rgba(207,79,79,0.2)]" />
                <span className="text-[10px] font-extrabold uppercase tracking-[0.12em] text-spill-wrong">recurring</span>
                <span className="ml-auto text-[10px] text-spill-muted">{recurringTag.count} boards</span>
              </div>
              <h3 className="mt-2 text-lg font-bold tracking-[-0.01em] text-spill-fg">#{recurringTag.tag}</h3>
              <p className="mt-0.5 text-xs text-spill-muted">Appears across board action tags.</p>
            </Tile>
          ) : (
            <Tile className="mt-4 border-dashed bg-transparent">
              <p className="text-[10px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">no recurring themes yet</p>
              <p className="mt-1.5 text-xs leading-5 text-spill-muted">Meaningful action tags will appear here after they repeat across boards.</p>
            </Tile>
          )}

          <div className="mt-5">
            <div className="flex items-center gap-2">
              <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">open actions</p>
              <Pill tone="ghost">{openActionCount(overview)}</Pill>
            </div>
            <div className="mt-3 space-y-2.5 text-[12.5px]">
              {openActions.length === 0 ? (
                <p className="text-spill-muted">No open actions.</p>
              ) : (
                openActions.map(({ board, ...action }) => (
                  <div className="flex items-center gap-2 rounded-[8px] py-1 hover:bg-[var(--panel-hi)]" key={`action-${action.id}`}>
                    <Link className="min-w-0 flex-1 truncate text-spill-fg hover:underline" href={`/retros/${board.id}#action-${action.id}`}>
                      {action.title}
                    </Link>
                    <span className="max-w-[86px] truncate text-[10.5px] text-spill-muted">{board.title}</span>
                    <form action={completeActionItemAction} className="shrink-0">
                      <input name="retro_id" type="hidden" value={board.id} />
                      <input name="action_id" type="hidden" value={action.id} />
                      <button
                        aria-label={`Mark ${action.title} done`}
                        className="grid h-7 w-7 place-items-center rounded-[8px] border border-spill-line bg-[var(--panel-hi)] text-[13px] font-extrabold text-spill-well shadow-[inset_0_1px_0_rgba(255,255,255,0.55),0_1px_0_rgba(74,52,20,0.06)] transition hover:border-spill-well hover:bg-[var(--panel)] focus-visible:outline-none focus-visible:shadow-[var(--focus)]"
                        title="Mark done"
                        type="submit"
                      >
                        ✓
                      </button>
                    </form>
                  </div>
                ))
              )}
            </div>
          </div>

          <div className="mt-5 border-t border-dashed border-spill-line pt-4">
            <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">recent</p>
            <div className="mt-3 space-y-2 text-[12.5px]">
              {recentBoards.length === 0 ? (
                <p className="text-spill-muted">No boards yet.</p>
              ) : (
                recentBoards.map((board) => (
                <Link className="group flex justify-between gap-4 py-1 text-spill-fg transition hover:text-spill-wrong" href={`/retros/${board.id}`} key={`recent-${board.id}`}>
                  <span className="flex min-w-0 items-center gap-2 truncate">
                    <span className="h-1.5 w-1.5 rounded-full" style={{ backgroundColor: phaseColor(board.phase) }} />
                    <span className="truncate group-hover:underline group-hover:decoration-spill-wrong/40 group-hover:underline-offset-2">{boardDisplayTitle(board)}</span>
                  </span>
                  <span className="shrink-0 text-spill-muted">{boardStatusLabel(board)}</span>
                </Link>
                ))
              )}
            </div>
          </div>

          <div className="mt-5 border-t border-dashed border-spill-line pt-4">
            <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">quick start</p>
            <div className="mt-3 grid gap-2">
              <QuickStartLink href="/retros/new?template=standard" title="Standard retro" detail="How are you feeling?, went well, to improve, actions" />
              <QuickStartLink href="/retros/new?template=4ls" title="4 Ls" detail="Liked, lacked, learned, longed for" />
              <QuickStartLink href="/retros/new?template=custom" title="Custom columns" detail="Start from editable column names" />
            </div>
          </div>

        </aside>
      </div>
    </AppChrome>
  );
}

function QuickStartLink({ href, title, detail }: { href: string; title: string; detail: string }) {
  return (
    <Link className="group rounded-[8px] border border-dashed border-spill-line px-3 py-2 transition hover:border-spill-wrong/50 hover:bg-spill-wrong/5" href={href}>
      <span className="block text-[12.5px] font-extrabold text-spill-fg group-hover:text-spill-wrong">{title}</span>
      <span className="mt-0.5 block text-[11px] leading-4 text-spill-muted">{detail}</span>
    </Link>
  );
}

function BoardCard({ board }: { board: RetroOverview["active"][number] }) {
  const color = phaseColor(board.phase);
  return (
    <div className="group relative">
      <Link
        className="sp-panel-grain relative flex h-[156px] flex-col rounded-[12px] border border-spill-line bg-spill-panel p-4 shadow-[var(--shadow-1)] transition hover:-translate-y-0.5 hover:border-[color:var(--board-phase-color)] hover:shadow-[var(--shadow-2)]"
        href={`/retros/${board.id}`}
        style={{ "--board-phase-color": color } as CSSProperties}
      >
        <span className="absolute -top-2.5 left-3.5">
          <PhaseBadge phase={boardStatusLabel(board)} color={color} />
        </span>
        <h2 className="mt-4 truncate text-[15px] font-extrabold leading-tight tracking-[-0.01em] text-spill-fg">{boardDisplayTitle(board)}</h2>
        <p className="mt-2 text-[12.5px] text-spill-muted">{displayRetroDate(board)}</p>
        <div className="mt-auto flex items-end justify-between">
          <span className="rounded-full bg-[var(--panel-hi)] px-2.5 py-1 text-[10.5px] font-extrabold text-spill-muted">
            {board.participant_count} {board.participant_count === 1 ? "person" : "people"}
            {board.phase === "writing" && board.participant_count > 0 && ` · ${board.ready_count} ready`}
          </span>
          <span className="h-1.5 w-1.5 rounded-full" style={{ backgroundColor: color }} />
        </div>
      </Link>
      {board.current_user_role === "host" ? (
        <div className="absolute right-2 top-2">
          <DeleteBoardButton retroId={board.id} boardTitle={board.title} />
        </div>
      ) : null}
    </div>
  );
}

function openActionCount(overview: RetroOverview) {
  return [...overview.active, ...overview.completed].reduce((sum, board) => sum + board.unresolved_action_count, 0);
}

function boardStatusLabel(board: RetroSummary) {
  return board.phase === "scheduled" && isPlannedForDue(board.planned_for) ? "ready" : phaseLabel(board.phase);
}

function boardDisplayTitle(board: RetroSummary) {
  return board.group_name ? `[${board.group_name}] ${board.title}` : board.title;
}

function topRecurringTag(boards: RetroSummary[]) {
  const counts = new Map<string, number>();
  for (const board of boards) {
    for (const tag of meaningfulRecurringTags(board)) {
      counts.set(tag, (counts.get(tag) ?? 0) + 1);
    }
  }

  const [tag, count] = [...counts.entries()].sort((a, b) => b[1] - a[1])[0] ?? [];
  if (!tag || !count || count < 2) return null;
  return { tag, count };
}

function meaningfulRecurringTags(board: RetroSummary) {
  return board.recurring_tags.filter((tag) => !SYSTEM_RECURRING_TAGS.has(tag.toLowerCase()));
}

async function loadOverview(): Promise<{ ok: true; overview: RetroOverview } | { ok: false; message: string }> {
  try {
    return { ok: true, overview: await listRetros() };
  } catch (error) {
    return {
      ok: false,
      message: error instanceof Error ? error.message : "Unable to reach the Spill API.",
    };
  }
}

function ApiUnavailable({ identity, message }: { identity: SpillIdentity; message: string }) {
  return (
    <AppChrome actions={<Btn href="/" kind="secondary">retry</Btn>} presence={<UserAvatar identity={identity} status="away" />}>
      <div className="flex flex-1 items-center justify-center p-6">
        <Tile className="max-w-lg border-spill-wrong/60 bg-spill-wrong/10">
          <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-wrong">boards are taking a coffee break</p>
          <h1 className="mt-3 text-[28px] font-extrabold tracking-[-0.03em] text-spill-fg">We can’t reach your boards right now.</h1>
          <p className="mt-2 text-[13.5px] leading-6 text-[var(--fg-2)]">Give it a moment and try again. Your retros are not shown until Spill can reconnect.</p>
          <p className="mt-4 text-[11px] font-semibold text-spill-muted">Details: {message}</p>
        </Tile>
      </div>
    </AppChrome>
  );
}

function UserAvatar({ identity, status }: { identity: SpillIdentity; status: "ready" | "away" }) {
  return (
    <Avatar
      color={avatarColorForSeed(identity.subject)}
      k={avatarInitials(identity.displayName)}
      size={28}
      status={status}
    />
  );
}
