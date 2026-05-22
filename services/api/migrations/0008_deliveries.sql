CREATE TABLE deliveries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    retro_id UUID NOT NULL REFERENCES retros(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('summary_export', 'external_action_link')),
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'succeeded', 'failed')),
    output JSONB,
    error_message TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0 CHECK (retry_count >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX deliveries_retro_status_idx ON deliveries (retro_id, status);
