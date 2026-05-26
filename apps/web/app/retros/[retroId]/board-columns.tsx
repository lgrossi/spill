import Link from "next/link";
import { ColumnHeader, Pill, columnIcons } from "../../components/spill-ui";
import type { RetroBoard } from "../../lib/api";
import { DropColumn, DropEndMarker } from "./board-dnd";
import { CardView } from "./card-view";
import { InlineComposer } from "./card-composer";
import { columnSemantic, sortedCards } from "./board-presentation";

type BoardSearchParams = {
  addColumn?: string;
  editCard?: string;
};

export function BoardColumns({ board, query }: { board: RetroBoard; query: BoardSearchParams }) {
  const columns = board.columns;
  const activeColumnId = query.editCard ? undefined : query.addColumn;
  const hasDeck = board.retro.phase === "writing" && board.deck.length > 0;
  const canAddCards = board.retro.phase !== "completed";

  return (
    <section className="flex min-h-0 flex-1 flex-col p-3 md:p-4" id="board">
      <div className="relative flex min-h-0 flex-1 flex-col overflow-hidden rounded-[14px] border border-spill-line bg-spill-panel shadow-[var(--shadow-2)]">
        <div className={`min-h-0 flex-1 overflow-x-auto px-4 pt-4 ${hasDeck ? "pb-36" : "pb-4"}`}>
          <div
            className="grid min-h-full gap-4 md:gap-[18px]"
            style={{
              gridTemplateColumns: `repeat(${Math.max(1, columns.length)}, minmax(250px, 1fr))`,
              minWidth: `${Math.max(1, columns.length) * 270}px`,
            }}
          >
            {columns.map((column, index) => {
              const semantic = columnSemantic(column, index);
              const visibleCards = sortedCards(column.cards, board.retro.phase);
              const isActiveColumn = activeColumnId === column.id;
              const canDrag = board.retro.phase !== "completed";

              return (
                <DropColumn columnId={column.id} enabled={canDrag} key={column.id}>
                  <ColumnHeader
                    accent={semantic.color}
                    count={column.cards.length || "-"}
                    icon={columnIcons[semantic.kind]}
                    name={semantic.label}
                    sub={semantic.kind === "mood" ? ". one per person" : semantic.kind === "action" ? ". action items" : undefined}
                  />

                  {canAddCards ? <ColumnComposer board={board} columnId={column.id} columnLabel={semantic.label} color={semantic.color} isActive={isActiveColumn} /> : null}

                  <div className="sp-scroll min-h-0 flex-1 space-y-2.5 overflow-auto pr-1">
                    {visibleCards.map((card) => <CardView board={board} card={card} color={semantic.color} draggable={canDrag} editing={query.editCard === card.id} key={card.id} moving={canDrag} clustering={canDrag} semantic={semantic.kind} />)}
                    <DropEndMarker accent={semantic.color} columnId={column.id} enabled={canDrag} />
                  </div>
                </DropColumn>
              );
            })}
          </div>
        </div>
        {hasDeck ? <DeckRail board={board} /> : null}
      </div>
    </section>
  );
}

function ColumnComposer({
  board,
  columnId,
  columnLabel,
  color,
  isActive,
}: {
  board: RetroBoard;
  columnId: string;
  columnLabel: string;
  color: string;
  isActive: boolean;
}) {
  return (
    <div className="mb-2.5">
      {isActive ? (
        <InlineComposer columnId={columnId} columnTitle={columnLabel} color={color} draftText="" retroId={board.retro.id} />
      ) : (
        <Link
          className="flex h-10 items-center justify-center gap-2 rounded-[8px] border border-dashed bg-[var(--panel-hi)]/35 text-[11.5px] font-extrabold leading-none shadow-none transition hover:bg-[var(--panel-hi)]"
          href={`/retros/${board.retro.id}?addColumn=${columnId}`}
          style={{ borderColor: `${color}66`, color }}
        >
          <span className="text-[15px] leading-none" style={{ color }}>+</span>
          add {columnLabel} card
        </Link>
      )}
    </div>
  );
}

function DeckRail({ board }: { board: RetroBoard }) {
  return (
    <aside className="absolute bottom-4 left-4 right-4 rounded-[14px] border border-spill-line bg-[rgba(251,243,223,0.94)] p-3 shadow-[var(--shadow-3)] backdrop-blur">
      <div className="mb-2 flex items-center gap-2">
        <Pill tone="action">AI . your deck</Pill>
        <span className="-rotate-2 font-hand text-[20px] leading-none text-spill-fg">spill suggests</span>
        <span className="text-[11px] text-spill-muted">{board.deck.length} cards</span>
        <span className="ml-auto text-[10.5px] italic text-spill-muted">drag a card into any column</span>
      </div>
      {board.deck.length === 0 ? (
        <div className="rounded-[8px] border border-dashed border-spill-line px-3 py-2 text-[11.5px] text-spill-muted">No deck suggestions yet.</div>
      ) : (
        <div className="flex gap-2 overflow-x-auto">
          {board.deck.map((item) => (
            <div className="min-h-[92px] w-[180px] shrink-0 rounded-[8px] border border-spill-line bg-spill-panel p-2.5 shadow-[var(--shadow-1)]" key={item.id}>
              <p className="text-[10px] font-extrabold uppercase tracking-[0.08em] text-spill-muted">{item.source}</p>
              <p className="mt-2 line-clamp-3 text-[12px] leading-4 text-spill-fg">{item.suggested_text || (item.gif_url ? "media suggestion" : "suggestion")}</p>
            </div>
          ))}
        </div>
      )}
    </aside>
  );
}
