CREATE TABLE IF NOT EXISTS schema_migrations_marker (
    id INTEGER PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO schema_migrations_marker (id)
VALUES (1)
ON CONFLICT (id) DO NOTHING;
