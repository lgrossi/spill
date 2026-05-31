ALTER TABLE retros
  ADD COLUMN planned_for DATE NOT NULL DEFAULT CURRENT_DATE,
  ADD COLUMN happened_at TIMESTAMPTZ;

UPDATE retros
SET
  planned_for = COALESCE(completed_at::date, created_at::date, CURRENT_DATE),
  happened_at = CASE WHEN phase = 'completed' THEN completed_at ELSE NULL END;

ALTER TABLE retros DROP CONSTRAINT IF EXISTS retros_phase_check;
ALTER TABLE retros
  ADD CONSTRAINT retros_phase_check
  CHECK (phase IN ('scheduled', 'writing', 'discussion', 'voting', 'action_discussion', 'completed'));
