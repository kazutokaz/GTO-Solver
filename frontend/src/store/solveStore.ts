import { create } from 'zustand';
import { api } from '../api/client';

interface SolveState {
  // Input
  stackSize: number;
  potSize: number;
  board: string[];
  oopRange: string;
  ipRange: string;

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

  jobId: null,
  status: 'idle',
  result: null,
  error: null,

  setBoard: (board) => set({ board }),
  setOopRange: (range) => set({ oopRange: range }),
  setIpRange: (range) => set({ ipRange: range }),
  setStackSize: (size) => set({ stackSize: size }),
  setPotSize: (size) => set({ potSize: size }),

  submitSolve: async () => {
    const { stackSize, potSize, board, oopRange, ipRange } = get();
    set({ status: 'queued', error: null, result: null });

    try {
      const { jobId } = await api.submitSolve({
        game: { stackSize, potSize, board, oopRange, ipRange },
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
