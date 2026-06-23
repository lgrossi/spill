-- Board-level "anonymous authors" toggle. When TRUE, the read model
-- redacts author_participant_id on every card except the caller's own.
-- Default FALSE preserves the existing named behavior.
ALTER TABLE retros
    ADD COLUMN anonymous_authors BOOLEAN NOT NULL DEFAULT FALSE;
