import Link from "next/link";
import type { CSSProperties } from "react";
import { Avatar, CardFooter, Pill, SpillCard, Tile, avatarColorForSeed, avatarInitials, shortAuthorName } from "@/components/spill-ui";
import { CoverSquare, RetroCoverPicker } from "@/components/retro-cover-picker";
import type { RetroBoard, RetroCard } from "@/lib/api";
import { completeActionItemAction, confirmActionItemAction } from "@/lib/actions";
import { displayRetroDate, formatDateOnly } from "@/lib/retro-dates";
import { BoardMedia } from "./media-card";
import { actionVoteCount, cardLabel, columnSemantic, isActionsColumn, voteLabel } from "./board-presentation";
import { AiWrapTile } from "./ai-wrap-tile";

export function WrappedSummary({ board, isHost = false }: { board: RetroBoard; isHost?: boolean }) {
  const boardColumns = board.columns.filter((column) => !isActionsColumn(column));
  const allCards = board.columns.flatMap((column) => column.cards.filter((card) => !card.hidden));
  const cards = boardColumns.flatMap((column) => column.cards.filter((card) => !card.hidden));
  const mood = generatedTeamMood(board.ai_artifacts);
  const boardCategories = generatedBoardCategories(board.ai_artifacts);
  const actionColumnCards = board.columns
    .filter(isActionsColumn)
    .flatMap((column) => column.cards.filter((card) => !card.hidden && card.parent_card_id === null));
  // Cards in the actions column that aren't already linked to an action_item.
  // These are user-added during action_discussion and would otherwise vanish
  // from the wrap because the column is hidden and they have no action row.
  const linkedCardIds = new Set(
    board.actions
      .map((action) => action.source_card_id)
      .filter((id): id is string => Boolean(id)),
  );
  const adhocActionCards = actionColumnCards.filter((card) => !linkedCardIds.has(card.id));
  const cover = { url: board.retro.cover_gif_url, altText: board.retro.cover_gif_alt_text };
  const hasCover = Boolean(cover.url);

  return (
    <section className="grid flex-1 grid-cols-1 gap-8 overflow-auto p-6 md:p-8 lg:grid-cols-[minmax(0,1fr)_340px]">
      <main className="min-w-0">
        <div className="flex items-baseline gap-3">
          <h1 className="text-[38px] font-extrabold leading-none tracking-[-0.035em] text-spill-fg">That's a wrap.</h1>
          <span className="-rotate-2 font-hand text-[24px] text-spill-muted">nice work, team.</span>
        </div>
        <p className="mt-2 text-[13px] font-semibold text-spill-muted">{displayRetroDate(board.retro)}</p>
        {board.series ? (
          <p className="mt-1 text-[12px] font-bold text-spill-muted">[{board.series.name}] {board.retro.title}</p>
        ) : null}

        <div className="mt-5 flex items-center gap-5">
          {isHost ? (
            <RetroCoverPicker
              initialCover={cover}
              mode="update"
              retroId={board.retro.id}
              returnTo={`/retros/${board.retro.id}`}
              size="hero"
            />
          ) : hasCover ? (
            <CoverSquare cover={cover} size="hero" />
          ) : (
            <MoodVisual mood={mood} />
          )}
          <div>
            <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">team mood . {mood ? "ai generated" : "reading the room"}</p>
            <h2 className="mt-0.5 text-2xl font-extrabold tracking-[-0.02em] text-spill-fg">{mood?.title ?? "AI is still reading the room."}</h2>
            <p className="mt-1 max-w-xl text-[13.5px] leading-6 text-[var(--fg-2)]">
              {mood ? "Generated from the summary artifact using board cards, votes, actions, and meeting notes." : "We are turning the board into a mood. Give it a second."}
            </p>
          </div>
        </div>

        <CommittedActions allCards={allCards} board={board} adhocActionCards={adhocActionCards} />
        <FinalBoard board={board} boardColumns={boardColumns} cards={cards} />
      </main>

      <aside className="space-y-5 border-t border-spill-line pt-5 lg:border-l lg:border-t-0 lg:pl-6 lg:pt-0">
        <ParticipantsSummary board={board} />
        <NextRetroTile board={board} />
        <AiWrapTile retroId={board.retro.id} artifacts={board.ai_artifacts} categories={boardCategories} />
      </aside>
    </section>
  );
}

