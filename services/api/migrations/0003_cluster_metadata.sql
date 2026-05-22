ALTER TABLE card_clusters
    ADD COLUMN category TEXT CHECK (category IS NULL OR btrim(category) <> ''),
    ADD COLUMN tags JSONB NOT NULL DEFAULT '[]'::JSONB;
