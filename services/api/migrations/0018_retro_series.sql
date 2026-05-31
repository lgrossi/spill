CREATE TABLE retro_groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL CHECK (btrim(name) <> ''),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE retros
    ADD COLUMN group_id UUID REFERENCES retro_groups(id) ON DELETE SET NULL,
    ADD COLUMN previous_retro_id UUID REFERENCES retros(id) ON DELETE SET NULL;

CREATE UNIQUE INDEX retros_previous_retro_id_unique
    ON retros(previous_retro_id)
    WHERE previous_retro_id IS NOT NULL;