function ParticipantsSummary({ board }: { board: RetroBoard }) {
  return (
    <div>
      <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">who participated</p>
      <Tile className="mt-2">
        <div className="space-y-2.5">
        {board.participants.slice(0, 5).map((participant) => (
          <div className="flex items-center gap-2.5 text-[12.5px]" key={participant.id}>
            <Avatar color={avatarColorForSeed(participant.id)} k={avatarInitials(participant.display_name)} ring="var(--panel)" size={28} />
            <div className="min-w-0 flex-1">
              <p className="truncate font-bold text-spill-fg">
                {participant.display_name}
                {participant.role === "host" ? <span className="ml-1 text-[10.5px] font-medium text-spill-muted">· host</span> : null}
              </p>
              <p className="mt-0.5 text-[10.5px] text-spill-muted">{countLabel(participant.card_count ?? 0, "card")} · {countLabel(participant.vote_count ?? 0, "vote")}</p>
            </div>
          </div>
        ))}
        {board.participants.length > 5 ? (
          <p className="text-[11.5px] font-semibold text-spill-muted">+{board.participants.length - 5} more</p>
        ) : null}
        </div>
      </Tile>
    </div>
  );
}

function NextRetroTile({ board }: { board: RetroBoard }) {
  const next = board.next_retro;
  const groupName = next?.group_name ?? board.series?.name ?? "";
  return (
    <div className="border-t border-dashed border-spill-line pt-4">
      <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">next time{next ? ` · opens ${formatDateOnly(next.planned_for)}` : ""}</p>
      {next ? (
        <Tile className="mt-2">
          <Link className="text-[15px] font-extrabold tracking-[-0.02em] text-spill-fg hover:text-spill-wrong hover:underline hover:decoration-spill-wrong/40 hover:underline-offset-4" href={`/retros/${next.id}`}>
            {groupName ? `[${groupName}] ` : ""}{next.title}
          </Link>
          <p className="mt-1 text-[11.5px] font-semibold text-spill-muted">auto-scheduled · same settings</p>
        </Tile>
      ) : (
        <p className="mt-3 text-[11.5px] leading-5 text-spill-muted">Next retro is being planned when this board wraps.</p>
      )}
    </div>
  );
}

function countLabel(count: number, noun: string) {
  return `${count} ${count === 1 ? noun : `${noun}s`}`;
}

type MoodPresentation = {
  badge: string;
  title: string;
  style: CSSProperties;
};

const fallbackMoodStyle: CSSProperties = {
  background: "radial-gradient(circle at 35% 30%, #8a8177, #62564d 70%)",
  borderColor: "#62564d",
};

const moodPresentations: Record<string, MoodPresentation> = {
  "quietly-proud": {
    badge: "quietly proud",
    title: "Quietly proud.",
    style: {
      background: "radial-gradient(circle at 35% 30%, #79c38c, #3b8f58 70%)",
      borderColor: "#2d7646",
    },
  },
  "smooth-sailing": {
    badge: "smooth sailing",
    title: "Smooth sailing.",
    style: {
      background: "radial-gradient(circle at 35% 30%, #6fb6d6, #347fa2 70%)",
      borderColor: "#246983",
    },
  },
  "good-sparks": {
    badge: "good sparks",
    title: "Good sparks.",
    style: {
      background: "radial-gradient(circle at 35% 30%, #f3b64f, #c9792d 70%)",
      borderColor: "#a35f24",
    },
  },
  "productive-chaos": {
    badge: "productive chaos",
    title: "Productive chaos.",
    style: {
      background: "radial-gradient(circle at 35% 30%, #cf8a3f, #9f5f30 70%)",
      borderColor: "#804923",
    },
  },
  foggy: {
    badge: "foggy",
    title: "Foggy.",
    style: {
      background: "radial-gradient(circle at 35% 30%, #9ca3af, #64748b 70%)",
      borderColor: "#475569",
    },
  },
  spicy: {
    badge: "spicy",
    title: "Spicy.",
    style: {
      background: "radial-gradient(circle at 35% 30%, #ef6f5e, #b64232 70%)",
      borderColor: "#8f3328",
    },
  },
  "stuck-in-mud": {
    badge: "stuck in mud",
    title: "Stuck in mud.",
    style: {
      background: "radial-gradient(circle at 35% 30%, #8f7660, #5f4938 70%)",
      borderColor: "#4b382b",
    },
  },
  "needs-a-map": {
    badge: "needs a map",
    title: "Needs a map.",
    style: {
      background: "radial-gradient(circle at 35% 30%, #8b7bd4, #5947a3 70%)",
      borderColor: "#463783",
    },
  },
};

function MoodVisual({ mood }: { mood: MoodPresentation | null }) {
  return (
    <div
      className="grid h-[118px] w-[118px] shrink-0 place-items-center rounded-[14px] border-2 text-center text-[16px] font-extrabold leading-[1.05] tracking-[-0.02em] text-white shadow-[var(--shadow-3),inset_0_2px_0_rgba(255,255,255,0.25)]"
      style={mood?.style ?? fallbackMoodStyle}
    >
      {mood?.badge ?? "mood"}
    </div>
  );
}

