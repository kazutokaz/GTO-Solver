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
  turnCards: string[];
  riverCards: string[];
  onChange: (board: string[]) => void;
  onTurnCardsChange: (cards: string[]) => void;
  onRiverCardsChange: (cards: string[]) => void;
}

type PickerTarget = 'board' | 'turn' | 'river';

export function BoardPicker({ board, turnCards, riverCards, onChange, onTurnCardsChange, onRiverCardsChange }: BoardPickerProps) {
  const [pickerTarget, setPickerTarget] = useState<PickerTarget | null>(null);

  const allUsedCards = [...board, ...turnCards, ...riverCards];

  const addCard = (card: string) => {
    if (allUsedCards.includes(card)) return;

    if (pickerTarget === 'board') {
      if (board.length >= 3) return;
      const next = [...board, card];
      onChange(next);
      if (next.length >= 3) setPickerTarget(null);
    } else if (pickerTarget === 'turn') {
      if (turnCards.length >= 5) return;
      onTurnCardsChange([...turnCards, card]);
    } else if (pickerTarget === 'river') {
      if (riverCards.length >= 5) return;
      onRiverCardsChange([...riverCards, card]);
    }
  };

  const removeCard = (target: PickerTarget, index: number) => {
    if (target === 'board') onChange(board.filter((_, i) => i !== index));
    else if (target === 'turn') onTurnCardsChange(turnCards.filter((_, i) => i !== index));
    else onRiverCardsChange(riverCards.filter((_, i) => i !== index));
  };

  const togglePicker = (target: PickerTarget) => {
    setPickerTarget(pickerTarget === target ? null : target);
  };

  return (
    <div className="flex flex-col gap-3">
      {/* Flop */}
      <div>
        <div className="text-sm font-medium mb-1" style={{ color: 'var(--text-secondary)' }}>
          Board (Flop) *
        </div>
        <div className="flex gap-2 items-center">
          {[0, 1, 2].map(i => (
            <CardSlot key={i} card={board[i]} onRemove={() => removeCard('board', i)} onPick={() => togglePicker('board')} placeholder="?" />
          ))}
          <button
            className="px-3 py-1 rounded text-sm"
            style={{ background: 'var(--bg-secondary)', color: 'var(--text-primary)' }}
            onClick={() => togglePicker('board')}
          >
            {pickerTarget === 'board' ? 'Close' : 'Pick'}
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
      </div>

      {/* Turn Cards */}
      <div>
        <div className="text-sm font-medium mb-1" style={{ color: 'var(--text-secondary)' }}>
          Turn Cards <span className="font-normal">(optional, max 5)</span>
        </div>
        <div className="flex gap-2 items-center flex-wrap">
          {turnCards.map((card, i) => (
            <CardSlot key={i} card={card} onRemove={() => removeCard('turn', i)} onPick={() => {}} />
          ))}
          {turnCards.length < 5 && (
            <button
              className="px-3 py-1 rounded text-sm"
              style={{ background: 'var(--bg-secondary)', color: 'var(--text-primary)' }}
              onClick={() => togglePicker('turn')}
            >
              {pickerTarget === 'turn' ? 'Close' : '+ Add Turn Card'}
            </button>
          )}
          {turnCards.length > 0 && (
            <button
              className="px-2 py-1 rounded text-xs"
              style={{ background: 'var(--danger)', color: '#fff' }}
              onClick={() => onTurnCardsChange([])}
            >
              Clear
            </button>
          )}
        </div>
      </div>

      {/* River Cards */}
      <div>
        <div className="text-sm font-medium mb-1" style={{ color: 'var(--text-secondary)' }}>
          River Cards <span className="font-normal">(optional, max 5)</span>
        </div>
        <div className="flex gap-2 items-center flex-wrap">
          {riverCards.map((card, i) => (
            <CardSlot key={i} card={card} onRemove={() => removeCard('river', i)} onPick={() => {}} />
          ))}
          {riverCards.length < 5 && (
            <button
              className="px-3 py-1 rounded text-sm"
              style={{ background: 'var(--bg-secondary)', color: 'var(--text-primary)' }}
              onClick={() => togglePicker('river')}
            >
              {pickerTarget === 'river' ? 'Close' : '+ Add River Card'}
            </button>
          )}
          {riverCards.length > 0 && (
            <button
              className="px-2 py-1 rounded text-xs"
              style={{ background: 'var(--danger)', color: '#fff' }}
              onClick={() => onRiverCardsChange([])}
            >
              Clear
            </button>
          )}
        </div>
      </div>

      {/* Card Picker */}
      {pickerTarget && (
        <div className="p-2 rounded" style={{ background: 'var(--bg-secondary)' }}>
          <div className="text-xs mb-1" style={{ color: 'var(--text-secondary)' }}>
            Selecting: {pickerTarget === 'board' ? 'Flop' : pickerTarget === 'turn' ? 'Turn' : 'River'} cards
          </div>
          {SUITS.map(suit => (
            <div key={suit.char} className="flex gap-1 mb-1">
              {RANKS.map(rank => {
                const card = `${rank}${suit.char}`;
                const used = allUsedCards.includes(card);
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
                    onClick={() => addCard(card)}
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

function CardSlot({ card, onRemove, onPick, placeholder }: {
  card?: string; onRemove: () => void; onPick: () => void; placeholder?: string;
}) {
  return (
    <div
      className="w-10 h-14 rounded flex items-center justify-center text-sm font-bold cursor-pointer"
      style={{
        background: card ? 'var(--bg-card)' : 'var(--bg-secondary)',
        border: card ? '2px solid var(--accent)' : '2px dashed #444',
      }}
      onClick={() => card ? onRemove() : onPick()}
    >
      {card ? (
        <span>
          {card[0]}
          <span style={{ color: getSuitColor(card[1]) }}>
            {getSuitSymbol(card[1])}
          </span>
        </span>
      ) : placeholder || ''}
    </div>
  );
}

function getSuitSymbol(s: string): string {
  return SUITS.find(x => x.char === s)?.symbol || s;
}

function getSuitColor(s: string): string {
  return SUITS.find(x => x.char === s)?.color || '#fff';
}
