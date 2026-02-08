ALTER TABLE settings ADD COLUMN slack_proactive_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE settings ADD COLUMN slack_proactive_snippet TEXT NOT NULL DEFAULT '';

ALTER TABLE tasks ADD COLUMN is_proactive INTEGER NOT NULL DEFAULT 0;

