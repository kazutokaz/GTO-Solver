/**
 * Dev server — no PostgreSQL, no Redis, no auth.
 * Runs CFR engine directly in-process and stores results in memory.
 */
import Fastify from 'fastify';
import cors from '@fastify/cors';
import { runCfrEngine } from './services/solver';
import { v4 as uuidv4 } from 'uuid';

const jobs = new Map<string, any>();

async function main() {
  const app = Fastify({ logger: true });
  await app.register(cors, { origin: true });

  app.post('/api/solve', async (request) => {
    const body = request.body as any;
    const jobId = uuidv4();

    const cfrInput: any = {
      job_id: jobId,
      game: {
        stack_size: body.game.stackSize,
        pot_size: body.game.potSize,
        board: body.game.board,
        turn_cards: body.game.turnCards || undefined,
        river_cards: body.game.riverCards || undefined,
        players: {
          oop: { range: body.game.oopRange },
          ip: { range: body.game.ipRange },
        },
      },
      rake: body.rake ? {
        percentage: body.rake.percentage,
        cap: body.rake.cap,
        no_flop_no_drop: body.rake.noFlopNoDrop,
      } : undefined,
      solve_config: body.solveConfig ? {
        max_iterations: body.solveConfig.maxIterations,
        target_exploitability: body.solveConfig.targetExploitability,
        timeout_seconds: body.solveConfig.timeoutSeconds,
      } : undefined,
    };

    // Convert bet sizes camelCase → snake_case
    if (body.betSizes) {
      const conv = (s: any) => s ? {
        ip_bet: s.ipBet, oop_bet: s.oopBet,
        ip_raise: s.ipRaise, oop_raise: s.oopRaise,
        oop_donk: s.oopDonk,
      } : undefined;
      cfrInput.bet_sizes = {
        flop: conv(body.betSizes.flop),
        turn: conv(body.betSizes.turn),
        river: conv(body.betSizes.river),
      };
    }

    // Pass through node locks (already in snake_case from frontend)
    if (body.nodeLocks && body.nodeLocks.length > 0) {
      cfrInput.node_locks = body.nodeLocks;
    }

    jobs.set(jobId, { status: 'running' });
    console.log(`[solve] Job ${jobId} started`);

    runCfrEngine(cfrInput).then(result => {
      console.log(`[solve] Job ${jobId} completed: ${result.iterations} iters, expl=${result.exploitability}`);
      jobs.set(jobId, {
        status: 'completed',
        result: result.solution,
        exploitability: result.exploitability,
        iterations: result.iterations,
        elapsedSeconds: result.elapsed_seconds,
      });
    }).catch(err => {
      console.error(`[solve] Job ${jobId} failed:`, err.message);
      jobs.set(jobId, { status: 'failed', error: err.message });
    });

    return { jobId, status: 'running' };
  });

  app.get('/api/solve/:jobId', async (request) => {
    const { jobId } = request.params as { jobId: string };
    const job = jobs.get(jobId);
    if (!job) return { jobId, status: 'not_found' };
    return { jobId, ...job };
  });

  app.get('/api/solve/:jobId/status', async (request) => {
    const { jobId } = request.params as { jobId: string };
    const job = jobs.get(jobId);
    return { jobId, status: job?.status || 'not_found' };
  });

  app.get('/api/health', async () => ({ status: 'ok', mode: 'dev' }));

  await app.listen({ port: 3000, host: '0.0.0.0' });
  console.log('Dev server running on http://localhost:3000 (no DB, no Redis, no auth)');
}

main();
