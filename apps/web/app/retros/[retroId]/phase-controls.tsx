import type { RetroBoard } from "@/lib/contracts";
import { BoardInviteButton } from "@/components/board-invite-button";
import { DeleteBoardButton } from "@/components/delete-board-button";
import { isActionsColumn } from "./board-presentation";
import { BoardSettingsButton } from "./board-settings-button";

// Right-side topbar: people button and host delete affordance.
// Phase advance happens in the centered PhaseLine (auto-advance countdown
// for ready-gated phases; host-only text link for the rest).
export function PhaseControls({
  board,
  isHost = false,
  currentUserEmail = "",
}: {
  board: RetroBoard;
  isHost?: boolean;
  currentUserEmail?: string;
}) {
  const peopleButton = currentUserEmail ? (
    <BoardInviteButton
      retroId={board.retro.id}
      currentUserEmail={currentUserEmail}
      isHost={isHost}
    />
  ) : null;
  const deleteButton = isHost ? (
    <DeleteBoardButton retroId={board.retro.id} boardTitle={board.retro.title} />
  ) : null;
  const settingsButton =
    isHost && board.retro.phase !== "completed" && board.retro.phase !== "action_discussion" ? (
      <BoardSettingsButton
        retroId={board.retro.id}
        phase={board.retro.phase}
        returnTo={`/retros/${board.retro.id}`}
        voteLimit={board.retro.vote_limit}
        actionDiscussionLimit={board.retro.action_discussion_limit}
        clusteringMode={board.retro.clustering_mode}
        hasActionColumn={board.columns.some(isActionsColumn)}
        cardEditPolicy={board.retro.card_edit_policy}
      />
    ) : null;

  if (board.retro.phase === "completed") {
    return (
      <>
        {peopleButton}
        {deleteButton}
      </>
    );
  }

  return (
    <>
      {settingsButton}
      {peopleButton}
      {deleteButton}
    </>
  );
}
