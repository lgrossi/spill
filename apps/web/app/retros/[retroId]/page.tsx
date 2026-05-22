import Link from "next/link";
import { notFound } from "next/navigation";
import { getRetro } from "../../lib/api";
import { createDraftCardAction, markReadyAction, revealRetroAction } from "../../lib/actions";

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
            <p className="eyebrow">{board.retro.phase.replaceAll("_", " ")}</p>
            <h2>{board.retro.phase === "writing" ? "Draft board" : "Revealed board"}</h2>
            <p>
              {board.retro.phase === "writing"
                ? "Cards are private by default. Other participants' card contents remain blurred until everyone is ready and the board is revealed."
                : "The same board is now the shared discussion surface."}
            </p>
          </div>
          <article className="board">
            <div className="board-head">
              <div>
                <div className="chips">
                  <span className="chip blue">{board.retro.phase.replaceAll("_", " ")}</span>
                  <span className="chip">
                    Ready: {board.ready.ready_count}/{board.ready.participant_count}
                  </span>
                  <span className="chip">{board.retro.vote_limit} votes/person</span>
                </div>
                <h3>{board.retro.title}</h3>
                <p className="muted">Persisted board id: {board.retro.id}</p>
              </div>
              <div className="actions">
                {board.retro.phase === "writing" ? (
                  <>
                    <form action={markReadyAction}>
                      <input name="retro_id" type="hidden" value={board.retro.id} />
                      <button className="primary" type="submit">
                        {board.ready.current_user_ready ? "Ready marked" : "Mark ready"}
                      </button>
                    </form>
                    <form action={revealRetroAction}>
                      <input name="retro_id" type="hidden" value={board.retro.id} />
                      <button type="submit">Reveal board</button>
                    </form>
                  </>
                ) : null}
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
                    <span className="chip">{column.cards.length}</span>
                  </div>
                  {column.cards.map((card) => (
                    <article className={`card ${card.hidden ? "hidden" : ""}`} key={card.id}>
                      {card.hidden ? (
                        <>
                          <span className="skeleton" />
                          <span className="skeleton short" />
                        </>
                      ) : (
                        <p>{card.body_text}</p>
                      )}
                    </article>
                  ))}
                  {board.retro.phase === "writing" ? (
                    <form className="draft-form" action={createDraftCardAction}>
                      <input name="retro_id" type="hidden" value={board.retro.id} />
                      <input name="column_id" type="hidden" value={column.id} />
                      <textarea name="body_text" rows={3} required placeholder={`Add private ${column.title} draft`} />
                      <button type="submit">Add draft</button>
                    </form>
                  ) : null}
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
