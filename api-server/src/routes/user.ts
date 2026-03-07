import { FastifyInstance, FastifyRequest } from 'fastify';
import { queryOne, query } from '../db/pool';

export async function userRoutes(app: FastifyInstance) {
  // Get profile
  app.get('/api/user/profile', {
    preHandler: [(app as any).authenticate],
  }, async (request: FastifyRequest) => {
    const userId = (request.user as any).userId;
    const user = await queryOne<{
      id: string; email: string; name: string; plan: string;
      solve_limit: number; solves_used_this_month: number;
      created_at: string;
    }>(
      'SELECT id, email, name, plan, solve_limit, solves_used_this_month, created_at FROM users WHERE id = $1',
      [userId]
    );
    return user;
  });

  // Get usage stats
  app.get('/api/user/usage', {
    preHandler: [(app as any).authenticate],
  }, async (request: FastifyRequest) => {
    const userId = (request.user as any).userId;
    const user = await queryOne<{
      plan: string; solve_limit: number; solves_used_this_month: number;
    }>(
      'SELECT plan, solve_limit, solves_used_this_month FROM users WHERE id = $1',
      [userId]
    );
    if (!user) return { error: 'User not found' };

    return {
      plan: user.plan,
      solveLimit: user.solve_limit,
      solvesUsed: user.solves_used_this_month,
      solvesRemaining: user.solve_limit === -1 ? -1 : user.solve_limit - user.solves_used_this_month,
    };
  });

  // Get solve history
  app.get('/api/user/history', {
    preHandler: [(app as any).authenticate],
  }, async (request: FastifyRequest) => {
    const userId = (request.user as any).userId;
    const jobs = await query<{
      id: string; type: string; status: string;
      exploitability: number; iterations: number;
      elapsed_seconds: number; created_at: string; completed_at: string;
    }>(
      `SELECT id, type, status, exploitability, iterations, elapsed_seconds, created_at, completed_at
       FROM solve_jobs WHERE user_id = $1 ORDER BY created_at DESC LIMIT 50`,
      [userId]
    );
    return { jobs };
  });
}
