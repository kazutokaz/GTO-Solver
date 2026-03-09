import { useState, useCallback } from 'react';
import { StrategyMatrix, handToCell } from './StrategyMatrix';

interface TreeNode {
  player?: string;
  actions?: string[];
  strategy?: Record<string, number[]>;
  ev?: Record<string, number>;
  children?: Record<string, TreeNode>;
}

export interface NodeLockData {
  actionPath: string[];
  handStrategies: Record<string, number[]>;
}

interface Props {
  root: TreeNode;
  nodeLocks?: NodeLockData[];
  onNodeLock?: (lock: NodeLockData) => void;
  onRemoveNodeLock?: (actionPath: string[]) => void;
}

const SUIT_SYMBOL: Record<string, string> = { s: '\u2660', h: '\u2665', d: '\u2666', c: '\u2663' };
const SUIT_COLOR: Record<string, string> = { s: '#4a9eff', h: '#e0245e', d: '#4a9eff', c: '#17bf63' };

function CardButton({ card, onClick }: { card: string; onClick: () => void }) {
  const rank = card.slice(0, -1);
  const suitChar = card.slice(-1);
  const symbol = SUIT_SYMBOL[suitChar] || suitChar;
  const color = SUIT_COLOR[suitChar] || '#fff';

  return (
    <button
      onClick={onClick}
      className="flex items-center gap-0.5 px-3 py-2 rounded text-sm font-bold"
      style={{
        background: '#1a1a2e',
        border: '1px solid #333',
        color: '#fff',
        cursor: 'pointer',
        minWidth: 48,
        justifyContent: 'center',
      }}
    >
      <span>{rank}</span>
      <span style={{ color }}>{symbol}</span>
    </button>
  );
}

