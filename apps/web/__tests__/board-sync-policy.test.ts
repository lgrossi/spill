import { describe, it, expect } from 'vitest';
import { shouldRefreshBoard } from '../app/retros/[retroId]/board-sync-policy';

describe('shouldRefreshBoard', () => {
  it.each(['board_snapshot', 'card_changed', 'ready_changed', 'phase_changed', 'clustering_changed'])(
    'returns true for event type "%s"',
    (type) => {
      expect(shouldRefreshBoard({ type })).toBe(true);
    },
  );

  it.each(['user_joined', 'heartbeat', 'unknown_event', ''])(
    'returns false for event type "%s"',
    (type) => {
      expect(shouldRefreshBoard({ type })).toBe(false);
    },
  );

  it('returns false when type is undefined', () => {
    expect(shouldRefreshBoard({})).toBe(false);
  });
});