function generatedTeamMood(artifacts: RetroBoard["ai_artifacts"]) {
  const summary = artifacts.find((artifact) => artifact.kind === "summary");
  if (summary?.status !== "succeeded") {
    return null;
  }
  if (!summary?.output || typeof summary.output !== "object") {
    return null;
  }
  const value = (summary.output as Record<string, unknown>).team_mood;
  if (typeof value !== "string") {
    return null;
  }
  return moodPresentations[value] ?? null;
}

function generatedBoardCategories(artifacts: RetroBoard["ai_artifacts"]) {
  const summary = artifacts.find((artifact) => artifact.kind === "summary");
  if (summary?.status !== "succeeded" || !summary.output || typeof summary.output !== "object") {
    return [];
  }
  const value = (summary.output as Record<string, unknown>).board_categories;
  return Array.isArray(value)
    ? value.filter((category): category is string => typeof category === "string" && category.trim().length > 0).slice(0, 4)
    : [];
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
                  {visibleCards.map((card) => <WrappedBoardCard board={board} card={card} color={semantic.color} semantic={semantic.kind} key={card.id} />)}
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

function CommittedActions({
  allCards,
  board,
  adhocActionCards,
}: {
  allCards: RetroCard[];
  board: RetroBoard;
  adhocActionCards: RetroCard[];
}) {
  const total = board.actions.length + adhocActionCards.length;
  return (
    <div className="mt-6">
      <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-action">actions committed . {total}</p>
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
        {adhocActionCards.map((card) => (
          <div className="sp-panel-grain flex items-center gap-3 rounded-[12px] border border-spill-line bg-spill-panel p-4 shadow-[var(--shadow-1)]" id={`card-${card.id}`} key={card.id}>
            <span className="grid h-[22px] w-[22px] shrink-0 place-items-center rounded-[6px] bg-spill-action text-[12px] font-extrabold text-white">ok</span>
            <Link className="min-w-0 flex-1 truncate text-[13.5px] font-medium text-spill-fg hover:underline" href={`/retros/${board.retro.id}#card-${card.id}`}>{cardLabel(card)}</Link>
            <Pill tone="neutral">added</Pill>
          </div>
        ))}
      </div>
    </div>
  );
}

function WrappedBoardCard({
  board,
  card,
  color,
  semantic,
}: {
  board: RetroBoard;
  card: RetroCard;
  color: string;
  semantic: string;
}) {
  const author = participantById(board, card.author_participant_id);
  const cluster = card.cluster_id ? board.clusters.find((item) => item.id === card.cluster_id) : null;
  const clusterChips = [cluster?.category, ...(cluster?.tags ?? [])]
    .filter((value): value is string => Boolean(value && value.trim()))
    .slice(0, 4);
  return (
    <SpillCard accent={color}>
      {card.gif_url ? <BoardMedia alt={card.gif_alt_text ?? "Attached media"} src={card.gif_url} /> : null}
      <div className="flex items-start gap-2">
        <p className="min-w-0 flex-1 whitespace-pre-wrap">{cardLabel(card)}</p>
        {card.vote_count > 0 ? <span className="shrink-0 rounded-full bg-white px-2 py-0.5 text-[10px] font-bold text-[var(--card-button-fg)]">{voteLabel(card.vote_count)}</span> : null}
      </div>
      {card.cluster_members.length > 0 ? (
        <div className="mt-2 space-y-1.5 border-t border-white/20 pt-2">
          {card.cluster_members.map((member) => (
            <div className="rounded-[6px] bg-white/15 px-2 py-1.5 text-[11.5px] leading-4 text-white/90" key={member.id}>
              {member.gif_url ? <BoardMedia alt={member.gif_alt_text ?? "Grouped media"} src={member.gif_url} /> : null}
              <p className="mt-1 first:mt-0">{member.body_text || member.gif_alt_text || "media card"}</p>
            </div>
          ))}
        </div>
      ) : null}
      {clusterChips.length > 0 ? (
        <div className="mt-2 flex flex-wrap gap-1">
          {clusterChips.map((tag) => (
            <span className="rounded-full bg-white/15 px-2 py-0.5 text-[10px] font-bold text-white/85" key={tag}>
              #{tag}
            </span>
          ))}
        </div>
      ) : null}
      <CardFooter
        author={avatarInitials(author?.display_name)}
        authorName={shortAuthorName(author?.display_name)}
        color={avatarColorForSeed(author?.id)}
        tag={semantic}
      />
    </SpillCard>
  );
}

function participantById(board: RetroBoard, participantId: string) {
  return board.participants.find((participant) => participant.id === participantId);
}

function actionCheckClass(checked: boolean) {
  return `grid h-7 w-7 place-items-center rounded-[7px] border text-[13px] font-extrabold leading-none transition hover:brightness-[0.98] focus-visible:outline-none focus-visible:shadow-[var(--focus)] ${
    checked ? "border-spill-well bg-spill-well text-white" : "border-[var(--line-2)] bg-[var(--panel-hi)] text-transparent hover:border-spill-well hover:text-spill-well"
  }`;
}
