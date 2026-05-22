import Link from "next/link";
import { notFound } from "next/navigation";
import { getRetro, type GifResult, type RetroBoard } from "../../lib/api";
import {
  acceptDeckItemAction,
  castVoteAction,
  clusterBoardAction,
  completeRetroAction,
  confirmActionItemAction,
  createDeliveryAction,
  createDraftCardAction,
  createMeetingNoteAction,
  markReadyAction,
  rejectActionItemAction,
  retryAiJobAction,
  retryDeliveryAction,
  revealRetroAction,
  searchGifsAction,
  startActionDiscussionAction,
  startAiJobAction,
  startVotingAction,
  updateActionItemAction,
} from "../../lib/actions";
import { BoardSync } from "./board-sync";
import { BoardMedia } from "./media-card";

export default async function RetroBoardPage({
  params,
  searchParams,
}: {
  params: Promise<{ retroId: string }>;
  searchParams: Promise<{ addColumn?: string; gif?: string; gifColumn?: string; gifPage?: string; gifResults?: string; gifDegraded?: string; mediaKind?: string }>;
}) {
  const { retroId } = await params;
  const gifSearch = await searchParams;
  const board = await loadBoard(retroId);

  if (!board) {
    notFound();
  }

  const gifResults = parseGifResults(gifSearch.gifResults);
  const gifColumnId = gifSearch.gifColumn;
  const activeColumnId = gifColumnId ?? gifSearch.addColumn;
  const gifPage = Math.max(0, Number(gifSearch.gifPage ?? 0) || 0);
  const gifDegraded = gifSearch.gifDegraded === "1";
  const boardPhase = phaseLabel(board.retro.phase);
  const showToolRail =
    (board.retro.phase === "writing" && board.deck.length > 0) ||
    board.retro.phase === "completed" ||
    board.meeting_notes.length > 0 ||
    board.ai_artifacts.length > 0;

  return (
    <main className="board-app">
      <BoardSync retroId={board.retro.id} />
      <header className="board-topbar">
        <div className="board-title">
          <p className="eyebrow">SpillItOut</p>
          <h1>{board.retro.title}</h1>
        </div>
        <div className="chips board-status">
          <span className="chip blue">{boardPhase}</span>
          <span className="chip">
            Ready {board.ready.ready_count}/{board.ready.participant_count}
          </span>
          {board.retro.phase === "voting" ? <span className="chip">Votes left {board.voting.votes_remaining}</span> : null}
        </div>
        <nav className="tabs" aria-label="Main navigation">
          <Link className="button" href="/">
            Overview
          </Link>
          <Link className="button" href="/retros/new">
            New
          </Link>
          <Link className="button" href="/history">
            History
          </Link>
        </nav>
      </header>

      <section className={`board-shell ${showToolRail ? "with-tools" : ""}`}>
        <article className="board board-primary" aria-label="Retro board">
          <div className="board-head">
            <div>
              <div className="chips">
                <span className="chip blue">{board.retro.phase === "writing" ? "Draft board" : "Revealed board"}</span>
                <span className="chip">{board.retro.vote_limit} votes/person</span>
              </div>
              <h2>{board.retro.title}</h2>
              <p className="muted">
                {board.retro.phase === "writing"
                  ? "Private writing: cards stay blurred for everyone else until reveal."
                  : "Shared board: discuss, vote, and turn the right cards into actions."}
              </p>
            </div>
            <PhaseControls board={board} />
          </div>

          <div className={`columns board-columns ${board.columns.length === 5 ? "five" : ""}`}>
            {board.columns.map((column) => {
              const columnActions = isActionsColumn(column) ? board.actions : [];
              const columnCount = column.cards.length + columnActions.length;
              const isActiveColumn = activeColumnId === column.id;
              return (
                <section className="column" key={column.id}>
                  <div className="column-head">
                    <div>
                      <h3>{column.title}</h3>
                      <small>{board.retro.phase === "writing" ? "Private drafts" : "Shared cards"}</small>
                    </div>
                    <span className="chip">{columnCount}</span>
                  </div>

                  <div className="card-stack">
                    {column.cards.map((card) => (
                      <article className={`card ${card.hidden ? "hidden" : ""}`} key={card.id}>
                        {card.hidden ? (
                          <>
                            <span className="skeleton" />
                            <span className="skeleton short" />
                          </>
                        ) : (
                          <>
                            {card.gif_url ? <BoardMedia alt={card.gif_alt_text ?? "Attached media"} src={card.gif_url} /> : null}
                            {card.body_text ? <p>{card.body_text}</p> : null}
                            {card.cluster_title ? (
                              <p className="merged">
                                {card.cluster_title} · {card.cluster_category}
                              </p>
                            ) : null}
                            {board.retro.phase === "voting" ? (
                              <div className="vote-row">
                                <span className="vote">{card.vote_count} votes</span>
                                {card.current_user_vote_count > 0 ? <span className="chip">You {card.current_user_vote_count}</span> : null}
                                <form action={castVoteAction}>
                                  <input name="retro_id" type="hidden" value={board.retro.id} />
                                  <input name="card_id" type="hidden" value={card.id} />
                                  <button type="submit" disabled={board.voting.votes_remaining <= 0}>
                                    Vote
                                  </button>
                                </form>
                              </div>
                            ) : null}
                          </>
                        )}
                      </article>
                    ))}
                    {columnActions.map((action) => (
                      <ActionBoardCard action={action} key={action.id} retroId={board.retro.id} />
                    ))}
                  </div>

                  {board.retro.phase === "writing" ? (
                    isActiveColumn ? (
                      <InlineComposer
                        columnId={column.id}
                        columnTitle={column.title}
                        degraded={gifDegraded && gifColumnId === column.id}
                        gifPage={gifPage}
                        gifQuery={gifColumnId === column.id ? gifSearch.gif ?? "" : ""}
                        gifResults={gifColumnId === column.id ? gifResults : []}
                        mediaKind={gifSearch.mediaKind ?? "gif"}
                        retroId={board.retro.id}
                      />
                    ) : (
                      <Link className="add-card-row" href={`/retros/${board.retro.id}?addColumn=${column.id}`}>
                        +
                      </Link>
                    )
                  ) : null}
                </section>
              );
            })}
          </div>
        </article>

        {showToolRail ? (
          <aside className="board-tools" aria-label="Board tools">
            {board.retro.phase === "writing" && board.deck.length > 0 ? <DeckTool board={board} /> : null}
            {board.retro.phase === "completed" ? <DeliveryTool board={board} /> : null}
            {board.meeting_notes.length > 0 ? <NotesTool board={board} /> : null}
            {board.ai_artifacts.length > 0 ? <AiTool board={board} /> : null}
          </aside>
        ) : null}
      </section>
    </main>
  );
}

