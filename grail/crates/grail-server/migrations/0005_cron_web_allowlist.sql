-- Add single-workspace pinning and Slack allowlist.
ALTER TABLE settings ADD COLUMN workspace_id TEXT;
ALTER TABLE settings ADD COLUMN slack_allow_from TEXT NOT NULL DEFAULT '';

-- Optional MCP servers.
ALTER TABLE settings ADD COLUMN allow_web_mcp INTEGER NOT NULL DEFAULT 0;

-- Scheduled tasks (cron).
ALTER TABLE settings ADD COLUMN allow_cron INTEGER NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS cron_jobs (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,

  schedule_kind TEXT NOT NULL, -- every | cron | at
  every_seconds INTEGER,
  cron_expr TEXT,
  at_ts INTEGER,

  -- Single-tenant: tie jobs to the Slack workspace we are connected to.
  workspace_id TEXT NOT NULL,
  -- Slack delivery target. If thread_ts is empty, the job posts into the channel (no thread).
  channel_id TEXT NOT NULL,
  thread_ts TEXT NOT NULL DEFAULT '',

  prompt_text TEXT NOT NULL,

  next_run_at INTEGER,
  last_run_at INTEGER,
  last_status TEXT,
  last_error TEXT,

  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS cron_jobs_enabled_next_run_at_idx
  ON cron_jobs(enabled, next_run_at);

