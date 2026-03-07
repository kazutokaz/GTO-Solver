# GTO Exploit Solver

Cloud-based poker GTO solver that calculates optimal exploit strategies against specific opponents.

## Architecture

```
Frontend (React + Vite)  →  API Server (Fastify)  →  Worker (BullMQ)  →  CFR Engine (Rust)
                                    ↕                      ↕
                              PostgreSQL                 Redis
```

## Components

### CFR Engine (`cfr-engine/`)
Rust-based Discounted CFR solver with JSON CLI interface.

- DCFR algorithm (alpha=1.5, beta=0.0, gamma=2.0)
- Postflop game tree generation with configurable bet sizes
- PioSOLVER-compatible range parser
- Rake calculation with no-flop-no-drop support
- Node locking with street-chained propagation
- 5/7-card hand evaluation and equity computation

### API Server (`api-server/`)
Node.js Fastify server handling authentication, job management, and billing.

- JWT authentication
- BullMQ job queue for async solve processing
- WebSocket real-time progress notifications
- Stripe subscription billing (Free/Starter/Pro/Unlimited)
- Aggregate analysis (multi-flop bulk solving)

### Frontend (`frontend/`)
React SPA with poker-specific UI components.

- 13x13 hand matrix range editor with drag selection
- Visual board card picker
- Game tree navigation and strategy display
- Aggregate analysis with sortable results table and CSV export

## Prerequisites

- Rust (stable, GNU toolchain on Windows)
- Node.js 18+
- PostgreSQL 16
- Redis 7

## Quick Start

### 1. Start infrastructure

```bash
docker compose up -d
```

This starts PostgreSQL (port 5432) and Redis (port 6379).

### 2. Build CFR Engine

```bash
cd cfr-engine
cargo build --release
```

### 3. Setup API Server

```bash
cd api-server
npm install
cp .env.example .env   # Edit with your config
npm run build
npm run migrate        # Create database tables
npm start              # Start API server on port 3000
```

In a separate terminal, start the worker:
```bash
cd api-server
npm run worker
```

### 4. Start Frontend

```bash
cd frontend
npm install
npm run dev            # Starts on port 5173
```

Open http://localhost:5173 in your browser.

## Configuration

Environment variables for `api-server/.env`:

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `3000` | API server port |
| `DATABASE_URL` | `postgresql://postgres:postgres@localhost:5432/gto_solver` | PostgreSQL connection |
| `REDIS_URL` | `redis://localhost:6379` | Redis connection |
| `JWT_SECRET` | `dev-secret-change-in-production` | JWT signing secret |
| `STRIPE_SECRET_KEY` | (empty) | Stripe API key |
| `STRIPE_WEBHOOK_SECRET` | (empty) | Stripe webhook secret |
| `CFR_ENGINE_PATH` | `../cfr-engine/target/release/cfr_engine.exe` | Path to CFR binary |

## Testing

### Rust CFR Engine
```bash
cd cfr-engine
cargo test
```

29 tests covering hand evaluation, range parsing, game tree construction, and solver convergence.

### API Server
```bash
cd api-server
npm run build          # TypeScript compilation check
```

### Frontend
```bash
cd frontend
npx tsc --noEmit       # TypeScript compilation check
```

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/auth/signup` | User registration |
| POST | `/api/auth/login` | Login |
| POST | `/api/solve` | Submit solve job |
| GET | `/api/solve/:jobId` | Get solve result |
| GET | `/api/solve/:jobId/status` | Check solve status |
| DELETE | `/api/solve/:jobId` | Cancel solve job |
| POST | `/api/aggregate` | Submit aggregate analysis |
| GET | `/api/aggregate/:jobId` | Get aggregate results |
| GET | `/api/aggregate/:jobId/csv` | Download CSV |
| GET | `/api/user/profile` | User profile |
| GET | `/api/user/usage` | Monthly usage stats |
| GET | `/api/user/history` | Solve history |
| POST | `/api/billing/subscribe` | Subscribe to plan |
| GET | `/api/billing/status` | Billing status |
| GET | `/api/health` | Health check |

## Subscription Plans

| Plan | Price | Solves/month | Aggregate Analysis |
|------|-------|-------------|-------------------|
| Free | $0 | 10 | No |
| Starter | $29 | 100 | No |
| Pro | $69 | 500 | 5/month |
| Unlimited | $149 | Unlimited | Unlimited |

## License

Private - All rights reserved.
