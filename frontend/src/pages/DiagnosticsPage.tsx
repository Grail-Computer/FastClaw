import { useState } from 'react';
import { api, type DiagnosticsData } from '../lib/api';

export function DiagnosticsPage() {
  const [data, setData] = useState<DiagnosticsData | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState('');

  const runTest = async () => {
    setRunning(true);
    setError('');
    try {
      const result = await api.runCodexTest();
      setData(result);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed');
    }
    setRunning(false);
  };

  return (
    <>
      <h2>Diagnostics</h2>
      <p className="section-desc">Run diagnostic tests against the Codex backend.</p>
      <div className="card">
        <div className="card-title">Codex Test</div>
        <p style={{ color: 'var(--text-secondary)', fontSize: 13, marginBottom: 16 }}>
          Run a basic Codex API call to verify the agent can communicate with the backend.
        </p>
        <button className="btn btn-primary" onClick={runTest} disabled={running}>
          {running ? 'Running…' : 'Run Codex Test'}
        </button>
      </div>
      {error && <div className="card" style={{ color: 'var(--red)' }}>Error: {error}</div>}
      {data && (
        <div className="card">
          <div className="card-title">Result</div>
          {data.codex_error ? (
            <pre style={{ color: 'var(--red)', fontFamily: 'var(--mono)', fontSize: 13, whiteSpace: 'pre-wrap' }}>{data.codex_error}</pre>
          ) : (
            <pre style={{ fontFamily: 'var(--mono)', fontSize: 13, whiteSpace: 'pre-wrap' }}>{data.codex_result || 'No output'}</pre>
          )}
        </div>
      )}
    </>
  );
}
