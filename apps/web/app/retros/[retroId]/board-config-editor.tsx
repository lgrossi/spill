"use client";

import { useState, useTransition } from "react";
import { Btn, Tile, spillColors } from "@/components/spill-ui";
import { updateRetroDetailsAction } from "@/lib/actions";

const numberInputStyle = {
  borderWidth: "0 0 1px",
  borderStyle: "solid",
  borderColor: "var(--line)",
  borderRadius: 0,
  boxShadow: "none",
} as const;

export function BoardConfigEditor({
  retroId,
  returnTo,
  voteLimit,
  actionDiscussionLimit,
  clusteringMode,
  hasActionColumn,
}: {
  retroId: string;
  returnTo: string;
  voteLimit: number;
  actionDiscussionLimit: number;
  clusteringMode: string;
  hasActionColumn: boolean;
}) {
  const [open, setOpen] = useState(false);
  const [votingEnabled, setVotingEnabled] = useState(voteLimit > 0);
  const [topVotedToActions, setTopVotedToActions] = useState(actionDiscussionLimit > 0);
  const [autoOrganize, setAutoOrganize] = useState(clusteringMode === "auto_on_vote_start");
  const [isPending, startTransition] = useTransition();

  if (!open) {
    return (
      <button
        className="mx-auto mt-4 block text-[11px] font-semibold uppercase tracking-[0.12em] text-spill-muted underline decoration-dashed underline-offset-4 transition hover:text-spill-fg"
        onClick={() => setOpen(true)}
        type="button"
      >
        edit board settings
      </button>
    );
  }

  return (
    <form
      action={(formData) => startTransition(() => void updateRetroDetailsAction(formData))}
      className="mx-auto mt-4 max-w-md space-y-2.5 text-left"
    >
      <input name="retro_id" type="hidden" value={retroId} />
      <input name="return_to" type="hidden" value={returnTo} />
      <Tile className="flex items-center gap-3">
        <RuleMark>●●●</RuleMark>
        <label className="group/check flex min-w-0 flex-1 cursor-pointer items-center justify-between gap-3">
          <input name="voting_enabled" type="hidden" value="0" />
          <input className="sr-only" name="voting_enabled" type="checkbox" value="1" checked={votingEnabled} onChange={(event) => setVotingEnabled(event.currentTarget.checked)} />
          <span className="min-w-0">
            <span className="block text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">voting</span>
            <span className={`mt-1 flex items-center gap-1.5 text-[12px] font-semibold transition ${votingEnabled ? "text-[var(--fg-2)]" : "text-spill-muted/60"}`}>
              <input
                className="h-6 w-8 bg-transparent px-0 text-center text-[12px] font-extrabold text-spill-fg outline-none disabled:text-spill-muted/45"
                name="vote_limit"
                type="number"
                min="1"
                defaultValue={Math.max(1, voteLimit)}
                disabled={!votingEnabled}
                aria-label="Votes per person"
                style={numberInputStyle}
              />
              votes per person
            </span>
          </span>
          <Check />
        </label>
      </Tile>
      {hasActionColumn ? (
        <Tile className="flex items-center gap-3">
          <RuleMark>★</RuleMark>
          <label className="group/check flex min-w-0 flex-1 cursor-pointer items-center justify-between gap-3">
            <input name="action_discussion_enabled" type="hidden" value="0" />
            <input className="sr-only" name="action_discussion_enabled" type="checkbox" value="1" checked={topVotedToActions} onChange={(event) => setTopVotedToActions(event.currentTarget.checked)} />
            <span className="min-w-0">
              <span className="block text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">top voted to actions</span>
              <span className={`mt-1 flex items-center gap-1.5 text-[12px] font-semibold transition ${topVotedToActions ? "text-[var(--fg-2)]" : "text-spill-muted/60"}`}>
                move
                <input
                  className="h-6 w-8 bg-transparent px-0 text-center text-[12px] font-extrabold text-spill-fg outline-none disabled:text-spill-muted/45"
                  name="action_discussion_limit"
                  type="number"
                  min="1"
                  defaultValue={Math.max(1, actionDiscussionLimit)}
                  disabled={!topVotedToActions}
                  aria-label="Number of top voted cards moved to actions"
                  style={numberInputStyle}
                />
                top voted cards
              </span>
            </span>
            <Check />
          </label>
        </Tile>
      ) : null}
      <Tile className="flex items-center gap-3">
        <RuleMark>◆</RuleMark>
        <label className="group/check flex min-w-0 flex-1 cursor-pointer items-center justify-between gap-3">
          <input name="clustering_mode" type="hidden" value="disabled" />
          <input className="sr-only" name="clustering_mode" type="checkbox" value="auto_on_vote_start" checked={autoOrganize} onChange={(event) => setAutoOrganize(event.currentTarget.checked)} />
          <span className="min-w-0">
            <span className="block text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">AI clustering</span>
            <span className={`mt-1 block text-[11px] font-semibold transition ${autoOrganize ? "text-[var(--fg-2)]" : "text-spill-muted/60"}`}>
              {autoOrganize ? "AI groups and tags cards before voting" : "manual grouping"}
            </span>
          </span>
          <Check />
        </label>
      </Tile>
      <div className="flex items-center justify-end gap-2">
        <button
          className="text-[11px] font-semibold uppercase tracking-[0.12em] text-spill-muted transition hover:text-spill-fg"
          onClick={() => setOpen(false)}
          type="button"
        >
          cancel
        </button>
        <Btn accent={spillColors.well} disabled={isPending} kind="primary" type="submit">
          {isPending ? "saving..." : "save settings"}
        </Btn>
      </div>
    </form>
  );
}

function RuleMark({ children }: { children: React.ReactNode }) {
  return (
    <span className="grid h-7 w-7 shrink-0 place-items-center rounded-[8px] border border-spill-line bg-[var(--paper)] text-[10px] font-extrabold tracking-[-0.04em] text-spill-muted">
      {children}
    </span>
  );
}

function Check() {
  return (
    <span className="grid h-6 w-6 shrink-0 place-items-center rounded-[6px] border border-spill-line bg-[var(--paper)] text-[14px] font-extrabold text-transparent transition group-has-[input:checked]/check:border-spill-well group-has-[input:checked]/check:bg-spill-well group-has-[input:checked]/check:text-white">
      ✓
    </span>
  );
}
