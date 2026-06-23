import Link from "next/link";
import { ColumnHeader, Pill } from "@/components/spill-ui";
import type { RetroBoard } from "@/lib/api";
import { revealColumnCommand } from "@/lib/commands/board-phase-commands";
import { DropColumn, DropEndMarker } from "./board-dnd";
import { CardView } from "./card-view";
import { InlineComposer } from "./card-composer";
import { columnSemantic } from "./board-presentation";

type BoardSearchParams = {
  addColumn?: string;
  editCard?: string;
};

export function BoardColumns({
  board,
  query,
  isHost = false,
  currentUserParticipantId = null,
}: {
  board: RetroBoard;
  query: BoardSearchParams;
  // Drive per-card edit/delete affordances when card_edit_policy is
  // 'author_only'. Optional + safe defaults so the ScheduledBoard overlay
  // call site (no real cards visible) doesn't need to thread these through.
  isHost?: boolean;
  currentUserParticipantId?: string | null;
}) {
  const columns = board.columns;
  const activeColumnId = query.editCard ? undefined : query.addColumn;
  const hasDeck = board.retro.phase === "writing" && board.deck.length > 0;
  const isInteractive = board.retro.phase !== "completed" && board.retro.phase !== "scheduled";
  const isWriting = board.retro.phase === "writing";
  // Per-column reveal pill shows only to the host, only during writing, and
  // only once everyone-present is ready -- the same gate the global reveal
  // button uses. The route accepts ?force=true so a stragglers-but-host case
  // still works, but we don't surface the pill prematurely.
  const everyoneReady =
    board.ready.participant_count > 0
    && board.ready.ready_count >= board.ready.participant_count;
  // Reveal mode gates which affordance is shown: per_column means walk
  // through columns one by one (pills appear; big-bang button hidden via
  // phase-line.tsx); big_bang keeps the legacy single-action reveal.
  const isPerColumnReveal = board.retro.reveal_mode === "per_column";
  const canRevealColumns = isWriting && isHost && everyoneReady && isPerColumnReveal;

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
            {columns.map((column) => {
              const semantic = columnSemantic(column);
              // Render the server-persisted order for every phase. Notably,
              // voting must NOT re-sort by votes: it would both disorient people
              // and leak the otherwise-hidden relative vote tallies.
              const visibleCards = column.cards;
              const isActiveColumn = activeColumnId === column.id;
              const canDrag = isInteractive;
              const isColumnRevealed = column.revealed_at != null;
              // Lock the composer once a column is revealed mid-writing: the
              // backend rejects new drafts there anyway (see cards.rs guard),
              // and surfacing the affordance would invite a confusing 4xx.
              const canComposeHere = isInteractive && !(isWriting && isColumnRevealed);

              return (
                <DropColumn columnId={column.id} enabled={canDrag} key={column.id}>
                  <ColumnHeader
                    accent={semantic.color}
                    count={column.cards.length || "-"}
                    name={semantic.label}
                    sub={semantic.kind === "mood" ? ". one per person" : semantic.kind === "action" ? ". action items" : undefined}
                  />

                  {isWriting && isColumnRevealed ? (
                    <div className="mb-2 flex items-center gap-1.5 px-0.5 text-[11px] font-semibold uppercase tracking-[0.06em]" style={{ color: semantic.color }}>
                      <span className="h-1.5 w-1.5 rounded-full" style={{ backgroundColor: semantic.color }} />
                      revealed
                    </div>
                  ) : canRevealColumns ? (
                    <ColumnRevealPill board={board} columnId={column.id} color={semantic.color} />
                  ) : null}

                  {canComposeHere ? <ColumnComposer board={board} columnId={column.id} columnLabel={semantic.label} color={semantic.color} isActive={isActiveColumn} /> : null}

                  <div className="sp-scroll min-h-0 flex-1 space-y-2.5 overflow-auto pr-1">
                    {visibleCards.map((card) => <CardView board={board} card={card} color={semantic.color} draggable={canDrag} editing={query.editCard === card.id} key={card.id} moving={canDrag} clustering={canDrag} semanticLabel={semantic.label} isHost={isHost} currentUserParticipantId={currentUserParticipantId} />)}
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

// Host-only affordance during writing: reveals just this column's drafts
// (vs reveal_board which dumps the entire board at once). Submits a form
// with retro_id + column_id to revealColumnCommand, which calls the
// `/columns/{id}/reveal` route. force=true so a stragglers-but-host case
// still works; the gate is the surrounding `canRevealColumns` calculation.
function ColumnRevealPill({ board, columnId, color }: { board: RetroBoard; columnId: string; color: string }) {
  return (
    <form action={revealColumnCommand} className="mb-2.5">
      <input name="retro_id" type="hidden" value={board.retro.id} />
      <input name="column_id" type="hidden" value={columnId} />
      <button
        className="flex h-10 w-full items-center justify-center gap-2 rounded-[8px] border text-[11.5px] font-extrabold uppercase leading-none tracking-[0.04em] transition hover:bg-[var(--panel-hi)]"
        style={{ borderColor: color, color, backgroundColor: `${color}10` }}
        type="submit"
      >
        reveal this column
      </button>
    </form>
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
