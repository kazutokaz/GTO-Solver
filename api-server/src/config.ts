import dotenv from 'dotenv';
dotenv.config();

export const config = {
  port: parseInt(process.env.PORT || '3000', 10),
  host: process.env.HOST || '0.0.0.0',

  database: {
    url: process.env.DATABASE_URL || 'postgresql://postgres:postgres@localhost:5432/gto_solver',
  },

  redis: {
    url: process.env.REDIS_URL || 'redis://localhost:6379',
  },

  jwt: {
    secret: process.env.JWT_SECRET || 'dev-secret-change-in-production',
  },

  stripe: {
    secretKey: process.env.STRIPE_SECRET_KEY || '',
    webhookSecret: process.env.STRIPE_WEBHOOK_SECRET || '',
  },

  cfrEngine: {
    path: process.env.CFR_ENGINE_PATH || '../cfr-engine/target/release/cfr_engine.exe',
  },

  plans: {
    free:      { solveLimit: 10,  aggregateLimit: 0,  priceId: '' },
    starter:   { solveLimit: 100, aggregateLimit: 0,  priceId: 'price_starter_monthly' },
    pro:       { solveLimit: 500, aggregateLimit: 5,  priceId: 'price_pro_monthly' },
    unlimited: { solveLimit: -1,  aggregateLimit: -1, priceId: 'price_unlimited_monthly' },
  } as Record<string, { solveLimit: number; aggregateLimit: number; priceId: string }>,
};
