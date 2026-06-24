"use client";

import { useState } from "react";
import { Tile } from "@/components/spill-ui";

// The eight named inputs this component emits map 1:1 to the create + update
// payload fields read in retro-commands.ts. Anything new lives here once
// (markup + state) so the create form and settings dialog can't drift apart
// again -- the reveal_mode drift that motivated this extraction was caused
// by adding the field in only one of the two places.
export type BoardBehaviorValues = {
  votingEnabled: boolean;
  voteLimit: number;
  topVotedToActions: boolean;
  actionDiscussionLimit: number;
  autoOrganize: boolean;
  authorOnly: boolean;
  hideAuthors: boolean;
  perColumnReveal: boolean;
};

export type BoardBehaviorTogglesProps = {
  initial: BoardBehaviorValues;
  // Settings dialog: locked-on (submits voting_enabled=1 unconditionally and
  // hides the off switch) when phase === "voting" so vote counts can't
  // disappear mid-flight. Create form always passes true.
  canToggleVoting?: boolean;
  // Settings dialog: hides the entire tile when the template has no action
  // column (nothing to flow top-voted cards into). Create form always passes
  // true -- the template picker controls whether an action column is added
  // at creation.
  showActionDiscussion?: boolean;
  // Live snapshot for callers that mirror values into other UI (e.g. the
  // create-form live preview gates the action column on topVotedToActions).
  // Settings dialog ignores it.
  onChange?: (values: BoardBehaviorValues) => void;
  // Caller-owned wrapper class so each context picks its layout: settings
  // dialog passes "space-y-2.5" (vertical stack in a narrow modal), create
  // form passes "grid gap-2.5 md:grid-cols-2" (two-column on wide screens).
  className?: string;
};

