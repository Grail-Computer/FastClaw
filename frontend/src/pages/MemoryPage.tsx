import { useEffect, useState } from 'react';
import { api, type SessionData } from '../lib/api';

export function MemoryPage() {
  const [sessions, setSessions] = useState<SessionData[]>([]);
  const [error, setError] = useState('');

  const load = () => api.getMemory().then((d) => setSessions(d.sessions)).catch((e) => setError(e.message));
  useEffect(() => { load(); }, []);

  return (
    <>
      <h2>Memory</h2>
      <p className="section-desc">Active conversation sessions and their memory summaries.</p>

      {error && <div className="card" style={{ color: 'var(--red)' }}>Error: {error}</div>}

      <table>
        <thead>
          <tr><th>Conversation Key</th><th>Thread ID</th><th>Summary</th><th>Last Used</th><th>Actions</th></tr>
        </thead>
        <tbody>
          {sessions.map((s) => (
            <tr key={s.conversation_key}>
              <td style={{ fontFamily: 'var(--mono)', fontSize: 12 }}>{s.conversation_key}</td>
              <td style={{ fontFamily: 'var(--mono)', fontSize: 12 }}>{s.codex_thread_id || '—'}</td>
              <td style={{ maxWidth: 300, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', fontSize: 13 }}>{s.memory_summary || '—'}</td>
              <td style={{ fontSize: 12, color: 'var(--text-secondary)' }}>{s.last_used_at}</td>
              <td>
                <button className="btn btn-sm btn-danger" onClick={() => { api.clearMemory(s.conversation_key).then(load); }}>Clear</button>
              </td>
            </tr>
          ))}
          {sessions.length === 0 && (
            <tr><td colSpan={5} style={{ textAlign: 'center', color: 'var(--text-tertiary)', padding: 32 }}>No sessions</td></tr>
          )}
        </tbody>
      </table>
    </>
  );
}
