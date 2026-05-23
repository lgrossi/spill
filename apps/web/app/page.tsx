import Link from "next/link";
import { AppChrome, PhaseBadge, Pill, SectionTitle, Tile, phaseColor } from "./components/spill-ui";
import { listRetros, type RetroOverview } from "./lib/api";

export default async function OverviewPage() {
  const overview = await loadOverview();
  const activeBoards = overview.active;
  const completedBoards = overview.completed;
  const featured = activeBoards[0];

  return (
    <AppChrome
      title="boards"
      subtitle={`${activeBoards.length} active · ${completedBoards.length} archived`}
      actions={
        <>
          <Pill href="/history">history</Pill>
          <Pill href="/retros/new" tone="danger">new board</Pill>
        </>
      }
    >
      <div className="grid min-h-[calc(100dvh-5rem)] grid-cols-1 gap-6 p-6 lg:grid-cols-[1fr_360px]">
        <section>
          <SectionTitle kicker="boards in motion">still pinned</SectionTitle>
          <div className="mt-5 grid gap-4 md:grid-cols-2">
            {activeBoards.length === 0 ? (
              <Tile className="md:col-span-2">
                <p className="text-sm font-semibold uppercase tracking-wider text-spill-muted">quick start</p>
                <h2 className="mt-4 text-2xl font-bold">nothing on the wall yet</h2>
                <p className="mt-2 max-w-md text-sm text-spill-muted">Create a standard board and start privately. Drafts stay hidden until reveal.</p>
                <div className="mt-5">
                  <Pill href="/retros/new" tone="danger">+ new standard board</Pill>
                </div>
              </Tile>
            ) : (
              activeBoards.map((board) => (
                <BoardCard board={board} featured={board.id === featured?.id} key={board.id} />
              ))
            )}
          </div>

          <div className="mt-7">
            <p className="text-xs font-bold uppercase tracking-wider text-spill-muted">quick start</p>
            <div className="mt-2 flex flex-wrap gap-2">
              <Pill href="/retros/new" tone="danger">+ new standard board</Pill>
              <Pill href="/retros/new">+ custom columns</Pill>
              <Pill href="/retros/new" dashed>+ from template</Pill>
            </div>
          </div>
        </section>

        <aside className="border-l border-spill-line pl-6">
          <SectionTitle kicker="themes that keep coming back">still on the wall</SectionTitle>
          <div className="mt-5 rounded-xl border border-spill-wrong/40 bg-spill-wrong/10 p-4">
            <p className="text-xs font-extrabold uppercase tracking-wider text-spill-wrong">● recurring</p>
            <h3 className="mt-2 text-lg font-extrabold">"flaky CI"</h3>
            <p className="text-sm text-spill-muted">3 boards in a row · 2 open actions</p>
          </div>

          <div className="mt-5">
            <p className="text-xs font-bold uppercase tracking-wider text-spill-muted">open actions · {openActionCount(overview)}</p>
            <div className="mt-3 space-y-2 text-sm">
              {["quarantine flake suite", "flake counter on deploy gate", "own the onboarding doc", "demo retro process to product"].map((action, index) => (
                <div className="flex items-center gap-3" key={action}>
                  <span className={`h-4 w-4 rounded border ${index === 2 ? "border-spill-wrong" : "border-spill-line"}`} />
                  <span className="flex-1">{action}</span>
                  <span className={index === 2 ? "text-spill-wrong" : "text-spill-muted"}>{index === 2 ? "@nat" : index === 1 ? "@sam" : "@lucas"}</span>
                </div>
              ))}
            </div>
          </div>

          <div className="mt-5 border-t border-dashed border-spill-line pt-4">
            <p className="text-xs font-bold uppercase tracking-wider text-spill-muted">recent</p>
            <div className="mt-3 space-y-2 text-sm">
              {completedBoards.slice(0, 4).map((board) => (
                <Link className="flex justify-between gap-4" href={`/retros/${board.id}`} key={board.id}>
                  <span>{board.title}</span>
                  <span className="text-spill-muted">{phaseLabel(board.phase)}</span>
                </Link>
              ))}
              {completedBoards.length === 0 ? <p className="text-spill-muted">No past boards yet.</p> : null}
            </div>
          </div>
        </aside>
      </div>
    </AppChrome>
  );
}

function BoardCard({ board, featured }: { board: RetroOverview["active"][number]; featured?: boolean }) {
  const color = phaseColor(board.phase);
  return (
    <Link
      className="relative flex min-h-44 flex-col rounded-xl border bg-spill-panel p-4 shadow-[0_1px_0_#d4c39d,0_5px_12px_rgba(42,34,27,0.07)] transition hover:-translate-y-0.5 hover:shadow-[0_1px_0_#d4c39d,0_8px_18px_rgba(42,34,27,0.12)]"
      href={`/retros/${board.id}`}
      style={{ borderColor: featured ? `${color}80` : "#d4c39d", boxShadow: featured ? `0 0 0 2px ${color}55, 0 5px 12px rgba(42,34,27,0.08)` : undefined }}
    >
      <span className="absolute -top-2 left-4">
        <PhaseBadge phase={phaseLabel(board.phase)} color={color} />
      </span>
      <h2 className="mt-4 text-lg font-extrabold">{board.title}</h2>
      <p className="mt-1 text-sm text-spill-muted">
        {board.participant_count} participant{board.participant_count === 1 ? "" : "s"} · {board.vote_limit} votes/person
      </p>
      <div className="mt-auto flex items-end justify-between pt-8">
        <div className="flex">
          {["na", "lu", "sa", "kt"].slice(0, Math.max(1, Math.min(4, board.participant_count || 1))).map((initials, index) => (
            <span className="-ml-1 first:ml-0 grid h-6 w-6 place-items-center rounded-full border border-spill-line bg-spill-bg text-[10px]" key={initials}>
              {initials}
            </span>
          ))}
        </div>
        <span className="text-xs font-semibold" style={{ color }}>open →</span>
      </div>
    </Link>
  );
}

function openActionCount(overview: RetroOverview) {
  return [...overview.active, ...overview.completed].reduce((sum, board) => sum + board.unresolved_action_count, 0);
}

async function loadOverview(): Promise<RetroOverview> {
  try {
    return await listRetros();
  } catch {
    return { active: [], completed: [] };
  }
}

function phaseLabel(phase: string) {
  return phase.replaceAll("_", " ");
}
