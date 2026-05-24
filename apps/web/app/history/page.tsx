import Link from "next/link";
import { AppChrome, Pill, SectionTitle, Tile, phaseColor } from "../components/spill-ui";
import { listRetros, type RetroOverview, type RetroSummary } from "../lib/api";

export default async function HistoryPage() {
  const overview = await loadOverview();
  const boards = [...overview.active, ...overview.completed];
  const activeBoards = overview.active.slice(0, 6);
  const recentBoards = boards.slice(0, 10);

  return (
    <AppChrome
      title="history"
      subtitle={`${overview.active.length} active · ${overview.completed.length} wrapped · ${recurringThemeCount(boards)} recurring theme`}
      actions={
        <>
          <Pill>recent</Pill>
          <Pill>themes</Pill>
          <Pill href="/retros/new" tone="danger">new board</Pill>
        </>
      }
    >
      <div className="grid min-h-[calc(100dvh-5rem)] grid-cols-1 gap-7 p-7 lg:grid-cols-[minmax(0,1fr)_340px]">
        <section className="min-w-0">
          <SectionTitle kicker="compact board memory, newest first">history</SectionTitle>
          {boards.length === 0 ? (
            <Tile className="mt-5">
              <p className="font-bold">No boards yet.</p>
              <p className="mt-1 text-sm text-spill-muted">Once retros wrap, they show up here as team memory.</p>
            </Tile>
          ) : (
            <div className="mt-5 space-y-6">
              {activeBoards.length > 0 ? (
                <div>
                  <p className="text-xs font-extrabold uppercase tracking-widest text-spill-muted">still pinned</p>
                  <div className="mt-3 grid gap-3 md:grid-cols-2">
                    {activeBoards.map((board) => (
                      <BoardTile board={board} key={board.id} />
                    ))}
                  </div>
                </div>
              ) : null}

              <div>
                <div className="flex items-baseline justify-between gap-4">
                  <p className="text-xs font-extrabold uppercase tracking-widest text-spill-muted">recent boards</p>
                  <p className="text-xs text-spill-muted">showing {recentBoards.length} of {boards.length}</p>
                </div>
                <div className="mt-3 overflow-hidden rounded-xl border border-spill-line bg-spill-panel">
                  {recentBoards.map((board) => (
                    <Link
                      className="grid grid-cols-[minmax(0,1fr)_120px_90px] items-center gap-4 border-b border-spill-line px-4 py-3 text-sm transition last:border-b-0 hover:bg-spill-soft"
                      href={`/retros/${board.id}`}
                      key={board.id}
                    >
                      <span className="min-w-0">
                        <span className="block truncate font-extrabold">{board.title}</span>
                        <span className="text-xs text-spill-muted">actions {Math.max(0, board.unresolved_action_count)}/{board.action_discussion_limit}</span>
                      </span>
                      <span className="text-xs font-bold uppercase tracking-wider" style={{ color: phaseColor(board.phase) }}>{phaseLabel(board.phase)}</span>
                      <span className="text-right text-xs text-spill-muted">{board.column_count} cols</span>
                    </Link>
                  ))}
                </div>
              </div>
            </div>
          )}
        </section>

        <aside className="border-l border-spill-line pl-6">
          <p className="text-xs font-extrabold uppercase tracking-widest text-spill-muted">recurring themes</p>
          <div className="mt-4 space-y-3">
            <div className="rounded-xl border border-spill-wrong/30 bg-spill-wrong/10 p-4">
              <p className="text-xs font-extrabold uppercase tracking-wider text-spill-wrong">● open</p>
              <h3 className="mt-2 text-lg font-extrabold">"flaky CI"</h3>
              <p className="text-sm text-spill-muted">3 retros · 2 unresolved actions</p>
              <div className="mt-3">
                <Pill>see all 3 →</Pill>
              </div>
            </div>
            <div className="rounded-xl border border-spill-line bg-spill-panel p-4">
              <p className="text-xs font-extrabold uppercase tracking-wider text-spill-well">● resolved</p>
              <h3 className="mt-2 text-lg font-extrabold">"slow review cycles"</h3>
              <p className="text-sm text-spill-muted">4 retros · last seen 4/2</p>
            </div>
          </div>

          <div className="mt-6">
            <p className="text-xs font-extrabold uppercase tracking-widest text-spill-muted">open actions · {openActionCount(boards)}</p>
            <div className="mt-3 space-y-3 text-sm">
              {["quarantine flake", "flake counter", "own onboarding doc", "demo retro to product"].map((action, index) => (
                <div className="flex items-center gap-3" key={action}>
                  <span className={`h-4 w-4 rounded border ${index === 2 ? "border-spill-wrong" : "border-spill-line"}`} />
                  <span className="flex-1">{action}</span>
                  <span className={index === 2 ? "text-spill-wrong" : "text-spill-muted"}>{index === 2 ? "@nat" : "@sam"} · s42</span>
                </div>
              ))}
            </div>
          </div>
        </aside>
      </div>
    </AppChrome>
  );
}

function BoardTile({ board }: { board: RetroSummary }) {
  return (
    <Link
      className="rounded-xl border border-spill-line bg-spill-panel p-4 shadow-[0_1px_0_#d4c39d,0_5px_12px_rgba(42,34,27,0.07)] transition hover:-translate-y-0.5"
      href={`/retros/${board.id}`}
      style={{ borderLeft: `4px solid ${phaseColor(board.phase)}` }}
    >
      <div className="flex justify-between gap-4">
        <h2 className="truncate font-extrabold">{board.title}</h2>
        <span className="shrink-0 text-xs font-bold uppercase tracking-wider" style={{ color: phaseColor(board.phase) }}>{phaseLabel(board.phase)}</span>
      </div>
      <p className="mt-2 text-sm text-spill-muted">
        actions {Math.max(0, board.unresolved_action_count)}/{board.action_discussion_limit}
        <span> · {board.column_count} cols</span>
      </p>
    </Link>
  );
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

function openActionCount(boards: RetroSummary[]) {
  return boards.reduce((sum, board) => sum + board.unresolved_action_count, 0);
}

function recurringThemeCount(boards: RetroSummary[]) {
  return boards.reduce((sum, board) => sum + board.recurring_tags.length, 0);
}
