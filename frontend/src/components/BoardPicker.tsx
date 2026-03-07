import { useState } from 'react';

const RANKS = ['A', 'K', 'Q', 'J', 'T', '9', '8', '7', '6', '5', '4', '3', '2'];
const SUITS = [
  { char: 's', symbol: '♠', color: '#333' },
  { char: 'h', symbol: '♥', color: '#e0245e' },
  { char: 'd', symbol: '♦', color: '#1d9bf0' },
  { char: 'c', symbol: '♣', color: '#17bf63' },
];

interface BoardPickerProps {
  board: string[];
  onChange: (board: string[]) => void;
}

export function BoardPicker({ board, onChange }: BoardPickerProps) {
  const [showPicker, setShowPicker] = useState(false);

  const addCard = (card: string) => {
    if (board.length >= 5) return;
    if (board.includes(card)) return;
    onChange([...board, card]);
  };

  const removeCard = (index: number) => {
    onChange(board.filter((_, i) => i !== index));
  };

  const streetLabel = board.length < 3 ? 'Select Flop' : board.length === 3 ? 'Flop' : board.length === 4 ? 'Turn' : 'River';

  return (
    <div className="flex flex-col gap-2">
      <div className="text-sm font-medium" style={{ color: 'var(--text-secondary)' }}>
        Board ({streetLabel})
      </div>

      <div className="flex gap-2 items-center">
        {[0, 1, 2, 3, 4].map(i => (
          <div
            key={i}
            className="w-10 h-14 rounded flex items-center justify-center text-sm font-bold cursor-pointer"
            style={{
              background: board[i] ? 'var(--bg-card)' : 'var(--bg-secondary)',
              border: board[i] ? '2px solid var(--accent)' : '2px dashed #444',
            }}
            onClick={() => board[i] ? removeCard(i) : setShowPicker(true)}
          >
            {board[i] ? (
              <span>
                {board[i][0]}
                <span style={{ color: getSuitColor(board[i][1]) }}>
                  {getSuitSymbol(board[i][1])}
                </span>
              </span>
            ) : i < 3 ? '?' : ''}
          </div>
        ))}

        <button
          className="px-3 py-1 rounded text-sm"
          style={{ background: 'var(--bg-secondary)', color: 'var(--text-primary)' }}
          onClick={() => setShowPicker(!showPicker)}
        >
          {showPicker ? 'Close' : 'Pick'}
        </button>

        {board.length > 0 && (
          <button
            className="px-3 py-1 rounded text-sm"
            style={{ background: 'var(--danger)', color: '#fff' }}
            onClick={() => onChange([])}
          >
            Clear
          </button>
        )}
      </div>

      {showPicker && (
        <div className="p-2 rounded" style={{ background: 'var(--bg-secondary)' }}>
          {SUITS.map(suit => (
            <div key={suit.char} className="flex gap-1 mb-1">
              {RANKS.map(rank => {
                const card = `${rank}${suit.char}`;
                const used = board.includes(card);
                return (
                  <button
                    key={card}
                    className="w-7 h-8 rounded text-xs font-mono flex items-center justify-center"
                    style={{
                      background: used ? '#555' : 'var(--bg-card)',
                      color: used ? '#777' : suit.color,
                      cursor: used ? 'not-allowed' : 'pointer',
                      border: '1px solid #333',
                    }}
                    disabled={used}
                    onClick={() => { addCard(card); if (board.length + 1 >= 5) setShowPicker(false); }}
                  >
                    {rank}
                  </button>
                );
              })}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function getSuitSymbol(s: string): string {
  return SUITS.find(x => x.char === s)?.symbol || s;
}

function getSuitColor(s: string): string {
  return SUITS.find(x => x.char === s)?.color || '#fff';
}
