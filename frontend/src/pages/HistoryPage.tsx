import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { api } from '../api/client';

export function HistoryPage() {
  const [jobs, setJobs] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api.getHistory().then(data => {
      setJobs(data.jobs || []);
      setLoading(false);
    }).catch(() => setLoading(false));
  }, []);

  return (
    <div className="max-w-4xl mx-auto p-4">
      <h1 className="text-xl font-bold mb-4">Solve History</h1>

      {loading ? (
        <div style={{ color: 'var(--text-secondary)' }}>Loading...</div>
      ) : jobs.length === 0 ? (
        <div style={{ color: 'var(--text-secondary)' }}>
          No solves yet. <Link to="/app/solve" style={{ color: 'var(--accent)' }}>Create your first solve</Link>
        </div>
      ) : (
        <table className="w-full text-sm">
          <thead>
            <tr style={{ color: 'var(--text-secondary)', borderBottom: '1px solid #333' }}>
              <th className="text-left py-2">Date</th>
              <th className="text-left py-2">Status</th>
              <th className="text-left py-2">Iterations</th>
              <th className="text-left py-2">Exploitability</th>
              <th className="text-left py-2">Time</th>
            </tr>
          </thead>
          <tbody>
            {jobs.map(job => (
              <tr key={job.id} style={{ borderBottom: '1px solid #222' }}>
                <td className="py-2">{new Date(job.created_at).toLocaleString()}</td>
                <td className="py-2">
                  <span className="px-2 py-0.5 rounded text-xs" style={{
                    background: job.status === 'completed' ? 'var(--success)' :
                      job.status === 'failed' ? 'var(--danger)' : 'var(--warning)',
                    color: '#fff',
                  }}>
                    {job.status}
                  </span>
                </td>
                <td className="py-2">{job.iterations || '-'}</td>
                <td className="py-2">{job.exploitability ? `${(job.exploitability * 100).toFixed(2)}%` : '-'}</td>
                <td className="py-2">{job.elapsed_seconds ? `${job.elapsed_seconds.toFixed(1)}s` : '-'}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
