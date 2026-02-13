-- Long-term observational memory (Mastra-style OM: observer + reflector).

CREATE TABLE IF NOT EXISTS observational_memory (
  memory_key TEXT PRIMARY KEY,
  scope TEXT NOT NULL, -- thread | resource
  observation_log TEXT NOT NULL DEFAULT '',
  reflection_summary TEXT NOT NULL DEFAULT '',
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS observational_memory_scope_updated_at_idx
  ON observational_memory(scope, updated_at);

