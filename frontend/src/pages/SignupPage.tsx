import { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { useAuthStore } from '../store/authStore';

export function SignupPage() {
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [name, setName] = useState('');
  const [error, setError] = useState('');
  const { signup } = useAuthStore();
  const navigate = useNavigate();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    try {
      await signup(email, password, name || undefined);
      navigate('/app/solve');
    } catch (err: any) {
      setError(err.message);
    }
  };

  return (
    <div className="flex justify-center items-center min-h-[80vh]">
      <form onSubmit={handleSubmit} className="p-6 rounded-lg w-96" style={{ background: 'var(--bg-secondary)' }}>
        <h2 className="text-xl font-bold mb-4">Sign Up</h2>
        {error && <div className="text-sm mb-3 p-2 rounded" style={{ background: 'var(--danger)', color: '#fff' }}>{error}</div>}

        <label className="block text-sm mb-1" style={{ color: 'var(--text-secondary)' }}>Name (optional)</label>
        <input type="text" value={name} onChange={e => setName(e.target.value)}
          className="w-full p-2 rounded mb-3 text-sm"
          style={{ background: 'var(--bg-card)', color: 'var(--text-primary)', border: '1px solid #333' }} />

        <label className="block text-sm mb-1" style={{ color: 'var(--text-secondary)' }}>Email</label>
        <input type="email" value={email} onChange={e => setEmail(e.target.value)}
          className="w-full p-2 rounded mb-3 text-sm"
          style={{ background: 'var(--bg-card)', color: 'var(--text-primary)', border: '1px solid #333' }}
          required />

        <label className="block text-sm mb-1" style={{ color: 'var(--text-secondary)' }}>Password</label>
        <input type="password" value={password} onChange={e => setPassword(e.target.value)}
          className="w-full p-2 rounded mb-4 text-sm"
          style={{ background: 'var(--bg-card)', color: 'var(--text-primary)', border: '1px solid #333' }}
          required minLength={8} />

        <button type="submit" className="w-full py-2 rounded font-medium"
          style={{ background: 'var(--accent)', color: '#fff' }}>
          Create Account
        </button>

        <p className="text-sm mt-3 text-center" style={{ color: 'var(--text-secondary)' }}>
          Already have an account? <Link to="/login" style={{ color: 'var(--accent)' }}>Login</Link>
        </p>
      </form>
    </div>
  );
}
