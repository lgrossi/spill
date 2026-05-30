import type { ReactNode } from "react";
import { notFound } from "next/navigation";
import { BoardAccessDenied, IdentityGate, IdentityUnavailable } from "@/components/identity-gate";
import { AppChrome, Stack, avatarColorForSeed, avatarInitials } from "@/components/spill-ui";
import { ApiError, getRetro, type RetroBoard } from "@/lib/api";
import { currentIdentity, localIdentityEnabled } from "@/lib/identity";
import { listGrantsAction, markReadyAction, unmarkReadyAction } from "@/lib/actions";
import { BoardColumns } from "./board-columns";
import { BoardSync } from "./board-sync";
import { presenceForPhase } from "./board-presentation";
import { PhaseControls } from "./phase-controls";
import { PhaseLine } from "./phase-line";
import { WrappedSummary } from "./wrapped-summary";

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

  return (
    <AppChrome
      actions={
        <PhaseControls
          board={board}
          isHost={isHost}
          currentUserEmail={currentUserEmail}
        />
      }
      presence={<ParticipantStack board={board} />}
      center={<CenterPhase board={board} isHost={isHost} />}
      subtitle={<TitleSubtitle board={board} />}
      title={board.retro.title}
    >
      <BoardSync retroId={board.retro.id} />
      {board.retro.phase === "completed" ? (
        <WrappedSummary board={board} isHost={isHost} />
      ) : (
        <BoardColumns board={board} query={query} />
      )}
    </AppChrome>
  );
}

function ParticipantStack({ board }: { board: RetroBoard }) {
  return (
    <Stack
      people={board.participants.map((participant) => ({
        color: avatarColorForSeed(participant.id),
        k: avatarInitials(participant.display_name),
        status: presenceForPhase(board.retro.phase),
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

function TitleSubtitle({ board }: { board: RetroBoard }): ReactNode {
  const phase = board.retro.phase;
  const date = boardDateLabel(board);
  if (phase === "writing") {
    return (
      <>
        {date ? <span>{date}</span> : null}
        <ReadyCheckbox retroId={board.retro.id} checked={board.ready.current_user_ready} />
      </>
    );
  }
  if (phase === "voting") {
    return (
      <span className="flex items-center gap-1.5">
        {date ? <span>{date} ·</span> : null}
        <span className="truncate">{board.voting.votes_remaining} of {board.retro.vote_limit} votes left</span>
        <VoteLeftDots remaining={board.voting.votes_remaining} total={board.retro.vote_limit} />
      </span>
    );
  }
  if (phase === "discussion") return <span>{date ? `${date} · ` : ""}review and group cards</span>;
  if (phase === "action_discussion") return <span>{date ? `${date} · ` : ""}top {board.retro.action_discussion_limit} actions</span>;
  return <span>{date ? `${date} · ` : ""}wrapped recap</span>;
}

function boardDateLabel(board: RetroBoard) {
  if (board.retro.scheduled_at) return `scheduled ${shortDate(board.retro.scheduled_at)}`;
  return `created ${shortDate(board.retro.created_at)}`;
}

function shortDate(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "recently";
  return new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric", year: "numeric" }).format(date);
}

function CenterPhase({ board, isHost }: { board: RetroBoard; isHost: boolean }): ReactNode {
  // The completed phase swaps in WrappedSummary on its own page, so no
  // center widget is needed there.
  if (board.retro.phase === "completed") return null;
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
    />
  );
}

async function loadBoard(retroId: string): Promise<RetroBoard | "forbidden" | null> {
  try {
    return await getRetro(retroId);
  } catch (e) {
    if (e instanceof ApiError && e.status === 403) return "forbidden";
    return null;
  }
}
