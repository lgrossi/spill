CREATE TABLE board_grants (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    retro_id        UUID NOT NULL REFERENCES retros(id) ON DELETE CASCADE,
    principal_email TEXT NOT NULL CHECK (btrim(principal_email) <> '' AND principal_email = lower(principal_email)),
    role            TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('host', 'member')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (retro_id, principal_email)
);

CREATE INDEX board_grants_retro_idx ON board_grants (retro_id);
