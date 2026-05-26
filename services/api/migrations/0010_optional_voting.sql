ALTER TABLE retros DROP CONSTRAINT IF EXISTS retros_vote_limit_check;
ALTER TABLE retros ADD CONSTRAINT retros_vote_limit_check CHECK (vote_limit >= 0);
