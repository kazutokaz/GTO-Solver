const API_BASE = '/api';

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
  const token = localStorage.getItem('token');
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(options.headers as Record<string, string> || {}),
  };
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }

  const res = await fetch(`${API_BASE}${path}`, { ...options, headers });

  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(body.error || `HTTP ${res.status}`);
  }

  return res.json();
}

export const api = {
  // Auth
  signup: (email: string, password: string, name?: string) =>
    request<{ token: string; userId: string }>('/auth/signup', {
      method: 'POST',
      body: JSON.stringify({ email, password, name }),
    }),
  login: (email: string, password: string) =>
    request<{ token: string; userId: string; plan: string }>('/auth/login', {
      method: 'POST',
      body: JSON.stringify({ email, password }),
    }),

  // Solve
  submitSolve: (input: any) =>
    request<{ jobId: string; status: string }>('/solve', {
      method: 'POST',
      body: JSON.stringify(input),
    }),
  getSolveResult: (jobId: string) =>
    request<any>(`/solve/${jobId}`),
  getSolveStatus: (jobId: string) =>
    request<{ jobId: string; status: string }>(`/solve/${jobId}/status`),

  // User
  getProfile: () => request<any>('/user/profile'),
  getUsage: () => request<any>('/user/usage'),
  getHistory: () => request<any>('/user/history'),

  // Billing
  subscribe: (plan: string) =>
    request<{ url: string }>('/billing/subscribe', {
      method: 'POST',
      body: JSON.stringify({ plan }),
    }),
  getBillingStatus: () => request<any>('/billing/status'),
};
