import {
  completeRetroAction,
  revealRetroAction,
  forceRevealRetroAction,
  startActionDiscussionAction,
  startVotingAction,
} from "@/lib/actions";
import type { RetroBoard } from "@/lib/contracts";
import { Btn, spillColors } from "@/components/spill-ui";
import { BoardInviteButton } from "@/components/board-invite-button";

type NextSpec = {
  action: (formData: FormData) => void | Promise<void>;
  kind: "primary" | "dashed";
  accent?: string;
  icon: string;
  label: string;
  disabled?: boolean;
};

function nextActionFor(board: RetroBoard, isHost: boolean): NextSpec | null {
  const phase = board.retro.phase;
  const allReady =
    board.ready.participant_count > 0 &&
    board.ready.ready_count >= board.ready.participant_count;
  if (phase === "writing") {
    return {
      action: isHost ? forceRevealRetroAction : revealRetroAction,
      kind: allReady ? "primary" : "dashed",
      icon: "»",
      label: "next: discuss",
      disabled: !isHost && !allReady,
    };
  }
  if (phase === "discussion") {
    if (board.retro.vote_limit <= 0) {
      return {
        action: startActionDiscussionAction,
        kind: "primary",
        accent: spillColors.action,
        icon: "»",
        label: "next: act",
      };
    }
    return {
      action: startVotingAction,
      kind: "primary",
      icon: "»",
      label: "next: vote",
    };
  }
  if (phase === "voting") {
    return {
      action: startActionDiscussionAction,
      kind: allReady ? "primary" : "dashed",
      accent: spillColors.action,
      icon: "»",
      label: "next: act",
      disabled:
        (!isHost && !allReady) || board.retro.action_discussion_limit <= 0,
    };
  }
  if (phase === "action_discussion") {
    return {
      action: completeRetroAction,
      kind: "primary",
      icon: "✓",
      label: "wrap retro",
    };
  }
  return null;
}

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

  const next = nextActionFor(board, isHost);
  return (
    <>
      {peopleButton}
      {next ? (
        <form action={next.action}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <Btn
            aria-label={next.label}
            title={next.label}
            kind={next.kind}
            accent={next.accent}
            type="submit"
            disabled={next.disabled}
            className="min-w-[44px] text-base"
          >
            {next.icon}
          </Btn>
        </form>
      ) : null}
    </>
  );
}
