import { useState } from 'react';

const RANKS = ['A', 'K', 'Q', 'J', 'T', '9', '8', '7', '6', '5', '4', '3', '2'];

const ACTION_COLORS: Record<string, string> = {
  fold: '#8899a6',
  check: '#17bf63',
  call: '#17bf63',
  allin: '#ffad1f',
};

function getActionColor(action: string): string {
  if (action.startsWith('bet')) return '#e0245e';
  if (action.startsWith('raise')) return '#ff6b35';
  return ACTION_COLORS[action] || '#8899a6';
}

const SUIT_SYMBOL: Record<string, string> = { s: '\u2660', h: '\u2665', d: '\u2666', c: '\u2663' };
const SUIT_COLOR: Record<string, string> = { s: '#4a9eff', h: '#e0245e', d: '#4a9eff', c: '#17bf63' };

interface StrategyMatrixProps {
  strategy: Record<string, number[]>;
  actions: string[];
  ev?: Record<string, number>;
  onCellClick?: (cell: string, combos: string[]) => void;
  highlightedCells?: Set<string>;
}

export function StrategyMatrix({ strategy, actions, ev, onCellClick, highlightedCells }: StrategyMatrixProps) {
  const [selectedCell, setSelectedCell] = useState<string | null>(null);

  // Build cell data and track combos per cell
  const cellData = new Map<string, number[]>();
  const cellCombos = new Map<string, string[]>();
  const cellCounts = new Map<string, number>();

  for (const [hand, probs] of Object.entries(strategy)) {
    const cell = handToCell(hand);
    if (!cell) continue;

    if (!cellCombos.has(cell)) cellCombos.set(cell, []);
    cellCombos.get(cell)!.push(hand);

    if (!cellData.has(cell)) {
      cellData.set(cell, new Array(actions.length).fill(0));
      cellCounts.set(cell, 0);
    }
    const existing = cellData.get(cell)!;
    const count = cellCounts.get(cell)!;
    for (let i = 0; i < probs.length && i < actions.length; i++) {
      existing[i] = (existing[i] * count + probs[i]) / (count + 1);
    }
    cellCounts.set(cell, count + 1);
  }

  const handleCellClick = (label: string) => {
    if (onCellClick) {
      onCellClick(label, cellCombos.get(label) || []);
    } else {
      setSelectedCell(selectedCell === label ? null : label);
    }
  };

  const selectedCombos = selectedCell ? (cellCombos.get(selectedCell) || []) : [];

  return (
    <div>
      {/* Legend */}
      <div className="flex gap-2 mb-2 flex-wrap">
        {actions.map((a) => (
          <div key={a} className="flex items-center gap-1 text-xs">
            <div className="w-3 h-3 rounded-sm" style={{ background: getActionColor(a) }} />
            <span>{a}</span>
          </div>
        ))}
      </div>

      {/* Matrix */}
      <div
        className="grid gap-0.5"
        style={{ gridTemplateColumns: 'repeat(13, 1fr)' }}
      >
        {RANKS.map((r1, i) =>
          RANKS.map((r2, j) => {
            const label = i === j ? `${r1}${r2}` : i < j ? `${r1}${r2}s` : `${r2}${r1}o`;
            const probs = cellData.get(label);
            const isSelected = selectedCell === label;
            const isHighlighted = highlightedCells?.has(label);

            if (!probs) {
              return (
                <div
                  key={label}
                  className="flex items-center justify-center text-xs font-mono rounded-sm"
                  style={{ background: 'var(--bg-secondary)', aspectRatio: '1', fontSize: '0.55rem', color: '#555' }}
                >
                  {label}
                </div>
              );
            }

            return (
              <div
                key={label}
                className="rounded-sm overflow-hidden relative cursor-pointer"
                style={{
                  aspectRatio: '1',
                  outline: isSelected ? '2px solid #fff' : isHighlighted ? '2px solid #ffad1f' : 'none',
                  outlineOffset: '-1px',
                }}
                title={`${label}: ${actions.map((a, k) => `${a} ${(probs[k] * 100).toFixed(0)}%`).join(', ')}`}
                onClick={() => handleCellClick(label)}
              >
                <div className="absolute inset-0 flex">
                  {probs.map((p, k) => (
                    p > 0.01 ? (
                      <div
                        key={k}
                        style={{
                          width: `${p * 100}%`,
                          background: getActionColor(actions[k]),
                        }}
                      />
                    ) : null
                  ))}
                </div>
                <div className="absolute inset-0 flex items-center justify-center text-xs font-mono"
                  style={{ fontSize: '0.5rem', color: '#fff', textShadow: '0 0 2px #000' }}>
                  {label}
                </div>
              </div>
            );
          })
        )}
      </div>

      {/* Combo Detail Popup */}
      {selectedCell && selectedCombos.length > 0 && !onCellClick && (
        <ComboDetailPopup
          cell={selectedCell}
          combos={selectedCombos}
          strategy={strategy}
          actions={actions}
          ev={ev}
          onClose={() => setSelectedCell(null)}
        />
      )}
    </div>
  );
}

