import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';

import {
  BoardBehaviorToggles,
  type BoardBehaviorValues,
} from '../app/components/board-behavior-toggles';

const defaults: BoardBehaviorValues = {
  votingEnabled: true,
  voteLimit: 3,
  topVotedToActions: true,
  actionDiscussionLimit: 3,
  autoOrganize: false,
  authorOnly: false,
  hideAuthors: false,
  perColumnReveal: true,
};

// Helper: collect form data from the rendered component wrapped in a form so
// we exercise the same submit path as the production callers.
function renderInForm(props: Partial<React.ComponentProps<typeof BoardBehaviorToggles>> = {}) {
  const submitted: Array<Record<string, string[]>> = [];
  function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const formData = new FormData(event.currentTarget);
    const collected: Record<string, string[]> = {};
    formData.forEach((value, key) => {
      collected[key] = collected[key] ?? [];
      collected[key].push(String(value));
    });
    submitted.push(collected);
  }
  render(
    <form onSubmit={handleSubmit}>
      <BoardBehaviorToggles initial={defaults} {...props} />
      <button type="submit">submit</button>
    </form>,
  );
  return { submit: () => fireEvent.click(screen.getByText('submit')), submitted };
}

describe('BoardBehaviorToggles', () => {
  it('emits every payload field at defaults so create + settings forms see the same wire shape', () => {
    const { submit, submitted } = renderInForm();
    submit();
    const last = submitted.at(-1)!;
    // Hidden + checkbox pairs both submit; formData.getAll(...).at(-1) wins
    // (see retro-commands.ts:buildCreatePayload / updateRetroDetails).
    expect(last.voting_enabled.at(-1)).toBe('1');
    expect(last.vote_limit.at(-1)).toBe('3');
    expect(last.action_discussion_enabled.at(-1)).toBe('1');
    expect(last.action_discussion_limit.at(-1)).toBe('3');
    expect(last.clustering_mode.at(-1)).toBe('disabled');
    expect(last.card_edit_policy.at(-1)).toBe('collaborative');
    expect(last.anonymous_authors.at(-1)).toBe('0');
    expect(last.reveal_mode.at(-1)).toBe('per_column');
  });

  it('flips each toggle so the off-value submits when unchecked', () => {
    const { submit, submitted } = renderInForm({
      initial: {
        ...defaults,
        votingEnabled: false,
        topVotedToActions: false,
        autoOrganize: false,
        authorOnly: false,
        hideAuthors: false,
        perColumnReveal: false,
      },
    });
    submit();
    const last = submitted.at(-1)!;
    expect(last.voting_enabled.at(-1)).toBe('0');
    expect(last.action_discussion_enabled.at(-1)).toBe('0');
    expect(last.clustering_mode.at(-1)).toBe('disabled');
    expect(last.card_edit_policy.at(-1)).toBe('collaborative');
    expect(last.anonymous_authors.at(-1)).toBe('0');
    expect(last.reveal_mode.at(-1)).toBe('big_bang');
  });

  it('canToggleVoting=false locks voting on and hides the checkbox', () => {
    renderInForm({ canToggleVoting: false });
    // The off-value hidden input is replaced by a single hidden=1 input --
    // submit MUST report voting_enabled=1 regardless of user action.
    const inputs = Array.from(
      document.querySelectorAll<HTMLInputElement>('input[name="voting_enabled"]'),
    );
    expect(inputs).toHaveLength(1);
    expect(inputs[0].value).toBe('1');
    expect(inputs[0].type).toBe('hidden');
  });

  it('showActionDiscussion=false hides the action_discussion fields entirely', () => {
    renderInForm({ showActionDiscussion: false });
    expect(
      document.querySelector('input[name="action_discussion_enabled"]'),
    ).toBeNull();
    expect(
      document.querySelector('input[name="action_discussion_limit"]'),
    ).toBeNull();
  });

  it('onChange fires a complete values snapshot on each toggle so callers (live preview) can react', () => {
    const onChange = vi.fn();
    render(<BoardBehaviorToggles initial={defaults} onChange={onChange} />);
    // Per-column reveal tile -- uncheck it.
    const revealCheckbox = document.querySelector<HTMLInputElement>(
      'input[name="reveal_mode"][type="checkbox"]',
    );
    fireEvent.click(revealCheckbox!);
    expect(onChange).toHaveBeenLastCalledWith(
      expect.objectContaining({ perColumnReveal: false }),
    );
  });
});