export function GameTreeNav({ root, nodeLocks, onNodeLock, onRemoveNodeLock }: Props) {
  const [path, setPath] = useState<string[]>([]);
  const [lockMode, setLockMode] = useState(false);
  // hand class -> action probabilities for the lock being edited
  const [lockEdits, setLockEdits] = useState<Record<string, number[]>>({});
  const [editingCell, setEditingCell] = useState<string | null>(null);

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
    cancelLock();
  };

  const navigate = (action: string) => {
    setPath(prev => [...prev, action]);
    cancelLock();
  };

  const goToRoot = () => {
    setPath([]);
    cancelLock();
  };

  const isChance = current.player === 'chance';
  const isTerminal = current.player?.startsWith('terminal');
  const isActionNode = !isChance && !isTerminal && current.player;

  // Check if current node has an existing lock
  const currentPathStr = JSON.stringify(validPath);
  const hasExistingLock = nodeLocks?.some(l => JSON.stringify(l.actionPath) === currentPathStr);

  const startLock = () => {
    setLockMode(true);
    setLockEdits({});
    setEditingCell(null);
  };

  const cancelLock = () => {
    setLockMode(false);
    setLockEdits({});
    setEditingCell(null);
  };

  const saveLock = () => {
    if (!onNodeLock || Object.keys(lockEdits).length === 0) return;

    // Convert hand-class edits to combo-level strategies
    const handStrategies: Record<string, number[]> = {};
    const strategy = current.strategy || {};

    for (const [cell, probs] of Object.entries(lockEdits)) {
      // Find all combos belonging to this cell
      for (const combo of Object.keys(strategy)) {
        if (handToCell(combo) === cell) {
          handStrategies[combo] = probs;
        }
      }
    }

    onNodeLock({
      actionPath: [...validPath],
      handStrategies,
    });

    cancelLock();
  };

  const removeLock = () => {
    if (onRemoveNodeLock) {
      onRemoveNodeLock([...validPath]);
    }
  };

  const handleLockCellClick = useCallback((cell: string) => {
    setEditingCell(prev => prev === cell ? null : cell);
  }, []);

  const setEditAction = (cell: string, actionIdx: number) => {
    const numActions = current.actions?.length || 0;
    const probs = new Array(numActions).fill(0);
    probs[actionIdx] = 1.0;
    setLockEdits(prev => ({ ...prev, [cell]: probs }));
  };

  const setEditSlider = (cell: string, actionIdx: number, value: number) => {
    const numActions = current.actions?.length || 0;
    setLockEdits(prev => {
      const existing = prev[cell] || new Array(numActions).fill(1 / numActions);
      const updated = [...existing];
      updated[actionIdx] = value / 100;
      // Normalize
      const sum = updated.reduce((a, b) => a + b, 0);
      if (sum > 0) {
        for (let i = 0; i < updated.length; i++) updated[i] /= sum;
      }
      return { ...prev, [cell]: updated };
    });
  };

  const removeEdit = (cell: string) => {
    setLockEdits(prev => {
      const next = { ...prev };
      delete next[cell];
      return next;
    });
    if (editingCell === cell) setEditingCell(null);
  };

  const lockedCells = new Set(Object.keys(lockEdits));

  return (
    <div className="flex flex-col gap-3">
      {/* Breadcrumb */}
      <div className="flex items-center gap-1 flex-wrap text-xs" style={{ color: 'var(--text-secondary)' }}>
        <button onClick={goToRoot} className="px-1 rounded"
          style={{ color: 'var(--accent)', background: 'none' }}>
          Root
        </button>
        {validPath.map((action, i) => {
          const subPath = validPath.slice(0, i + 1);
          const subPathStr = JSON.stringify(subPath);
          const isLocked = nodeLocks?.some(l => JSON.stringify(l.actionPath) === subPathStr);
          return (
            <span key={i} className="flex items-center gap-1">
              <span style={{ color: 'var(--text-secondary)' }}>{'\u2192'}</span>
              <button
                className="px-1 rounded"
                style={{ color: isLocked ? '#ffad1f' : 'var(--accent)', background: 'none' }}
                onClick={() => { setPath(subPath); cancelLock(); }}
              >
                {action}{isLocked ? ' \uD83D\uDD12' : ''}
              </button>
            </span>
          );
        })}
      </div>

      {/* Current node info + Lock button */}
      <div className="p-2 rounded text-sm flex items-center justify-between" style={{ background: 'var(--bg-secondary)' }}>
        <div style={{ color: 'var(--text-secondary)' }}>
          {isChance ? (
            <span>Deal card {'\u2192'} select a card below</span>
          ) : isTerminal ? (
            <span>Terminal: <span className="font-medium" style={{ color: 'var(--text-primary)' }}>
              {current.player === 'terminal:showdown' ? 'Showdown' :
               current.player === 'terminal:oop_wins' ? 'OOP wins (IP folded)' :
               current.player === 'terminal:ip_wins' ? 'IP wins (OOP folded)' :
               current.player}
            </span></span>
          ) : (
            <span>Player: <span className="font-medium" style={{ color: 'var(--text-primary)' }}>
              {current.player?.toUpperCase() || '?'}
            </span></span>
          )}
        </div>

        {isActionNode && onNodeLock && !lockMode && (
          <div className="flex gap-2">
            {hasExistingLock && (
              <button
                className="text-xs px-2 py-1 rounded"
                style={{ background: '#e0245e33', color: '#e0245e' }}
                onClick={removeLock}
              >
                Unlock
              </button>
            )}
            <button
              className="text-xs px-2 py-1 rounded"
              style={{ background: hasExistingLock ? '#ffad1f33' : 'var(--bg-card)', color: hasExistingLock ? '#ffad1f' : 'var(--text-secondary)' }}
              onClick={startLock}
            >
              {hasExistingLock ? '\uD83D\uDD12 Edit Lock' : '\uD83D\uDD13 Lock'}
            </button>
          </div>
        )}
      </div>

      {/* Lock mode controls */}
      {lockMode && (
        <div className="p-3 rounded" style={{ background: '#ffad1f15', border: '1px solid #ffad1f44' }}>
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium" style={{ color: '#ffad1f' }}>
              Lock Mode — Click cells to edit strategy
            </span>
            <div className="flex gap-2">
              <button
                className="text-xs px-3 py-1 rounded"
                style={{ background: 'var(--bg-card)', color: 'var(--text-secondary)' }}
                onClick={cancelLock}
              >
                Cancel
              </button>
              <button
                className="text-xs px-3 py-1 rounded font-medium"
                style={{
                  background: Object.keys(lockEdits).length > 0 ? '#ffad1f' : '#555',
                  color: '#000',
                  cursor: Object.keys(lockEdits).length > 0 ? 'pointer' : 'not-allowed',
                }}
                disabled={Object.keys(lockEdits).length === 0}
                onClick={saveLock}
              >
                Save Lock ({Object.keys(lockEdits).length} hands)
              </button>
            </div>
          </div>

          {/* Quick lock buttons */}
          {current.actions && (
            <div className="flex gap-2 flex-wrap mb-2">
              <span className="text-xs" style={{ color: 'var(--text-secondary)', lineHeight: '24px' }}>
                Lock all to:
              </span>
              {current.actions.map((action, idx) => (
                <button
                  key={action}
                  className="text-xs px-2 py-0.5 rounded"
                  style={{ background: getActionBg(action), color: '#fff' }}
                  onClick={() => {
                    if (!current.strategy) return;
                    const edits: Record<string, number[]> = {};
                    const seen = new Set<string>();
                    for (const combo of Object.keys(current.strategy)) {
                      const cell = handToCell(combo);
                      if (cell && !seen.has(cell)) {
                        seen.add(cell);
                        const probs = new Array(current.actions!.length).fill(0);
                        probs[idx] = 1.0;
                        edits[cell] = probs;
                      }
                    }
                    setLockEdits(edits);
                  }}
                >
                  100% {action}
                </button>
              ))}
            </div>
          )}

          {/* Locked hands list */}
          {Object.keys(lockEdits).length > 0 && (
            <div className="text-xs" style={{ color: 'var(--text-secondary)' }}>
              Locked: {Object.entries(lockEdits).map(([cell, probs]) => {
                const dominant = current.actions?.[probs.indexOf(Math.max(...probs))] || '?';
                return (
                  <span key={cell} className="inline-flex items-center gap-1 mr-2 mb-1 px-1.5 py-0.5 rounded"
                    style={{ background: '#ffad1f22' }}>
                    <span style={{ color: '#ffad1f' }}>{cell}</span>
                    <span>{'\u2192'} {dominant} {(Math.max(...probs) * 100).toFixed(0)}%</span>
                    <button onClick={() => removeEdit(cell)} style={{ color: '#e0245e', marginLeft: 2 }}>{'\u00D7'}</button>
                  </span>
                );
              })}
            </div>
          )}
        </div>
      )}

      {/* Cell editing popup */}
      {lockMode && editingCell && current.actions && (
        <CellLockEditor
          cell={editingCell}
          actions={current.actions}
          probs={lockEdits[editingCell] || new Array(current.actions.length).fill(1 / current.actions.length)}
          onSetAction={(idx) => setEditAction(editingCell, idx)}
          onSetSlider={(idx, val) => setEditSlider(editingCell, idx, val)}
          onRemove={() => removeEdit(editingCell)}
          onClose={() => setEditingCell(null)}
        />
      )}

      {/* Chance node: card selection buttons */}
      {isChance && current.actions && current.actions.length > 0 && (
        <div>
          <div className="text-xs mb-2" style={{ color: 'var(--text-secondary)' }}>
            Select card:
          </div>
          <div className="flex gap-2 flex-wrap">
            {current.actions.map(card => (
              <CardButton key={card} card={card} onClick={() => navigate(card)} />
            ))}
          </div>
        </div>
      )}

      {/* Action buttons for player nodes */}
      {!isChance && !lockMode && current.children && Object.keys(current.children).length > 0 && (
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
      {validPath.length > 0 && !lockMode && (
        <button
          className="text-xs px-2 py-1 rounded w-fit"
          style={{ background: 'var(--bg-card)', color: 'var(--text-secondary)' }}
          onClick={goBack}
        >
          {'\u2190'} Back
        </button>
      )}

      {/* Strategy display */}
      {current.strategy && current.actions && Object.keys(current.strategy).length > 0 && (
        <StrategyMatrix
          strategy={current.strategy}
          actions={current.actions}
          ev={current.ev}
          onCellClick={lockMode ? handleLockCellClick : undefined}
          highlightedCells={lockMode ? lockedCells : undefined}
        />
      )}

      {/* EV display */}
      {!lockMode && current.ev && Object.keys(current.ev).length > 0 && (
        <div className="text-xs" style={{ color: 'var(--text-secondary)' }}>
          Avg EV: {(Object.values(current.ev).reduce((a, b) => a + b, 0) / Object.values(current.ev).length).toFixed(2)} BB
        </div>
      )}
    </div>
  );
}

function CellLockEditor({
  cell, actions, probs, onSetAction, onSetSlider, onRemove, onClose,
}: {
  cell: string;
  actions: string[];
  probs: number[];
  onSetAction: (idx: number) => void;
  onSetSlider: (idx: number, value: number) => void;
  onRemove: () => void;
  onClose: () => void;
}) {
  return (
    <div className="p-3 rounded" style={{ background: 'var(--bg-secondary)', border: '1px solid #444' }}>
      <div className="flex items-center justify-between mb-2">
        <span className="text-sm font-medium" style={{ color: 'var(--text-primary)' }}>
          Edit: {cell}
        </span>
        <div className="flex gap-2">
          <button
            className="text-xs px-2 py-0.5 rounded"
            style={{ background: '#e0245e33', color: '#e0245e' }}
            onClick={onRemove}
          >
            Remove
          </button>
          <button
            className="text-xs px-2 py-0.5 rounded"
            style={{ background: 'var(--bg-card)', color: 'var(--text-secondary)' }}
            onClick={onClose}
          >
            Done
          </button>
        </div>
      </div>

      {/* Quick presets */}
      <div className="flex gap-1 mb-3 flex-wrap">
        {actions.map((a, idx) => (
          <button
            key={a}
            className="text-xs px-2 py-0.5 rounded"
            style={{ background: getActionBg(a), color: '#fff', opacity: 0.8 }}
            onClick={() => onSetAction(idx)}
          >
            100% {a}
          </button>
        ))}
      </div>

      {/* Sliders */}
      <div className="flex flex-col gap-2">
        {actions.map((a, idx) => (
          <div key={a} className="flex items-center gap-2">
            <span className="text-xs w-16 text-right" style={{ color: getActionBg(a) }}>
              {a}
            </span>
            <input
              type="range"
              min={0}
              max={100}
              value={Math.round(probs[idx] * 100)}
              onChange={e => onSetSlider(idx, +e.target.value)}
              className="flex-1"
              style={{ accentColor: getActionBg(a) }}
            />
            <span className="text-xs w-10 text-right font-mono" style={{ color: 'var(--text-primary)' }}>
              {(probs[idx] * 100).toFixed(0)}%
            </span>
          </div>
        ))}
      </div>
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
