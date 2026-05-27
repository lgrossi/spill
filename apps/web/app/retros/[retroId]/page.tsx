import type { ReactNode } from "react";
import { notFound } from "next/navigation";
import { IdentityGate, IdentityUnavailable } from "../../components/identity-gate";
import { BoardAccessDenied } from "../../components/identity-gate";
import { AppChrome, Stack, avatarColorForSeed, avatarInitials } from "../../components/spill-ui";
import { ApiError, getRetro, type RetroBoard } from "../../lib/api";
import { currentIdentity, localIdentityEnabled } from "../../lib/identity";
import { listGrantsAction } from "../../lib/actions";
import { BoardColumns } from "./board-columns";
import { BoardSync } from "./board-sync";
import { presenceForPhase } from "./board-presentation";
import { PhaseControls } from "./phase-controls";
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
    <span className="flex shrink-0 gap-0.5" aria-label={`${remaining} votes left`}>
      {Array.from({ length: total }).map((_, index) => (
        <span className={`h-1.5 w-1.5 rounded-full border border-spill-well/45 ${index < remaining ? "bg-spill-well/75" : "bg-transparent"}`} key={index} />
      ))}
    </span>
  );
}

function phaseSubtitle(board: RetroBoard): ReactNode {
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

async function loadBoard(retroId: string): Promise<RetroBoard | "forbidden" | null> {
  try {
    return await getRetro(retroId);
  } catch (e) {
    if (e instanceof ApiError && e.status === 403) return "forbidden";
    return null;
  }
}
