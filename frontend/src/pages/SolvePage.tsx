import { RangeEditor } from '../components/RangeEditor';
import { BoardPicker } from '../components/BoardPicker';
import { BetSizeConfig } from '../components/BetSizeConfig';
import { RakeConfig } from '../components/RakeConfig';
import { StrategyMatrix } from '../components/StrategyMatrix';
import { GameTreeNav } from '../components/GameTreeNav';
import { useSolveStore } from '../store/solveStore';

export function SolvePage() {
  const {
    stackSize, potSize, board, oopRange, ipRange, betSizes, rake,
    status, result, error, jobId,
    setBoard, setOopRange, setIpRange, setStackSize, setPotSize,
    setBetSizes, setRake,
    submitSolve, reset,
  } = useSolveStore();

  const canSubmit = board.length >= 3 && oopRange && ipRange && status === 'idle';

  return (
    <div className="max-w-6xl mx-auto p-4">
      <h1 className="text-xl font-bold mb-4">New Solve</h1>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Left: Input */}
        <div className="flex flex-col gap-4">
          {/* Stack & Pot */}
          <div className="flex gap-4">
            <div className="flex-1">
              <label className="block text-sm mb-1" style={{ color: 'var(--text-secondary)' }}>
                Effective Stack (BB)
              </label>
              <input type="number" value={stackSize} onChange={e => setStackSize(+e.target.value)}
                className="w-full p-2 rounded text-sm"
                style={{ background: 'var(--bg-secondary)', color: 'var(--text-primary)', border: '1px solid #333' }}
                min={1} />
            </div>
            <div className="flex-1">
              <label className="block text-sm mb-1" style={{ color: 'var(--text-secondary)' }}>
                Pot Size (BB)
              </label>
              <input type="number" value={potSize} onChange={e => setPotSize(+e.target.value)}
                className="w-full p-2 rounded text-sm"
                style={{ background: 'var(--bg-secondary)', color: 'var(--text-primary)', border: '1px solid #333' }}
                min={0} step={0.5} />
            </div>
          </div>

          {/* Board */}
          <BoardPicker board={board} onChange={setBoard} />

          {/* Ranges */}
          <div className="grid grid-cols-2 gap-4">
            <RangeEditor value={oopRange} onChange={setOopRange} label="OOP Range" />
            <RangeEditor value={ipRange} onChange={setIpRange} label="IP Range" />
          </div>

          {/* Bet Sizes & Rake */}
          <div className="flex flex-col gap-2">
            <BetSizeConfig value={betSizes} onChange={setBetSizes} />
            <RakeConfig value={rake} onChange={setRake} />
          </div>

          {/* Submit */}
          <div className="flex gap-3 items-center">
            <button
              className="px-6 py-2 rounded font-medium"
              style={{
                background: canSubmit ? 'var(--accent)' : '#555',
                color: '#fff',
                cursor: canSubmit ? 'pointer' : 'not-allowed',
              }}
              disabled={!canSubmit}
              onClick={submitSolve}
            >
              Solve
            </button>

            {status !== 'idle' && (
              <button
                className="px-4 py-2 rounded text-sm"
                style={{ background: 'var(--bg-secondary)', color: 'var(--text-secondary)' }}
                onClick={reset}
              >
                Reset
              </button>
            )}
          </div>

          {/* Status */}
          {status === 'queued' && (
            <div className="p-3 rounded text-sm" style={{ background: 'var(--bg-secondary)' }}>
              Queued... waiting for worker
            </div>
          )}
          {status === 'running' && (
            <div className="p-3 rounded text-sm" style={{ background: 'var(--bg-secondary)' }}>
              Solving... please wait
            </div>
          )}
          {status === 'failed' && (
            <div className="p-3 rounded text-sm" style={{ background: 'rgba(224,36,94,0.2)', color: 'var(--danger)' }}>
              Failed: {error}
            </div>
          )}
        </div>

        {/* Right: Results */}
        <div>
          {result && (
            <div className="flex flex-col gap-4">
              <div className="p-3 rounded" style={{ background: 'var(--bg-secondary)' }}>
                <div className="text-sm" style={{ color: 'var(--text-secondary)' }}>
                  Iterations: {result.iterations} | Exploitability: {(result.exploitability * 100).toFixed(2)}% |
                  Time: {result.elapsedSeconds?.toFixed(1)}s
                </div>
              </div>

              {result.result && (
                <div>
                  <h3 className="text-sm font-medium mb-2" style={{ color: 'var(--text-secondary)' }}>
                    Game Tree
                  </h3>
                  {result.result.children ? (
                    <GameTreeNav root={result.result} />
                  ) : (
                    <>
                      <div className="text-xs mb-2" style={{ color: 'var(--text-secondary)' }}>
                        Root Strategy ({result.result.player?.toUpperCase()})
                      </div>
                      <StrategyMatrix
                        strategy={result.result.strategy || {}}
                        actions={result.result.actions || []}
                      />
                    </>
                  )}
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
