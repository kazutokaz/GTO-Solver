import { FastifyInstance, FastifyRequest } from 'fastify';
import { v4 as uuidv4 } from 'uuid';
import { z } from 'zod';
import { query, queryOne } from '../db/pool';
import { addSolveJob } from '../services/queue';
import { checkSolveLimit, incrementSolveCount } from '../services/billing';

const solveRequestSchema = z.object({
  game: z.object({
    stackSize: z.number().positive(),
    potSize: z.number().positive(),
    board: z.array(z.string()).min(3).max(5),
    oopRange: z.string().min(1),
    ipRange: z.string().min(1),
  }),
  betSizes: z.object({
    flop: z.any().optional(),
    turn: z.any().optional(),
    river: z.any().optional(),
  }).optional(),
  rake: z.object({
    percentage: z.number().min(0).max(1),
    cap: z.number().min(0),
    noFlopNoDrop: z.boolean(),
  }).optional(),
  nodeLocks: z.array(z.any()).optional(),
  solveConfig: z.object({
    maxIterations: z.number().optional(),
    targetExploitability: z.number().optional(),
    timeoutSeconds: z.number().optional(),
  }).optional(),
});

export async function solveRoutes(app: FastifyInstance) {
  // Submit solve job
  app.post('/api/solve', {
    preHandler: [(app as any).authenticate],
  }, async (request: FastifyRequest, reply) => {
    const userId = (request.user as any).userId;

    // Check solve limit
    const canSolve = await checkSolveLimit(userId);
    if (!canSolve) {
      return reply.code(429).send({ error: 'Monthly solve limit reached. Upgrade your plan.' });
    }

    const body = solveRequestSchema.parse(request.body);
    const jobId = uuidv4();

    // Build CFR engine input
    const cfrInput = {
      job_id: jobId,
      game: {
        stack_size: body.game.stackSize,
        pot_size: body.game.potSize,
        board: body.game.board,
        players: {
          oop: { range: body.game.oopRange },
          ip: { range: body.game.ipRange },
        },
      },
      bet_sizes: body.betSizes || undefined,
      rake: body.rake ? {
        percentage: body.rake.percentage,
        cap: body.rake.cap,
        no_flop_no_drop: body.rake.noFlopNoDrop,
      } : undefined,
      node_locks: body.nodeLocks || undefined,
      solve_config: body.solveConfig ? {
        max_iterations: body.solveConfig.maxIterations,
        target_exploitability: body.solveConfig.targetExploitability,
        timeout_seconds: body.solveConfig.timeoutSeconds,
      } : undefined,
    };

    // Save job to DB
    await query(
      'INSERT INTO solve_jobs (id, user_id, type, status, input) VALUES ($1, $2, $3, $4, $5)',
      [jobId, userId, 'single', 'queued', JSON.stringify(cfrInput)]
    );

    // Increment solve count
    await incrementSolveCount(userId);

    // Add to queue
    await addSolveJob({ jobId, userId, input: cfrInput });

    return {
      jobId,
      status: 'queued',
    };
  });

  // Get solve result
  app.get('/api/solve/:jobId', {
    preHandler: [(app as any).authenticate],
  }, async (request: FastifyRequest<{ Params: { jobId: string } }>, reply) => {
    const userId = (request.user as any).userId;
    const { jobId } = request.params;

    const job = await queryOne<{
      id: string; status: string; result: any;
      exploitability: number; iterations: number;
      elapsed_seconds: number; error_message: string;
    }>(
      'SELECT id, status, result, exploitability, iterations, elapsed_seconds, error_message FROM solve_jobs WHERE id = $1 AND user_id = $2',
      [jobId, userId]
    );

    if (!job) {
      return reply.code(404).send({ error: 'Job not found' });
    }

    return {
      jobId: job.id,
      status: job.status,
      result: job.result,
      exploitability: job.exploitability,
      iterations: job.iterations,
      elapsedSeconds: job.elapsed_seconds,
      error: job.error_message,
    };
  });

  // Get solve status
  app.get('/api/solve/:jobId/status', {
    preHandler: [(app as any).authenticate],
  }, async (request: FastifyRequest<{ Params: { jobId: string } }>, reply) => {
    const userId = (request.user as any).userId;
    const { jobId } = request.params;

    const job = await queryOne<{ status: string }>(
      'SELECT status FROM solve_jobs WHERE id = $1 AND user_id = $2',
      [jobId, userId]
    );

    if (!job) {
      return reply.code(404).send({ error: 'Job not found' });
    }

    return { jobId, status: job.status };
  });

  // Cancel solve job
  app.delete('/api/solve/:jobId', {
    preHandler: [(app as any).authenticate],
  }, async (request: FastifyRequest<{ Params: { jobId: string } }>, reply) => {
    const userId = (request.user as any).userId;
    const { jobId } = request.params;

    const job = await queryOne<{ status: string }>(
      'SELECT status FROM solve_jobs WHERE id = $1 AND user_id = $2',
      [jobId, userId]
    );

    if (!job) {
      return reply.code(404).send({ error: 'Job not found' });
    }

    if (job.status === 'queued' || job.status === 'running') {
      await query(
        "UPDATE solve_jobs SET status = 'cancelled', completed_at = NOW() WHERE id = $1",
        [jobId]
      );
      return { jobId, status: 'cancelled' };
    }

    return { jobId, status: job.status };
  });
}
