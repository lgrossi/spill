import Link from "next/link";
import { Pill, Tile } from "../../components/spill-ui";
import type { RetroBoard, RetroCard } from "../../lib/api";
import { completeActionItemAction, confirmActionItemAction } from "../../lib/actions";
import { BoardMedia } from "./media-card";
import { actionVoteCount, cardLabel, columnSemantic, isActionsColumn, voteLabel } from "./board-presentation";

export function WrappedSummary({ board }: { board: RetroBoard }) {
  const boardColumns = board.columns.filter((column) => !isActionsColumn(column));
  const allCards = board.columns.flatMap((column) => column.cards.filter((card) => !card.hidden));
  const cards = boardColumns.flatMap((column) => column.cards.filter((card) => !card.hidden));
  const actionCounts = board.actions.reduce(
    (counts, action) => ({ ...counts, [action.status]: (counts[action.status] ?? 0) + 1 }),
    {} as Record<string, number>,
  );

  return (
    <section className="grid flex-1 grid-cols-1 gap-8 overflow-auto p-6 md:p-8 lg:grid-cols-[minmax(0,1fr)_340px]">
      <main className="min-w-0">
        <div className="flex items-baseline gap-3">
          <h1 className="text-[38px] font-extrabold leading-none tracking-[-0.035em] text-spill-fg">That's a wrap.</h1>
          <span className="-rotate-2 font-hand text-[24px] text-spill-muted">nice work, team.</span>
        </div>

        <div className="mt-5 flex items-center gap-5">
          <div className="grid h-[100px] w-[100px] shrink-0 place-items-center rounded-full border-2 border-[#246f4e] bg-[radial-gradient(circle_at_35%_30%,#3eb486,#2f9469_70%)] text-[22px] font-extrabold tracking-[-0.02em] text-white shadow-[var(--shadow-3),inset_0_2px_0_rgba(255,255,255,0.25)]">steady</div>
          <div>
            <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">team mood . stub preview - coming soon</p>
            <h2 className="mt-0.5 text-2xl font-extrabold tracking-[-0.02em] text-spill-fg">Steady.</h2>
            <p className="mt-1 max-w-xl text-[13.5px] leading-6 text-[var(--fg-2)]">AI summarization is not wired yet. The rest of this wrap uses the actual board cards, votes, and actions.</p>
          </div>
        </div>

        <FinalBoard board={board} boardColumns={boardColumns} cards={cards} />
        <CommittedActions allCards={allCards} board={board} />
      </main>

      <aside className="space-y-5 border-t border-spill-line pt-5 lg:border-l lg:border-t-0 lg:pl-6 lg:pt-0">
        <Tile>
          <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">board totals</p>
          <div className="mt-3 grid gap-2 text-[12.5px] text-spill-fg">
            <div className="flex justify-between gap-3"><span>Cards</span><strong>{cards.length}</strong></div>
            <div className="flex justify-between gap-3"><span>Votes cast</span><strong>{cards.reduce((sum, card) => sum + card.vote_count, 0)}</strong></div>
            <div className="flex justify-between gap-3"><span>Actions</span><strong>{board.actions.length}</strong></div>
            <div className="flex justify-between gap-3"><span>Done</span><strong>{actionCounts.done ?? 0}</strong></div>
          </div>
        </Tile>

        <Tile className="border-spill-action/50 bg-spill-action/10">
          <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-action">AI wrap-up</p>
          <h3 className="mt-2 text-[15px] font-bold text-spill-fg">Coming soon</h3>
          <p className="mt-1 text-[11.5px] leading-5 text-spill-muted">Summaries, recurring themes, and suggested next-retro setup will appear here once the AI summarizer is wired.</p>
        </Tile>
      </aside>
    </section>
  );
}

