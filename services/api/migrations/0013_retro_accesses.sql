CREATE TABLE retro_accesses (
    retro_id UUID NOT NULL REFERENCES retros(id) ON DELETE CASCADE,
    participant_id UUID NOT NULL REFERENCES participants(id) ON DELETE CASCADE,
    opened_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (retro_id, participant_id)
);

CREATE INDEX retro_accesses_participant_opened_idx ON retro_accesses (participant_id, opened_at DESC);
