import { useState } from 'react';
import { StrategyMatrix } from './StrategyMatrix';

interface TreeNode {
  player?: string;
  actions?: string[];
  strategy?: Record<string, number[]>;
  ev?: Record<string, number>;
  children?: Record<string, TreeNode>;
}

interface Props {
  root: TreeNode;
}

export function GameTreeNav({ root }: Props) {
  const [path, setPath] = useState<string[]>([]);

  // Navigate to current node
  let current: TreeNode = root;
  const validPath: string[] = [];
  for (const action of path) {
    if (current.children && current.children[action]) {
      current = current.children[action];
      validPath.push(action);
    } else {
      break;
    }
  }

  const goBack = () => {
    setPath(prev => prev.slice(0, -1));
  };

  const navigate = (action: string) => {
    setPath(prev => [...prev, action]);
  };

  const goToRoot = () => {
    setPath([]);
  };

  return (
    <div className="flex flex-col gap-3">
      {/* Breadcrumb */}
      <div className="flex items-center gap-1 flex-wrap text-xs" style={{ color: 'var(--text-secondary)' }}>
        <button onClick={goToRoot} className="px-1 rounded"
          style={{ color: 'var(--accent)', background: 'none' }}>
          Root
        </button>
        {validPath.map((action, i) => (
          <span key={i} className="flex items-center gap-1">
            <span>→</span>
            <button
              className="px-1 rounded"
              style={{ color: 'var(--accent)', background: 'none' }}
              onClick={() => setPath(validPath.slice(0, i + 1))}
            >
              {action}
            </button>
          </span>
        ))}
      </div>

      {/* Current node info */}
      <div className="p-2 rounded text-sm" style={{ background: 'var(--bg-secondary)' }}>
        <div style={{ color: 'var(--text-secondary)' }}>
          Player: <span className="font-medium" style={{ color: 'var(--text-primary)' }}>
            {current.player?.toUpperCase() || 'Terminal'}
          </span>
        </div>
      </div>

      {/* Action buttons to navigate deeper */}
      {current.children && Object.keys(current.children).length > 0 && (
        <div className="flex gap-2 flex-wrap">
          {Object.keys(current.children).map(action => (
            <button
              key={action}
              className="px-3 py-1 rounded text-sm"
              style={{
                background: getActionBg(action),
                color: '#fff',
              }}
              onClick={() => navigate(action)}
            >
              {action}
            </button>
          ))}
        </div>
      )}

      {/* Back button */}
      {validPath.length > 0 && (
        <button
          className="text-xs px-2 py-1 rounded w-fit"
          style={{ background: 'var(--bg-card)', color: 'var(--text-secondary)' }}
          onClick={goBack}
        >
          ← Back
        </button>
      )}

      {/* Strategy display */}
      {current.strategy && current.actions && (
        <StrategyMatrix strategy={current.strategy} actions={current.actions} />
      )}

      {/* EV display */}
      {current.ev && Object.keys(current.ev).length > 0 && (
        <div className="text-xs" style={{ color: 'var(--text-secondary)' }}>
          Avg EV: {(Object.values(current.ev).reduce((a, b) => a + b, 0) / Object.values(current.ev).length).toFixed(2)} BB
        </div>
      )}
    </div>
  );
}

function getActionBg(action: string): string {
  if (action.startsWith('bet')) return '#e0245e';
  if (action.startsWith('raise')) return '#ff6b35';
  if (action === 'fold') return '#8899a6';
  if (action === 'check' || action === 'call') return '#17bf63';
  if (action === 'allin') return '#ffad1f';
  return '#1d9bf0';
}
