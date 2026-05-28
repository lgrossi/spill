import type { RetroBoard } from "@/lib/contracts";
import { Btn } from "@/components/spill-ui";
import { BoardInviteButton } from "@/components/board-invite-button";

// Right-side topbar: people button + home shortcut on completed.
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

  if (board.retro.phase === "completed") {
    return (
      <>
        {peopleButton}
        <Btn href="/" kind="primary">home</Btn>
      </>
    );
  }

  return <>{peopleButton}</>;
}
