import Link from "next/link";
import { notFound } from "next/navigation";
import { AppChrome, PhaseBadge, Pill, StatusPill, spillColors } from "../../components/spill-ui";
import { getRetro, type GifResult, type RetroBoard } from "../../lib/api";
import {
  acceptDeckItemAction,
  castVoteAction,
  clusterBoardAction,
  completeRetroAction,
  confirmActionItemAction,
  createDraftCardAction,
  markReadyAction,
  rejectActionItemAction,
  revealRetroAction,
  searchGifsAction,
  startActionDiscussionAction,
  startVotingAction,
} from "../../lib/actions";
import { BoardSync } from "./board-sync";
import { BoardMedia } from "./media-card";

export default async function RetroBoardPage({
  params,
  searchParams,
}: {
  params: Promise<{ retroId: string }>;
  searchParams: Promise<{ addColumn?: string; draftText?: string; gif?: string; gifColumn?: string; gifPage?: string; gifResults?: string; gifDegraded?: string; mediaKind?: string }>;
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

  return (
    <AppChrome
      title={board.retro.title}
      subtitle={`${phaseLabel(board.retro.phase)} · ${board.ready.ready_count} of ${board.ready.participant_count} ready`}
      actions={<PhaseControls board={board} />}
    >
      <BoardSync retroId={board.retro.id} />
      <section className="flex min-h-[calc(100dvh-5rem)] flex-col">
        <div className="grid flex-1 grid-cols-1 gap-4 p-5 lg:grid-cols-4">
          {board.columns.map((column, index) => {
            const color = columnColor(column, index);
            const columnActions = isActionsColumn(column) ? board.actions : [];
            const columnCount = column.cards.length + columnActions.length;
            const isActiveColumn = activeColumnId === column.id;

            return (
              <section className="grid min-h-[460px] grid-rows-[auto_minmax(0,1fr)_auto] gap-3" key={column.id}>
                <header className="flex items-center gap-2">
                  <span className="h-2.5 w-2.5 rounded-full" style={{ backgroundColor: color }} />
                  <h2 className="font-bold lowercase">{column.title}</h2>
                  <span className="ml-auto text-xs text-spill-muted">{columnCount || "—"}</span>
                </header>

                <div className="space-y-3 overflow-auto pr-1">
                  {column.cards.map((card) => (
                    <CardView board={board} card={card} color={color} key={card.id} />
                  ))}
                  {columnActions.map((action) => (
                    <ActionCard action={action} key={action.id} retroId={board.retro.id} />
                  ))}
                  {isActionsColumn(column) && board.retro.phase !== "action_discussion" && board.actions.length === 0 ? (
                    <div className="rounded-xl border border-dashed border-spill-line p-8 text-center text-sm text-spill-muted">
                      {board.retro.phase === "writing" ? "opens after voting" : "fills with top-voted cards"}
                    </div>
                  ) : null}
                </div>

                {board.retro.phase === "writing" && !isActionsColumn(column) ? (
                  isActiveColumn ? (
                    <InlineComposer
                      columnId={column.id}
                      columnTitle={column.title}
                      color={color}
                      draftText={gifColumnId === column.id ? gifSearch.draftText ?? "" : ""}
                      degraded={gifDegraded && gifColumnId === column.id}
                      gifPage={gifPage}
                      gifQuery={gifColumnId === column.id ? gifSearch.gif ?? "" : ""}
                      gifResults={gifColumnId === column.id ? gifResults : []}
                      mediaKind={gifSearch.mediaKind ?? "gif"}
                      retroId={board.retro.id}
                    />
                  ) : (
                    <Link
                      className="grid min-h-10 place-items-center rounded-xl border border-dashed text-sm transition hover:bg-spill-panel"
                      href={`/retros/${board.retro.id}?addColumn=${column.id}`}
                      style={{ borderColor: `${color}55`, color }}
                    >
                      + add {column.title.toLowerCase()} card
                    </Link>
                  )
                ) : null}
              </section>
            );
          })}
        </div>

        {board.retro.phase === "writing" && board.deck.length > 0 ? <Deck board={board} /> : null}
      </section>
    </AppChrome>
  );
}

function PhaseControls({ board }: { board: RetroBoard }) {
  if (board.retro.phase === "writing") {
    const allReady = board.ready.participant_count > 0 && board.ready.ready_count >= board.ready.participant_count;
    return (
      <>
        <Presence ready={board.ready.ready_count} total={board.ready.participant_count} />
        <PhaseBadge phase="writing" color={spillColors.mood} />
        <form action={markReadyAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <Pill tone="danger" type="submit">{board.ready.current_user_ready ? "pinned" : "pin yours up"}</Pill>
        </form>
        <form action={revealRetroAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <Pill type="submit" disabled={!allReady}>reveal →</Pill>
        </form>
      </>
    );
  }

  if (board.retro.phase === "discussion") {
    return (
      <>
        <PhaseBadge phase="cluster-fy" color={spillColors.action} />
        <form action={clusterBoardAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <Pill type="submit" disabled={board.clusters.length > 0}>cluster-fy</Pill>
        </form>
        <form action={startVotingAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <Pill tone="success" type="submit">start voting</Pill>
        </form>
      </>
    );
  }

  if (board.retro.phase === "voting") {
    return (
      <>
        <PhaseBadge phase="voting" color={spillColors.action} />
        <StatusPill>{board.voting.votes_remaining} votes left</StatusPill>
        <form action={startActionDiscussionAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <Pill tone="danger" type="submit">actions →</Pill>
        </form>
      </>
    );
  }

  if (board.retro.phase === "action_discussion") {
    return (
      <>
        <PhaseBadge phase="action" color={spillColors.action} />
        <Pill>← prev</Pill>
        <Pill>next →</Pill>
        <form action={completeRetroAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <Pill type="submit">wrap retro</Pill>
        </form>
      </>
    );
  }

  return (
    <>
      <PhaseBadge phase="wrapped" color={spillColors.well} />
      <Pill href="/history">history</Pill>
    </>
  );
}

function Presence({ ready, total }: { ready: number; total: number }) {
  const people = ["na", "lu", "sa", "kt"].slice(0, Math.max(1, Math.min(4, total || 1)));
  return (
    <div className="hidden items-center md:flex">
      {people.map((person, index) => (
        <span className="-ml-2 first:ml-0 grid h-7 w-7 place-items-center rounded-full border-2 border-spill-bg bg-spill-soft text-[10px] font-bold" key={person}>
          {person}
          <span className={`absolute mt-5 h-2 w-2 rounded-full ${index < ready ? "bg-spill-well" : "bg-spill-mood"}`} />
        </span>
      ))}
    </div>
  );
}

function CardView({ board, card, color }: { board: RetroBoard; card: RetroBoard["columns"][number]["cards"][number]; color: string }) {
  if (card.hidden) {
    return (
      <article className="rounded-xl border border-dashed p-5 text-center text-sm" style={{ borderColor: `${color}55`, backgroundColor: `${color}18`, color }}>
        ··· someone's draft ···
      </article>
    );
  }

  return (
    <article className="rounded-xl p-3 text-white shadow-[0_8px_14px_rgba(42,34,27,0.13)]" style={{ backgroundColor: color }}>
      {card.gif_url ? <BoardMedia alt={card.gif_alt_text ?? "Attached media"} src={card.gif_url} /> : null}
      {card.body_text ? <p className="mt-2 whitespace-pre-wrap text-sm font-medium leading-5 first:mt-0">{card.body_text}</p> : null}
      {card.cluster_title ? <p className="mt-3 border-t border-white/30 pt-2 text-xs uppercase tracking-wider">{card.cluster_title}</p> : null}
      {board.retro.phase === "voting" ? (
        <div className="mt-3 flex items-center gap-2">
          <span className="rounded-full bg-white/20 px-2 py-1 text-xs font-bold">{card.vote_count}v</span>
          <form action={castVoteAction}>
            <input name="retro_id" type="hidden" value={board.retro.id} />
            <input name="card_id" type="hidden" value={card.id} />
            <button className="rounded-full bg-white px-3 py-1 text-xs font-bold" style={{ color }} type="submit" disabled={board.voting.votes_remaining <= 0}>
              vote
            </button>
          </form>
        </div>
      ) : null}
    </article>
  );
}

function InlineComposer({
  columnId,
  columnTitle,
  color,
  draftText,
  degraded,
  gifPage,
  gifQuery,
  gifResults,
  mediaKind,
  retroId,
}: {
  columnId: string;
  columnTitle: string;
  color: string;
  draftText: string;
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
    <div className="rounded-xl p-3 text-white" aria-label={`Add ${columnTitle} card`} style={{ backgroundColor: color }}>
      <form className="grid gap-3" action={createDraftCardAction}>
        <input name="retro_id" type="hidden" value={retroId} />
        <input name="column_id" type="hidden" value={columnId} />
        <textarea className="border-0 bg-transparent p-0 text-sm font-medium text-white placeholder:text-white/75 focus:shadow-none" name="body_text" rows={2} defaultValue={draftText} placeholder={`add ${columnTitle.toLowerCase()} card`} />
        {gifResults.length > 0 ? (
          <div className="flex gap-2 overflow-x-auto pb-1">
            {gifResults.map((gif) => (
              <label className="grid w-24 shrink-0 gap-1 rounded-lg bg-spill-bg p-1 text-[10px] text-spill-fg" key={`${gif.id}-${gif.url}`}>
                <input name="gif_choice" type="radio" value={JSON.stringify({ url: gif.url, altText: gif.alt_text })} />
                <img className="aspect-[1.2] w-full rounded object-contain" src={gif.preview_url || gif.url} alt="" loading="lazy" />
                <span className="line-clamp-2">{gif.alt_text}</span>
              </label>
            ))}
          </div>
        ) : null}
        <div className="flex items-center gap-2">
          <span className="text-xs text-white/75">esc</span>
          <button className="ml-auto rounded-full bg-white px-3 py-1 text-xs font-bold" style={{ color }} type="submit">pin ↵</button>
        </div>

        <input name="gif_page" type="hidden" value="0" />
        <div className="mt-1 grid grid-cols-[auto_minmax(0,1fr)_auto] gap-2">
          <div className="flex gap-1">
            <button className="rounded-full bg-white px-2 py-1 text-xs font-bold" formAction={searchGifsAction} name="media_kind" style={{ color: selectedKind === "gif" ? color : "#7a6c54" }} type="submit" value="gif">+ GIF</button>
            <button className="rounded-full bg-white/25 px-2 py-1 text-xs font-bold text-white" formAction={searchGifsAction} name="media_kind" type="submit" value="sticker">+ sticker</button>
          </div>
          <input className="rounded-full border-0 bg-white px-3 py-1 text-xs text-spill-fg focus:shadow-none" name="gif_query" defaultValue={gifQuery} placeholder="Search media" autoComplete="off" aria-label="Media search" />
          <button className="rounded-full bg-white/25 px-2 py-1 text-xs font-bold text-white" formAction={searchGifsAction} name="media_kind" type="submit" value={selectedKind}>search</button>
        </div>
        {degraded ? <p className="mt-2 text-xs text-white/80">Media search failed. Text cards still work.</p> : null}
        {gifQuery && gifResults.length > 0 ? (
          <div className="mt-2">
            <input name="media_kind" type="hidden" value={selectedKind} />
            <input name="gif_existing_results" type="hidden" value={serializedResults} />
            <button className="w-full rounded-lg border border-dashed border-white/45 py-2 text-xs font-bold text-white" formAction={searchGifsAction} name="gif_page" type="submit" value={gifPage + 1}>more</button>
          </div>
        ) : null}
      </form>
    </div>
  );
}

function ActionCard({ action, retroId }: { action: RetroBoard["actions"][number]; retroId: string }) {
  return (
    <article className="rounded-xl border border-spill-line bg-spill-panel p-3">
      <h3 className="font-bold">{action.title}</h3>
      {action.details ? <p className="mt-1 text-sm text-spill-muted">{action.details}</p> : null}
      <div className="mt-3 flex flex-wrap gap-2">
        <form action={confirmActionItemAction}>
          <input name="retro_id" type="hidden" value={retroId} />
          <input name="action_id" type="hidden" value={action.id} />
          <Pill tone="success" type="submit">✓ confirm</Pill>
        </form>
        <form action={rejectActionItemAction}>
          <input name="retro_id" type="hidden" value={retroId} />
          <input name="action_id" type="hidden" value={action.id} />
          <Pill type="submit">skip</Pill>
        </form>
      </div>
    </article>
  );
}

function Deck({ board }: { board: RetroBoard }) {
  return (
    <aside className="m-5 mt-0 rounded-xl border border-spill-line bg-spill-panel p-4">
      <div className="mb-3 flex items-center gap-3">
        <p className="text-xs font-extrabold uppercase tracking-wider text-spill-muted">your deck</p>
        <span className="rounded-full bg-spill-action px-3 py-1 text-xs font-bold text-white">suggestions · {board.deck.length}</span>
        <span className="ml-auto text-xs italic text-spill-muted">↑ drag chips into a column</span>
      </div>
      <div className="flex gap-3 overflow-x-auto pb-1">
        {board.deck.map((item) => (
          <form className="w-64 shrink-0 rounded-lg border border-spill-line bg-spill-bg p-3" action={acceptDeckItemAction} key={item.id}>
            <input name="retro_id" type="hidden" value={board.retro.id} />
            <input name="item_id" type="hidden" value={item.id} />
            <select className="mb-2 px-2 py-1 text-xs" name="column_id" defaultValue={board.columns[0]?.id}>
              {board.columns.filter((column) => !isActionsColumn(column)).map((column) => (
                <option key={column.id} value={column.id}>{column.title}</option>
              ))}
            </select>
            <p className="min-h-12 text-sm font-semibold">{item.suggested_text ?? item.gif_url ?? "Connector item"}</p>
            <button className="mt-3 text-xs font-bold text-spill-muted" type="submit">↑ drag / accept</button>
          </form>
        ))}
      </div>
    </aside>
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

function phaseLabel(phase: string) {
  return phase.replaceAll("_", " ");
}

function columnColor(column: RetroBoard["columns"][number], index: number) {
  const title = column.title.toLowerCase();
  if (title.includes("mood")) return spillColors.mood;
  if (title.includes("well") || title.includes("liked") || title.includes("learned")) return spillColors.well;
  if (title.includes("wrong") || title.includes("lacked")) return spillColors.wrong;
  if (isActionsColumn(column)) return spillColors.action;
  return [spillColors.mood, spillColors.well, spillColors.wrong, spillColors.action][index % 4];
}

function isActionsColumn(column: RetroBoard["columns"][number]) {
  return column.column_key === "actions" || column.title.toLowerCase().includes("action");
}
