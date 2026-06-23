import type { ReactNode } from "react";
import { notFound } from "next/navigation";
import { BoardAccessDenied, IdentityGate, IdentityUnavailable } from "@/components/identity-gate";
import { AppChrome, Btn, Stack, Tile, avatarColorForSeed, avatarInitials, spillColors } from "@/components/spill-ui";
import { type RetroBoard } from "@/lib/api";
import { displayRetroDate, isPlannedForDue } from "@/lib/retro-dates";
import { currentIdentity, localIdentityEnabled } from "@/lib/identity";
import { listGrantsAction, markReadyAction, setParticipationAction, startScheduledRetroAction, unmarkReadyAction } from "@/lib/actions";
import { BoardColumns } from "./board-columns";
import { loadBoard } from "./board-loader";
import { BoardSync } from "./board-sync";
import { presenceForPhase } from "./board-presentation";
import { PhaseControls } from "./phase-controls";
import { PhaseLine } from "./phase-line";
import { PlannedDateEditor } from "./planned-date-editor";
import { ScheduledAutoStart } from "./scheduled-auto-start";
import { WrappedSummary } from "./wrapped-summary";
import { InlineDetailEditor } from "./inline-detail-editor";

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

  if (board === "forbidden") {
    return <BoardAccessDenied />;
  }

  if (!board) {
    notFound();
  }

  const currentUserEmail = identity.email ?? "";
  let isHost = false;
  try {
    const grants = await listGrantsAction(retroId);
    isHost = grants.some(
      (g) => g.principal_email === currentUserEmail && g.role === "host",
    );
  } catch {
    // treat as non-host if grant check fails
  }

  // Used by per-card UI gates (card-edit policy). Server-side enforcement
  // lives in cards.rs SQL; this only hides affordances client-side.
  const currentUserParticipant =
    board.participants.find((participant) => participant.external_subject === identity.subject)
    ?? null;
  const currentUserParticipantId = currentUserParticipant?.id ?? null;

  return (
    <AppChrome
      actions={
        <PhaseControls
          board={board}
          isHost={isHost}
          currentUserEmail={currentUserEmail}
          currentUserParticipantId={currentUserParticipantId}
        />
      }
      presence={<ParticipantStack board={board} />}
      center={<CenterPhase board={board} isHost={isHost} />}
      subtitle={<TitleSubtitle board={board} currentUserParticipant={currentUserParticipant} />}
      title={board.retro.title}
    >
      <BoardSync retroId={board.retro.id} apiBaseUrl={browserApiBaseUrl()} />
      {board.retro.phase === "completed" ? (
        <WrappedSummary board={board} isHost={isHost} />
      ) : board.retro.phase === "scheduled" ? (
        <ScheduledBoard board={board} isHost={isHost} query={query} />
      ) : (
        <BoardColumns board={board} query={query} isHost={isHost} currentUserParticipantId={currentUserParticipantId} />
      )}
    </AppChrome>
  );
}

function browserApiBaseUrl() {
  return process.env.SPILLIO_BROWSER_API_URL
    ?? process.env.NEXT_PUBLIC_SPILLIO_API_URL
    ?? process.env.SPILLIO_API_URL
    ?? "";
}

function ParticipantStack({ board }: { board: RetroBoard }) {
  return (
    <Stack
      people={board.participants.map((participant) => ({
        color: avatarColorForSeed(participant.id),
        k: avatarInitials(participant.display_name),
        // Non-participating people render in the existing "away" style so
        // they're visually distinct from active peers without needing a new
        // avatar variant.
        status: participant.is_participating
          ? presenceForPhase(board.retro.phase)
          : ("away" as const),
      }))}
      size={26}
    />
  );
}

function VoteLeftDots({ remaining, total }: { remaining: number; total: number }) {
  return (
    <span className="flex shrink-0 gap-0.5" aria-label={`${remaining} of ${total} votes left`}>
      {Array.from({ length: total }).map((_, index) => (
        <span className={`h-1.5 w-1.5 rounded-full ${index < remaining ? "bg-spill-wrong" : "border border-spill-line bg-transparent"}`} key={index} />
      ))}
    </span>
  );
}

function ReadyCheckbox({ retroId, checked }: { retroId: string; checked: boolean }) {
  const action = checked ? unmarkReadyAction : markReadyAction;
  return (
    <form action={action} className="contents">
      <input name="retro_id" type="hidden" value={retroId} />
      <button
        type="submit"
        className={`inline-flex items-center gap-1.5 rounded-full px-2 py-[2px] text-[10.5px] font-semibold leading-none transition ${
          checked
            ? "bg-spill-wrong text-white"
            : "border border-dashed border-spill-line text-spill-muted hover:border-spill-fg/30 hover:text-spill-fg"
        }`}
      >
        <span
          aria-hidden
          className={`grid h-3 w-3 place-items-center rounded-sm border text-[8px] leading-none ${
            checked ? "border-white/70 bg-white/15 text-white" : "border-spill-muted/60 text-transparent"
          }`}
        >
          ✓
        </span>
        {checked ? "i'm ready" : "i'm ready"}
      </button>
    </form>
  );
}

// Pill button that toggles the caller's own participation flag. Shown when
// the caller is currently sitting out so they can rejoin in one click.
function RejoinButton({ retroId, participantId }: { retroId: string; participantId: string }) {
  return (
    <form action={setParticipationAction} className="contents">
      <input name="retro_id" type="hidden" value={retroId} />
      <input name="participant_id" type="hidden" value={participantId} />
      <input name="is_participating" type="hidden" value="1" />
      <button
        type="submit"
        className="inline-flex items-center gap-1.5 rounded-full border border-dashed border-spill-line px-2 py-[2px] text-[10.5px] font-semibold leading-none text-spill-muted transition hover:border-spill-fg/30 hover:text-spill-fg"
        title="rejoin this round"
      >
        <span aria-hidden className="text-[9px]">↩</span>
        sitting out — rejoin
      </button>
    </form>
  );
}