function ComboDetailPopup({
  cell, combos, strategy, actions, ev, onClose,
}: {
  cell: string;
  combos: string[];
  strategy: Record<string, number[]>;
  actions: string[];
  ev?: Record<string, number>;
  onClose: () => void;
}) {
  return (
    <div className="mt-3 p-3 rounded" style={{ background: 'var(--bg-secondary)', border: '1px solid #333' }}>
      <div className="flex items-center justify-between mb-2">
        <span className="text-sm font-medium" style={{ color: 'var(--text-primary)' }}>
          {cell} — {combos.length} combos
        </span>
        <button
          className="text-xs px-2 py-0.5 rounded"
          style={{ background: 'var(--bg-card)', color: 'var(--text-secondary)' }}
          onClick={onClose}
        >
          Close
        </button>
      </div>

      <div style={{ overflowX: 'auto' }}>
        <table className="w-full text-xs" style={{ color: 'var(--text-primary)' }}>
          <thead>
            <tr style={{ color: 'var(--text-secondary)', borderBottom: '1px solid #333' }}>
              <th className="text-left py-1 pr-3">Combo</th>
              {actions.map(a => (
                <th key={a} className="text-center py-1 px-1" style={{ minWidth: 60 }}>
                  <span style={{ color: getActionColor(a) }}>{a}</span>
                </th>
              ))}
              {ev && <th className="text-right py-1 pl-3">EV</th>}
            </tr>
          </thead>
          <tbody>
            {combos.sort().map(combo => {
              const probs = strategy[combo];
              const comboEv = ev?.[combo];
              return (
                <tr key={combo} style={{ borderTop: '1px solid #222' }}>
                  <td className="py-1.5 pr-3 font-mono">
                    <ComboLabel combo={combo} />
                  </td>
                  {probs.map((p, k) => (
                    <td key={k} className="text-center py-1.5 px-1">
                      <div className="flex items-center gap-1 justify-center">
                        <div
                          className="rounded-sm"
                          style={{
                            width: `${Math.max(p * 40, 1)}px`,
                            height: 10,
                            background: p > 0.01 ? getActionColor(actions[k]) : '#333',
                            minWidth: 1,
                          }}
                        />
                        <span style={{
                          color: p > 0.01 ? '#fff' : '#555',
                          fontWeight: p > 0.5 ? 600 : 400,
                        }}>
                          {(p * 100).toFixed(0)}%
                        </span>
                      </div>
                    </td>
                  ))}
                  {ev && (
                    <td className="text-right py-1.5 pl-3 font-mono" style={{ color: 'var(--text-secondary)' }}>
                      {comboEv != null ? comboEv.toFixed(2) : '\u2014'}
                    </td>
                  )}
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function ComboLabel({ combo }: { combo: string }) {
  if (combo.length !== 4) return <span>{combo}</span>;
  const r1 = combo[0], s1 = combo[1], r2 = combo[2], s2 = combo[3];
  return (
    <span>
      {r1}<span style={{ color: SUIT_COLOR[s1] || '#fff' }}>{SUIT_SYMBOL[s1] || s1}</span>
      {r2}<span style={{ color: SUIT_COLOR[s2] || '#fff' }}>{SUIT_SYMBOL[s2] || s2}</span>
    </span>
  );
}

export function handToCell(hand: string): string | null {
  if (hand.length !== 4) return null;
  const r1 = hand[0], s1 = hand[1], r2 = hand[2], s2 = hand[3];
  const ri1 = RANKS.indexOf(r1);
  const ri2 = RANKS.indexOf(r2);
  if (ri1 < 0 || ri2 < 0) return null;

  if (r1 === r2) return `${r1}${r2}`;
  if (s1 === s2) {
    return ri1 < ri2 ? `${r1}${r2}s` : `${r2}${r1}s`;
  }
  return ri1 < ri2 ? `${r1}${r2}o` : `${r2}${r1}o`;
}
