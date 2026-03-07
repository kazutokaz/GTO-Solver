import { useEffect, useState } from 'react';
import { api } from '../api/client';

const PLANS = [
  { id: 'free', name: 'Free', price: '$0/mo', limit: '10 solves' },
  { id: 'starter', name: 'Starter', price: '$29/mo', limit: '100 solves' },
  { id: 'pro', name: 'Pro', price: '$69/mo', limit: '500 solves + Aggregate' },
  { id: 'unlimited', name: 'Unlimited', price: '$149/mo', limit: 'Unlimited' },
];

export function SettingsPage() {
  const [profile, setProfile] = useState<any>(null);
  const [usage, setUsage] = useState<any>(null);

  useEffect(() => {
    api.getProfile().then(setProfile).catch(() => {});
    api.getUsage().then(setUsage).catch(() => {});
  }, []);

  const handleSubscribe = async (plan: string) => {
    try {
      const { url } = await api.subscribe(plan);
      if (url) window.location.href = url;
    } catch (err: any) {
      alert(err.message);
    }
  };

  return (
    <div className="max-w-4xl mx-auto p-4">
      <h1 className="text-xl font-bold mb-4">Settings</h1>

      {/* Profile */}
      <div className="p-4 rounded-lg mb-6" style={{ background: 'var(--bg-secondary)' }}>
        <h2 className="text-lg font-medium mb-2">Profile</h2>
        {profile ? (
          <div className="text-sm" style={{ color: 'var(--text-secondary)' }}>
            <div>Email: {profile.email}</div>
            <div>Plan: <span className="font-medium" style={{ color: 'var(--accent)' }}>{profile.plan}</span></div>
          </div>
        ) : (
          <div style={{ color: 'var(--text-secondary)' }}>Loading...</div>
        )}
      </div>

      {/* Usage */}
      {usage && (
        <div className="p-4 rounded-lg mb-6" style={{ background: 'var(--bg-secondary)' }}>
          <h2 className="text-lg font-medium mb-2">Usage This Month</h2>
          <div className="text-sm" style={{ color: 'var(--text-secondary)' }}>
            <div>Solves Used: {usage.solvesUsed} / {usage.solveLimit === -1 ? 'Unlimited' : usage.solveLimit}</div>
            <div className="mt-2 h-2 rounded-full overflow-hidden" style={{ background: 'var(--bg-card)' }}>
              <div
                className="h-full rounded-full"
                style={{
                  background: 'var(--accent)',
                  width: usage.solveLimit === -1 ? '10%' : `${Math.min(100, (usage.solvesUsed / usage.solveLimit) * 100)}%`,
                }}
              />
            </div>
          </div>
        </div>
      )}

      {/* Plans */}
      <h2 className="text-lg font-medium mb-3">Plans</h2>
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        {PLANS.map(plan => {
          const isCurrent = profile?.plan === plan.id;
          return (
            <div key={plan.id} className="p-4 rounded-lg" style={{
              background: 'var(--bg-secondary)',
              border: isCurrent ? '2px solid var(--accent)' : '2px solid transparent',
            }}>
              <div className="text-lg font-bold">{plan.name}</div>
              <div className="text-xl font-bold mt-1" style={{ color: 'var(--accent)' }}>{plan.price}</div>
              <div className="text-sm mt-1" style={{ color: 'var(--text-secondary)' }}>{plan.limit}</div>
              {isCurrent ? (
                <div className="mt-3 text-sm font-medium" style={{ color: 'var(--success)' }}>Current Plan</div>
              ) : plan.id !== 'free' ? (
                <button
                  className="mt-3 px-4 py-1 rounded text-sm"
                  style={{ background: 'var(--accent)', color: '#fff' }}
                  onClick={() => handleSubscribe(plan.id)}
                >
                  Upgrade
                </button>
              ) : null}
            </div>
          );
        })}
      </div>
    </div>
  );
}
