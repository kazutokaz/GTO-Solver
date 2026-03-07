import { Worker, Job } from 'bullmq';
import { config } from './config';
import { query } from './db/pool';
import { runCfrEngine, SolveResult } from './services/solver';
import { SolveJobData, redisConfig } from './services/queue';

// Solve worker
const solveWorker = new Worker<SolveJobData>(
  'solve',
  async (job: Job<SolveJobData>) => {
    const { jobId, userId, input } = job.data;

    console.log(`[Worker] Processing solve job ${jobId}`);

    // Update status to running
    await query(
      "UPDATE solve_jobs SET status = 'running' WHERE id = $1",
      [jobId]
    );

    try {
      const result: SolveResult = await runCfrEngine(input);

      // Save result to DB
      await query(
        `UPDATE solve_jobs
         SET status = 'completed',
             result = $1,
             exploitability = $2,
             iterations = $3,
             elapsed_seconds = $4,
             completed_at = NOW()
         WHERE id = $5`,
        [
          JSON.stringify(result.solution),
          result.exploitability,
          result.iterations,
          result.elapsed_seconds,
          jobId,
        ]
      );

      console.log(
        `[Worker] Solve job ${jobId} completed: ${result.iterations} iterations, ` +
        `exploitability=${result.exploitability.toFixed(6)}, ${result.elapsed_seconds.toFixed(1)}s`
      );

      return result;
    } catch (err: any) {
      console.error(`[Worker] Solve job ${jobId} failed:`, err.message);

      await query(
        "UPDATE solve_jobs SET status = 'failed', error_message = $1, completed_at = NOW() WHERE id = $2",
        [err.message, jobId]
      );

      throw err;
    }
  },
  {
    connection: redisConfig,
    concurrency: 2,
  }
);

solveWorker.on('completed', (job) => {
  console.log(`[Worker] Job ${job.id} completed`);
});

solveWorker.on('failed', (job, err) => {
  console.error(`[Worker] Job ${job?.id} failed:`, err.message);
});

console.log('[Worker] Solve worker started, waiting for jobs...');
