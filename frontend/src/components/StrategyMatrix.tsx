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

interface StrategyMatrixProps {
  strategy: Record<string, number[]>;
  actions: string[];
}

export function StrategyMatrix({ strategy, actions }: StrategyMatrixProps) {
  // Convert specific combos (e.g., AhKh) to grid cells (AKs)
  const cellData = new Map<string, number[]>();

  for (const [hand, probs] of Object.entries(strategy)) {
    const cell = handToCell(hand);
    if (!cell) continue;
    if (!cellData.has(cell)) {
      cellData.set(cell, new Array(actions.length).fill(0));
    }
    const existing = cellData.get(cell)!;
    // Average the strategies for all combos of this cell
    const count = (cellData as any).__count?.get(cell) || 0;
    for (let i = 0; i < probs.length && i < actions.length; i++) {
      existing[i] = (existing[i] * count + probs[i]) / (count + 1);
    }
    if (!(cellData as any).__count) (cellData as any).__count = new Map();
    (cellData as any).__count.set(cell, count + 1);
  }

  return (
    <div>
      <div className="flex gap-2 mb-2 flex-wrap">
        {actions.map((a, i) => (
          <div key={a} className="flex items-center gap-1 text-xs">
            <div className="w-3 h-3 rounded-sm" style={{ background: getActionColor(a) }} />
            <span>{a}</span>
          </div>
        ))}
      </div>

      <div
        className="grid gap-0.5"
        style={{ gridTemplateColumns: 'repeat(13, 1fr)' }}
      >
        {RANKS.map((r1, i) =>
          RANKS.map((r2, j) => {
            const label = i === j ? `${r1}${r2}` : i < j ? `${r1}${r2}s` : `${r2}${r1}o`;
            const probs = cellData.get(label);

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

            // Render as stacked bar
            return (
              <div
                key={label}
                className="rounded-sm overflow-hidden relative"
                style={{ aspectRatio: '1' }}
                title={`${label}: ${actions.map((a, k) => `${a} ${(probs[k] * 100).toFixed(0)}%`).join(', ')}`}
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
    </div>
  );
}

function handToCell(hand: string): string | null {
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
