import Link from "next/link";
import { notFound } from "next/navigation";
import { getRetro, type GifResult } from "../../lib/api";
import { acceptDeckItemAction, castVoteAction, clusterBoardAction, completeRetroAction, confirmActionItemAction, createDraftCardAction, markReadyAction, rejectActionItemAction, retryAiJobAction, revealRetroAction, searchGifsAction, startActionDiscussionAction, startAiJobAction, startVotingAction, updateActionItemAction } from "../../lib/actions";
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
              {board.retro.phase === "completed"
                ? "This retro is completed. Confirmed/proposed actions remain visible for follow-through."
                : board.retro.phase === "writing"
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
                  <>
                    <form action={markReadyAction}>
                      <input name="retro_id" type="hidden" value={board.retro.id} />
                      <button className="primary" type="submit">
                        {board.ready.current_user_ready ? "Voting ready" : "Mark voting ready"}
                      </button>
                    </form>
                    <form action={startActionDiscussionAction}>
                      <input name="retro_id" type="hidden" value={board.retro.id} />
                      <button type="submit">Discuss top actions</button>
                    </form>
                  </>
                ) : null}
              </div>
            </div>
            {board.retro.phase === "action_discussion" ? (
              <form action={completeRetroAction}>
                <input name="retro_id" type="hidden" value={board.retro.id} />
                <button className="primary" type="submit">Complete retro</button>
              </form>
            ) : null}
            {board.retro.phase === "completed" ? <Link className="button" href="/history">Back to history</Link> : null}
            {board.retro.phase === "writing" && board.deck.length > 0 ? (
              <section className="scene action-panel">
                <div className="scene-head">
                  <p className="eyebrow">User deck</p>
                  <h3>Connector suggestions</h3>
                  <p>Private items from Pi, Claude Code, uploads, or other connectors. Accept one into a board column when ready.</p>
                </div>
                {board.deck.map((item) => (
                  <article className="card action-card" key={item.id}>
                    <p>{item.suggested_text ?? item.gif_url ?? "Connector item"}</p>
                    <div className="chips">
                      <span className="chip">{item.source.replaceAll("_", " ")}</span>
                      <span className="chip">{item.status}</span>
                    </div>
                    <form className="vote-row" action={acceptDeckItemAction}>
                      <input name="retro_id" type="hidden" value={board.retro.id} />
                      <input name="item_id" type="hidden" value={item.id} />
                      <label>
                        Column
                        <select name="column_id" defaultValue={board.columns[0]?.id}>
                          {board.columns.map((column) => (
                            <option key={column.id} value={column.id}>
                              {column.title}
                            </option>
                          ))}
                        </select>
                      </label>
                      <button className="primary" type="submit">Accept into board</button>
                    </form>
                  </article>
                ))}
              </section>
            ) : null}
            <section className="scene action-panel">
              <div className="scene-head">
                <p className="eyebrow">Optional AI</p>
                <h3>Human-reviewable board jobs</h3>
                <p>Fake-provider jobs persist status, retry failures, and expose outputs for review before any human uses them.</p>
              </div>
              <form className="vote-row" action={startAiJobAction}>
                <input name="retro_id" type="hidden" value={board.retro.id} />
                <label>
                  Job
                  <select name="kind" defaultValue="summary">
                    <option value="gif_suggestions">GIF suggestions</option>
                    <option value="clustering">Clustering</option>
                    <option value="action_suggestions">Action proposals</option>
                    <option value="summary">Summary</option>
                    <option value="mood">Team mood</option>
                    <option value="tagging">Tagging</option>
                  </select>
                </label>
                <label>
                  <input name="fail" type="checkbox" /> Simulate failure
                </label>
                <button className="primary" type="submit">Run AI job</button>
              </form>
              {board.ai_artifacts.map((artifact) => (
                <article className={`card action-card ${artifact.status}`} key={artifact.id}>
                  <div className="chips">
                    <span className="chip">{artifact.kind.replaceAll("_", " ")}</span>
                    <span className="chip">{artifact.status}</span>
                    <span className="chip">Retries: {artifact.retry_count}</span>
                  </div>
                  {artifact.error_message ? <p className="muted">{artifact.error_message}</p> : null}
                  {artifact.output ? <pre>{JSON.stringify(artifact.output, null, 2)}</pre> : null}
                  {artifact.status === "failed" ? (
                    <form action={retryAiJobAction}>
                      <input name="retro_id" type="hidden" value={board.retro.id} />
                      <input name="artifact_id" type="hidden" value={artifact.id} />
                      <button type="submit">Retry</button>
                    </form>
                  ) : null}
                </article>
              ))}
            </section>
            {board.retro.phase === "action_discussion" || board.retro.phase === "completed" ? (
              <section className="scene action-panel">
                <div className="scene-head">
                  <p className="eyebrow">Action agenda</p>
                  <h3>{board.retro.phase === "completed" ? "Action follow-through" : "Top voted action cards"}</h3>
                  <p>Default top {board.retro.action_discussion_limit}; ties follow oldest-card order. Tags support recurring-action history.</p>
                </div>
                {board.actions.map((action) => (
                  <article className={`card action-card ${action.status}`} key={action.id}>
                    <form className="draft-form" action={updateActionItemAction}>
                      <input name="retro_id" type="hidden" value={board.retro.id} />
                      <input name="action_id" type="hidden" value={action.id} />
                      <label>
                        Action
                        <input name="title" defaultValue={action.title} />
                      </label>
                      <label>
                        Details
                        <textarea name="details" rows={2} defaultValue={action.details ?? ""} />
                      </label>
                      <div className="vote-row">
                        <span className="chip">{action.status}</span>
                        {action.tags.map((tag) => <span className="chip" key={tag}>{tag}</span>)}
                        <button type="submit">Save edit</button>
                      </div>
                    </form>
                    <div className="vote-row">
                      <form action={confirmActionItemAction}>
                        <input name="retro_id" type="hidden" value={board.retro.id} />
                        <input name="action_id" type="hidden" value={action.id} />
                        <button className="primary" type="submit">Confirm</button>
                      </form>
                      <form action={rejectActionItemAction}>
                        <input name="retro_id" type="hidden" value={board.retro.id} />
                        <input name="action_id" type="hidden" value={action.id} />
                        <button type="submit">Reject</button>
                      </form>
                    </div>
                  </article>
                ))}
              </section>
            ) : null}
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
