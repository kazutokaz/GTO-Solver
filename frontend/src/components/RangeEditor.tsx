import { useState, useCallback, useRef } from 'react';

const RANKS = ['A', 'K', 'Q', 'J', 'T', '9', '8', '7', '6', '5', '4', '3', '2'];

interface CellData {
  label: string;
  type: 'pair' | 'suited' | 'offsuit';
  row: number;
  col: number;
}

function buildGrid(): CellData[][] {
  return RANKS.map((r1, i) =>
    RANKS.map((r2, j) => {
      if (i === j) return { label: `${r1}${r2}`, type: 'pair' as const, row: i, col: j };
      if (i < j) return { label: `${r1}${r2}s`, type: 'suited' as const, row: i, col: j };
      return { label: `${r2}${r1}o`, type: 'offsuit' as const, row: i, col: j };
    })
  );
}

const grid = buildGrid();

interface RangeEditorProps {
  value: string;
  onChange: (range: string) => void;
  label?: string;
}

export function RangeEditor({ value, onChange, label }: RangeEditorProps) {
  const [selected, setSelected] = useState<Map<string, number>>(() => parseRange(value));
  const [isDragging, setIsDragging] = useState(false);
  const dragMode = useRef<'select' | 'deselect'>('select');
  const [textMode, setTextMode] = useState(false);
  const [textValue, setTextValue] = useState(value);

  const toggleCell = useCallback((label: string) => {
    setSelected(prev => {
      const next = new Map(prev);
      if (next.has(label) && next.get(label)! > 0) {
        next.set(label, 0);
      } else {
        next.set(label, 100);
      }
      const rangeStr = mapToString(next);
      onChange(rangeStr);
      return next;
    });
  }, [onChange]);

  const applyDrag = useCallback((label: string) => {
    setSelected(prev => {
      const next = new Map(prev);
      next.set(label, dragMode.current === 'select' ? 100 : 0);
      const rangeStr = mapToString(next);
      onChange(rangeStr);
      return next;
    });
  }, [onChange]);

  const handleMouseDown = (label: string) => {
    setIsDragging(true);
    const current = selected.get(label) || 0;
    dragMode.current = current > 0 ? 'deselect' : 'select';
    applyDrag(label);
  };

  const handleMouseEnter = (label: string) => {
    if (isDragging) applyDrag(label);
  };

  const handleMouseUp = () => setIsDragging(false);

  const handleTextChange = (text: string) => {
    setTextValue(text);
    onChange(text);
    setSelected(parseRange(text));
  };

  const combos = countCombos(selected);

  return (
    <div className="flex flex-col gap-2" onMouseUp={handleMouseUp} onMouseLeave={handleMouseUp}>
      {label && <div className="text-sm font-medium" style={{ color: 'var(--text-secondary)' }}>{label}</div>}

      <div className="flex gap-3 items-center mb-1">
        <span className="text-xs" style={{ color: 'var(--text-secondary)' }}>{combos} combos</span>
        <button
          className="text-xs px-2 py-0.5 rounded"
          style={{ background: 'var(--bg-secondary)', color: 'var(--text-primary)' }}
          onClick={() => setTextMode(!textMode)}
        >
          {textMode ? 'Grid' : 'Text'}
        </button>
      </div>

      {textMode ? (
        <textarea
          className="w-full h-24 p-2 rounded text-sm font-mono"
          style={{ background: 'var(--bg-secondary)', color: 'var(--text-primary)', border: '1px solid #333' }}
          value={textValue}
          onChange={(e) => handleTextChange(e.target.value)}
          placeholder="AA,KK,QQ,AKs,AKo..."
        />
      ) : (
        <div
          className="grid gap-0.5 select-none"
          style={{ gridTemplateColumns: 'repeat(13, 1fr)', userSelect: 'none' }}
        >
          {grid.flat().map((cell) => {
            const freq = selected.get(cell.label) || 0;
            const bg = freq >= 100 ? 'var(--accent)'
              : freq >= 75 ? 'rgba(29,155,240,0.8)'
              : freq >= 50 ? 'rgba(29,155,240,0.6)'
              : freq >= 25 ? 'rgba(29,155,240,0.4)'
              : 'var(--bg-secondary)';

            return (
              <div
                key={cell.label}
                className="flex items-center justify-center text-xs font-mono cursor-pointer rounded-sm"
                style={{
                  background: bg,
                  color: freq > 0 ? '#fff' : 'var(--text-secondary)',
                  aspectRatio: '1',
                  fontSize: '0.6rem',
                  minWidth: 0,
                }}
                onMouseDown={() => handleMouseDown(cell.label)}
                onMouseEnter={() => handleMouseEnter(cell.label)}
              >
                {cell.label}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

function parseRange(str: string): Map<string, number> {
  const map = new Map<string, number>();
  if (!str) return map;
  for (const token of str.split(',')) {
    const t = token.trim();
    if (!t) continue;
    const [hand, freqStr] = t.split(':');
    const freq = freqStr ? Math.round(parseFloat(freqStr) * 100) : 100;
    map.set(hand, freq);
  }
  return map;
}

function mapToString(map: Map<string, number>): string {
  const parts: string[] = [];
  for (const [hand, freq] of map) {
    if (freq <= 0) continue;
    if (freq >= 100) {
      parts.push(hand);
    } else {
      parts.push(`${hand}:${(freq / 100).toFixed(2)}`);
    }
  }
  return parts.join(',');
}

function countCombos(map: Map<string, number>): number {
  let total = 0;
  for (const [hand, freq] of map) {
    if (freq <= 0) continue;
    const f = freq / 100;
    if (hand.endsWith('s')) total += 4 * f;
    else if (hand.endsWith('o')) total += 12 * f;
    else if (hand.length === 2) total += 6 * f; // pair
    else total += f; // specific combo
  }
  return Math.round(total);
}
