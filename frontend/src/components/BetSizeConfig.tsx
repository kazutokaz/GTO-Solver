import { useState } from 'react';

interface StreetBetSizes {
  ipBet: number[];
  oopBet: number[];
  ipRaise: number[];
  oopRaise: number[];
  oopDonk: number[];
}

export interface FullBetSizeConfig {
  flop: StreetBetSizes;
  turn: StreetBetSizes;
  river: StreetBetSizes;
}

const DEFAULTS: FullBetSizeConfig = {
  flop: {
    ipBet: [0.33, 0.67, 1.0],
    oopBet: [0.33, 0.67, 1.0],
    ipRaise: [2.5, 4.0],
    oopRaise: [2.5, 4.0],
    oopDonk: [0.33, 0.67],
  },
  turn: {
    ipBet: [0.5, 0.75, 1.0],
    oopBet: [0.5, 0.75, 1.0],
    ipRaise: [2.5, 3.5],
    oopRaise: [2.5, 3.5],
    oopDonk: [0.5, 0.75],
  },
  river: {
    ipBet: [0.5, 0.75, 1.0, 1.5],
    oopBet: [0.5, 0.75, 1.0, 1.5],
    ipRaise: [2.5],
    oopRaise: [2.5],
    oopDonk: [0.75, 1.0],
  },
};

interface Props {
  value: FullBetSizeConfig | null;
  onChange: (config: FullBetSizeConfig | null) => void;
}

function SizesInput({ label, values, onChange }: {
  label: string;
  values: number[];
  onChange: (v: number[]) => void;
}) {
  const text = values.map(v => `${Math.round(v * 100)}%`).join(', ');

  const handleChange = (raw: string) => {
    const nums = raw.split(',')
      .map(s => s.trim().replace('%', ''))
      .filter(s => s.length > 0)
      .map(s => parseFloat(s) / 100)
      .filter(n => !isNaN(n) && n > 0);
    onChange(nums);
  };

  return (
    <div className="flex items-center gap-2">
      <span className="text-xs w-20" style={{ color: 'var(--text-secondary)' }}>{label}</span>
      <input
        className="flex-1 p-1 rounded text-xs font-mono"
        style={{ background: 'var(--bg-card)', color: 'var(--text-primary)', border: '1px solid #333' }}
        defaultValue={text}
        onBlur={e => handleChange(e.target.value)}
        placeholder="33%, 67%, 100%"
      />
    </div>
  );
}

export function BetSizeConfig({ value, onChange }: Props) {
  const [expanded, setExpanded] = useState(false);
  const config = value || DEFAULTS;

  const update = (street: 'flop' | 'turn' | 'river', field: keyof StreetBetSizes, vals: number[]) => {
    const next = {
      ...config,
      [street]: { ...config[street], [field]: vals },
    };
    onChange(next);
  };

  return (
    <div>
      <button
        className="text-sm flex items-center gap-1"
        style={{ color: 'var(--text-secondary)' }}
        onClick={() => setExpanded(!expanded)}
      >
        Bet Sizes {expanded ? '▼' : '▶'}
        {!value && <span className="text-xs" style={{ color: '#555' }}>(default)</span>}
      </button>

      {expanded && (
        <div className="mt-2 p-3 rounded" style={{ background: 'var(--bg-secondary)' }}>
          {(['flop', 'turn', 'river'] as const).map(street => (
            <div key={street} className="mb-3">
              <div className="text-xs font-medium mb-1" style={{ color: 'var(--accent)' }}>
                {street.charAt(0).toUpperCase() + street.slice(1)}
              </div>
              <div className="flex flex-col gap-1">
                <SizesInput label="IP Bet" values={config[street].ipBet} onChange={v => update(street, 'ipBet', v)} />
                <SizesInput label="OOP Bet" values={config[street].oopBet} onChange={v => update(street, 'oopBet', v)} />
                <SizesInput label="IP Raise" values={config[street].ipRaise} onChange={v => update(street, 'ipRaise', v)} />
                <SizesInput label="OOP Raise" values={config[street].oopRaise} onChange={v => update(street, 'oopRaise', v)} />
                <SizesInput label="OOP Donk" values={config[street].oopDonk} onChange={v => update(street, 'oopDonk', v)} />
              </div>
            </div>
          ))}

          <div className="flex gap-2 mt-2">
            <button className="text-xs px-2 py-1 rounded"
              style={{ background: 'var(--bg-card)', color: 'var(--text-secondary)' }}
              onClick={() => onChange(null)}>
              Reset to Defaults
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