function FinalBoard({
  board,
  boardColumns,
  cards,
}: {
  board: RetroBoard;
  boardColumns: RetroBoard["columns"];
  cards: RetroCard[];
}) {
  return (
    <div className="mt-6">
      <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">final board . {cards.length} cards</p>
      <div className="sp-scroll mt-2.5 overflow-x-auto pb-2">
        <div
          className="grid gap-3"
          style={{
            gridTemplateColumns: `repeat(${Math.max(1, boardColumns.length)}, minmax(220px, 1fr))`,
            minWidth: `${Math.max(1, boardColumns.length) * 240}px`,
          }}
        >
          {boardColumns.map((column, index) => {
            const semantic = columnSemantic(column, index);
            const visibleCards = column.cards.filter((card) => !card.hidden);
            return (
              <Tile className="min-w-0" key={column.id}>
                <div className="mb-2 flex items-center gap-2">
                  <span className="h-2 w-2 rounded-full" style={{ background: semantic.color }} />
                  <p className="truncate text-[10.5px] font-extrabold uppercase tracking-[0.12em]" style={{ color: semantic.color }}>{column.title}</p>
                  <span className="ml-auto text-[10.5px] font-semibold text-spill-muted">{visibleCards.length}</span>
                </div>
                <div className="space-y-2">
                  {visibleCards.map((card) => <WrappedBoardCard card={card} key={card.id} />)}
                  {visibleCards.length === 0 ? <p className="rounded-[8px] border border-dashed border-spill-line px-3 py-2 text-[11.5px] text-spill-muted">No cards</p> : null}
                </div>
              </Tile>
            );
          })}
        </div>
      </div>
    </div>
  );
}

function CommittedActions({ allCards, board }: { allCards: RetroCard[]; board: RetroBoard }) {
  return (
    <div className="mt-6">
      <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-action">actions committed . {board.actions.length}</p>
      <div className="mt-2.5 space-y-2">
        {board.actions.map((action) => (
          <div className="sp-panel-grain flex items-center gap-3 rounded-[12px] border border-spill-line bg-spill-panel p-4 shadow-[var(--shadow-1)]" id={`action-${action.id}`} key={action.id}>
            <span className="grid h-[22px] w-[22px] shrink-0 place-items-center rounded-[6px] bg-spill-action text-[12px] font-extrabold text-white">ok</span>
            <Link className="min-w-0 flex-1 truncate text-[13.5px] font-medium text-spill-fg hover:underline" href={`/retros/${board.retro.id}#action-${action.id}`}>{action.title}</Link>
            <span className="shrink-0 rounded-full bg-[var(--paper)] px-2 py-0.5 text-[10px] font-bold text-spill-muted">{voteLabel(actionVoteCount(action, allCards))}</span>
            <Pill tone={action.status === "done" ? "success" : "neutral"}>{action.status}</Pill>
            {action.status === "done" ? (
              <form action={confirmActionItemAction}>
                <input name="retro_id" type="hidden" value={board.retro.id} />
                <input name="action_id" type="hidden" value={action.id} />
                <button aria-label="Reopen action" className={actionCheckClass(true)} type="submit">ok</button>
              </form>
            ) : action.status !== "rejected" ? (
              <form action={completeActionItemAction}>
                <input name="retro_id" type="hidden" value={board.retro.id} />
                <input name="action_id" type="hidden" value={action.id} />
                <button aria-label="Mark action done" className={actionCheckClass(false)} type="submit">ok</button>
              </form>
            ) : null}
          </div>
        ))}
      </div>
    </div>
  );
}

function WrappedBoardCard({ card }: { card: RetroCard }) {
  return (
    <div className="rounded-[8px] border border-spill-line bg-[var(--panel-hi)] px-3 py-2 text-[12.5px] leading-5 text-spill-fg">
      {card.gif_url ? <BoardMedia alt={card.gif_alt_text ?? "Attached media"} src={card.gif_url} /> : null}
      <div className="flex items-start gap-2">
        <p className="min-w-0 flex-1 whitespace-pre-wrap">{cardLabel(card)}</p>
        {card.vote_count > 0 ? <span className="shrink-0 rounded-full bg-white px-2 py-0.5 text-[10px] font-bold text-spill-muted">{voteLabel(card.vote_count)}</span> : null}
      </div>
      {card.cluster_members.length > 0 ? (
        <div className="mt-2 space-y-1 border-t border-spill-line pt-2">
          {card.cluster_members.map((member) => (
            <div className="rounded-[6px] bg-white/55 px-2 py-1 text-[11.5px] text-spill-muted" key={member.id}>
              {member.gif_url ? <BoardMedia alt={member.gif_alt_text ?? "Grouped media"} src={member.gif_url} /> : null}
              <p className="mt-1 first:mt-0">{member.body_text || member.gif_alt_text || "media card"}</p>
            </div>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function actionCheckClass(checked: boolean) {
  return `grid h-7 w-7 place-items-center rounded-[7px] border text-[13px] font-extrabold leading-none transition hover:brightness-[0.98] focus-visible:outline-none focus-visible:shadow-[var(--focus)] ${
    checked ? "border-spill-well bg-spill-well text-white" : "border-[var(--line-2)] bg-white text-transparent hover:border-spill-well hover:text-spill-well"
  }`;
}
