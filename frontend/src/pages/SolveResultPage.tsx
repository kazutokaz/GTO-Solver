import { useEffect, useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { StrategyMatrix } from '../components/StrategyMatrix';
import { GameTreeNav } from '../components/GameTreeNav';
import { api } from '../api/client';

export function SolveResultPage() {
  const { id } = useParams<{ id: string }>();
  const [data, setData] = useState<any>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    if (!id) return;
    const poll = async () => {
      try {
        const result = await api.getSolveResult(id);
        setData(result);
        if (result.status === 'queued' || result.status === 'running') {
          setTimeout(poll, 2000);
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

  if (error) {
    return (
      <div className="max-w-4xl mx-auto p-4">
        <div style={{ color: 'var(--danger)' }}>Error: {error}</div>
        <Link to="/app/history" className="text-sm mt-3 block" style={{ color: 'var(--accent)' }}>
          Back to History
        </Link>
      </div>
    );
  }

  if (!data || loading) {
    return (
      <div className="max-w-4xl mx-auto p-4">
        <div style={{ color: 'var(--text-secondary)' }}>
          {data?.status === 'queued' ? 'Queued... waiting for worker' :
           data?.status === 'running' ? 'Solving... please wait' :
           'Loading...'}
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-6xl mx-auto p-4">
      <div className="flex items-center gap-4 mb-4">
        <Link to="/app/history" className="text-sm" style={{ color: 'var(--accent)' }}>
          &larr; History
        </Link>
        <h1 className="text-xl font-bold">Solve Result</h1>
        <span className="px-2 py-0.5 rounded text-xs" style={{
          background: data.status === 'completed' ? 'var(--success)' :
            data.status === 'failed' ? 'var(--danger)' : 'var(--warning)',
          color: '#fff',
        }}>
          {data.status}
        </span>
      </div>

      {data.status === 'failed' && (
        <div className="p-3 rounded mb-4" style={{ background: 'rgba(224,36,94,0.2)', color: 'var(--danger)' }}>
          {data.error || 'Solve failed'}
        </div>
      )}

      {data.status === 'completed' && (
        <div className="flex flex-col gap-4">
          <div className="p-3 rounded" style={{ background: 'var(--bg-secondary)' }}>
            <div className="text-sm" style={{ color: 'var(--text-secondary)' }}>
              Iterations: {data.iterations} |
              Exploitability: {data.exploitability != null ? `${(data.exploitability * 100).toFixed(2)}%` : '-'} |
              Time: {data.elapsedSeconds?.toFixed(1)}s
            </div>
          </div>

          {data.result && (
            <div>
              <h3 className="text-sm font-medium mb-2" style={{ color: 'var(--text-secondary)' }}>
                Game Tree
              </h3>
              {data.result.children ? (
                <GameTreeNav root={data.result} />
              ) : (
                <>
                  <div className="text-xs mb-2" style={{ color: 'var(--text-secondary)' }}>
                    Root Strategy ({data.result.player?.toUpperCase()})
                  </div>
                  <StrategyMatrix
                    strategy={data.result.strategy || {}}
                    actions={data.result.actions || []}
                  />
                </>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
