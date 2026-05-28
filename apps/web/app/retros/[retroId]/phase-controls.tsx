import type { CSSProperties } from "react";
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

export type NextSpec = {
  action: (formData: FormData) => void | Promise<void>;
  kind: "primary" | "dashed";
  accent?: string;
  cta: string;
  label: string;
  disabled?: boolean;
};

export function nextActionFor(board: RetroBoard, isHost: boolean): NextSpec | null {
  const phase = board.retro.phase;
  const allReady =
    board.ready.participant_count > 0 &&
    board.ready.ready_count >= board.ready.participant_count;
  if (phase === "writing") {
    return {
      action: isHost ? forceRevealRetroAction : revealRetroAction,
      kind: allReady ? "primary" : "dashed",
      cta: "proceed »",
      label: "proceed to discuss",
      disabled: !isHost && !allReady,
    };
  }
  if (phase === "discussion") {
    if (board.retro.vote_limit <= 0) {
      return {
        action: startActionDiscussionAction,
        kind: "primary",
        accent: spillColors.action,
        cta: "proceed »",
        label: "proceed to act",
      };
    }
    return {
      action: startVotingAction,
      kind: "primary",
      cta: "proceed »",
      label: "proceed to vote",
    };
  }
  if (phase === "voting") {
    return {
      action: startActionDiscussionAction,
      kind: allReady ? "primary" : "dashed",
      accent: spillColors.action,
      cta: "proceed »",
      label: "proceed to act",
      disabled:
        (!isHost && !allReady) || board.retro.action_discussion_limit <= 0,
    };
  }
  if (phase === "action_discussion") {
    return {
      action: completeRetroAction,
      kind: "primary",
      cta: "wrap ✓",
      label: "wrap retro",
    };
  }
  return null;
}

export function ProceedButton({ retroId, spec }: { retroId: string; spec: NextSpec }) {
  const accent = spec.accent ?? spillColors.wrong;
  const filled = spec.kind === "primary";
  const style: CSSProperties = filled ? { backgroundColor: accent } : {};
  return (
    <form action={spec.action} className="contents">
      <input name="retro_id" type="hidden" value={retroId} />
      <button
        aria-label={spec.label}
        className={`inline-flex h-[22px] items-center gap-1 rounded-full px-2.5 text-[10.5px] font-extrabold uppercase tracking-[0.08em] leading-none transition disabled:pointer-events-none disabled:opacity-55 ${
          filled
            ? "text-white shadow-[inset_0_1px_0_rgba(255,255,255,0.22),0_1px_0_rgba(74,52,20,0.12)] hover:brightness-[0.97]"
            : "border border-dashed border-spill-line text-spill-muted hover:border-spill-fg/30 hover:text-spill-fg"
        }`}
        disabled={spec.disabled}
        style={style}
        title={spec.label}
        type="submit"
      >
        {spec.cta}
      </button>
    </form>
  );
}

// Right-side action area: people button + (on completed) home shortcut.
// The proceed button lives in the centered phase block, not here.
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
