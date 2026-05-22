import Link from "next/link";
import { notFound } from "next/navigation";
import { getRetro, type GifResult } from "../../lib/api";
import { castVoteAction, clusterBoardAction, createDraftCardAction, markReadyAction, revealRetroAction, searchGifsAction, startVotingAction } from "../../lib/actions";
import { BoardSync } from "./board-sync";

export default async function RetroBoardPage({
  params,
  searchParams,
}: {
  params: Promise<{ retroId: string }>;
  searchParams: Promise<{ gif?: string; gifResults?: string; gifDegraded?: string }>;
}) {
  const { retroId } = await params;
  const gifSearch = await searchParams;
  const board = await loadBoard(retroId);

  if (!board) {
    notFound();
  }

  const gifResults = parseGifResults(gifSearch.gifResults);
  const gifDegraded = gifSearch.gifDegraded === "1";

  return (
    <main>
      <BoardSync retroId={board.retro.id} />
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
                  {board.retro.phase === "voting" ? <span className="chip">Votes left: {board.voting.votes_remaining}</span> : null}
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
                {board.retro.phase === "discussion" ? (
                  <>
                    <form action={clusterBoardAction}>
                      <input name="retro_id" type="hidden" value={board.retro.id} />
                      <button type="submit" disabled={board.clusters.length > 0}>Cluster-fy once</button>
                    </form>
                    <form action={startVotingAction}>
                      <input name="retro_id" type="hidden" value={board.retro.id} />
                      <button className="primary" type="submit">Start voting</button>
                    </form>
                  </>
                ) : null}
                {board.retro.phase === "voting" ? (
                  <form action={markReadyAction}>
                    <input name="retro_id" type="hidden" value={board.retro.id} />
                    <button className="primary" type="submit">
                      {board.ready.current_user_ready ? "Voting ready" : "Mark voting ready"}
                    </button>
                  </form>
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
                        <>
                          {card.body_text ? <p>{card.body_text}</p> : null}
                          {card.gif_url ? <div className="gif">{card.gif_alt_text ?? "Attached GIF"}</div> : null}
                          {card.cluster_title ? (
                            <p className="merged">{card.cluster_title} · {card.cluster_category}</p>
                          ) : null}
                          {board.retro.phase === "voting" ? (
                            <div className="vote-row">
                              <span className="vote">{card.vote_count} votes</span>
                              {card.current_user_vote_count > 0 ? <span className="chip">You: {card.current_user_vote_count}</span> : null}
                              <form action={castVoteAction}>
                                <input name="retro_id" type="hidden" value={board.retro.id} />
                                <input name="card_id" type="hidden" value={card.id} />
                                <button type="submit" disabled={board.voting.votes_remaining <= 0}>Vote</button>
                              </form>
                            </div>
                          ) : null}
                        </>
                      )}
                    </article>
                  ))}
                  {board.retro.phase === "writing" ? (
                    <div className="draft-form">
                      <form action={searchGifsAction}>
                        <input name="retro_id" type="hidden" value={board.retro.id} />
                        <label>
                          GIF search
                          <input name="gif_query" defaultValue={gifSearch.gif ?? ""} placeholder="high five, ship it, confused" />
                        </label>
                        <button type="submit">Search GIFs</button>
                      </form>
                      {gifDegraded ? <p className="muted">GIF search is degraded. You can still add a text-only card.</p> : null}
                      <form className="draft-form" action={createDraftCardAction}>
                        <input name="retro_id" type="hidden" value={board.retro.id} />
                        <input name="column_id" type="hidden" value={column.id} />
                        <textarea name="body_text" rows={3} placeholder={`Add private ${column.title} draft`} />
                        {gifResults.length > 0 ? (
                          <div className="gif-grid">
                            {gifResults.map((gif) => (
                              <label className="gif-option" key={gif.id}>
                                <input name="gif_choice" type="radio" value={JSON.stringify({ url: gif.url, altText: gif.alt_text })} />
                                <span>{gif.alt_text}</span>
                              </label>
                            ))}
                          </div>
                        ) : null}
                        <button type="submit">Add draft</button>
                      </form>
                    </div>
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

function parseGifResults(value?: string): GifResult[] {
  if (!value) {
    return [];
  }
  try {
    const parsed = JSON.parse(value);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}
