-- Per-board reveal mode: 'per_column' (host walks one column at a time;
-- last reveal auto-advances writing -> discussion) or 'big_bang' (the
-- legacy single "start discussing" action reveals everything at once).
-- The UI gates the affordances; the backend stays permissive on both
-- routes so tooling/CLI keeps working regardless of mode.
--
-- Default for the column is 'big_bang' so every retro that existed before
-- this migration keeps doing exactly what it did. New retros default to
-- 'per_column' from the API layer (the create form ships checked).
ALTER TABLE retros
    ADD COLUMN reveal_mode TEXT NOT NULL DEFAULT 'big_bang'
        CHECK (reveal_mode IN ('per_column', 'big_bang'));
