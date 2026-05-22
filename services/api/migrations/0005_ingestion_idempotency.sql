ALTER TABLE ingested_items
    ADD COLUMN idempotency_key TEXT CHECK (idempotency_key IS NULL OR btrim(idempotency_key) <> ''),
    ADD COLUMN source_metadata JSONB NOT NULL DEFAULT '{}'::JSONB;

CREATE UNIQUE INDEX ingested_items_recipient_source_idempotency_idx
    ON ingested_items (recipient_participant_id, source, idempotency_key)
    WHERE idempotency_key IS NOT NULL;
