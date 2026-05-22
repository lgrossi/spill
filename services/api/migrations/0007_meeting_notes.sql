CREATE TABLE meeting_notes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    retro_id UUID NOT NULL REFERENCES retros(id) ON DELETE CASCADE,
    author_participant_id UUID NOT NULL REFERENCES participants(id) ON DELETE CASCADE,
    title TEXT NOT NULL DEFAULT 'Meeting notes' CHECK (btrim(title) <> ''),
    body_text TEXT NOT NULL CHECK (btrim(body_text) <> ''),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX meeting_notes_retro_created_idx ON meeting_notes (retro_id, created_at DESC);
