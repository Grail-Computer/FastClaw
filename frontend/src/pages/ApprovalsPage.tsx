import { useEffect, useState } from 'react';
import { api, type ApprovalData } from '../lib/api';

export function ApprovalsPage() {
  const [approvals, setApprovals] = useState<ApprovalData[]>([]);
  const [error, setError] = useState('');

  const load = () => api.getApprovals().then((d) => setApprovals(d.approvals)).catch((e) => setError(e.message));
  useEffect(() => { load(); }, []);

  return (
    <>
      <h2>Approvals</h2>
      <p className="section-desc">Pending command approvals from the agent.</p>

      {error && <div className="card" style={{ color: 'var(--red)' }}>Error: {error}</div>}

      <table>
        <thead>
          <tr><th>ID</th><th>Kind</th><th>Status</th><th>Details</th><th>Created</th><th>Actions</th></tr>
        </thead>
        <tbody>
          {approvals.map((a) => (
            <tr key={a.id}>
              <td style={{ fontFamily: 'var(--mono)', fontSize: 12 }}>{a.id.slice(0, 8)}</td>
              <td>{a.kind}</td>
              <td>
                <span className={`pill ${a.status === 'pending' ? '' : a.decision === 'approve' ? 'pill-ok' : 'pill-bad'}`}>
                  <span className="pill-dot" />{a.decision || a.status}
                </span>
              </td>
              <td style={{ maxWidth: 300, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', fontSize: 12, fontFamily: 'var(--mono)' }}>{a.details}</td>
              <td style={{ fontSize: 12, color: 'var(--text-secondary)' }}>{a.created_at}</td>
              <td>
                {a.status === 'pending' && (
                  <div style={{ display: 'flex', gap: 4 }}>
                    <button className="btn btn-sm" style={{ color: 'var(--green)' }} onClick={() => { api.approveApproval(a.id).then(load); }}>Approve</button>
                    <button className="btn btn-sm" onClick={() => { api.alwaysApproval(a.id).then(load); }}>Always</button>
                    <button className="btn btn-sm btn-danger" onClick={() => { api.denyApproval(a.id).then(load); }}>Deny</button>
                  </div>
                )}
              </td>
            </tr>
          ))}
          {approvals.length === 0 && (
            <tr><td colSpan={6} style={{ textAlign: 'center', color: 'var(--text-tertiary)', padding: 32 }}>No approvals</td></tr>
          )}
        </tbody>
      </table>
    </>
  );
}
