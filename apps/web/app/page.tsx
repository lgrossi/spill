import Link from "next/link";
import { listRetros, type RetroOverview } from "./lib/api";

export default async function OverviewPage() {
  const overview = await loadOverview();

  return (
    <main>
      <Topbar />
      <div className="page single">
        <section className="scene">
          <div className="scene-head">
            <p className="eyebrow">Overview</p>
            <h1>Retro table</h1>
            <p>Start a board, rejoin active writing sessions, or reopen completed retros.</p>
          </div>
          <div className="grid two">
            <article className="panel tape">
              <h2>Start from the table</h2>
              <p className="muted">Standard boards open with Mood, Went well, Went wrong, and Actions.</p>
              <Link className="button primary" href="/retros/new">
                Create retro
              </Link>
            </article>
            <BoardList title="Active retros" empty="No active retros yet." boards={overview.active} />
            <BoardList title="Completed retros" empty="No completed retros yet." boards={overview.completed} />
          </div>
        </section>
      </div>
    </main>
  );
}

function Topbar() {
  return (
    <header className="topbar">
      <div>
        <p className="eyebrow">SpillItOut</p>
        <h1>Board-first retro table</h1>
      </div>
      <nav className="tabs" aria-label="Main navigation">
        <Link className="button" href="/">
          Overview
        </Link>
        <Link className="button" href="/retros/new">
          Create
        </Link>
        <Link className="button" href="/history">
          History
        </Link>
      </nav>
    </header>
  );
}

function BoardList({ title, empty, boards }: { title: string; empty: string; boards: RetroOverview["active"] }) {
  return (
    <article className="panel tape">
      <h2>{title}</h2>
      {boards.length === 0 ? <p className="muted">{empty}</p> : null}
      <div className="stack">
        {boards.map((board) => (
          <Link className="list-card" href={`/retros/${board.id}`} key={board.id}>
            <span className="row">
              <strong>{board.title}</strong>
              <span className="chip blue">{phaseLabel(board.phase)}</span>
            </span>
            <p className="muted">
              {board.participant_count} participant{board.participant_count === 1 ? "" : "s"} · {board.column_count} columns · {board.vote_limit} votes/person
            </p>
          </Link>
        ))}
      </div>
    </article>
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
