import { Link } from 'react-router-dom';

export function LandingPage() {
  return (
    <div className="max-w-4xl mx-auto px-4 py-16 text-center">
      <h1 className="text-4xl font-bold mb-4">
        GTO Exploit Solver
      </h1>
      <p className="text-lg mb-8" style={{ color: 'var(--text-secondary)' }}>
        Cloud-based poker solver that calculates optimal exploit strategies against specific opponents.
      </p>

      <div className="flex gap-4 justify-center mb-16">
        <Link to="/signup" className="px-6 py-3 rounded-lg font-medium text-lg no-underline"
          style={{ background: 'var(--accent)', color: '#fff' }}>
          Get Started Free
        </Link>
        <Link to="/login" className="px-6 py-3 rounded-lg font-medium text-lg no-underline"
          style={{ background: 'var(--bg-secondary)', color: 'var(--text-primary)' }}>
          Login
        </Link>
      </div>

      {/* Features */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6 text-left">
        {[
          { title: 'Custom Range Solve', desc: 'Input any OOP/IP range and get GTO-optimal strategies.' },
          { title: 'Node Locking', desc: 'Lock opponent strategies to compute exploit adjustments across streets.' },
          { title: 'Rake Support', desc: 'Configure rake percentage and cap for realistic cash game analysis.' },
          { title: 'Aggregate Analysis', desc: 'Solve 1755 flops at once and analyze trends across board textures.' },
        ].map(f => (
          <div key={f.title} className="p-4 rounded-lg" style={{ background: 'var(--bg-secondary)' }}>
            <h3 className="font-bold mb-1">{f.title}</h3>
            <p className="text-sm" style={{ color: 'var(--text-secondary)' }}>{f.desc}</p>
          </div>
        ))}
      </div>
    </div>
  );
}
