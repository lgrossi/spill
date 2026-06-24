import { describe, expect, it, vi } from 'vitest';
import React from 'react';
import { render, screen } from '@testing-library/react';
import type { RetroBoard } from '../app/lib/api';

vi.mock('../app/lib/actions', () => ({
  createDraftCardAction: vi.fn(),
  deleteDraftCardAction: vi.fn(),
  removeClusterMemberAction: vi.fn(),
  updateDraftCardAction: vi.fn(),
}));

vi.mock('../app/lib/commands/board-phase-commands', () => ({
  revealColumnCommand: vi.fn(),
}));

import { BoardColumns } from '../app/retros/[retroId]/board-columns';

function board(phase: RetroBoard['retro']['phase'], revealedAt: string | null = '2026-06-24T10:00:00Z'): RetroBoard {
  return {
    retro: {
      id: 'retro-1',
      title: 'Retro',
      phase,
      vote_limit: 3,
      action_discussion_limit: 3,
      cover_gif_url: null,
      cover_gif_alt_text: null,
      clustering_mode: 'disabled',
      clustering_status: 'not_run',
      anonymous_authors: false,
      planned_for: '',
      happened_at: null,
      card_edit_policy: 'collaborative',
      reveal_mode: 'per_column',
    },
    series: null,
    next_retro: null,
    participants: [],
    columns: [{
      id: 'col-1',
      retro_id: 'retro-1',
      column_key: 'went_well',
      title: 'Went well',
      position: 0,
      accent_color: '#2f9469',
      revealed_at: revealedAt,
      cards: [],
    }],
    ready: { participant_count: 1, ready_count: 0, current_user_ready: false },
    voting: { vote_limit: 3, votes_used: 0, votes_remaining: 3 },
    clusters: [],
    actions: [],
    deck: [],
    ai_artifacts: [],
    meeting_notes: [],
    deliveries: [],
  };
}

describe('BoardColumns composer affordance', () => {
  it('keeps add-card available after writing for revealed columns', () => {
    render(<BoardColumns board={board('discussion')} query={{}} />);
    expect(screen.getByRole('link', { name: 'add went well card' })).toBeInTheDocument();
  });

  it('hides add-card while a per-column discussion column is unrevealed', () => {
    render(<BoardColumns board={board('discussion', null)} query={{}} />);
    expect(screen.queryByRole('link', { name: 'add went well card' })).toBeNull();
    expect(screen.getByText('0 cards hidden')).toBeInTheDocument();
  });

  it('does not expose unrevealed discussion columns as drop targets', () => {
    const { container } = render(<BoardColumns board={board('discussion', null)} query={{}} />);
    expect(container.querySelector('[data-spill-column-id="col-1"]')).toBeNull();
  });
});
