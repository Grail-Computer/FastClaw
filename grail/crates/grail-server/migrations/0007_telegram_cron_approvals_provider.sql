-- Telegram channel, agent cron approvals, and task provider tagging.

-- Telegram enable + allow list (nanobot-style allowFrom).
ALTER TABLE settings ADD COLUMN allow_telegram INTEGER NOT NULL DEFAULT 0;
ALTER TABLE settings ADD COLUMN telegram_allow_from TEXT NOT NULL DEFAULT '';

-- When false, agent-proposed cron jobs require an approval (recommended).
ALTER TABLE settings ADD COLUMN auto_apply_cron_jobs INTEGER NOT NULL DEFAULT 0;

-- Tag tasks with the originating provider (slack | telegram).
ALTER TABLE tasks ADD COLUMN provider TEXT NOT NULL DEFAULT 'slack';

-- Minimal local history buffer for Telegram chats (Telegram Bot API can't fetch history).
CREATE TABLE IF NOT EXISTS telegram_messages (
  chat_id TEXT NOT NULL,
  message_id INTEGER NOT NULL,
  from_user_id TEXT,
  is_bot INTEGER NOT NULL DEFAULT 0,
  text TEXT,
  ts INTEGER NOT NULL,
  PRIMARY KEY (chat_id, message_id)
);

CREATE INDEX IF NOT EXISTS telegram_messages_chat_id_message_id_idx
  ON telegram_messages(chat_id, message_id);

-- Additional safe-by-default command guardrails.
INSERT INTO guardrail_rules (id, name, kind, pattern_kind, pattern, action, priority, enabled, created_at, updated_at)
  VALUES ('cmd_req_approval_env', 'env/printenv', 'command', 'regex', '(?i)\\b(env|printenv)\\b', 'require_approval', 16, 1, unixepoch(), unixepoch())
  ON CONFLICT(id) DO NOTHING;
INSERT INTO guardrail_rules (id, name, kind, pattern_kind, pattern, action, priority, enabled, created_at, updated_at)
  VALUES ('cmd_req_approval_proc_environ', '/proc/*/environ', 'command', 'regex', '(?i)/proc/(self|1)/environ', 'require_approval', 17, 1, unixepoch(), unixepoch())
  ON CONFLICT(id) DO NOTHING;

