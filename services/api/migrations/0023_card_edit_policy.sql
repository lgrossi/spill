ALTER TABLE retros
    ADD COLUMN card_edit_policy TEXT NOT NULL DEFAULT 'collaborative'
        CHECK (card_edit_policy IN ('collaborative', 'author_only'));
