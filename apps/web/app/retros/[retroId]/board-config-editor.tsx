"use client";

import { useTransition } from "react";
import { Btn, spillColors } from "@/components/spill-ui";
import { BoardBehaviorToggles } from "@/components/board-behavior-toggles";
import { updateRetroDetailsAction } from "@/lib/actions";
import type { RetroBoard } from "@/lib/contracts";

type CardEditPolicy = RetroBoard["retro"]["card_edit_policy"];
type RevealMode = RetroBoard["retro"]["reveal_mode"];

// Settings-dialog form: shares every behavior toggle with the create form
// via <BoardBehaviorToggles />. This wrapper only owns: the form action, the
// retro_id / return_to hidden inputs, the cancel + save buttons, and the
// phase-aware gating of the voting toggle. Anything visual or field-level
// belongs in the shared component so the two contexts can't drift again.
export function BoardConfigForm({
  retroId,
  phase,
  returnTo,
  voteLimit,
  actionDiscussionLimit,
  clusteringMode,
  hasActionColumn,
  cardEditPolicy,
  anonymousAuthors,
  revealMode,
  onCancel,
  className,
}: {
  retroId: string;
  phase?: string;
  returnTo: string;
  voteLimit: number;
  actionDiscussionLimit: number;
  clusteringMode: string;
  hasActionColumn: boolean;
  cardEditPolicy: CardEditPolicy;
  anonymousAuthors: boolean;
  revealMode: RevealMode;
  onCancel?: () => void;
  className?: string;
}) {
  const [isPending, startTransition] = useTransition();
  // Voting can't be turned off mid-flight -- existing votes have nowhere to
  // live if vote_limit drops to 0 while the phase is voting.
  const canToggleVoting = phase !== "voting";

  return (
    <form
      // Plain onSubmit (not action={...}) so React 19's form-action
      // auto-reset doesn't fire after save. The reset would flip every
      // controlled checkbox's DOM `checked` back to its initial render
      // value (the state when the dialog opened), out of sync with the
      // useState the toggles are driven by. The DOM only re-synced when
      // the dialog closed + reopened.
      onSubmit={(event) => {
        event.preventDefault();
        const formData = new FormData(event.currentTarget);
        startTransition(() => void updateRetroDetailsAction(formData));
      }}
      className={`space-y-2.5 text-left ${className ?? ""}`}
    >
      <input name="retro_id" type="hidden" value={retroId} />
      <input name="return_to" type="hidden" value={returnTo} />
      <BoardBehaviorToggles
        initial={{
          votingEnabled: voteLimit > 0,
          voteLimit: Math.max(1, voteLimit),
          topVotedToActions: actionDiscussionLimit > 0,
          actionDiscussionLimit: Math.max(1, actionDiscussionLimit),
          autoOrganize: clusteringMode === "auto_on_vote_start",
          authorOnly: cardEditPolicy === "author_only",
          hideAuthors: anonymousAuthors,
          perColumnReveal: revealMode === "per_column",
        }}
        canToggleVoting={canToggleVoting}
        showActionDiscussion={hasActionColumn}
      />
      <div className="flex items-center justify-end gap-2">
        {onCancel ? (
          <button
            className="text-[11px] font-semibold uppercase tracking-[0.12em] text-spill-muted transition hover:text-spill-fg"
            onClick={onCancel}
            type="button"
          >
            cancel
          </button>
        ) : null}
        <Btn accent={spillColors.well} disabled={isPending} kind="primary" type="submit">
          {isPending ? "saving..." : "save settings"}
        </Btn>
      </div>
    </form>
  );
}
