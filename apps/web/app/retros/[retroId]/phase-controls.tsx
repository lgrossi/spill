import type { RetroBoard } from "@/lib/contracts";
import { BoardInviteButton } from "@/components/board-invite-button";
import { DeleteBoardButton } from "@/components/delete-board-button";

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
      {peopleButton}
      {deleteButton}
    </>
  );
}