export function BoardBehaviorToggles({
  initial,
  canToggleVoting = true,
  showActionDiscussion = true,
  onChange,
  className = "space-y-2.5",
}: BoardBehaviorTogglesProps) {
  const [values, setValues] = useState<BoardBehaviorValues>(initial);

  // Notify in the same tick as the setState so callers see the new value
  // before the next render. Patch always reads the latest `values` from
  // closure -- safe because our toggles never fire two changes in the same
  // synchronous event.
  function patch(next: Partial<BoardBehaviorValues>) {
    const merged = { ...values, ...next };
    setValues(merged);
    onChange?.(merged);
  }

  return (
    <div className={className}>
      <Tile className="flex items-center gap-3">
        <RuleMark>●●●</RuleMark>
        <label className="group/check flex min-w-0 flex-1 cursor-pointer items-center justify-between gap-3">
          {canToggleVoting ? (
            <>
              <input name="voting_enabled" type="hidden" value="0" />
              <input
                className="sr-only"
                name="voting_enabled"
                type="checkbox"
                value="1"
                checked={values.votingEnabled}
                onChange={(event) => patch({ votingEnabled: event.currentTarget.checked })}
              />
            </>
          ) : (
            <input name="voting_enabled" type="hidden" value="1" />
          )}
          <span className="min-w-0">
            <span className="block text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">voting</span>
            <span className={`mt-1 flex items-center gap-1.5 text-[12px] font-semibold transition ${values.votingEnabled ? "text-[var(--fg-2)]" : "text-spill-muted/60"}`}>
              <input
                className="h-6 w-8 bg-transparent px-0 text-center text-[12px] font-extrabold text-spill-fg outline-none disabled:text-spill-muted/45"
                name="vote_limit"
                type="number"
                min="1"
                value={Math.max(1, values.voteLimit)}
                onChange={(event) => patch({ voteLimit: Number(event.currentTarget.value) || 1 })}
                disabled={!values.votingEnabled}
                required={values.votingEnabled}
                aria-label="Votes per person"
                style={numberInputStyle}
              />
              votes per person
            </span>
          </span>
          <Check />
        </label>
      </Tile>

      {showActionDiscussion ? (
        <Tile className="flex items-center gap-3">
          <RuleMark>★</RuleMark>
          <label className="group/check flex min-w-0 flex-1 cursor-pointer items-center justify-between gap-3">
            <input name="action_discussion_enabled" type="hidden" value="0" />
            <input
              className="sr-only"
              name="action_discussion_enabled"
              type="checkbox"
              value="1"
              checked={values.topVotedToActions}
              onChange={(event) => patch({ topVotedToActions: event.currentTarget.checked })}
            />
            <span className="min-w-0">
              <span className="block text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">top voted to actions</span>
              <span className={`mt-1 flex items-center gap-1.5 text-[12px] font-semibold transition ${values.topVotedToActions ? "text-[var(--fg-2)]" : "text-spill-muted/60"}`}>
                move
                <input
                  className="h-6 w-8 bg-transparent px-0 text-center text-[12px] font-extrabold text-spill-fg outline-none disabled:text-spill-muted/45"
                  name="action_discussion_limit"
                  type="number"
                  min="1"
                  value={Math.max(1, values.actionDiscussionLimit)}
                  onChange={(event) => patch({ actionDiscussionLimit: Number(event.currentTarget.value) || 1 })}
                  disabled={!values.topVotedToActions}
                  required={values.topVotedToActions}
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
          <input
            className="sr-only"
            name="clustering_mode"
            type="checkbox"
            value="auto_on_vote_start"
            checked={values.autoOrganize}
            onChange={(event) => patch({ autoOrganize: event.currentTarget.checked })}
          />
          <span className="min-w-0">
            <span className="block text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">AI clustering</span>
            <span className={`mt-1 block text-[11px] font-semibold transition ${values.autoOrganize ? "text-[var(--fg-2)]" : "text-spill-muted/60"}`}>
              {values.autoOrganize ? "AI groups and tags cards before voting" : "manual grouping"}
            </span>
          </span>
          <Check />
        </label>
      </Tile>

      <Tile className="flex items-center gap-3">
        <RuleMark>✎</RuleMark>
        <label className="group/check flex min-w-0 flex-1 cursor-pointer items-center justify-between gap-3">
          <input name="card_edit_policy" type="hidden" value="collaborative" />
          <input
            className="sr-only"
            name="card_edit_policy"
            type="checkbox"
            value="author_only"
            checked={values.authorOnly}
            onChange={(event) => patch({ authorOnly: event.currentTarget.checked })}
          />
          <span className="min-w-0">
            <span className="block text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">card editing</span>
            <span className={`mt-1 block text-[11px] font-semibold transition ${values.authorOnly ? "text-[var(--fg-2)]" : "text-spill-muted/60"}`}>
              {values.authorOnly ? "only the author (or host) can edit / delete a card" : "anyone on the board can edit / delete any card"}
            </span>
          </span>
          <Check />
        </label>
      </Tile>

      <Tile className="flex items-center gap-3">
        <RuleMark>◐</RuleMark>
        <label className="group/check flex min-w-0 flex-1 cursor-pointer items-center justify-between gap-3">
          <input name="anonymous_authors" type="hidden" value="0" />
          <input
            className="sr-only"
            name="anonymous_authors"
            type="checkbox"
            value="1"
            checked={values.hideAuthors}
            onChange={(event) => patch({ hideAuthors: event.currentTarget.checked })}
          />
          <span className="min-w-0">
            <span className="block text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">card authors</span>
            <span className={`mt-1 block text-[11px] font-semibold transition ${values.hideAuthors ? "text-[var(--fg-2)]" : "text-spill-muted/60"}`}>
              {values.hideAuthors ? "anonymous — only you see your own cards as yours" : "everyone sees who wrote each card"}
            </span>
          </span>
          <Check />
        </label>
      </Tile>

      <Tile className="flex items-center gap-3">
        <RuleMark>☉</RuleMark>
        <label className="group/check flex min-w-0 flex-1 cursor-pointer items-center justify-between gap-3">
          <input name="reveal_mode" type="hidden" value="big_bang" />
          <input
            className="sr-only"
            name="reveal_mode"
            type="checkbox"
            value="per_column"
            checked={values.perColumnReveal}
            onChange={(event) => patch({ perColumnReveal: event.currentTarget.checked })}
          />
          <span className="min-w-0">
            <span className="block text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">reveal mode</span>
            <span className={`mt-1 block text-[11px] font-semibold transition ${values.perColumnReveal ? "text-[var(--fg-2)]" : "text-spill-muted/60"}`}>
              {values.perColumnReveal ? "host reveals one column at a time" : "reveal everything at once when the room is ready"}
            </span>
          </span>
          <Check />
        </label>
      </Tile>
    </div>
  );
}

const numberInputStyle = {
  borderWidth: "0 0 1px",
  borderStyle: "solid",
  borderColor: "var(--line)",
  borderRadius: 0,
  boxShadow: "none",
} as const;

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
