import { create } from 'zustand';
import { api } from '../api/client';
import type { FullBetSizeConfig } from '../components/BetSizeConfig';
import type { RakeSettings } from '../components/RakeConfig';
import type { NodeLockData } from '../components/GameTreeNav';

interface SolveState {
  // Input
  stackSize: number;
  potSize: number;
  board: string[];
  turnCards: string[];
  riverCards: string[];
  oopRange: string;
  ipRange: string;
  betSizes: FullBetSizeConfig | null;
  rake: RakeSettings | null;
  nodeLocks: NodeLockData[];

  // Result
  jobId: string | null;
  status: 'idle' | 'queued' | 'running' | 'completed' | 'failed';
  result: any | null;
  error: string | null;

  // Actions
  setBoard: (board: string[]) => void;
  setTurnCards: (cards: string[]) => void;
  setRiverCards: (cards: string[]) => void;
  setOopRange: (range: string) => void;
  setIpRange: (range: string) => void;
  setStackSize: (size: number) => void;
  setPotSize: (size: number) => void;
  setBetSizes: (config: FullBetSizeConfig | null) => void;
  setRake: (config: RakeSettings | null) => void;
  addNodeLock: (lock: NodeLockData) => void;
  removeNodeLock: (actionPath: string[]) => void;
  clearNodeLocks: () => void;
  submitSolve: () => Promise<void>;
  pollResult: (jobId: string) => Promise<void>;
  reset: () => void;
}

export const useSolveStore = create<SolveState>((set, get) => ({
  stackSize: 100,
  potSize: 6.5,
  board: [],
  turnCards: [],
  riverCards: [],
  oopRange: '',
  ipRange: '',
  betSizes: null,
  rake: null,
  nodeLocks: [],

  jobId: null,
  status: 'idle',
  result: null,
  error: null,

  setBoard: (board) => set({ board }),
  setTurnCards: (cards) => set({ turnCards: cards }),
  setRiverCards: (cards) => set({ riverCards: cards }),
  setOopRange: (range) => set({ oopRange: range }),
  setIpRange: (range) => set({ ipRange: range }),
  setStackSize: (size) => set({ stackSize: size }),
  setPotSize: (size) => set({ potSize: size }),
  setBetSizes: (config) => set({ betSizes: config }),
  setRake: (config) => set({ rake: config }),

  addNodeLock: (lock) => set(state => {
    // Replace existing lock at same path, or add new
    const pathStr = JSON.stringify(lock.actionPath);
    const filtered = state.nodeLocks.filter(l => JSON.stringify(l.actionPath) !== pathStr);
    return { nodeLocks: [...filtered, lock] };
  }),

  removeNodeLock: (actionPath) => set(state => {
    const pathStr = JSON.stringify(actionPath);
    return { nodeLocks: state.nodeLocks.filter(l => JSON.stringify(l.actionPath) !== pathStr) };
  }),

  clearNodeLocks: () => set({ nodeLocks: [] }),

  submitSolve: async () => {
    const { stackSize, potSize, board, turnCards, riverCards, oopRange, ipRange, betSizes, rake, nodeLocks } = get();
    set({ status: 'queued', error: null, result: null });

    try {
      const payload: any = {
        game: {
          stackSize, potSize, board, oopRange, ipRange,
          turnCards: turnCards.length > 0 ? turnCards : undefined,
          riverCards: riverCards.length > 0 ? riverCards : undefined,
        },
        betSizes: betSizes || undefined,
        rake: rake || undefined,
      };

      if (nodeLocks.length > 0) {
        payload.nodeLocks = nodeLocks.map(lock => ({
          action_path: lock.actionPath,
          hand_strategies: lock.handStrategies,
        }));
      }

      const { jobId } = await api.submitSolve(payload);
      set({ jobId, status: 'queued' });

      // Start polling
      get().pollResult(jobId);
    } catch (err: any) {
      set({ status: 'failed', error: err.message });
    }
  },

  pollResult: async (jobId) => {
    const poll = async () => {
      try {
        const data = await api.getSolveResult(jobId);
        if (data.status === 'completed') {
          set({ status: 'completed', result: data });
        } else if (data.status === 'failed') {
          set({ status: 'failed', error: data.error });
        } else {
          set({ status: data.status as any });
          setTimeout(poll, 2000);
        }
      } catch {
        setTimeout(poll, 3000);
      }
    };
    poll();
  },

  reset: () => set({
    jobId: null,
    status: 'idle',
    result: null,
    error: null,
  }),
}));
