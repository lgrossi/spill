import type { ReactNode } from "react";
import { notFound } from "next/navigation";
import { IdentityGate, IdentityUnavailable } from "../../components/identity-gate";
import { AppChrome, Stack, TEAM } from "../../components/spill-ui";
import { getRetro, type RetroBoard } from "../../lib/api";
import { currentIdentity, localIdentityEnabled } from "../../lib/identity";
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

async function loadBoard(retroId: string) {
  try {
    return await getRetro(retroId);
  } catch {
    return null;
  }
}
