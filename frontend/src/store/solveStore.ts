import { create } from 'zustand';
import { api } from '../api/client';
import type { FullBetSizeConfig } from '../components/BetSizeConfig';
import type { RakeSettings } from '../components/RakeConfig';

interface SolveState {
  // Input
  stackSize: number;
  potSize: number;
  board: string[];
  oopRange: string;
  ipRange: string;
  betSizes: FullBetSizeConfig | null;
  rake: RakeSettings | null;

  // Result
  jobId: string | null;
  status: 'idle' | 'queued' | 'running' | 'completed' | 'failed';
  result: any | null;
  error: string | null;

  // Actions
  setBoard: (board: string[]) => void;
  setOopRange: (range: string) => void;
  setIpRange: (range: string) => void;
  setStackSize: (size: number) => void;
  setPotSize: (size: number) => void;
  setBetSizes: (config: FullBetSizeConfig | null) => void;
  setRake: (config: RakeSettings | null) => void;
  submitSolve: () => Promise<void>;
  pollResult: (jobId: string) => Promise<void>;
  reset: () => void;
}

export const useSolveStore = create<SolveState>((set, get) => ({
  stackSize: 100,
  potSize: 6.5,
  board: [],
  oopRange: '',
  ipRange: '',
  betSizes: null,
  rake: null,

  jobId: null,
  status: 'idle',
  result: null,
  error: null,

  setBoard: (board) => set({ board }),
  setOopRange: (range) => set({ oopRange: range }),
  setIpRange: (range) => set({ ipRange: range }),
  setStackSize: (size) => set({ stackSize: size }),
  setPotSize: (size) => set({ potSize: size }),
  setBetSizes: (config) => set({ betSizes: config }),
  setRake: (config) => set({ rake: config }),

  submitSolve: async () => {
    const { stackSize, potSize, board, oopRange, ipRange, betSizes, rake } = get();
    set({ status: 'queued', error: null, result: null });

    try {
      const { jobId } = await api.submitSolve({
        game: { stackSize, potSize, board, oopRange, ipRange },
        betSizes: betSizes || undefined,
        rake: rake || undefined,
      });
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
