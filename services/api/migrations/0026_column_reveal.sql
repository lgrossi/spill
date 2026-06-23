-- Per-column reveal: tracks when each column moved from "drafts hidden" to
-- "everyone can see it" so the host can reveal one column at a time during
-- a writing-phase retro. Cards still flip state='draft'->'revealed' in the
-- same step (see voting.rs::reveal_column); the column flag drives the
-- composer lock (no new drafts into a revealed column) and the UI badge.
ALTER TABLE retro_columns
    ADD COLUMN revealed_at TIMESTAMPTZ NULL;

-- Backfill: any column on a retro past 'writing' is implicitly revealed --
-- its cards are already state='revealed'. Use the retro's happened_at when
-- set so the timestamp is plausibly historical for any future audit.
UPDATE retro_columns rc
SET revealed_at = COALESCE(r.happened_at, NOW())
FROM retros r
WHERE r.id = rc.retro_id
  AND r.phase IN ('discussion', 'voting', 'completed');
