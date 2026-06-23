-- Per-participant "I'm not in this round" toggle. When FALSE the
-- participant is excluded from ready/reveal gating (their existing cards
-- and votes stay intact). Defaults to TRUE for every existing row.
ALTER TABLE participants
    ADD COLUMN is_participating BOOLEAN NOT NULL DEFAULT TRUE;
