-- Guardrails and human-in-the-loop approvals.

ALTER TABLE settings ADD COLUMN agent_name TEXT NOT NULL DEFAULT 'Grail';
ALTER TABLE settings ADD COLUMN role_description TEXT NOT NULL DEFAULT '';

-- Command approvals:
-- - auto: approve (subject to permissions_mode + cwd constraints)
-- - guardrails: approve unless a matching guardrail requires approval/denies
-- - always_ask: ask approval for every command
ALTER TABLE settings ADD COLUMN command_approval_mode TEXT NOT NULL DEFAULT 'guardrails';

-- If enabled, automatically apply *tightening* guardrails proposed by the agent (i.e. action != 'allow').
ALTER TABLE settings ADD COLUMN auto_apply_guardrail_tighten INTEGER NOT NULL DEFAULT 0;

-- Optional web access restrictions enforced inside grail-web-mcp.
ALTER TABLE settings ADD COLUMN web_allow_domains TEXT NOT NULL DEFAULT '';
ALTER TABLE settings ADD COLUMN web_deny_domains TEXT NOT NULL DEFAULT '';

-- Cron job execution mode:
-- - agent: enqueue a normal task that runs through Codex
-- - message: post prompt_text directly to Slack (no Codex usage)
ALTER TABLE cron_jobs ADD COLUMN mode TEXT NOT NULL DEFAULT 'agent';

CREATE TABLE IF NOT EXISTS guardrail_rules (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,          -- command | web_fetch | ...
  pattern_kind TEXT NOT NULL,  -- regex | exact | substring
  pattern TEXT NOT NULL,
  action TEXT NOT NULL,        -- allow | require_approval | deny
  priority INTEGER NOT NULL DEFAULT 100,
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS guardrail_rules_kind_enabled_priority_idx
  ON guardrail_rules(kind, enabled, priority);

CREATE TABLE IF NOT EXISTS approvals (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,          -- command_execution | guardrail_rule_add | cron_job_add
  status TEXT NOT NULL,        -- pending | approved | denied | expired
  decision TEXT,               -- approve | deny | always
  workspace_id TEXT,
  channel_id TEXT,
  thread_ts TEXT,
  requested_by_user_id TEXT,
  details_json TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  resolved_at INTEGER
);

CREATE INDEX IF NOT EXISTS approvals_status_created_at_idx
  ON approvals(status, created_at);

CREATE TABLE IF NOT EXISTS runtime_state (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  active_task_id INTEGER,
  active_task_started_at INTEGER,
  updated_at INTEGER NOT NULL
);

INSERT INTO runtime_state (id, updated_at)
  VALUES (1, unixepoch())
  ON CONFLICT(id) DO NOTHING;

-- Seed a minimal set of production-safe guardrails.
-- These are intentionally "require_approval" so the user can override.
INSERT INTO guardrail_rules (id, name, kind, pattern_kind, pattern, action, priority, enabled, created_at, updated_at)
  VALUES ('cmd_req_approval_rm_rf', 'rm -rf', 'command', 'regex', '(?i)\\brm\\b.*\\s-rf\\b', 'require_approval', 10, 1, unixepoch(), unixepoch())
  ON CONFLICT(id) DO NOTHING;
INSERT INTO guardrail_rules (id, name, kind, pattern_kind, pattern, action, priority, enabled, created_at, updated_at)
  VALUES ('cmd_req_approval_sudo', 'sudo', 'command', 'regex', '(?i)\\bsudo\\b', 'require_approval', 11, 1, unixepoch(), unixepoch())
  ON CONFLICT(id) DO NOTHING;
INSERT INTO guardrail_rules (id, name, kind, pattern_kind, pattern, action, priority, enabled, created_at, updated_at)
  VALUES ('cmd_req_approval_chmod_chown', 'chmod/chown', 'command', 'regex', '(?i)\\b(chmod|chown)\\b', 'require_approval', 12, 1, unixepoch(), unixepoch())
  ON CONFLICT(id) DO NOTHING;
INSERT INTO guardrail_rules (id, name, kind, pattern_kind, pattern, action, priority, enabled, created_at, updated_at)
  VALUES ('cmd_req_approval_pkg_mgr', 'package manager', 'command', 'regex', '(?i)\\b(apt-get|apt|yum|dnf|apk|brew)\\b', 'require_approval', 13, 1, unixepoch(), unixepoch())
  ON CONFLICT(id) DO NOTHING;
INSERT INTO guardrail_rules (id, name, kind, pattern_kind, pattern, action, priority, enabled, created_at, updated_at)
  VALUES ('cmd_req_approval_network_tools', 'network tools', 'command', 'regex', '(?i)\\b(curl|wget|nc|ncat|telnet|ssh|scp)\\b', 'require_approval', 14, 1, unixepoch(), unixepoch())
  ON CONFLICT(id) DO NOTHING;
INSERT INTO guardrail_rules (id, name, kind, pattern_kind, pattern, action, priority, enabled, created_at, updated_at)
  VALUES ('cmd_req_approval_shell_c', 'shell -c', 'command', 'regex', '(?i)\\b(bash|sh|zsh)\\b\\s+-c\\b', 'require_approval', 15, 1, unixepoch(), unixepoch())
  ON CONFLICT(id) DO NOTHING;
