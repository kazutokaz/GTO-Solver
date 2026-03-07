import { create } from 'zustand';
import { api } from '../api/client';

interface AuthState {
  token: string | null;
  userId: string | null;
  isAuthenticated: boolean;
  login: (email: string, password: string) => Promise<void>;
  signup: (email: string, password: string, name?: string) => Promise<void>;
  logout: () => void;
  loadFromStorage: () => void;
}

export const useAuthStore = create<AuthState>((set) => ({
  token: null,
  userId: null,
  isAuthenticated: false,

  login: async (email, password) => {
    const { token, userId } = await api.login(email, password);
    localStorage.setItem('token', token);
    set({ token, userId, isAuthenticated: true });
  },

  signup: async (email, password, name) => {
    const { token, userId } = await api.signup(email, password, name);
    localStorage.setItem('token', token);
    set({ token, userId, isAuthenticated: true });
  },

  logout: () => {
    localStorage.removeItem('token');
    set({ token: null, userId: null, isAuthenticated: false });
  },

  loadFromStorage: () => {
    const token = localStorage.getItem('token');
    if (token) {
      set({ token, isAuthenticated: true });
    }
  },
}));
