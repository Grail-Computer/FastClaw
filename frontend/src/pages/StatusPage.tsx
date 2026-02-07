import { useEffect, useState } from 'react';
import { api, type StatusData } from '../lib/api';

export function StatusPage() {
  const [data, setData] = useState<StatusData | null>(null);
  const [error, setError] = useState('');

  useEffect(() => {
    api.getStatus().then(setData).catch((e) => setError(e.message));
  }, []);

  if (error) return <div className="card" style={{ color: 'var(--red)' }}>Error: {error}</div>;
  if (!data) return <div className="loading">Loading…</div>;

  const boolPill = (set: boolean) => (
    <span className={`pill ${set ? 'pill-ok' : 'pill-bad'}`}>
      <span className="pill-dot" />{set ? 'Set' : 'Not set'}
    </span>
  );

  return (
    <>
      <h2>Status</h2>
      <p className="section-desc">System health and integration status at a glance.</p>

      <div className="card">
        <div className="card-title">Integrations</div>
        <div className="kv-grid">
          <div className="kv-item">
            <div className="kv-label">Slack Signing Secret</div>
            <div className="kv-value">{boolPill(data.slack_signing_secret_set)}</div>
          </div>
          <div className="kv-item">
            <div className="kv-label">Slack Bot Token</div>
            <div className="kv-value">{boolPill(data.slack_bot_token_set)}</div>
          </div>
          <div className="kv-item">
            <div className="kv-label">Telegram Bot Token</div>
            <div className="kv-value">{boolPill(data.telegram_bot_token_set)}</div>
          </div>
          <div className="kv-item">
            <div className="kv-label">Telegram Webhook Secret</div>
            <div className="kv-value">{boolPill(data.telegram_webhook_secret_set)}</div>
          </div>
          <div className="kv-item">
            <div className="kv-label">OpenAI API Key</div>
            <div className="kv-value">{boolPill(data.openai_api_key_set)}</div>
          </div>
          <div className="kv-item">
            <div className="kv-label">Master Key</div>
            <div className="kv-value">{boolPill(data.master_key_set)}</div>
          </div>
        </div>
      </div>

      <div className="card">
        <div className="card-title">Worker</div>
        <div className="kv-grid">
          <div className="kv-item">
            <div className="kv-label">Queue Depth</div>
            <div className="kv-value">{data.queue_depth}</div>
          </div>
          <div className="kv-item">
            <div className="kv-label">Permissions</div>
            <div className="kv-value">{data.permissions_mode}</div>
          </div>
          <div className="kv-item">
            <div className="kv-label">Worker Lock</div>
            <div className="kv-value" style={{ fontSize: 12 }}>{data.worker_lock_owner || '—'}</div>
          </div>
          <div className="kv-item">
            <div className="kv-label">Active Task</div>
            <div className="kv-value">{data.active_task_id || '—'}</div>
          </div>
          <div className="kv-item">
            <div className="kv-label">Pending Approvals</div>
            <div className="kv-value">{data.pending_approvals}</div>
          </div>
          <div className="kv-item">
            <div className="kv-label">Guardrails Enabled</div>
            <div className="kv-value">{data.guardrails_enabled}</div>
          </div>
        </div>
      </div>

      <div className="card">
        <div className="card-title">Endpoints</div>
        <div className="kv-grid">
          <div className="kv-item">
            <div className="kv-label">Slack Events</div>
            <div className="kv-value" style={{ fontSize: 12 }}>{data.slack_events_url || '—'}</div>
          </div>
          <div className="kv-item">
            <div className="kv-label">Slack Actions</div>
            <div className="kv-value" style={{ fontSize: 12 }}>{data.slack_actions_url || '—'}</div>
          </div>
          <div className="kv-item">
            <div className="kv-label">Telegram Webhook</div>
            <div className="kv-value" style={{ fontSize: 12 }}>{data.telegram_webhook_url || '—'}</div>
          </div>
        </div>
      </div>
    </>
  );
}
