-- Manually added actions used to live only as cards in the actions column with
-- no action_item, so they never showed up as open actions and could not be
-- completed. Actions are now a single concept (action_items), exactly like the
-- auto top-voted cards. Backfill a confirmed action_item for every revealed,
-- top-level actions-column card that does not have one yet.
INSERT INTO action_items (retro_id, source_card_id, title, status, position)
SELECT
    c.retro_id,
    c.id,
    COALESCE(NULLIF(btrim(c.body_text), ''), c.gif_alt_text, 'Untitled action'),
    'confirmed',
    COALESCE(
        (SELECT MAX(ai2.position) FROM action_items ai2 WHERE ai2.retro_id = c.retro_id),
        -1
    ) + ROW_NUMBER() OVER (PARTITION BY c.retro_id ORDER BY c.position, c.created_at)
FROM cards c
JOIN retro_columns col ON col.id = c.column_id
WHERE (col.column_key = 'actions' OR lower(col.title) LIKE '%action%')
  AND c.state = 'revealed'
  AND c.parent_card_id IS NULL
  AND NOT EXISTS (
      SELECT 1 FROM action_items ai
      WHERE ai.retro_id = c.retro_id AND ai.source_card_id = c.id
  );
