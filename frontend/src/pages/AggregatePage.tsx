import { useState } from 'react';
import { RangeEditor } from '../components/RangeEditor';
import { api } from '../api/client';

const FLOP_FILTERS = [
  { id: 'all', label: 'All (1755)' },
  { id: 'paired', label: 'Paired' },
  { id: 'monotone', label: 'Monotone' },
  { id: 'rainbow', label: 'Rainbow' },
];

interface AggregateResult {
  board: string;
  result: {
    oop_ev?: number;
    ip_ev?: number;
    oop_equity?: number;
    ip_equity?: number;
  };
}

export function AggregatePage() {
  const [oopRange, setOopRange] = useState('');
  const [ipRange, setIpRange] = useState('');
  const [stackSize, setStackSize] = useState(100);
  const [potSize, setPotSize] = useState(6.5);
  const [flopFilter, setFlopFilter] = useState('all');
  const [status, setStatus] = useState<'idle' | 'running' | 'completed' | 'failed'>('idle');
  const [results, setResults] = useState<AggregateResult[]>([]);
  const [jobId, setJobId] = useState<string | null>(null);
  const [progress, setProgress] = useState({ completed: 0, total: 0 });
  const [sortKey, setSortKey] = useState<string>('board');
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>('asc');
  const [error, setError] = useState('');

  const canSubmit = oopRange && ipRange && status === 'idle';

  const handleSubmit = async () => {
    setStatus('running');
    setError('');
    try {
      const res = await fetch('/api/aggregate', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${localStorage.getItem('token')}`,
        },
        body: JSON.stringify({
          game: { stackSize, potSize, oopRange, ipRange },
          flopFilter: { type: flopFilter },
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error);
      setJobId(data.jobId);
      setProgress({ completed: 0, total: data.totalFlops });
      pollResults(data.jobId);
    } catch (err: any) {
      setStatus('failed');
      setError(err.message);
    }
  };

  const pollResults = async (id: string) => {
    const poll = async () => {
      try {
        const res = await fetch(`/api/aggregate/${id}`, {
          headers: { 'Authorization': `Bearer ${localStorage.getItem('token')}` },
        });
        const data = await res.json();
        setProgress({ completed: data.completedFlops, total: data.totalFlops });

        if (data.status === 'completed') {
          setResults(data.results || []);
          setStatus('completed');
        } else if (data.status === 'failed') {
          setStatus('failed');
        } else {
          setTimeout(poll, 3000);
        }
      } catch {
        setTimeout(poll, 5000);
      }
    };
    poll();
  };

  const handleSort = (key: string) => {
    if (sortKey === key) {
      setSortDir(d => d === 'asc' ? 'desc' : 'asc');
    } else {
      setSortKey(key);
      setSortDir('asc');
    }
  };

  const sortedResults = [...results].sort((a, b) => {
    let va: any, vb: any;
    if (sortKey === 'board') { va = a.board; vb = b.board; }
    else { va = (a.result as any)[sortKey] ?? 0; vb = (b.result as any)[sortKey] ?? 0; }
    return sortDir === 'asc' ? (va > vb ? 1 : -1) : (va < vb ? 1 : -1);
  });

  const downloadCsv = () => {
    if (jobId) {
      window.open(`/api/aggregate/${jobId}/csv?token=${localStorage.getItem('token')}`, '_blank');
    }
  };

  return (
    <div className="max-w-6xl mx-auto p-4">
      <h1 className="text-xl font-bold mb-4">Aggregate Analysis</h1>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Input */}
        <div className="flex flex-col gap-4">
          <div className="flex gap-4">
            <div className="flex-1">
              <label className="text-sm" style={{ color: 'var(--text-secondary)' }}>Stack (BB)</label>
              <input type="number" value={stackSize} onChange={e => setStackSize(+e.target.value)}
                className="w-full p-2 rounded text-sm"
                style={{ background: 'var(--bg-secondary)', color: 'var(--text-primary)', border: '1px solid #333' }} />
            </div>
            <div className="flex-1">
              <label className="text-sm" style={{ color: 'var(--text-secondary)' }}>Pot (BB)</label>
              <input type="number" value={potSize} onChange={e => setPotSize(+e.target.value)}
                className="w-full p-2 rounded text-sm"
                style={{ background: 'var(--bg-secondary)', color: 'var(--text-primary)', border: '1px solid #333' }} />
            </div>
          </div>

          <div>
            <label className="text-sm" style={{ color: 'var(--text-secondary)' }}>Flop Filter</label>
            <div className="flex gap-2 mt-1">
              {FLOP_FILTERS.map(f => (
                <button key={f.id}
                  className="px-3 py-1 rounded text-sm"
                  style={{
                    background: flopFilter === f.id ? 'var(--accent)' : 'var(--bg-secondary)',
                    color: flopFilter === f.id ? '#fff' : 'var(--text-secondary)',
                  }}
                  onClick={() => setFlopFilter(f.id)}>
                  {f.label}
                </button>
              ))}
            </div>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <RangeEditor value={oopRange} onChange={setOopRange} label="OOP Range" />
            <RangeEditor value={ipRange} onChange={setIpRange} label="IP Range" />
          </div>

          <div className="flex gap-3 items-center">
            <button
              className="px-6 py-2 rounded font-medium"
              style={{ background: canSubmit ? 'var(--accent)' : '#555', color: '#fff' }}
              disabled={!canSubmit}
              onClick={handleSubmit}>
              Run Analysis
            </button>
            {status === 'completed' && (
              <button className="px-4 py-2 rounded text-sm"
                style={{ background: 'var(--success)', color: '#fff' }}
                onClick={downloadCsv}>
                Download CSV
              </button>
            )}
          </div>

          {status === 'running' && (
            <div className="p-3 rounded" style={{ background: 'var(--bg-secondary)' }}>
              <div className="text-sm">Processing: {progress.completed} / {progress.total} flops</div>
              <div className="mt-1 h-2 rounded-full overflow-hidden" style={{ background: 'var(--bg-card)' }}>
                <div className="h-full rounded-full" style={{
                  background: 'var(--accent)',
                  width: progress.total > 0 ? `${(progress.completed / progress.total) * 100}%` : '0%',
                }} />
              </div>
            </div>
          )}
          {error && <div className="text-sm" style={{ color: 'var(--danger)' }}>{error}</div>}
        </div>

        {/* Results Table */}
        <div>
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
      </div>
    </div>
  );
}
