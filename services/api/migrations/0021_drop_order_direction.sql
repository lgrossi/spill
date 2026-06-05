-- order_direction was never applied to any ordering and always defaulted to
-- 'chronological'. Phase-aware ordering now lives in cards.position, so the
-- column is dead weight.
ALTER TABLE retro_columns DROP COLUMN order_direction;
