import { useEffect, useState } from 'react';
import { useParams, Link } from 'react-router-dom';

interface AggregateResult {
  board: string;
  result: {
    oop_ev?: number;
    ip_ev?: number;
    oop_equity?: number;
    ip_equity?: number;
  };
}

export function AggregateResultPage() {
  const { id } = useParams<{ id: string }>();
  const [data, setData] = useState<any>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [sortKey, setSortKey] = useState<string>('board');
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>('asc');

  useEffect(() => {
    if (!id) return;
    const poll = async () => {
      try {
        const res = await fetch(`/api/aggregate/${id}`, {
          headers: { 'Authorization': `Bearer ${localStorage.getItem('token')}` },
        });
        const result = await res.json();
        setData(result);
        if (result.status === 'running' || result.status === 'queued') {
          setTimeout(poll, 3000);
        } else {
          setLoading(false);
        }
      } catch (err: any) {
        setError(err.message);
        setLoading(false);
      }
    };
    poll();
  }, [id]);

  const handleSort = (key: string) => {
    if (sortKey === key) {
      setSortDir(d => d === 'asc' ? 'desc' : 'asc');
    } else {
      setSortKey(key);
      setSortDir('asc');
    }
  };

  const results: AggregateResult[] = data?.results || [];
  const sortedResults = [...results].sort((a, b) => {
    let va: any, vb: any;
    if (sortKey === 'board') { va = a.board; vb = b.board; }
    else { va = (a.result as any)[sortKey] ?? 0; vb = (b.result as any)[sortKey] ?? 0; }
    return sortDir === 'asc' ? (va > vb ? 1 : -1) : (va < vb ? 1 : -1);
  });

  const downloadCsv = () => {
    window.open(`/api/aggregate/${id}/csv?token=${localStorage.getItem('token')}`, '_blank');
  };

  if (error) {
    return (
      <div className="max-w-4xl mx-auto p-4">
        <div style={{ color: 'var(--danger)' }}>Error: {error}</div>
        <Link to="/app/aggregate" className="text-sm mt-3 block" style={{ color: 'var(--accent)' }}>
          Back to Aggregate
        </Link>
      </div>
    );
  }

  return (
    <div className="max-w-6xl mx-auto p-4">
      <div className="flex items-center gap-4 mb-4">
        <Link to="/app/aggregate" className="text-sm" style={{ color: 'var(--accent)' }}>
          &larr; Aggregate
        </Link>
        <h1 className="text-xl font-bold">Aggregate Results</h1>
        {data?.status && (
          <span className="px-2 py-0.5 rounded text-xs" style={{
            background: data.status === 'completed' ? 'var(--success)' :
              data.status === 'failed' ? 'var(--danger)' : 'var(--warning)',
            color: '#fff',
          }}>
            {data.status}
          </span>
        )}
      </div>

      {(loading || data?.status === 'running') && (
        <div className="p-3 rounded mb-4" style={{ background: 'var(--bg-secondary)' }}>
          <div className="text-sm">Processing: {data?.completedFlops || 0} / {data?.totalFlops || '?'} flops</div>
          <div className="mt-1 h-2 rounded-full overflow-hidden" style={{ background: 'var(--bg-card)' }}>
            <div className="h-full rounded-full" style={{
              background: 'var(--accent)',
              width: data?.totalFlops > 0 ? `${(data.completedFlops / data.totalFlops) * 100}%` : '0%',
            }} />
          </div>
        </div>
      )}

      {data?.status === 'completed' && (
        <div className="flex gap-3 mb-4">
          <button className="px-4 py-2 rounded text-sm"
            style={{ background: 'var(--success)', color: '#fff' }}
            onClick={downloadCsv}>
            Download CSV
          </button>
          <span className="text-sm flex items-center" style={{ color: 'var(--text-secondary)' }}>
            {results.length} flops analyzed
          </span>
        </div>
      )}

      {sortedResults.length > 0 && (
        <div style={{ maxHeight: '70vh', overflowY: 'auto' }}>
          <table className="w-full text-sm">
            <thead>
              <tr style={{ color: 'var(--text-secondary)', position: 'sticky', top: 0, background: 'var(--bg-primary)' }}>
                {['board', 'oop_ev', 'ip_ev', 'oop_equity', 'ip_equity'].map(col => (
                  <th key={col} className="text-left py-2 px-2" style={{ cursor: 'pointer' }}
                    onClick={() => handleSort(col)}>
                    {col}{sortKey === col ? (sortDir === 'asc' ? ' ↑' : ' ↓') : ''}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {sortedResults.map(r => (
                <tr key={r.board} style={{ borderBottom: '1px solid #222' }}>
                  <td className="py-1 px-2 font-mono">{r.board}</td>
                  <td className="py-1 px-2">{r.result.oop_ev?.toFixed(2) ?? '-'}</td>
                  <td className="py-1 px-2">{r.result.ip_ev?.toFixed(2) ?? '-'}</td>
                  <td className="py-1 px-2">{r.result.oop_equity ? `${(r.result.oop_equity * 100).toFixed(1)}%` : '-'}</td>
                  <td className="py-1 px-2">{r.result.ip_equity ? `${(r.result.ip_equity * 100).toFixed(1)}%` : '-'}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
