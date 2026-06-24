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
    />,
  );
}

function renderPhase(phase: string, isHost: boolean, allReady: boolean) {
  render(
    <PhaseLine
      retroId="r1"
      phase={phase}
      clusteringMode="manual"
      clusteringStatus="idle"
      isHost={isHost}
      participantCount={3}
      readyCount={allReady ? 3 : 2}
      allReady={allReady}
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

describe('PhaseLine auto-advance countdown', () => {
  it('renders countdown only for hosts when everyone is ready', () => {
    renderPhase('voting', true, true);
    expect(screen.getByText(/advancing in/)).toBeInTheDocument();
    expect(screen.queryByText('all ready')).toBeNull();
  });

  it('does not submit host-only auto-advance from non-host tabs', () => {
    renderPhase('voting', false, true);
    expect(screen.getByText('all ready')).toBeInTheDocument();
    expect(screen.queryByText(/advancing in/)).toBeNull();
  });
});
