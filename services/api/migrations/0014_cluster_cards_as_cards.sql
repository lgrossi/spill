ALTER TABLE cards ADD COLUMN IF NOT EXISTS parent_card_id UUID REFERENCES cards(id) ON DELETE SET NULL;
ALTER TABLE cards ADD COLUMN IF NOT EXISTS cluster_details TEXT;
CREATE INDEX IF NOT EXISTS cards_parent_card_idx ON cards (parent_card_id);
