import { FastifyInstance, FastifyRequest } from 'fastify';
import { v4 as uuidv4 } from 'uuid';
import { z } from 'zod';
import { query, queryOne } from '../db/pool';
import { addAggregateJob } from '../services/queue';
import { checkSolveLimit } from '../services/billing';

const aggregateRequestSchema = z.object({
  game: z.object({
    stackSize: z.number().positive(),
    potSize: z.number().positive(),
    oopRange: z.string().min(1),
    ipRange: z.string().min(1),
  }),
  betSizes: z.any().optional(),
  rake: z.any().optional(),
  flopFilter: z.object({
    type: z.enum(['all', 'paired', 'monotone', 'rainbow', 'custom']),
    customFlops: z.array(z.array(z.string())).optional(),
    maxFlops: z.number().optional(),
  }).optional(),
  solveConfig: z.any().optional(),
});

export async function aggregateRoutes(app: FastifyInstance) {
  // Submit aggregate analysis
  app.post('/api/aggregate', {
    preHandler: [(app as any).authenticate],
  }, async (request: FastifyRequest, reply) => {
    const userId = (request.user as any).userId;

    // Check user has aggregate access (pro+)
    const user = await queryOne<{ plan: string }>(
      'SELECT plan FROM users WHERE id = $1',
      [userId]
    );
    if (!user || (user.plan !== 'pro' && user.plan !== 'unlimited')) {
      return reply.code(403).send({ error: 'Aggregate analysis requires Pro or Unlimited plan' });
    }

    const body = aggregateRequestSchema.parse(request.body);
    const jobId = uuidv4();

    const flopFilter = body.flopFilter || { type: 'all' as const };
    const maxFlops = flopFilter.maxFlops || 1755;

    // Generate flop list based on filter
    const flops = generateFlops(flopFilter.type, flopFilter.customFlops, maxFlops);

    await query(
      'INSERT INTO aggregate_jobs (id, user_id, status, input, total_flops) VALUES ($1, $2, $3, $4, $5)',
      [jobId, userId, 'queued', JSON.stringify(body), flops.length]
    );

    await addAggregateJob({ jobId, userId, input: { ...body, flops } });

    return { jobId, status: 'queued', totalFlops: flops.length };
  });

  // Get aggregate results
  app.get('/api/aggregate/:jobId', {
    preHandler: [(app as any).authenticate],
  }, async (request: FastifyRequest<{ Params: { jobId: string } }>, reply) => {
    const userId = (request.user as any).userId;
    const { jobId } = request.params;

    const job = await queryOne<any>(
      'SELECT * FROM aggregate_jobs WHERE id = $1 AND user_id = $2',
      [jobId, userId]
    );
    if (!job) return reply.code(404).send({ error: 'Job not found' });

    const flopResults = await query<any>(
      'SELECT board, result FROM aggregate_flop_results WHERE aggregate_job_id = $1 ORDER BY board',
      [jobId]
    );

    return {
      jobId: job.id,
      status: job.status,
      totalFlops: job.total_flops,
      completedFlops: job.completed_flops,
      results: flopResults,
    };
  });

  // CSV export
  app.get('/api/aggregate/:jobId/csv', {
    preHandler: [(app as any).authenticate],
  }, async (request: FastifyRequest<{ Params: { jobId: string } }>, reply) => {
    const userId = (request.user as any).userId;
    const { jobId } = request.params;

    const job = await queryOne<any>(
      'SELECT id FROM aggregate_jobs WHERE id = $1 AND user_id = $2',
      [jobId, userId]
    );
    if (!job) return reply.code(404).send({ error: 'Job not found' });

    const flopResults = await query<{ board: string; result: any }>(
      'SELECT board, result FROM aggregate_flop_results WHERE aggregate_job_id = $1 ORDER BY board',
      [jobId]
    );

    // Build CSV
    const headers = ['board', 'oop_ev', 'ip_ev', 'oop_equity', 'ip_equity', 'oop_eqr', 'ip_eqr'];
    const rows = flopResults.map(r => {
      const d = r.result;
      return [
        r.board,
        d.oop_ev ?? '',
        d.ip_ev ?? '',
        d.oop_equity ?? '',
        d.ip_equity ?? '',
        d.oop_eqr ?? '',
        d.ip_eqr ?? '',
      ].join(',');
    });

    const csv = [headers.join(','), ...rows].join('\n');

    reply.header('Content-Type', 'text/csv');
    reply.header('Content-Disposition', `attachment; filename="aggregate_${jobId}.csv"`);
    return csv;
  });
}

// Generate unique flop combinations (simplified)
function generateFlops(
  filterType: string,
  customFlops?: string[][],
  maxFlops: number = 1755
): string[][] {
  if (filterType === 'custom' && customFlops) {
    return customFlops.slice(0, maxFlops);
  }

  const ranks = ['A', 'K', 'Q', 'J', 'T', '9', '8', '7', '6', '5', '4', '3', '2'];
  const suits = ['s', 'h', 'd', 'c'];
  const deck: string[] = [];
  for (const r of ranks) {
    for (const s of suits) {
      deck.push(r + s);
    }
  }

  const flops: string[][] = [];
  for (let i = 0; i < deck.length && flops.length < maxFlops; i++) {
    for (let j = i + 1; j < deck.length && flops.length < maxFlops; j++) {
      for (let k = j + 1; k < deck.length && flops.length < maxFlops; k++) {
        const flop = [deck[i], deck[j], deck[k]];
        if (matchesFilter(flop, filterType)) {
          flops.push(flop);
        }
      }
    }
  }

  return flops;
}

function matchesFilter(flop: string[], filterType: string): boolean {
  const suits = flop.map(c => c[1]);
  const ranks = flop.map(c => c[0]);

  switch (filterType) {
    case 'paired':
      return new Set(ranks).size < 3;
    case 'monotone':
      return new Set(suits).size === 1;
    case 'rainbow':
      return new Set(suits).size === 3;
    default:
      return true;
  }
}
