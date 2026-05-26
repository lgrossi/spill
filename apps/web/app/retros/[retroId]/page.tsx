import Link from "next/link";
import { notFound } from "next/navigation";
import { IdentityGate, IdentityUnavailable } from "../../components/identity-gate";
import {
  AppChrome,
  Btn,
  CardComposer,
  CardFooter,
  ColumnHeader,
  GifTile,
  HiddenDraft,
  PhaseBadge,
  Pill,
  SpillCard,
  Stack,
  StageIndicator,
  TEAM,
  Tile,
  cardButtonClass,
  columnIcons,
  spillColors,
  type ColumnAccent,
} from "../../components/spill-ui";
import { getRetro, type RetroBoard, type RetroCard } from "../../lib/api";
import {
  castVoteAction,
  completeRetroAction,
  completeActionItemAction,
  confirmActionItemAction,
  createDraftCardAction,
  deleteDraftCardAction,
  markReadyAction,
  removeClusterMemberAction,
  removeVoteAction,
  revealRetroAction,
  startActionDiscussionAction,
  startVotingAction,
  unmarkReadyAction,
  updateDraftCardAction,
} from "../../lib/actions";
import { BoardSync } from "./board-sync";
import { DraggableCard, DropColumn, DropEndMarker } from "./board-dnd";
import { BoardMedia } from "./media-card";
import { GifDraftProvider, GifSearchPicker, GifSelectedPreview } from "./gif-search-picker";
import { ComposerSubmit } from "./composer-submit";
import { currentIdentity, localIdentityEnabled } from "../../lib/identity";

type BoardSearchParams = {
  addColumn?: string;
  editCard?: string;
};

export default async function RetroBoardPage({
  params,
  searchParams,
}: {
  params: Promise<{ retroId: string }>;
  searchParams: Promise<BoardSearchParams>;
}) {
  const { retroId } = await params;
  const query = await searchParams;
  const identity = await currentIdentity();
  if (!identity) {
    return localIdentityEnabled() ? <IdentityGate returnTo={`/retros/${retroId}`} /> : <IdentityUnavailable />;
  }

  const board = await loadBoard(retroId);

  if (!board) {
    notFound();
  }

  return (
    <AppChrome
      actions={<PhaseControls board={board} />}
      presence={<Stack people={TEAM.map((person) => ({ ...person, status: presenceForPhase(board.retro.phase) }))} size={26} />}
      subtitle={phaseSubtitle(board)}
      title={board.retro.title}
    >
      <BoardSync retroId={board.retro.id} />
      {board.retro.phase === "completed" ? (
        <WrappedSummary board={board} />
      ) : (
        <BoardColumns board={board} query={query} />
      )}
    </AppChrome>
  );
}

