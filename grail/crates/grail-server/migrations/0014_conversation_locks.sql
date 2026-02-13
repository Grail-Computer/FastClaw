-- Support concurrent task processing across conversations while keeping each
-- conversation strictly sequential.

-- Persist the computed conversation_key on tasks so we can lock/claim by it.
ALTER TABLE tasks ADD COLUMN conversation_key TEXT NOT NULL DEFAULT '';

-- Backfill conversation_key for existing rows.
UPDATE tasks
SET conversation_key = CASE
  WHEN is_proactive != 0 AND thread_ts != '' THEN workspace_id || ':' || channel_id || ':thread:' || thread_ts
  WHEN thread_ts != '' AND thread_ts != event_ts THEN workspace_id || ':' || channel_id || ':thread:' || thread_ts
  ELSE workspace_id || ':' || channel_id || ':main'
END
WHERE conversation_key = '';

CREATE INDEX IF NOT EXISTS tasks_status_conversation_created_at_idx
  ON tasks(status, conversation_key, created_at);

-- Per-conversation lease lock. Prevents concurrent turns within the same
-- conversation when worker concurrency > 1.
CREATE TABLE IF NOT EXISTS conversation_locks (
  conversation_key TEXT PRIMARY KEY,
  owner_id TEXT NOT NULL,
  lease_until INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS conversation_locks_lease_until_idx
  ON conversation_locks(lease_until);

-- Track multiple active tasks at once (for status UI / API).
CREATE TABLE IF NOT EXISTS runtime_active_tasks (
  task_id INTEGER PRIMARY KEY,
  started_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS runtime_active_tasks_started_at_idx
  ON runtime_active_tasks(started_at);

