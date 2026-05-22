import Link from "next/link";
import { notFound } from "next/navigation";
import { getRetro } from "../../lib/api";

export default async function RetroBoardPage({ params }: { params: Promise<{ retroId: string }> }) {
  const { retroId } = await params;
  const board = await loadBoard(retroId);

  if (!board) {
    notFound();
  }

  return (
    <main>
      <header className="topbar">
        <div>
          <p className="eyebrow">Board</p>
          <h1>{board.retro.title}</h1>
          <p>Phase: {board.retro.phase.replaceAll("_", " ")}</p>
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
      <div className="page single">
        <section className="scene">
          <div className="scene-head">
            <p className="eyebrow">Writing</p>
            <h2>Draft board</h2>
            <p>Cards are private by default. Other participants' card contents remain blurred until everyone is ready and the board is revealed.</p>
          </div>
          <article className="board">
            <div className="board-head">
              <div>
                <div className="chips">
                  <span className="chip blue">Writing</span>
                  <span className="chip">Reveal locked</span>
                  <span className="chip">{board.retro.vote_limit} votes/person</span>
                </div>
                <h3>{board.retro.title}</h3>
                <p className="muted">Persisted board id: {board.retro.id}</p>
              </div>
              <div className="actions">
                <button className="primary" disabled>
                  Mark ready
                </button>
                <button disabled>Reveal waits for writing slice</button>
              </div>
            </div>
            <div className={`columns ${board.columns.length === 5 ? "five" : ""}`}>
              {board.columns.map((column) => (
                <section className="column" key={column.id}>
                  <div className="column-head">
                    <div>
                      <h4>{column.title}</h4>
                      <small>Private drafts</small>
                    </div>
                    <span className="chip">0</span>
                  </div>
                  <article className="card hidden" aria-label={`${column.title} blurred draft placeholder`}>
                    <span className="skeleton" />
                    <span className="skeleton short" />
                  </article>
                </section>
              ))}
            </div>
          </article>
        </section>
      </div>
    </main>
  );
}

async function loadBoard(retroId: string) {
  try {
    return await getRetro(retroId);
  } catch {
    return null;
  }
}
