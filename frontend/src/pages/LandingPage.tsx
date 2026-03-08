import { Link } from 'react-router-dom';

const FEATURES = [
  {
    title: 'Custom Range Solve',
    desc: 'Input any OOP/IP range with our visual 13x13 hand matrix. PioSOLVER-compatible range strings supported.',
  },
  {
    title: 'Node Locking with Street Chaining',
    desc: 'Lock opponent strategies at any node. Changes propagate across streets — a feature only PioSOLVER offered until now.',
  },
  {
    title: 'Rake & Real-World Settings',
    desc: 'Configure rake percentage, cap, and no-flop-no-drop for cash game accuracy.',
  },
  {
    title: 'Aggregate Analysis',
    desc: 'Solve up to 1755 unique flops at once. Sort by EV, equity, EQR, or board texture. Export results as CSV.',
  },
  {
    title: 'Cloud-Based — No Install',
    desc: 'Run solves from any browser. No high-spec PC required. Results delivered via real-time WebSocket updates.',
  },
  {
    title: 'Configurable Bet Sizes',
    desc: 'Set custom bet and raise sizes per street for IP and OOP. Donk bet support included.',
  },
];

const COMPARISON = [
  { feature: 'Custom Range', us: true, wizard: false, deep: true, pio: true },
  { feature: 'Node Lock', us: true, wizard: false, deep: 'Partial', pio: true },
  { feature: 'Street-Chained Node Lock', us: true, wizard: false, deep: false, pio: true },
  { feature: 'Rake Config', us: true, wizard: true, deep: false, pio: true },
  { feature: 'Aggregate Analysis', us: true, wizard: 'Limited', deep: 'Limited', pio: true },
  { feature: 'CSV Export', us: true, wizard: false, deep: true, pio: true },
  { feature: 'Cloud (Browser)', us: true, wizard: true, deep: true, pio: false },
  { feature: 'Custom Bet Sizes', us: true, wizard: false, deep: 'Limited', pio: true },
];

const PLANS = [
  { name: 'Free', price: '$0', period: '', solves: '10 solves/mo', agg: 'No aggregate', cta: 'Start Free' },
  { name: 'Starter', price: '$29', period: '/mo', solves: '100 solves/mo', agg: 'No aggregate', cta: 'Get Started' },
  { name: 'Pro', price: '$69', period: '/mo', solves: '500 solves/mo', agg: '5 aggregates/mo', cta: 'Go Pro', highlight: true },
  { name: 'Unlimited', price: '$149', period: '/mo', solves: 'Unlimited solves', agg: 'Unlimited aggregates', cta: 'Go Unlimited' },
];

function renderCell(val: boolean | string) {
  if (val === true) return <span style={{ color: 'var(--success)' }}>&#10003;</span>;
  if (val === false) return <span style={{ color: '#555' }}>&#10007;</span>;
  return <span className="text-xs" style={{ color: 'var(--warning)' }}>{val}</span>;
}

