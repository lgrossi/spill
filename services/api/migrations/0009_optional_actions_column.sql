ALTER TABLE retros DROP CONSTRAINT IF EXISTS retros_action_discussion_limit_check;
ALTER TABLE retros ADD CONSTRAINT retros_action_discussion_limit_check CHECK (action_discussion_limit >= 0);
