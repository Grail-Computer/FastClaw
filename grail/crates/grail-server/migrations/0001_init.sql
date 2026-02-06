PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS settings (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  context_last_n INTEGER NOT NULL DEFAULT 20,
  model TEXT,
  reasoning_effort TEXT,
  reasoning_summary TEXT,
  permissions_mode TEXT NOT NULL DEFAULT 'read',
  allow_slack_mcp INTEGER NOT NULL DEFAULT 1,
  allow_context_writes INTEGER NOT NULL DEFAULT 1,
  updated_at INTEGER NOT NULL
);

INSERT INTO settings (id, updated_at)
  VALUES (1, unixepoch())
  ON CONFLICT(id) DO NOTHING;

CREATE TABLE IF NOT EXISTS processed_events (
  workspace_id TEXT NOT NULL,
  event_id TEXT NOT NULL,
  processed_at INTEGER NOT NULL,
  PRIMARY KEY (workspace_id, event_id)
);

CREATE TABLE IF NOT EXISTS tasks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  status TEXT NOT NULL,
  workspace_id TEXT NOT NULL,
  channel_id TEXT NOT NULL,
  thread_ts TEXT NOT NULL,
  event_ts TEXT NOT NULL,
  requested_by_user_id TEXT NOT NULL,
  prompt_text TEXT NOT NULL,
  result_text TEXT,
  error_text TEXT,
  created_at INTEGER NOT NULL,
  started_at INTEGER,
  finished_at INTEGER
);

CREATE INDEX IF NOT EXISTS tasks_status_created_at_idx
  ON tasks(status, created_at);

CREATE TABLE IF NOT EXISTS sessions (
  conversation_key TEXT PRIMARY KEY,
  codex_thread_id TEXT,
  memory_summary TEXT NOT NULL DEFAULT '',
  last_used_at INTEGER NOT NULL
);
