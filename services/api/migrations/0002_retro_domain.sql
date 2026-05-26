CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE retros (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(), title TEXT NOT NULL CHECK (btrim(title) <> ''),
    phase TEXT NOT NULL DEFAULT 'writing' CHECK (phase IN ('writing', 'discussion', 'voting', 'action_discussion', 'completed')),
    vote_limit INTEGER NOT NULL DEFAULT 3 CHECK (vote_limit >= 0), action_discussion_limit INTEGER NOT NULL DEFAULT 3 CHECK (action_discussion_limit >= 0),
    clustering_mode TEXT NOT NULL DEFAULT 'manual' CHECK (clustering_mode IN ('manual', 'auto_on_vote_start', 'disabled')),
    clustering_status TEXT NOT NULL DEFAULT 'not_run' CHECK (clustering_status IN ('not_run', 'running', 'completed', 'failed')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), completed_at TIMESTAMPTZ
);

CREATE TABLE participants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(), retro_id UUID NOT NULL REFERENCES retros(id) ON DELETE CASCADE,
    external_subject TEXT, display_name TEXT NOT NULL CHECK (btrim(display_name) <> ''), role TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('host', 'member')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), UNIQUE (retro_id, external_subject)
);

CREATE TABLE retro_columns (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(), retro_id UUID NOT NULL REFERENCES retros(id) ON DELETE CASCADE,
    column_key TEXT NOT NULL CHECK (btrim(column_key) <> ''), title TEXT NOT NULL CHECK (btrim(title) <> ''),
    position INTEGER NOT NULL CHECK (position >= 0), order_direction TEXT NOT NULL DEFAULT 'chronological' CHECK (order_direction IN ('chronological', 'reverse_chronological')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), UNIQUE (retro_id, column_key), UNIQUE (retro_id, position)
);

CREATE TABLE participant_ready_marks (
    participant_id UUID NOT NULL REFERENCES participants(id) ON DELETE CASCADE, retro_id UUID NOT NULL REFERENCES retros(id) ON DELETE CASCADE,
    phase TEXT NOT NULL CHECK (phase IN ('writing', 'voting')), ready_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), PRIMARY KEY (participant_id, phase)
);

CREATE TABLE card_clusters (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(), retro_id UUID NOT NULL REFERENCES retros(id) ON DELETE CASCADE,
    title TEXT CHECK (title IS NULL OR btrim(title) <> ''), created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE cards (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(), retro_id UUID NOT NULL REFERENCES retros(id) ON DELETE CASCADE,
    column_id UUID NOT NULL REFERENCES retro_columns(id) ON DELETE CASCADE, author_participant_id UUID NOT NULL REFERENCES participants(id) ON DELETE CASCADE,
    cluster_id UUID REFERENCES card_clusters(id) ON DELETE SET NULL, body_text TEXT, gif_url TEXT, gif_alt_text TEXT,
    state TEXT NOT NULL DEFAULT 'draft' CHECK (state IN ('draft', 'revealed')), author_visibility TEXT NOT NULL DEFAULT 'named' CHECK (author_visibility IN ('named', 'anonymous')),
    position INTEGER NOT NULL DEFAULT 0 CHECK (position >= 0), created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK ((body_text IS NOT NULL AND btrim(body_text) <> '') OR (gif_url IS NOT NULL AND btrim(gif_url) <> ''))
);

CREATE TABLE votes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(), retro_id UUID NOT NULL REFERENCES retros(id) ON DELETE CASCADE,
    participant_id UUID NOT NULL REFERENCES participants(id) ON DELETE CASCADE, target_card_id UUID REFERENCES cards(id) ON DELETE CASCADE,
    target_cluster_id UUID REFERENCES card_clusters(id) ON DELETE CASCADE, count INTEGER NOT NULL DEFAULT 1 CHECK (count > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), CHECK ((target_card_id IS NOT NULL)::INTEGER + (target_cluster_id IS NOT NULL)::INTEGER = 1)
);

CREATE TABLE action_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(), retro_id UUID NOT NULL REFERENCES retros(id) ON DELETE CASCADE,
    source_card_id UUID REFERENCES cards(id) ON DELETE SET NULL, source_cluster_id UUID REFERENCES card_clusters(id) ON DELETE SET NULL,
    title TEXT NOT NULL CHECK (btrim(title) <> ''), details TEXT, status TEXT NOT NULL DEFAULT 'proposed' CHECK (status IN ('proposed', 'confirmed', 'rejected')),
    position INTEGER NOT NULL DEFAULT 0 CHECK (position >= 0), created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), confirmed_at TIMESTAMPTZ
);

CREATE TABLE ingested_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(), recipient_participant_id UUID NOT NULL REFERENCES participants(id) ON DELETE CASCADE,
    retro_id UUID REFERENCES retros(id) ON DELETE CASCADE, source TEXT NOT NULL CHECK (source IN ('pi', 'claude_code', 'upload', 'other')),
    placement TEXT NOT NULL CHECK (placement IN ('user_deck', 'retro_draft')), target_column_id UUID REFERENCES retro_columns(id) ON DELETE SET NULL,
    suggested_text TEXT, gif_url TEXT, raw_payload JSONB NOT NULL DEFAULT '{}'::JSONB, status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'accepted', 'dismissed')),
    accepted_card_id UUID REFERENCES cards(id) ON DELETE SET NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), CHECK (placement = 'user_deck' OR retro_id IS NOT NULL),
    CHECK ((suggested_text IS NOT NULL AND btrim(suggested_text) <> '') OR (gif_url IS NOT NULL AND btrim(gif_url) <> ''))
);

CREATE TABLE ai_artifacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(), retro_id UUID NOT NULL REFERENCES retros(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('gif_suggestions', 'clustering', 'action_suggestions', 'summary', 'mood', 'tagging')),
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'running', 'succeeded', 'failed')),
    input JSONB NOT NULL DEFAULT '{}'::JSONB, output JSONB, error_message TEXT, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX participant_ready_marks_retro_phase_idx ON participant_ready_marks (retro_id, phase);
CREATE INDEX cards_retro_column_idx ON cards (retro_id, column_id, state);
CREATE INDEX votes_retro_participant_idx ON votes (retro_id, participant_id);
CREATE INDEX ingested_items_recipient_status_idx ON ingested_items (recipient_participant_id, status);
CREATE INDEX ai_artifacts_retro_kind_status_idx ON ai_artifacts (retro_id, kind, status);
