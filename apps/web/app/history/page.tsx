import Link from "next/link";
import { listRetros } from "../lib/api";

export default async function HistoryPage() {
  const overview = await loadOverview();
  const boards = [...overview.completed, ...overview.active];

  return (
    <main>
      <header className="topbar">
        <div>
          <p className="eyebrow">History</p>
          <h1>Previous retros</h1>
        </div>
        <nav className="tabs" aria-label="Main navigation">
          <Link className="button" href="/">
            Overview
          </Link>
          <Link className="button" href="/retros/new">
            Create
          </Link>
        </nav>
      </header>
      <div className="page single">
        <section className="scene">
          <div className="scene-head">
            <p className="eyebrow">Memory</p>
            <h2>Boards, not analytics cosplay</h2>
            <p>History currently reopens persisted boards. Tags, unresolved actions, and recurring pain come in later slices.</p>
          </div>
          <div className="grid three">
            {boards.length === 0 ? <p className="muted">No boards yet.</p> : null}
            {boards.map((board) => (
              <Link className="panel tape" href={`/retros/${board.id}`} key={board.id}>
                <h3>{board.title}</h3>
                <p className="muted">{board.phase.replaceAll("_", " ")}</p>
                <div className="chips">
                  <span className="chip">{board.column_count} columns</span>
                  <span className="chip">{board.participant_count} participants</span>
                </div>
              </Link>
            ))}
          </div>
        </section>
      </div>
    </main>
  );
}

async function loadOverview() {
  try {
    return await listRetros();
  } catch {
    return { active: [], completed: [] };
  }
}
