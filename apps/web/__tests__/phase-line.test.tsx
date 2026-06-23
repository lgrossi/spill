import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { render, screen } from '@testing-library/react';

vi.mock('../app/lib/actions', () => ({
  autoAdvanceAction: vi.fn(),
  completeRetroAction: vi.fn(),
  forceRevealRetroAction: vi.fn(),
  startActionDiscussionAction: vi.fn(),
  startVotingAction: vi.fn(),
  applyClusteringAction: vi.fn(),
  retryClusteringAction: vi.fn(),
}));

import { PhaseLine } from '../app/retros/[retroId]/phase-line';

function renderDiscussion(status: string, isHost: boolean) {
  render(
    <PhaseLine
      retroId="r1"
      phase="discussion"
      clusteringMode="auto_on_vote_start"
      clusteringStatus={status}
      isHost={isHost}
      participantCount={3}
      readyCount={0}
      allReady={false}
      revealMode="big_bang"
    />,
  );
}

describe('PhaseLine discussion-phase organization', () => {
  it('host can apply a ready proposal and still start voting', () => {
    renderDiscussion('ready', true);
    expect(screen.getByRole('button', { name: 'apply organizing' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'start voting' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'retry organizing' })).toBeNull();
  });

  it('host can retry a failed proposal', () => {
    renderDiscussion('failed', true);
    expect(screen.getByRole('button', { name: 'retry organizing' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'start voting' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'apply organizing' })).toBeNull();
  });

  it('host sees a computing indicator while organizing', () => {
    renderDiscussion('computing', true);
    expect(screen.getByText('Organizing…')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'start voting' })).toBeInTheDocument();
  });

  it('non-host sees status text but no host controls', () => {
    renderDiscussion('ready', false);
    expect(screen.getByText('organized')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'apply organizing' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'start voting' })).toBeNull();
  });
});