function PhaseControls({ board }: { board: RetroBoard }) {
  if (board.retro.phase === "writing") {
    return (
      <div className="actions">
        <form action={markReadyAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <button className="primary" type="submit">
            {board.ready.current_user_ready ? "Ready marked" : "Mark ready"}
          </button>
        </form>
        <form action={revealRetroAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <button type="submit">Reveal</button>
        </form>
      </div>
    );
  }

  if (board.retro.phase === "discussion") {
    return (
      <div className="actions">
        <form action={clusterBoardAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <button type="submit" disabled={board.clusters.length > 0}>
            Cluster-fy
          </button>
        </form>
        <form action={startVotingAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <button className="primary" type="submit">
            Start voting
          </button>
        </form>
      </div>
    );
  }

  if (board.retro.phase === "voting") {
    return (
      <div className="actions">
        <form action={markReadyAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <button className="primary" type="submit">
            {board.ready.current_user_ready ? "Voting ready" : "Mark ready"}
          </button>
        </form>
        <form action={startActionDiscussionAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <button type="submit">Actions</button>
        </form>
      </div>
    );
  }

  if (board.retro.phase === "action_discussion") {
    return (
      <form action={completeRetroAction}>
        <input name="retro_id" type="hidden" value={board.retro.id} />
        <button className="primary" type="submit">
          Complete
        </button>
      </form>
    );
  }

  return (
    <Link className="button" href="/history">
      History
    </Link>
  );
}

function InlineComposer({
  columnId,
  columnTitle,
  degraded,
  gifPage,
  gifQuery,
  gifResults,
  mediaKind,
  retroId,
}: {
  columnId: string;
  columnTitle: string;
  degraded: boolean;
  gifPage: number;
  gifQuery: string;
  gifResults: GifResult[];
  mediaKind: string;
  retroId: string;
}) {
  const selectedKind = mediaKind === "sticker" ? "sticker" : "gif";
  const serializedResults = JSON.stringify(gifResults);

  return (
    <div className="inline-composer" aria-label={`Add ${columnTitle} card`}>
      <form className="draft-form quick-card-form" action={createDraftCardAction}>
        <input name="retro_id" type="hidden" value={retroId} />
        <input name="column_id" type="hidden" value={columnId} />
        <textarea name="body_text" rows={1} placeholder={`Add ${columnTitle} card`} />
        {gifResults.length > 0 ? (
          <div className="media-result-strip">
            {gifResults.map((gif) => (
              <label className="media-result" key={`${gif.id}-${gif.url}`}>
                <input name="gif_choice" type="radio" value={JSON.stringify({ url: gif.url, altText: gif.alt_text })} />
                <img className="gif-thumb" src={gif.preview_url || gif.url} alt="" loading="lazy" />
                <span>{gif.alt_text}</span>
              </label>
            ))}
          </div>
        ) : null}
        <button className="primary" type="submit">
          Add
        </button>
      </form>
      <form className="gif-search-form inline-gif-search media-search-row" action={searchGifsAction} autoComplete="off">
        <input name="retro_id" type="hidden" value={retroId} />
        <input name="column_id" type="hidden" value={columnId} />
        <input name="gif_page" type="hidden" value="0" />
        <div className="media-kind-toggle" aria-label="Media type">
          <button aria-pressed={selectedKind === "gif"} name="media_kind" type="submit" value="gif">
            GIFs
          </button>
          <button aria-pressed={selectedKind === "sticker"} name="media_kind" type="submit" value="sticker">
            Stickers
          </button>
        </div>
        <input name="gif_query" defaultValue={gifQuery} placeholder="Search media" autoComplete="off" aria-label="Media search" />
        <button name="media_kind" type="submit" value={selectedKind}>
          Search
        </button>
      </form>
      {degraded ? <p className="muted compact-copy">Media search failed. Text cards still work.</p> : null}
      {gifQuery && gifResults.length > 0 ? (
        <form className="gif-pager" action={searchGifsAction}>
          <input name="retro_id" type="hidden" value={retroId} />
          <input name="column_id" type="hidden" value={columnId} />
          <input name="gif_query" type="hidden" value={gifQuery} />
          <input name="gif_page" type="hidden" value={gifPage + 1} />
          <input name="media_kind" type="hidden" value={selectedKind} />
          <input name="gif_existing_results" type="hidden" value={serializedResults} />
          <button className="media-result media-more" type="submit">More</button>
        </form>
      ) : null}
    </div>
  );
}

function ActionBoardCard({ action, retroId }: { action: RetroBoard["actions"][number]; retroId: string }) {
  return (
    <article className={`card action-board-card ${action.status}`}>
      <form className="action-card-form" action={updateActionItemAction}>
        <input name="retro_id" type="hidden" value={retroId} />
        <input name="action_id" type="hidden" value={action.id} />
        <input name="title" defaultValue={action.title} aria-label="Action title" />
        <textarea name="details" rows={2} defaultValue={action.details ?? ""} aria-label="Action details" />
        <div className="vote-row">
          <span className="chip">{action.status}</span>
          <button type="submit">Save</button>
        </div>
      </form>
      <div className="vote-row">
        <form action={confirmActionItemAction}>
          <input name="retro_id" type="hidden" value={retroId} />
          <input name="action_id" type="hidden" value={action.id} />
          <button className="primary" type="submit">Confirm</button>
        </form>
        <form action={rejectActionItemAction}>
          <input name="retro_id" type="hidden" value={retroId} />
          <input name="action_id" type="hidden" value={action.id} />
          <button type="submit">Reject</button>
        </form>
      </div>
    </article>
  );
}

function DeckTool({ board }: { board: RetroBoard }) {
  return (
    <section className="tool-card">
      <h3>Deck</h3>
      {board.deck.map((item) => (
        <article className="tool-item" key={item.id}>
          <p>{item.suggested_text ?? item.gif_url ?? "Connector item"}</p>
          <form className="compact-form" action={acceptDeckItemAction}>
            <input name="retro_id" type="hidden" value={board.retro.id} />
            <input name="item_id" type="hidden" value={item.id} />
            <select name="column_id" defaultValue={board.columns[0]?.id}>
              {board.columns.map((column) => (
                <option key={column.id} value={column.id}>
                  {column.title}
                </option>
              ))}
            </select>
            <button type="submit">Accept</button>
          </form>
        </article>
      ))}
    </section>
  );
}

function NotesTool({ board }: { board: RetroBoard }) {
  return (
    <details className="tool-card">
      <summary>Notes</summary>
      <form className="compact-form" action={createMeetingNoteAction}>
        <input name="retro_id" type="hidden" value={board.retro.id} />
        <input name="title" defaultValue="Meeting notes" aria-label="Notes title" />
        <textarea name="body_text" rows={3} placeholder="Paste notes" aria-label="Notes" />
        <button type="submit">Attach</button>
      </form>
      {board.meeting_notes.map((note) => (
        <article className="tool-item" key={note.id}>
          <strong>{note.title}</strong>
          <p>{note.body_text}</p>
        </article>
      ))}
    </details>
  );
}

function AiTool({ board }: { board: RetroBoard }) {
  return (
    <details className="tool-card">
      <summary>AI</summary>
      <form className="compact-form" action={startAiJobAction}>
        <input name="retro_id" type="hidden" value={board.retro.id} />
        <select name="kind" defaultValue="summary" aria-label="AI job">
          <option value="gif_suggestions">GIF suggestions</option>
          <option value="clustering">Clustering</option>
          <option value="action_suggestions">Action proposals</option>
          <option value="summary">Summary</option>
          <option value="mood">Team mood</option>
          <option value="tagging">Tagging</option>
        </select>
        <label className="inline-check">
          <input name="fail" type="checkbox" /> fail
        </label>
        <button type="submit">Run</button>
      </form>
      {board.ai_artifacts.map((artifact) => (
        <article className={`tool-item ${artifact.status}`} key={artifact.id}>
          <div className="chips">
            <span className="chip">{artifact.kind.replaceAll("_", " ")}</span>
            <span className="chip">{artifact.status}</span>
          </div>
          {artifact.error_message ? <p>{artifact.error_message}</p> : null}
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
    </details>
  );
}

function DeliveryTool({ board }: { board: RetroBoard }) {
  return (
    <section className="tool-card">
      <h3>Delivery</h3>
      <form className="compact-form" action={createDeliveryAction}>
        <input name="retro_id" type="hidden" value={board.retro.id} />
        <select name="kind" defaultValue="summary_export" aria-label="Delivery kind">
          <option value="summary_export">Summary export</option>
          <option value="external_action_link">External action link</option>
        </select>
        <label className="inline-check">
          <input name="fail" type="checkbox" /> fail
        </label>
        <button type="submit">Create</button>
      </form>
      {board.deliveries.map((delivery) => (
        <article className={`tool-item ${delivery.status}`} key={delivery.id}>
          <div className="chips">
            <span className="chip">{delivery.kind.replaceAll("_", " ")}</span>
            <span className="chip">{delivery.status}</span>
          </div>
          {delivery.error_message ? <p>{delivery.error_message}</p> : null}
          {delivery.output ? <pre>{JSON.stringify(delivery.output, null, 2)}</pre> : null}
          {delivery.status === "failed" ? (
            <form action={retryDeliveryAction}>
              <input name="retro_id" type="hidden" value={board.retro.id} />
              <input name="delivery_id" type="hidden" value={delivery.id} />
              <button type="submit">Retry</button>
            </form>
          ) : null}
        </article>
      ))}
    </section>
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

function phaseLabel(phase: RetroBoard["retro"]["phase"]) {
  return phase.replaceAll("_", " ");
}

function isActionsColumn(column: RetroBoard["columns"][number]) {
  return column.column_key === "actions" || column.title.toLowerCase().includes("action");
}
