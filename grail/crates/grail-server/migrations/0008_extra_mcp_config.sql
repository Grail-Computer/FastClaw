-- Allow advanced users to extend Codex MCP server config without code changes.
ALTER TABLE settings ADD COLUMN extra_mcp_config TEXT NOT NULL DEFAULT '';

