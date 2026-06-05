-- Discussion-phase clustering introduces a proposal lifecycle:
-- not_run -> computing -> ready -> applied (+ failed). The legacy keyword
-- `cluster_board` path still uses running/completed, so keep them allowed.
ALTER TABLE retros DROP CONSTRAINT IF EXISTS retros_clustering_status_check;
ALTER TABLE retros ADD CONSTRAINT retros_clustering_status_check
    CHECK (clustering_status IN ('not_run', 'running', 'completed', 'failed', 'computing', 'ready', 'applied'));
