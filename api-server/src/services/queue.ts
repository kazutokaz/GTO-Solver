import { Queue, Job } from 'bullmq';
import { config } from '../config';

const redisConnection = {
  host: new URL(config.redis.url).hostname || 'localhost',
  port: parseInt(new URL(config.redis.url).port || '6379', 10),
};

// Solve job queue
export const solveQueue = new Queue('solve', {
  connection: redisConnection,
  defaultJobOptions: {
    attempts: 2,
    backoff: { type: 'exponential', delay: 5000 },
    removeOnComplete: { count: 1000 },
    removeOnFail: { count: 500 },
  },
});

// Aggregate analysis queue
export const aggregateQueue = new Queue('aggregate', {
  connection: redisConnection,
  defaultJobOptions: {
    attempts: 1,
    removeOnComplete: { count: 100 },
    removeOnFail: { count: 100 },
  },
});

export const redisConfig = redisConnection;

export interface SolveJobData {
  jobId: string;
  userId: string;
  input: any; // SolveInput JSON for CFR engine
}

export interface AggregateJobData {
  jobId: string;
  userId: string;
  input: any;
}

export async function addSolveJob(data: SolveJobData): Promise<Job> {
  return solveQueue.add('solve', data, {
    jobId: data.jobId,
  });
}

export async function addAggregateJob(data: AggregateJobData): Promise<Job> {
  return aggregateQueue.add('aggregate', data, {
    jobId: data.jobId,
  });
}