export function LandingPage() {
  return (
    <div>
      {/* Hero */}
      <section className="max-w-4xl mx-auto px-4 py-20 text-center">
        <h1 className="text-4xl font-bold mb-4">
          Beat Your Opponents,<br />Not Just Study GTO
        </h1>
        <p className="text-lg mb-2" style={{ color: 'var(--text-secondary)' }}>
          Cloud-based poker solver that calculates optimal exploit strategies against specific opponents.
        </p>
        <p className="text-sm mb-8" style={{ color: 'var(--text-secondary)' }}>
          PioSOLVER-level features. No install. No high-spec PC.
        </p>

        <div className="flex gap-4 justify-center">
          <Link to="/signup" className="px-8 py-3 rounded-lg font-medium text-lg no-underline"
            style={{ background: 'var(--accent)', color: '#fff' }}>
            Get Started Free
          </Link>
          <Link to="/login" className="px-8 py-3 rounded-lg font-medium text-lg no-underline"
            style={{ background: 'var(--bg-secondary)', color: 'var(--text-primary)', border: '1px solid #333' }}>
            Login
          </Link>
        </div>
      </section>

      {/* Features */}
      <section className="max-w-5xl mx-auto px-4 py-12">
        <h2 className="text-2xl font-bold text-center mb-8">Features</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {FEATURES.map(f => (
            <div key={f.title} className="p-5 rounded-lg" style={{ background: 'var(--bg-secondary)' }}>
              <h3 className="font-bold mb-2">{f.title}</h3>
              <p className="text-sm" style={{ color: 'var(--text-secondary)' }}>{f.desc}</p>
            </div>
          ))}
        </div>
      </section>

      {/* Comparison */}
      <section className="max-w-4xl mx-auto px-4 py-12">
        <h2 className="text-2xl font-bold text-center mb-8">How We Compare</h2>
        <div style={{ overflowX: 'auto' }}>
          <table className="w-full text-sm">
            <thead>
              <tr style={{ borderBottom: '1px solid #333' }}>
                <th className="text-left py-2 px-3" style={{ color: 'var(--text-secondary)' }}>Feature</th>
                <th className="py-2 px-3 text-center font-bold" style={{ color: 'var(--accent)' }}>GTO Exploit</th>
                <th className="py-2 px-3 text-center" style={{ color: 'var(--text-secondary)' }}>GTO Wizard</th>
                <th className="py-2 px-3 text-center" style={{ color: 'var(--text-secondary)' }}>DeepSolver</th>
                <th className="py-2 px-3 text-center" style={{ color: 'var(--text-secondary)' }}>PioSOLVER</th>
              </tr>
            </thead>
            <tbody>
              {COMPARISON.map(row => (
                <tr key={row.feature} style={{ borderBottom: '1px solid #222' }}>
                  <td className="py-2 px-3">{row.feature}</td>
                  <td className="py-2 px-3 text-center">{renderCell(row.us)}</td>
                  <td className="py-2 px-3 text-center">{renderCell(row.wizard)}</td>
                  <td className="py-2 px-3 text-center">{renderCell(row.deep)}</td>
                  <td className="py-2 px-3 text-center">{renderCell(row.pio)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      {/* Pricing */}
      <section className="max-w-5xl mx-auto px-4 py-12">
        <h2 className="text-2xl font-bold text-center mb-8">Pricing</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {PLANS.map(plan => (
            <div key={plan.name} className="p-5 rounded-lg flex flex-col" style={{
              background: 'var(--bg-secondary)',
              border: plan.highlight ? '2px solid var(--accent)' : '2px solid transparent',
            }}>
              {plan.highlight && (
                <div className="text-xs font-bold mb-2 text-center" style={{ color: 'var(--accent)' }}>
                  MOST POPULAR
                </div>
              )}
              <div className="text-lg font-bold">{plan.name}</div>
              <div className="mt-2">
                <span className="text-3xl font-bold">{plan.price}</span>
                <span className="text-sm" style={{ color: 'var(--text-secondary)' }}>{plan.period}</span>
              </div>
              <div className="text-sm mt-3" style={{ color: 'var(--text-secondary)' }}>{plan.solves}</div>
              <div className="text-sm" style={{ color: 'var(--text-secondary)' }}>{plan.agg}</div>
              <Link to="/signup" className="mt-auto pt-4 block text-center py-2 rounded font-medium no-underline"
                style={{
                  background: plan.highlight ? 'var(--accent)' : 'var(--bg-card)',
                  color: plan.highlight ? '#fff' : 'var(--text-primary)',
                }}>
                {plan.cta}
              </Link>
            </div>
          ))}
        </div>
      </section>

      {/* Footer CTA */}
      <section className="max-w-4xl mx-auto px-4 py-16 text-center">
        <h2 className="text-2xl font-bold mb-3">Ready to exploit your opponents?</h2>
        <p className="text-sm mb-6" style={{ color: 'var(--text-secondary)' }}>
          Start with 10 free solves. No credit card required.
        </p>
        <Link to="/signup" className="px-8 py-3 rounded-lg font-medium text-lg no-underline"
          style={{ background: 'var(--accent)', color: '#fff' }}>
          Create Free Account
        </Link>
      </section>
    </div>
  );
}
