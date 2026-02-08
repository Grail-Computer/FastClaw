-- Optional Slack channel allow list (Coworker-style enabled channels).
-- If non-empty, Grail will only respond in these channel IDs (DMs are still allowed).
ALTER TABLE settings ADD COLUMN slack_allow_channels TEXT NOT NULL DEFAULT '';