function PhaseControls({ board }: { board: RetroBoard }) {
  if (board.retro.phase === "writing") {
    const allReady = board.ready.participant_count > 0 && board.ready.ready_count >= board.ready.participant_count;
    return (
      <>
        <StageIndicator phase={board.retro.phase} retroId={board.retro.id} />
        <Pill tone="soft" accent={spillColors.mood}>
          <span className="sp-live-dot h-1.5 w-1.5 bg-spill-mood" />
          writing
        </Pill>
        <form action={board.ready.current_user_ready ? unmarkReadyAction : markReadyAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <Btn kind={board.ready.current_user_ready ? "secondary" : "primary"} type="submit">{board.ready.current_user_ready ? "not ready" : "I'm ready"}</Btn>
        </form>
        <form action={revealRetroAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <Btn kind="dashed" type="submit" disabled={!allReady}>reveal -&gt;</Btn>
        </form>
      </>
    );
  }

  if (board.retro.phase === "discussion") {
    return (
      <>
        <StageIndicator phase={board.retro.phase} retroId={board.retro.id} />
        <Pill tone="soft" accent={spillColors.action}>review</Pill>
        <form action={startVotingAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <Btn kind="dashed" type="submit" disabled={board.retro.vote_limit <= 0}>vote -&gt;</Btn>
        </form>
        {board.retro.vote_limit <= 0 ? (
          <form action={startActionDiscussionAction}>
            <input name="retro_id" type="hidden" value={board.retro.id} />
            <Btn kind="primary" type="submit">act -&gt;</Btn>
          </form>
        ) : null}
      </>
    );
  }

  if (board.retro.phase === "voting") {
    return (
      <>
        <StageIndicator phase={board.retro.phase} retroId={board.retro.id} />
        <form action={startActionDiscussionAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <Btn kind="primary" type="submit" disabled={board.retro.action_discussion_limit <= 0}>act -&gt;</Btn>
        </form>
      </>
    );
  }

  if (board.retro.phase === "action_discussion") {
    return (
      <>
        <StageIndicator phase={board.retro.phase} retroId={board.retro.id} />
        <Pill tone="soft" accent={spillColors.action}>action</Pill>
        <form action={completeRetroAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <Btn kind="dashed" type="submit">wrap retro</Btn>
        </form>
      </>
    );
  }

  return (
    <>
      <PhaseBadge color={spillColors.well} phase="wrapped" />
      <Btn href="/" kind="primary">home</Btn>
    </>
  );
}

function VoteLeftDots({ remaining, total }: { remaining: number; total: number }) {
  return (
    <span className="flex shrink-0 gap-0.5" aria-label={`${remaining} votes left`}>
      {Array.from({ length: total }).map((_, index) => (
        <span className={`h-1.5 w-1.5 rounded-full border border-spill-well/45 ${index < remaining ? "bg-spill-well/75" : "bg-transparent"}`} key={index} />
      ))}
    </span>
  );
}

function BoardColumns({ board, query }: { board: RetroBoard; query: BoardSearchParams }) {
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
            const canWrite = canAddCards;
            const canDrag = board.retro.phase !== "completed";
            const canMoveCards = board.retro.phase !== "completed";
            const canClusterCards = board.retro.phase !== "completed";

            return (
              <DropColumn columnId={column.id} enabled={canDrag} key={column.id}>
                <ColumnHeader
                  accent={semantic.color}
                  count={column.cards.length || "-"}
                  icon={columnIcons[semantic.kind]}
                  name={semantic.label}
                  sub={semantic.kind === "mood" ? ". one per person" : semantic.kind === "action" ? ". action items" : undefined}
                />

                {canWrite ? (
                  <div className="mb-2.5">
                    {isActiveColumn ? (
                      <InlineComposer
                        columnId={column.id}
                        columnTitle={semantic.label}
                        color={semantic.color}
                        draftText=""
                        retroId={board.retro.id}
                      />
                    ) : (
                      <Link
                        className="flex h-10 items-center justify-center gap-2 rounded-[8px] border border-dashed bg-[var(--panel-hi)]/35 text-[11.5px] font-extrabold leading-none shadow-none transition hover:bg-[var(--panel-hi)]"
                        href={`/retros/${board.retro.id}?addColumn=${column.id}`}
                        style={{ borderColor: `${semantic.color}66`, color: semantic.color }}
                      >
                        <span className="text-[15px] leading-none" style={{ color: semantic.color }}>+</span>
                        add {semantic.label} card
                      </Link>
                    )}
                  </div>
                ) : null}

                <div className="sp-scroll min-h-0 flex-1 space-y-2.5 overflow-auto pr-1">
                  {visibleCards.map((card) => <CardView board={board} card={card} color={semantic.color} draggable={canDrag} editing={query.editCard === card.id} key={card.id} moving={canMoveCards} clustering={canClusterCards} semantic={semantic.kind} />)}
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

function DeckRail({ board }: { board: RetroBoard }) {
  return (
    <aside className="absolute bottom-4 left-4 right-4 rounded-[14px] border border-spill-line bg-[rgba(251,243,223,0.94)] p-3 shadow-[var(--shadow-3)] backdrop-blur">
      <div className="mb-2 flex items-center gap-2">
        <Pill tone="action">AI · your deck</Pill>
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

function CardView({
  board,
  card,
  color,
  draggable,
  editing,
  moving,
  clustering,
  semantic,
}: {
  board: RetroBoard;
  card: RetroCard;
  color: string;
  draggable: boolean;
  editing: boolean;
  moving: boolean;
  clustering: boolean;
  semantic: ColumnAccent;
}) {
  if (card.hidden) {
    return <HiddenDraft accent={color} />;
  }

  const isEditingGroup = editing && board.retro.phase !== "completed" && card.parent_card_id === null && card.cluster_id !== null;

  if (editing && !isEditingGroup && board.retro.phase !== "completed" && card.parent_card_id === null) {
    return <DraftCardEditor board={board} card={card} color={color} semantic={semantic} />;
  }

  return (
    <DraggableCard accent={color} cardId={card.id} columnId={card.column_id} enabled={draggable} clusteringEnabled={clustering} movingEnabled={moving} retroId={board.retro.id}>
      <SpillCard accent={color}>
        {!card.cluster_id && card.gif_url ? card.gif_url === "demo-gif" ? <GifTile className="mb-2" /> : <BoardMedia alt={card.gif_alt_text ?? "Attached media"} src={card.gif_url} /> : null}
        {isEditingGroup ? (
          <form action={updateDraftCardAction} className="mb-2 flex items-center gap-1.5 pr-16" data-spill-no-drag>
            <input name="retro_id" type="hidden" value={board.retro.id} />
            <input name="column_id" type="hidden" value={card.column_id} />
            <input name="card_id" type="hidden" value={card.id} />
            <input name="editing_group_title" type="hidden" value="1" />
            <input
              autoFocus
              className="min-w-0 flex-1 rounded-[6px] border border-white/25 bg-white/15 px-2 py-1.5 text-[13px] font-extrabold leading-5 text-white outline-none placeholder:text-white/60"
              defaultValue={card.body_text ?? ""}
              name="body_text"
              placeholder="group title"
              required
            />
            <button aria-label="Save group title" className="grid h-6 w-6 place-items-center rounded-full border border-white/35 bg-black/20 text-[12px] font-extrabold leading-none text-white/90 transition hover:bg-black/30" type="submit">✓</button>
            <Link aria-label="Cancel edit" className="grid h-6 w-6 place-items-center rounded-full border border-white/35 bg-black/20 text-[12px] font-extrabold leading-none text-white/90 transition hover:bg-black/30" href={`/retros/${board.retro.id}`}>×</Link>
          </form>
        ) : card.body_text ? <p className="whitespace-pre-wrap first:mt-0">{card.body_text}</p> : null}
        {!card.cluster_id && card.cluster_details ? <p className="mt-2 text-[12px] italic text-white/80">{card.cluster_details}</p> : null}
        {card.cluster_members.length > 0 ? (
          <div className="mt-2 space-y-1.5 border-t border-white/20 pt-2">
            {card.cluster_members.map((member) => (
              <div className="rounded-[6px] bg-white/15 px-2 py-1.5 text-[11.5px] leading-4 text-white/90" key={member.id}>
                {member.gif_url ? <BoardMedia alt={member.gif_alt_text ?? "Grouped media"} src={member.gif_url} /> : null}
                <div className="mt-1 flex items-start gap-2">
                  <span className="min-w-0 flex-1">{member.hidden ? ". . . someone's draft . . ." : member.body_text || member.gif_alt_text || "media card"}</span>
                  <form action={removeClusterMemberAction} data-spill-no-drag>
                  <input name="retro_id" type="hidden" value={board.retro.id} />
                  <input name="card_id" type="hidden" value={member.id} />
                    <button aria-label="Remove from group" className="grid h-5 w-5 place-items-center rounded-full border border-white/35 text-[11px] font-extrabold text-white/85 transition hover:bg-white/20" title="Remove from group" type="submit">↗</button>
                  </form>
                </div>
              </div>
            ))}
          </div>
        ) : card.cluster_title ? <p className="mt-2 border-t border-white/20 pt-2 text-[10px] font-extrabold uppercase tracking-[0.1em] text-white/85">{card.cluster_title}</p> : null}
        <CardFooter
          author={authorForCard(card.id)}
          color={authorColorForCard(card.id)}
          tag={semantic}
          trailing={board.retro.phase === "voting" ? <VoteControls board={board} card={card} color={color} /> : undefined}
          votes={board.retro.phase === "action_discussion" ? card.vote_count : undefined}
        />
        {board.retro.phase !== "completed" && card.parent_card_id === null && !isEditingGroup ? (
          <div className="absolute right-2 top-2 flex gap-1" data-spill-no-drag>
            <Link aria-label="Edit card" className="grid h-6 w-6 place-items-center rounded-full border border-white/35 bg-black/20 text-[12px] font-extrabold leading-none text-white/90 shadow-[0_1px_2px_rgba(0,0,0,0.16)] transition hover:bg-black/30" href={card.cluster_id ? `/retros/${board.retro.id}?editCard=${card.id}` : `/retros/${board.retro.id}?addColumn=${card.column_id}&editCard=${card.id}`}>✎</Link>
            <form action={deleteDraftCardAction}>
              <input name="retro_id" type="hidden" value={board.retro.id} />
              <input name="card_id" type="hidden" value={card.id} />
              <button aria-label="Delete card" className="grid h-6 w-6 place-items-center rounded-full border border-white/35 bg-black/20 text-[13px] font-extrabold leading-none text-white/90 shadow-[0_1px_2px_rgba(0,0,0,0.16)] transition hover:bg-black/30" type="submit">×</button>
            </form>
          </div>
        ) : null}
      </SpillCard>
    </DraggableCard>
  );
}

function VoteControls({ board, card, color }: { board: RetroBoard; card: RetroCard; color: string }) {
  return (
    <div className="flex items-center gap-1" data-spill-no-drag>
      <form action={removeVoteAction}>
        <input name="retro_id" type="hidden" value={board.retro.id} />
        <input name="card_id" type="hidden" value={card.id} />
        <button className={cardButtonClass} disabled={card.current_user_vote_count <= 0} style={{ "--card-button-fg": color } as React.CSSProperties} type="submit">−</button>
      </form>
      <span className="grid h-6 min-w-6 place-items-center rounded-full border border-white/35 bg-white/15 px-2 text-[11px] font-extrabold text-white" aria-label={`${card.current_user_vote_count} of your votes on this card`}>
        {card.current_user_vote_count}
      </span>
      <form action={castVoteAction}>
        <input name="retro_id" type="hidden" value={board.retro.id} />
        <input name="card_id" type="hidden" value={card.id} />
        <button className={cardButtonClass} disabled={board.voting.votes_remaining <= 0} style={{ "--card-button-fg": color } as React.CSSProperties} type="submit">+</button>
      </form>
    </div>
  );
}

function DraftCardEditor({ board, card, color, semantic }: { board: RetroBoard; card: RetroCard; color: string; semantic: ColumnAccent }) {
  return (
    <form action={updateDraftCardAction} className="grid min-w-0 gap-2">
      <input name="card_id" type="hidden" value={card.id} />
      <input name="existing_gif_url" type="hidden" value={card.gif_url ?? ""} />
      <input name="existing_gif_alt_text" type="hidden" value={card.gif_alt_text ?? ""} />
      <GifDraftProvider initialGif={card.gif_url ? { id: card.id, url: card.gif_url, preview_url: card.gif_url, alt_text: card.gif_alt_text ?? "Attached media" } : null}>
        <CardComposer
          accent={color}
          before={<GifSelectedPreview />}
          after={<GifSearchPicker columnTitle={semantic} />}
          actions={
            <>
              <Link aria-label="Cancel edit" className={composerButtonClass("ghost")} href={`/retros/${board.retro.id}?addColumn=${card.column_id}`}>×</Link>
              <ComposerSubmit className={composerButtonClass("solid")} existingGif={Boolean(card.gif_url)} />
            </>
          }
          columnId={card.column_id}
          draftText={card.body_text ?? ""}
          placeholder="edit this card"
          retroId={board.retro.id}
        />
      </GifDraftProvider>
    </form>
  );
}

function InlineComposer({
  columnId,
  columnTitle,
  color,
  draftText,
  retroId,
}: {
  columnId: string;
  columnTitle: string;
  color: string;
  draftText: string;
  retroId: string;
}) {
  return (
    <form action={createDraftCardAction} className="grid min-w-0 gap-2">
      <GifDraftProvider>
        <CardComposer
          accent={color}
          before={<GifSelectedPreview />}
          after={<GifSearchPicker columnTitle={columnTitle} />}
          actions={
            <>
              <Link aria-label="Cancel card" className={composerButtonClass("ghost")} href={`/retros/${retroId}`}>×</Link>
              <ComposerSubmit className={composerButtonClass("solid")} />
            </>
          }
          columnId={columnId}
          draftText={draftText}
          placeholder={`what's on your mind, na?`}
          retroId={retroId}
        />
      </GifDraftProvider>
    </form>
  );
}

function composerButtonClass(kind: "ghost" | "solid") {
  const base = "grid h-7 w-7 place-items-center rounded-full border text-[13px] font-extrabold leading-none shadow-[0_1px_2px_rgba(0,0,0,0.14)] transition focus-visible:outline-none focus-visible:shadow-[var(--focus)]";
  if (kind === "solid") {
    return `${base} border-white bg-white text-[var(--card-button-fg)] hover:bg-white/90`;
  }
  return `${base} border-white/35 bg-white/15 text-white/90 hover:bg-white/25`;
}

function WrappedSummary({ board }: { board: RetroBoard }) {
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
                    {(visibleCards.length ? visibleCards : []).map((card) => <WrappedBoardCard card={card} key={card.id} />)}
                    {visibleCards.length === 0 ? <p className="rounded-[8px] border border-dashed border-spill-line px-3 py-2 text-[11.5px] text-spill-muted">No cards</p> : null}
                  </div>
                </Tile>
              );
            })}
            </div>
          </div>
        </div>

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
                    <button aria-label="Reopen action" className={actionCheckClass(true)} type="submit">✓</button>
                  </form>
                ) : action.status !== "rejected" ? (
                  <form action={completeActionItemAction}>
                    <input name="retro_id" type="hidden" value={board.retro.id} />
                    <input name="action_id" type="hidden" value={action.id} />
                    <button aria-label="Mark action done" className={actionCheckClass(false)} type="submit">✓</button>
                  </form>
                ) : null}
              </div>
            ))}
          </div>
        </div>
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

function actionVoteCount(action: RetroBoard["actions"][number], cards: RetroCard[]) {
  if (action.source_card_id) {
    return cards.find((card) => card.id === action.source_card_id)?.vote_count ?? 0;
  }
  if (action.source_cluster_id) {
    return cards.find((card) => card.cluster_id === action.source_cluster_id && card.parent_card_id === null)?.vote_count ?? 0;
  }
  return 0;
}

function voteLabel(votes: number) {
  if (votes === 0) return "no votes";
  return votes === 1 ? "1 vote" : `${votes} votes`;
}

function actionCheckClass(checked: boolean) {
  return `grid h-7 w-7 place-items-center rounded-[7px] border text-[13px] font-extrabold leading-none transition hover:brightness-[0.98] focus-visible:outline-none focus-visible:shadow-[var(--focus)] ${
    checked ? "border-spill-well bg-spill-well text-white" : "border-[var(--line-2)] bg-white text-transparent hover:border-spill-well hover:text-spill-well"
  }`;
}

function sortedCards(cards: RetroCard[], phase: string) {
  const visible = [...cards];
  if (phase === "voting") {
    visible.sort((a, b) => b.vote_count - a.vote_count);
  }
  return visible;
}

function columnSemantic(column: RetroBoard["columns"][number], index: number): { kind: ColumnAccent; label: string; color: string } {
  const title = column.title.toLowerCase();
  const savedColor = column.accent_color || undefined;
  if (isActionsColumn(column) || title.includes("action")) return { kind: "action", label: column.title.toLowerCase(), color: spillColors.action };
  if (title.includes("feeling")) return { kind: "mood", label: column.title.toLowerCase(), color: savedColor ?? "#0f5f72" };
  if (title.includes("mood") || title.includes("mad") || title.includes("sad") || title.includes("glad")) return { kind: "mood", label: column.title.toLowerCase(), color: savedColor ?? spillColors.mood };
  if (title.includes("well") || title.includes("liked") || title.includes("learned") || title.includes("wind") || title.includes("continue") || title.includes("glad")) return { kind: "well", label: column.title.toLowerCase(), color: savedColor ?? spillColors.well };
  if (title.includes("wrong") || title.includes("lacked") || title.includes("improve") || title.includes("anchor") || title.includes("rocks") || title.includes("stop") || title.includes("mad") || title.includes("sad")) return { kind: "wrong", label: column.title.toLowerCase(), color: savedColor ?? spillColors.wrong };
  const fallback = [
    { kind: "mood" as const, color: spillColors.mood },
    { kind: "well" as const, color: spillColors.well },
    { kind: "wrong" as const, color: spillColors.wrong },
    { kind: "action" as const, color: spillColors.action },
  ][index % 4];
  return { ...fallback, color: savedColor ?? fallback.color, label: column.title.toLowerCase() };
}

function isActionsColumn(column: RetroBoard["columns"][number]) {
  return column.column_key === "actions" || column.title.toLowerCase().includes("action");
}

function cardLabel(card: RetroCard) {
  return card.body_text || card.gif_alt_text || "media card";
}

function authorForCard(id: string) {
  return TEAM[Math.abs(hash(id)) % TEAM.length].k;
}

function authorColorForCard(id: string) {
  return TEAM[Math.abs(hash(id)) % TEAM.length].color;
}

function hash(value: string) {
  let result = 0;
  for (let index = 0; index < value.length; index += 1) {
    result = (result << 5) - result + value.charCodeAt(index);
    result |= 0;
  }
  return result;
}

function presenceForPhase(phase: RetroBoard["retro"]["phase"]) {
  if (phase === "voting") return "voting" as const;
  if (phase === "completed" || phase === "action_discussion") return "ready" as const;
  return "writing" as const;
}

function phaseLabel(phase: RetroBoard["retro"]["phase"]) {
  if (phase === "discussion") return "review";
  if (phase === "action_discussion") return "action";
  if (phase === "completed") return "done";
  return phase.replaceAll("_", " ");
}

function phaseSubtitle(board: RetroBoard) {
  if (board.retro.phase === "writing") return `writing. ${board.ready.ready_count} of ${board.ready.participant_count} ready`;
  if (board.retro.phase === "discussion") return "review. manual grouping is available";
  if (board.retro.phase === "voting") {
    return (
      <>
        <span className="truncate">voting. {board.voting.votes_remaining} votes left</span>
        <VoteLeftDots remaining={board.voting.votes_remaining} total={board.retro.vote_limit} />
      </>
    );
  }
  if (board.retro.phase === "action_discussion") return `action discussion. top ${board.retro.action_discussion_limit}`;
  return "completed. wrapped recap";
}

async function loadBoard(retroId: string) {
  try {
    return await getRetro(retroId);
  } catch {
    return null;
  }
}

function demoBoard(retroId: string): RetroBoard {
  const phase =
    retroId.includes("cluster") ? "discussion"
    : retroId.includes("voting") ? "voting"
    : retroId.includes("action") ? "action_discussion"
    : retroId.includes("wrapped") ? "completed"
    : "writing";

  const columns = [
    {
      id: "demo-col-mood",
      retro_id: retroId,
      column_key: "mood",
      title: "mood",
      position: 0,
      order_direction: "asc",
      cards: [
        demoCard(retroId, "demo-mood-1", "demo-col-mood", "tired but alive", 0),
        demoCard(retroId, "demo-mood-2", "demo-col-mood", "caffeinated chaos", 0, phase === "writing"),
        demoCard(retroId, "demo-mood-3", "demo-col-mood", "wired but ok", 0),
      ],
    },
    {
      id: "demo-col-well",
      retro_id: retroId,
      column_key: "well",
      title: "went well",
      position: 1,
      order_direction: "asc",
      cards: phase === "writing"
        ? [
            demoCard(retroId, "demo-well-1", "demo-col-well", "deploy gate landed ahead of plan - months of work", 2),
            demoCard(retroId, "demo-well-hidden", "demo-col-well", "private well draft", 0, true),
            demoCard(retroId, "demo-well-3", "demo-col-well", "paired with nat on the migrator. learned more than the last 3 sprints combined.", 1),
          ]
        : [
            demoCard(retroId, "demo-well-1", "demo-col-well", "deploy gate landed ahead of plan - months of work", 2, false, "demo-cluster-deploys", "DEPLOYS"),
            demoCard(retroId, "demo-well-2", "demo-col-well", "shipped 3 days early", 1, false, "demo-cluster-deploys", "DEPLOYS"),
            demoCard(retroId, "demo-well-3", "demo-col-well", "paired with nat on the migrator. learned more than the last 3 sprints combined.", 1),
            demoCard(retroId, "demo-well-4", "demo-col-well", "customer demo went well", 0),
          ],
    },
    {
      id: "demo-col-wrong",
      retro_id: retroId,
      column_key: "wrong",
      title: "went wrong",
      position: 2,
      order_direction: "asc",
      cards: phase === "writing"
        ? [
            demoCard(retroId, "demo-wrong-1", "demo-col-wrong", "e2e flake. again. friday afternoon.", 4, false, null, null, "demo-gif"),
            demoCard(retroId, "demo-wrong-hidden", "demo-col-wrong", "private wrong draft", 0, true),
          ]
        : [
            demoCard(retroId, "demo-wrong-1", "demo-col-wrong", "e2e flake AGAIN friday - lost the afternoon", 4, false, "demo-cluster-flake", "THE FLAKE", "demo-gif"),
            demoCard(retroId, "demo-wrong-2", "demo-col-wrong", "stage timeouts cost me 30m", 0, false, "demo-cluster-flake", "THE FLAKE"),
            demoCard(retroId, "demo-wrong-3", "demo-col-wrong", "flake suite still red. it never went green.", 0, false, "demo-cluster-flake", "THE FLAKE"),
            demoCard(retroId, "demo-wrong-4", "demo-col-wrong", "onboarding doc still empty", 1),
          ],
    },
  ];

  return {
    retro: {
      id: retroId,
      title: "Sprint 42 . platform",
      phase,
      vote_limit: 3,
      action_discussion_limit: 3,
    },
    columns,
    ready: {
      participant_count: 4,
      ready_count: 2,
      current_user_ready: false,
    },
    voting: {
      vote_limit: 3,
      votes_used: phase === "voting" ? 1 : 0,
      votes_remaining: phase === "voting" ? 2 : 3,
    },
    clusters: phase === "writing" ? [] : [
      {
        id: "demo-cluster-deploys",
        retro_id: retroId,
        title: "DEPLOYS",
        category: "well",
        tags: ["deploys"],
      },
      {
        id: "demo-cluster-flake",
        retro_id: retroId,
        title: "THE FLAKE",
        category: "wrong",
        tags: ["flake"],
      },
    ],
    actions: phase === "writing" || phase === "discussion" || phase === "voting" ? [] : [
      {
        id: "demo-action-1",
        retro_id: retroId,
        source_card_id: "demo-wrong-1",
        source_cluster_id: "demo-cluster-flake",
        title: "quarantine flake suite, kill next sprint",
        details: "Owner: Lucas. Due fri 5/30.",
        status: "confirmed",
        position: 0,
        tags: ["flake"],
      },
      {
        id: "demo-action-2",
        retro_id: retroId,
        source_card_id: "demo-wrong-4",
        source_cluster_id: null,
        title: "flake counter on deploy gate",
        details: "Show test instability before merge.",
        status: "proposed",
        position: 1,
        tags: ["deploy"],
      },
      {
        id: "demo-action-3",
        retro_id: retroId,
        source_card_id: "demo-wrong-4",
        source_cluster_id: null,
        title: "own the onboarding doc",
        details: "Nat to publish a first pass.",
        status: "proposed",
        position: 2,
        tags: ["docs"],
      },
    ],
    deck: [],
    ai_artifacts: [],
    meeting_notes: [],
    deliveries: [],
  };
}

function demoCard(
  retroId: string,
  id: string,
  columnId: string,
  text: string,
  votes: number,
  hidden = false,
  clusterId: string | null = null,
  clusterTitle: string | null = null,
  gifUrl: string | null = null,
): RetroCard {
  return {
    id,
    retro_id: retroId,
    column_id: columnId,
    body_text: text,
    gif_url: gifUrl,
    gif_alt_text: gifUrl ? "animated reaction" : null,
    state: hidden ? "draft" : "revealed",
    position: 0,
    hidden,
    vote_count: votes,
    current_user_vote_count: 0,
    cluster_id: clusterId,
    parent_card_id: null,
    cluster_details: null,
    cluster_title: clusterTitle,
    cluster_category: null,
    cluster_members: [],
  };
}
