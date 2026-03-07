import { pool } from './pool';

const migration = `
-- Users
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    name VARCHAR(255),
    password_hash VARCHAR(255),
    stripe_customer_id VARCHAR(255),
    plan VARCHAR(50) DEFAULT 'free',
    solve_limit INTEGER DEFAULT 10,
    solves_used_this_month INTEGER DEFAULT 0,
    billing_cycle_start TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Solve jobs
CREATE TABLE IF NOT EXISTS solve_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id),
    type VARCHAR(20) NOT NULL DEFAULT 'single',
    status VARCHAR(20) DEFAULT 'queued',
    input JSONB NOT NULL,
    result JSONB,
    exploitability FLOAT,
    iterations INTEGER,
    elapsed_seconds FLOAT,
    error_message TEXT,
    created_at TIMESTAMP DEFAULT NOW(),
    completed_at TIMESTAMP
);

-- Aggregate analysis jobs
CREATE TABLE IF NOT EXISTS aggregate_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id),
    status VARCHAR(20) DEFAULT 'queued',
    input JSONB NOT NULL,
    total_flops INTEGER,
    completed_flops INTEGER DEFAULT 0,
    result JSONB,
    created_at TIMESTAMP DEFAULT NOW(),
    completed_at TIMESTAMP
);

-- Aggregate flop results
CREATE TABLE IF NOT EXISTS aggregate_flop_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    aggregate_job_id UUID REFERENCES aggregate_jobs(id),
    board VARCHAR(10) NOT NULL,
    result JSONB NOT NULL,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Saved ranges
CREATE TABLE IF NOT EXISTS saved_ranges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id),
    name VARCHAR(255) NOT NULL,
    range_string TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_solve_jobs_user_id ON solve_jobs(user_id);
CREATE INDEX IF NOT EXISTS idx_solve_jobs_status ON solve_jobs(status);
CREATE INDEX IF NOT EXISTS idx_aggregate_jobs_user_id ON aggregate_jobs(user_id);
CREATE INDEX IF NOT EXISTS idx_saved_ranges_user_id ON saved_ranges(user_id);
`;

async function migrate() {
  console.log('Running migrations...');
  try {
    await pool.query(migration);
    console.log('Migrations completed successfully.');
  } catch (err) {
    console.error('Migration failed:', err);
    process.exit(1);
  } finally {
    await pool.end();
  }
}

migrate();