function TitleSubtitle({
  board,
  currentUserParticipant,
}: {
  board: RetroBoard;
  currentUserParticipant: RetroBoard["participants"][number] | null;
}): ReactNode {
  const phase = board.retro.phase;
  if (phase === "scheduled") return <span>{displayRetroDate(board.retro)}</span>;
  if (phase === "writing") {
    // Three states for the caller:
    //   - sitting out: rejoin pill
    //   - participating + not yet ready: "i'm ready" pill (unchecked)
    //   - participating + ready: "i'm ready" pill (checked, click to unmark)
    if (currentUserParticipant && !currentUserParticipant.is_participating) {
      return (
        <RejoinButton
          retroId={board.retro.id}
          participantId={currentUserParticipant.id}
        />
      );
    }
    return <ReadyCheckbox retroId={board.retro.id} checked={board.ready.current_user_ready} />;
  }
  if (phase === "voting") {
    return (
      <span className="flex items-center gap-1.5">
        <span className="truncate">{board.voting.votes_remaining} of {board.retro.vote_limit} votes left</span>
        <VoteLeftDots remaining={board.voting.votes_remaining} total={board.retro.vote_limit} />
      </span>
    );
  }
  if (phase === "discussion") return <span>review and group cards</span>;
  if (phase === "action_discussion") return <span>top {board.retro.action_discussion_limit} actions</span>;
  return <span>wrapped recap</span>;
}

function CenterPhase({ board, isHost }: { board: RetroBoard; isHost: boolean }): ReactNode {
  // The completed phase swaps in WrappedSummary on its own page, so no
  // center widget is needed there.
  if (board.retro.phase === "completed" || board.retro.phase === "scheduled") return null;
  const allReady =
    board.ready.participant_count > 0 &&
    board.ready.ready_count >= board.ready.participant_count;
  return (
    <PhaseLine
      retroId={board.retro.id}
      phase={board.retro.phase}
      clusteringMode={board.retro.clustering_mode}
      clusteringStatus={board.retro.clustering_status}
      isHost={isHost}
      participantCount={board.ready.participant_count}
      readyCount={board.ready.ready_count}
      allReady={allReady}
      revealMode={board.retro.reveal_mode}
    />
  );
}

function ScheduledBoard({ board, isHost, query }: { board: RetroBoard; isHost: boolean; query: BoardSearchParams }) {
  const date = displayRetroDate(board.retro);
  const readyToStart = isPlannedForDue(board.retro.planned_for);
  const groupName = board.series?.name ?? "Group";
  return (
    <div className="relative flex min-h-0 flex-1">
      <BoardColumns board={board} query={query} />
      {readyToStart ? <ScheduledAutoStart retroId={board.retro.id} /> : null}
      <div className="absolute inset-0 z-10 grid place-items-center bg-[rgba(243,232,207,0.78)] p-6 backdrop-blur-[2px]">
        <Tile className="sp-panel-grain max-w-xl border-spill-mood/50 bg-spill-panel p-6 text-center shadow-[var(--shadow-3)]">
          <p className="-rotate-1 font-hand text-[30px] leading-none text-spill-fg">{readyToStart ? "ready to start" : "not on the wall yet"}</p>
          <h2 className="mt-3 text-[26px] font-extrabold tracking-[-0.03em] text-spill-fg">
            {readyToStart ? (
              isHost ? (
                <>
                  <span className="block">
                    [<InlineDetailEditor field="group_name" label="Retro group" retroId={board.retro.id} returnTo={`/retros/${board.retro.id}`} value={groupName} />]{" "}
                    <InlineDetailEditor field="title" label="Retro title" retroId={board.retro.id} returnTo={`/retros/${board.retro.id}`} value={board.retro.title} />
                  </span>
                  <span className="block">was planned for {date}.</span>
                </>
              ) : (
                `${board.series ? `[${board.series.name}] ` : ""}${board.retro.title} was planned for ${date}.`
              )
            ) : isHost ? (
              <>
                <span className="block">
                  [<InlineDetailEditor field="group_name" label="Retro group" retroId={board.retro.id} returnTo={`/retros/${board.retro.id}`} value={groupName} />]{" "}
                  <InlineDetailEditor field="title" label="Retro title" retroId={board.retro.id} returnTo={`/retros/${board.retro.id}`} value={board.retro.title} />
                </span>
                <span className="block">is scheduled for <PlannedDateEditor plannedFor={board.retro.planned_for} retroId={board.retro.id} />.</span>
              </>
            ) : (
              `${board.series ? `[${board.series.name}] ` : ""}${board.retro.title} is scheduled for ${date}.`
            )}
          </h2>
          <p className="mx-auto mt-2 max-w-md text-[13px] leading-6 text-[var(--fg-2)]">
            {readyToStart ? "Opening the board now." : "The board is ready, but writing stays closed until the host starts it."}
          </p>
          {isHost && !readyToStart ? (
            <form action={startScheduledRetroAction} className="mt-5">
              <input name="retro_id" type="hidden" value={board.retro.id} />
              <Btn kind="primary" type="submit" accent={spillColors.wrong}>start now</Btn>
            </form>
          ) : !readyToStart ? (
            <p className="mt-5 text-[11px] font-semibold uppercase tracking-[0.12em] text-spill-muted">waiting for the host</p>
          ) : (
            <p className="mt-5 text-[11px] font-semibold uppercase tracking-[0.12em] text-spill-muted">starting</p>
          )}
        </Tile>
      </div>
    </div>
  );
}
