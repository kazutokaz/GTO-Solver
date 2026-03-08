import { useState } from 'react';

export interface RakeSettings {
  percentage: number;
  cap: number;
  noFlopNoDrop: boolean;
}

interface Props {
  value: RakeSettings | null;
  onChange: (config: RakeSettings | null) => void;
}

export function RakeConfig({ value, onChange }: Props) {
  const [expanded, setExpanded] = useState(false);
  const config = value || { percentage: 0, cap: 0, noFlopNoDrop: true };

  const update = (field: keyof RakeSettings, val: any) => {
    onChange({ ...config, [field]: val });
  };

  return (
    <div>
      <button
        className="text-sm flex items-center gap-1"
        style={{ color: 'var(--text-secondary)' }}
        onClick={() => setExpanded(!expanded)}
      >
        Rake {expanded ? '▼' : '▶'}
        {value && value.percentage > 0 && (
          <span className="text-xs" style={{ color: 'var(--accent)' }}>
            ({(value.percentage * 100).toFixed(0)}% / {value.cap}BB cap)
          </span>
        )}
        {!value && <span className="text-xs" style={{ color: '#555' }}>(none)</span>}
      </button>

      {expanded && (
        <div className="mt-2 p-3 rounded flex flex-col gap-2" style={{ background: 'var(--bg-secondary)' }}>
          <div className="flex gap-4">
            <div className="flex-1">
              <label className="text-xs block mb-1" style={{ color: 'var(--text-secondary)' }}>Rake %</label>
              <input
                type="number"
                className="w-full p-1 rounded text-sm"
                style={{ background: 'var(--bg-card)', color: 'var(--text-primary)', border: '1px solid #333' }}
                value={config.percentage * 100}
                onChange={e => update('percentage', parseFloat(e.target.value) / 100 || 0)}
                min={0} max={100} step={0.5}
              />
            </div>
            <div className="flex-1">
              <label className="text-xs block mb-1" style={{ color: 'var(--text-secondary)' }}>Cap (BB)</label>
              <input
                type="number"
                className="w-full p-1 rounded text-sm"
                style={{ background: 'var(--bg-card)', color: 'var(--text-primary)', border: '1px solid #333' }}
                value={config.cap}
                onChange={e => update('cap', parseFloat(e.target.value) || 0)}
                min={0} step={0.5}
              />
            </div>
          </div>

          <label className="flex items-center gap-2 text-sm" style={{ color: 'var(--text-secondary)' }}>
            <input
              type="checkbox"
              checked={config.noFlopNoDrop}
              onChange={e => update('noFlopNoDrop', e.target.checked)}
            />
            No Flop No Drop
          </label>

          <button className="text-xs px-2 py-1 rounded w-fit"
            style={{ background: 'var(--bg-card)', color: 'var(--text-secondary)' }}
            onClick={() => onChange(null)}>
            Clear Rake
          </button>
        </div>
      )}
    </div>
  );
}
