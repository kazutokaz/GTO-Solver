import { Link, useNavigate } from 'react-router-dom';
import { useAuthStore } from '../store/authStore';

export function Navbar() {
  const { isAuthenticated, logout } = useAuthStore();
  const navigate = useNavigate();

  const handleLogout = () => {
    logout();
    navigate('/login');
  };

  return (
    <nav className="flex items-center justify-between px-6 py-3" style={{ background: 'var(--bg-secondary)', borderBottom: '1px solid #2a3a4a' }}>
      <Link to="/" className="text-lg font-bold no-underline" style={{ color: 'var(--accent)' }}>
        GTO Exploit Solver
      </Link>

      <div className="flex gap-4 items-center">
        {isAuthenticated ? (
          <>
            <Link to="/app/solve" className="text-sm no-underline" style={{ color: 'var(--text-primary)' }}>
              Solve
            </Link>
            <Link to="/app/history" className="text-sm no-underline" style={{ color: 'var(--text-primary)' }}>
              History
            </Link>
            <Link to="/app/settings" className="text-sm no-underline" style={{ color: 'var(--text-primary)' }}>
              Settings
            </Link>
            <button
              className="text-sm px-3 py-1 rounded"
              style={{ background: 'var(--bg-card)', color: 'var(--text-secondary)' }}
              onClick={handleLogout}
            >
              Logout
            </button>
          </>
        ) : (
          <>
            <Link to="/login" className="text-sm no-underline" style={{ color: 'var(--text-primary)' }}>
              Login
            </Link>
            <Link to="/signup" className="text-sm px-3 py-1 rounded no-underline"
              style={{ background: 'var(--accent)', color: '#fff' }}>
              Sign Up
            </Link>
          </>
        )}
      </div>
    </nav>
  );
}
